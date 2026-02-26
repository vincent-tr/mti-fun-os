use core::sync::atomic::{AtomicBool, Ordering};

use crate::{
    error::ResultExt,
    loader::Loader,
    process::{
        ExitCode, Pid, Process, TerminationRegistration, find_live_process, find_process,
        get_termination_registration_by_handle, list_processes,
        list_termination_registrations_by_owner,
    },
    state::State,
};
use alloc::{collections::btree_map::BTreeMap, string::String, vec::Vec};
use lazy_static::lazy_static;
use libruntime::{
    ipc,
    kobject::{self, KObject},
    memory::AlignedBuffer,
    process::{
        self,
        iface::{
            KVBlock, ProcessInfo, ProcessServer, ProcessServerError, ProcessStatus, StartupInfo,
            SymBlock,
        },
    },
};
use log::{debug, error, info, warn};

/// The main server structure
#[derive(Debug)]
pub struct Server {
    initialized: AtomicBool,
    handles: ipc::HandleTable<'static, Process>,
    handle_generator: &'static ipc::HandleGenerator,
}

impl ProcessServer for Server {
    type Error = ProcessServerError;

    fn process_terminated(&self, pid: u64) {
        let pid = Pid::from(pid);

        let Some(info) = find_live_process(pid) else {
            warn!("Unknown process with PID {} terminated", pid);
            return;
        };

        info.mark_terminated();

        self.handles.process_terminated(pid.as_u64());

        for (target_pid, owner_handle) in list_termination_registrations_by_owner(pid) {
            let process = find_process(target_pid).expect("failed to get registration process");
            process.remove_termination_registration(owner_handle);
        }
    }

    fn bootstrap(
        &self,
        _sender_id: u64,
        init_binary: &[u8],
        process_server_binary: &[u8],
    ) -> Result<(), Self::Error> {
        // The server handle requests sequentially.
        if self.initialized.load(Ordering::SeqCst) {
            error!("Process server already initialized");
            return Err(ProcessServerError::InvalidArgument);
        }

        self.register_init(init_binary)?;
        self.register_idle()?;
        self.register_self(process_server_binary)?;

        // Now that we are initialized, we can run runtime process init, in a separate thread to avoid deadlock (process init will call process server)
        {
            let mut options = kobject::ThreadOptions::default();
            options.name("process-init");

            let entry = move || {
                // run process init
                process::SelfProcess::get();

                debug!("Process init completed");
            };

            kobject::Thread::start(entry, options).expect("failed to start process-init thread");
        }

        self.initialized.store(true, Ordering::SeqCst);

        debug!("Process server bootstrap completed");
        Ok(())
    }

    fn get_startup_info(&self, sender_id: u64) -> Result<StartupInfo, Self::Error> {
        self.check_initialized()?;
        let sender_id = Pid::from(sender_id);
        let info = find_live_process(sender_id).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        Ok(StartupInfo {
            name: String::from(info.name()),
            env: info.environment().clone(),
            args: info.arguments().clone(),
            symbols: info.symbols().clone(),
        })
    }

    fn update_name(&self, sender_id: u64, new_name: &str) -> Result<(), Self::Error> {
        self.check_initialized()?;
        let sender_id = Pid::from(sender_id);
        let info = find_live_process(sender_id).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        let new_name = String::from(new_name);

        info.kobject_process()
            .set_name(&new_name)
            .runtime_err("Failed to set process name")?;
        info.update_name(new_name);

        Ok(())
    }

    fn update_env(&self, sender_id: u64, new_env: KVBlock) -> Result<(), Self::Error> {
        self.check_initialized()?;
        let sender_id = Pid::from(sender_id);
        let info = find_live_process(sender_id).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        info.update_environment(new_env);

        Ok(())
    }

    fn set_exit_code(&self, sender_id: u64, exit_code: i32) -> Result<(), Self::Error> {
        self.check_initialized()?;
        let sender_id = Pid::from(sender_id);
        let info = find_live_process(sender_id).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        info.set_exit_code(ExitCode::try_from(exit_code).invalid_arg("Invalid exit code")?);

        Ok(())
    }

    fn create_process(
        &self,
        sender_id: u64,
        name: &str,
        binary: &[u8],
        environment: KVBlock,
        arguments: KVBlock,
    ) -> Result<(ipc::Handle, u64), Self::Error> {
        self.check_initialized()?;
        let sender_id = Pid::from(sender_id);

        info!("Creating process {}", name);

        let aligned_binary = AlignedBuffer::from_slice(binary, Loader::ALIGNMENT);
        let loader = Loader::new(aligned_binary.as_slice())?;

        let symbols = {
            let symbols = loader.get_symbols().unwrap_or_else(|e| {
                warn!("Failed to load symbols for process {}: {}", name, e);
                BTreeMap::new()
            });

            SymBlock::build(&symbols)
        };

        let process = kobject::Process::create(name).runtime_err("Failed to create process")?;

        let mappings = loader.map(&process)?;

        // Set up the process's main thread
        let stack_size = kobject::helpers::STACK_SIZE;
        let entry_point = loader.entry_point();
        let stack = kobject::helpers::AllocWithGuards::new_remote(stack_size, &process)
            .runtime_err("Failed to allocated stack")?;
        let tls =
            kobject::helpers::AllocWithGuards::new_remote(kobject::helpers::TLS_SIZE, &process)
                .runtime_err("Failed to allocated TLS block")?;

        let stack_top_addr = stack.address() + stack_size;
        let tls_addr = tls.address();

        debug!(
            "Creating main thread: entry_point={:#x}, stack_top={:#x}, tls={:#x}",
            entry_point as usize, stack_top_addr, tls_addr
        );

        // Use syscall directly to create remote thread
        let main_thread = {
            let thread_handle = libsyscalls::thread::create(
                Some("main"),
                unsafe { process.handle() },
                false,
                false,
                kobject::ThreadPriority::Normal,
                entry_point,
                stack_top_addr,
                0, // arg not used
                tls_addr,
            )
            .map_err(|e| kobject::Error::from(e))
            .runtime_err("Failed to create main thread")?;

            unsafe { kobject::Thread::from_handle_unchecked(thread_handle) }
        };

        // Process started, we can leak the allocations
        stack.leak();
        tls.leak();
        for mapping in mappings {
            mapping.leak();
        }

        let pid = process.pid();

        // Create associated Process
        let info = Process::new(
            sender_id,
            process,
            main_thread,
            String::from(name),
            environment,
            arguments,
            symbols,
        );

        let handle = self.handles.open(sender_id.as_u64(), info);

        Ok((handle, pid))
    }

    fn open_process(&self, sender_id: u64, pid: u64) -> Result<(ipc::Handle, u64), Self::Error> {
        self.check_initialized()?;
        let info = find_process(Pid::from(pid)).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        let handle = self.handles.open(sender_id, info);

        Ok((handle, pid))
    }

    fn close_process(&self, sender_id: u64, handle: ipc::Handle) -> Result<(), Self::Error> {
        self.check_initialized()?;
        self.handles.close(sender_id, handle).ok_or_else(|| {
            error!("Invalid process handle: {:?}", handle);
            ProcessServerError::InvalidArgument
        })?;

        Ok(())
    }

    fn get_process_name(&self, sender_id: u64, handle: ipc::Handle) -> Result<String, Self::Error> {
        self.check_initialized()?;
        let info = self.handles.read(sender_id, handle).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        Ok(String::from(info.name()))
    }

    fn get_process_env(&self, sender_id: u64, handle: ipc::Handle) -> Result<KVBlock, Self::Error> {
        self.check_initialized()?;
        let info = self.handles.read(sender_id, handle).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        Ok(info.environment().clone())
    }

    fn get_process_args(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
    ) -> Result<KVBlock, Self::Error> {
        self.check_initialized()?;
        let info = self.handles.read(sender_id, handle).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        Ok(info.arguments().clone())
    }

    fn get_process_status(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
    ) -> Result<ProcessStatus, Self::Error> {
        self.check_initialized()?;
        let info = self.handles.read(sender_id, handle).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        let status = if info.is_terminated() {
            ProcessStatus::Exited(info.exit_code().as_i32())
        } else {
            ProcessStatus::Running
        };

        Ok(status)
    }

    fn terminate_process(&self, sender_id: u64, handle: ipc::Handle) -> Result<(), Self::Error> {
        self.check_initialized()?;
        let info = self.handles.read(sender_id, handle).ok_or_else(|| {
            error!("Process not found: {}", sender_id);
            ProcessServerError::InvalidArgument
        })?;

        if info.is_terminated() {
            error!("Process already terminated: {:?}", handle);
            return Err(ProcessServerError::ProcessNotRunning);
        }

        info.kobject_process()
            .kill()
            .runtime_err("Could not kill process")?;

        info.set_exit_code(ExitCode::KILLED);
        // Note: we will get kernel notifications for the process exit, so we can just mark it as terminated when we receive the notification.
        // This allows the kernel to be the only source of truth for whether a process is alive or not

        Ok(())
    }

    fn list_processes(&self, _sender_id: u64) -> Result<Vec<ProcessInfo>, Self::Error> {
        self.check_initialized()?;
        let processes = list_processes()
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid().as_u64(),
                ppid: p.creator().as_u64(),

                name: p.name(),
                status: if p.is_terminated() {
                    ProcessStatus::Exited(p.exit_code().as_i32())
                } else {
                    ProcessStatus::Running
                },
            })
            .collect();

        Ok(processes)
    }

    fn register_process_terminated_notification(
        &self,
        sender_id: u64,
        handle: ipc::Handle,
        port: kobject::PortSender,
        correlation: u64,
    ) -> Result<ipc::Handle, Self::Error> {
        self.check_initialized()?;
        let sender_id = Pid::from(sender_id);
        let info = self
            .handles
            .read(sender_id.as_u64(), handle)
            .ok_or_else(|| {
                error!("Process not found: {}", sender_id);
                ProcessServerError::InvalidArgument
            })?;

        let owner_handle = self.handle_generator.generate();
        let registration =
            TerminationRegistration::new(sender_id, owner_handle, info.clone(), port, correlation);

        info.add_termination_registration(registration);

        Ok(owner_handle)
    }

    fn unregister_process_terminated_notification(
        &self,
        sender_id: u64,
        registration_handle: ipc::Handle,
    ) -> Result<(), Self::Error> {
        self.check_initialized()?;
        let sender_id = Pid::from(sender_id);
        let (target_pid, owner_handle) =
            get_termination_registration_by_handle(registration_handle).ok_or_else(|| {
                error!("Invalid registration handle: {:?}", registration_handle);
                ProcessServerError::InvalidArgument
            })?;

        let process =
            find_process(target_pid).expect("data inconsistency: registration process not found");

        // ensure the sender is the owner of the registration
        if process.get_registration_owner(owner_handle) != sender_id {
            error!(
                "Sender {} is not the owner of registration {:?} for process {}",
                sender_id, registration_handle, target_pid
            );
            return Err(ProcessServerError::InvalidArgument);
        }

        process.remove_termination_registration(owner_handle);

        Ok(())
    }
}

impl Server {
    pub fn new() -> Result<Self, kobject::Error> {
        let state = State::get();
        let handle_generator = state.handle_generator();
        let handles = ipc::HandleTable::new(handle_generator);

        let server = Self {
            initialized: AtomicBool::new(false),
            handles,
            handle_generator,
        };

        Ok(server)
    }

    fn check_initialized(&self) -> Result<(), ProcessServerError> {
        if !self.initialized.load(Ordering::SeqCst) {
            error!("Process server not initialized");
            return Err(ProcessServerError::RuntimeError);
        }
        Ok(())
    }

    const INIT_PID: u64 = 1;
    const IDLE_PID: u64 = 2;

    /// Register the init process in the system, so it shows up in process lists.
    fn register_init(&self, binary: &[u8]) -> Result<(), ProcessServerError> {
        // Note: this is fishy, we should really find the main thread differently
        const INIT_MAIN_THREAD_TID: u64 = 3;

        let process =
            kobject::Process::open(Self::INIT_PID).runtime_err("Failed to open init process")?;
        let main_thread = kobject::Thread::open(INIT_MAIN_THREAD_TID)
            .runtime_err("Failed to open init main thread")?;

        assert!(
            process.name().expect("failed to get process name") == "init",
            "PID 1 is expected to be init process"
        );

        assert!(
            main_thread.name().expect("failed to get thread name") == "main",
            "Init process's main thread is expected to be named 'main'"
        );

        let symbols = {
            let aligned_binary = AlignedBuffer::from_slice(binary, Loader::ALIGNMENT);
            let symbols = Loader::new(aligned_binary.as_slice())?
                .get_symbols()
                .unwrap_or_else(|e| {
                    warn!("Failed to load symbols for process init: {}", e);
                    BTreeMap::new()
                });

            SymBlock::build(&symbols)
        };

        Process::new(
            Pid::INVALID,
            process,
            main_thread,
            String::from("init"),
            Self::get_empty_kvblock(),
            Self::get_empty_kvblock(),
            symbols,
        );

        Ok(())
    }

    /// Register the idle process in the system, so it shows up in process lists.
    fn register_idle(&self) -> Result<(), ProcessServerError> {
        // Note: this is fishy, we should really find the idle thread differently
        const IDLE_MAIN_THREAD_TID: u64 = 4;

        let process =
            kobject::Process::open(Self::IDLE_PID).runtime_err("Failed to open idle process")?;
        let main_thread = kobject::Thread::open(IDLE_MAIN_THREAD_TID)
            .runtime_err("Failed to open idle main thread")?;

        assert!(
            process.name().expect("failed to get process name") == "idle",
            "PID 2 is expected to be idle process"
        );

        assert!(
            main_thread.name().expect("failed to get thread name") == "idle",
            "Idle process's main thread is expected to be named 'idle'"
        );

        // No symbols for idle process, since it's not a real process and doesn't have a real binary
        let symbols = SymBlock::build(&BTreeMap::new());

        Process::new(
            Pid::from(Self::INIT_PID),
            process,
            main_thread,
            String::from("idle"),
            Self::get_empty_kvblock(),
            Self::get_empty_kvblock(),
            symbols,
        );

        Ok(())
    }

    /// Register the process-server itself in the system
    fn register_self(&self, binary: &[u8]) -> Result<(), ProcessServerError> {
        let process = kobject::Process::current().clone();
        let main_thread =
            kobject::Thread::open_self().runtime_err("Failed to open self main thread")?;

        let symbols = {
            let aligned_binary = AlignedBuffer::from_slice(binary, Loader::ALIGNMENT);
            let symbols = Loader::new(aligned_binary.as_slice())?
                .get_symbols()
                .unwrap_or_else(|e| {
                    warn!("Failed to load symbols for process process-server: {}", e);
                    BTreeMap::new()
                });

            SymBlock::build(&symbols)
        };

        Process::new(
            Pid::from(Self::INIT_PID),
            process,
            main_thread,
            String::from("process-server"),
            Self::get_empty_kvblock(),
            Self::get_empty_kvblock(),
            symbols,
        );

        Ok(())
    }

    fn get_empty_kvblock() -> KVBlock {
        // Since kvblocks are immutable, we can cache an empty one
        lazy_static! {
            static ref EMPTY_KVBLOCK: kobject::MemoryObject = KVBlock::build(&[]);
        }

        KVBlock::from_memory_object(EMPTY_KVBLOCK.clone()).expect("Failed to create KVBlock")
    }
}
