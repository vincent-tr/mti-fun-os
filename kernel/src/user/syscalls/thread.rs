use core::{cmp::min, mem};

use alloc::sync::Arc;
use syscalls::{
    Exception, Permissions, ThreadContext, ThreadContextRegister, ThreadCreationParameters,
    ThreadInfo, ThreadPriority, ThreadState,
};

use crate::{
    memory::VirtAddr,
    user::{
        error::{check_arg, check_found, check_is_userspace, invalid_argument},
        thread::{self, thread_resume},
        Error,
    },
};

use super::{
    context::Context,
    helpers::{HandleOutputWriter, ListOutputWriter, StringReader},
};

pub async fn open_self(context: Context) -> Result<(), Error> {
    let handle_out_ptr = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let handle = process.handles().open_thread(thread.clone());

    handle_out.set(handle);
    Ok(())
}

pub async fn open(context: Context) -> Result<(), Error> {
    let tid = context.arg1();
    let handle_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let target_thread = check_found(thread::find(tid as u64))?;
    let handle = process.handles().open_thread(target_thread.clone());

    handle_out.set(handle);
    Ok(())
}

pub async fn create(context: Context) -> Result<(), Error> {
    let name_ptr = context.arg1();
    let name_len = context.arg2();
    let params_ptr = context.arg3();
    let handle_out_ptr = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    // Need to keep reader because name is borrowed from it
    let name_reader = if name_ptr > 0 {
        Some(StringReader::new(&context, name_ptr, name_len)?)
    } else {
        None
    };

    let name = if let Some(name_reader) = &name_reader {
        let name = name_reader.str()?;
        check_arg(name.len() > 0)?;
        Some(name)
    } else {
        None
    };

    let params_access = process.vm_access_typed::<ThreadCreationParameters>(
        VirtAddr::new(params_ptr as u64),
        Permissions::READ,
    )?;

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let params = params_access.get();

    let target_process = process
        .handles()
        .get_process(params.process_handle.into())?;

    // Forbid to thread threads on terminated processes
    check_arg(!target_process.terminated())?;

    let new_thread = thread::create(
        name,
        target_process.clone(),
        params.privileged,
        params.priority,
        check_is_userspace(VirtAddr::new(params.entry_point as u64))?,
        check_is_userspace(VirtAddr::new(params.stack_top as u64))?,
        params.arg,
        check_is_userspace(VirtAddr::new(params.tls as u64))?,
    );

    let handle = process.handles().open_thread(new_thread);

    handle_out.set(handle);
    Ok(())
}

pub async fn exit(context: Context) -> Result<(), Error> {
    super::exit(&context).await
}

pub async fn kill(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    // Forbid to kill self
    check_arg(!Arc::ptr_eq(&thread, &target_thread))?;

    thread::thread_terminate(&target_thread);

    Ok(())
}

pub async fn set_priority(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();
    let priority = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;
    let priority: ThreadPriority = unsafe { mem::transmute(priority) };

    thread::thread_set_priority(&target_thread, priority);

    Ok(())
}

pub async fn info(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();
    let info_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    let mut user_access = process.vm_access_typed::<ThreadInfo>(
        VirtAddr::new(info_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    // Convert kernel state into syscall state
    let state = match *target_thread.state() {
        thread::ThreadState::Executing => ThreadState::Executing,
        thread::ThreadState::Ready => ThreadState::Ready,
        thread::ThreadState::Waiting(_) => ThreadState::Waiting,
        thread::ThreadState::Error(_) => ThreadState::Error,
        thread::ThreadState::Terminated => ThreadState::Terminated,
    };

    let info = &mut *user_access.get_mut();

    *info = ThreadInfo {
        tid: target_thread.id(),
        pid: target_thread.process().id(),
        name: [0; ThreadInfo::NAME_LEN],
        privileged: target_thread.privileged(),
        priority: target_thread.priority(),
        state,
        ticks: target_thread.ticks(),
    };

    let thread_name = target_thread.name();
    if let Some(thread_name) = &*thread_name {
        let src_name = thread_name.as_bytes();
        let name_len = min(ThreadInfo::NAME_LEN, src_name.len());
        info.name[0..name_len].copy_from_slice(&src_name[0..name_len]);
    }

    Ok(())
}

/// count_ptr:
/// - on input -> element count in array
/// - on output -> real number of processes. Can be smaller or larger than array. If larger, the array is truncated
pub async fn list(context: Context) -> Result<(), Error> {
    let array_ptr = context.arg1();
    let count_ptr = context.arg2();

    //let thread = context.owner();
    //let process = thread.process();

    let mut writer = ListOutputWriter::<u64>::new(&context, array_ptr, count_ptr)?;

    writer.fill(&thread::list());

    Ok(())
}

pub async fn set_name(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();
    let name_ptr = context.arg2();
    let name_len = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    let name_reader = StringReader::new(&context, name_ptr, name_len)?;
    let name = name_reader.str()?;

    let name = if name.len() > 0 { Some(name) } else { None };

    target_thread.set_name(name);

    Ok(())
}

pub async fn get_name(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();
    let name_ptr = context.arg2();
    let name_len = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    let mut writer = ListOutputWriter::<u8>::new(&context, name_ptr, name_len)?;

    let name = target_thread.name();

    if let Some(name) = &*name {
        writer.fill(name.as_bytes());
    } else {
        writer.fill(&[]);
    }

    Ok(())
}
pub async fn error_info(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();
    let info_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    let mut user_access = process.vm_access_typed::<Exception>(
        VirtAddr::new(info_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    if let Some(exception) = target_thread.state().is_error() {
        *user_access.get_mut() = exception;
    } else {
        return Err(invalid_argument());
    }

    Ok(())
}

pub async fn context(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();
    let info_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    let mut user_access = process.vm_access_typed::<ThreadContext>(
        VirtAddr::new(info_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    check_arg(target_thread.state().is_error().is_some())?;

    // TODO: not atomic with check
    target_thread.get_user_context(user_access.get_mut());

    Ok(())
}

pub async fn update_context(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();
    let regs_array_ptr = context.arg2();
    let regs_count = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    let regs_access = process.vm_access_typed_slice::<(ThreadContextRegister, usize)>(
        VirtAddr::new(regs_array_ptr as u64),
        regs_count,
        Permissions::READ,
    )?;

    check_arg(target_thread.state().is_error().is_some())?;

    // TODO: not atomic with check
    target_thread.update_user_context(regs_access.get())
}

pub async fn resume(context: Context) -> Result<(), Error> {
    let thread_handle = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    check_arg(target_thread.state().is_error().is_some())?;

    thread_resume(&target_thread);

    Ok(())
}
