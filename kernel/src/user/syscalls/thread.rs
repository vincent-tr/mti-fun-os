use core::mem;

use alloc::sync::Arc;
use syscalls::{Permissions, ThreadInfo, ThreadPriority, ThreadState};

use crate::{
    memory::VirtAddr,
    user::{
        error::{check_arg, check_found},
        thread, Error,
    },
};

use super::{
    context::Context,
    helpers::{HandleOutputWriter, ListOutputWriter},
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
    let process_handle = context.arg1();
    let priority = context.arg2();
    let entry_point = context.arg3();
    let stack_top = context.arg4();
    let handle_out_ptr = context.arg5();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let target_process = process.handles().get_process(process_handle.into())?;
    let priority: ThreadPriority = unsafe { mem::transmute(priority) };

    // Forbid to thread threads on terminated processes
    check_arg(!target_process.terminated())?;

    let new_thread = thread::create(
        target_process.clone(),
        priority,
        VirtAddr::new(entry_point as u64),
        VirtAddr::new(stack_top as u64),
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

    *user_access.get_mut() = ThreadInfo {
        tid: target_thread.id(),
        pid: target_thread.process().id(),
        priority: target_thread.priority(),
        state,
        ticks: target_thread.ticks(),
    };

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
