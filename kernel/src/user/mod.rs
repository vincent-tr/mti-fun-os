mod error;
mod id_gen;
mod memory_object;
pub mod process;
mod syscalls;

use crate::{memory::{Permissions, VirtAddr}, user::error::{check_arg_res, invalid_argument}};
use alloc::{sync::Arc, str};
pub use error::Error;
use log::{info, Level};
pub use memory_object::MemoryObject;
use spin::Mutex;
pub use syscalls::execute_syscall;

use self::{process::Process, syscalls::register_syscall};

// TODO: share with userland
const SYSCALL_NOOP: usize = 1;
const SYSCALL_PANIC: usize = 2;
const SYSCALL_KLOG: usize = 3;

pub fn init() {
    register_syscall(SYSCALL_NOOP, syscall_noop);
    register_syscall(SYSCALL_PANIC, syscall_panic);
    register_syscall(SYSCALL_KLOG, syscall_klog);
}

// TODO: properly manage current thread/process

static mut TEMP_PROCESS: Mutex<Option<Arc<Process>>> = Mutex::new(Option::None);

pub fn temp_set_process(process: Arc<Process>) {
    let mut gprocess = unsafe { TEMP_PROCESS.lock() };

    *gprocess = Some(process);
}

fn current_process() -> Arc<Process> {
    let gprocess = unsafe { TEMP_PROCESS.lock() };
    gprocess.as_ref().expect("no current process").clone()
}

fn syscall_noop(
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    info!("syscall noop");

    Ok(())
}

fn syscall_panic(
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    panic!("syscall panic");
}

fn syscall_klog(
    level: usize,
    str_ptr: usize,
    len: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let process = current_process();

    let process_range = VirtAddr::new(str_ptr as u64)..VirtAddr::new((str_ptr + len) as u64);
    let access = process.vm_access(process_range, Permissions::READ)?;
    
    let pid = process.id();
    let level = parse_log_level(level)?;
    let message = check_arg_res(str::from_utf8(access.get_slice::<u8>()))?;

    log::log!(level, "From process {pid}: {message}");

    Ok(())
}

fn parse_log_level(level: usize) -> Result<Level, Error> {
    const ERROR_USIZE: usize = Level::Error as usize;
    const WARN_USIZE: usize = Level::Warn as usize;
    const INFO_USIZE: usize = Level::Info as usize;
    const DEBUG_USIZE: usize = Level::Debug as usize;
    const TRACE_USIZE: usize = Level::Trace as usize;
    match level {
        ERROR_USIZE => Ok(Level::Error),
        WARN_USIZE =>  Ok(Level::Warn),
        INFO_USIZE =>  Ok(Level::Info),
        DEBUG_USIZE =>  Ok(Level::Debug),
        TRACE_USIZE =>  Ok(Level::Trace),
        _ => Err(invalid_argument())
    }
}