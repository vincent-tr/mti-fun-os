use core::mem;

use syscalls::ThreadPriority;

use crate::{
    memory::VirtAddr,
    user::{thread, Error},
};

use super::helpers::{HandleOutputWriter, ListOutputWriter};

pub fn open_self(
    handle_out_ptr: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(handle_out_ptr)?;

    let handle = process.handles().open_thread(thread.clone());

    handle_out.set(handle);
    Ok(())
}

pub fn create(
    process_handle: usize,
    priority: usize,
    entry_point: usize,
    stack_top: usize,
    handle_out_ptr: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(handle_out_ptr)?;

    let target_process = process.handles().get_process(process_handle.into())?;
    let priority: ThreadPriority = unsafe { mem::transmute(priority) };

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

pub fn exit(
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    // let process = thread.process();

    thread::thread_terminate(&thread);

    Ok(())
}

pub fn kill(
    thread_handle: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;

    thread::thread_terminate(&target_thread);

    Ok(())
}

pub fn set_priority(
    thread_handle: usize,
    priority: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let target_thread = process.handles().get_thread(thread_handle.into())?;
    let priority: ThreadPriority = unsafe { mem::transmute(priority) };

    thread::thread_set_priority(&target_thread, priority);

    Ok(())
}

/// count_ptr:
/// - on input -> element count in array
/// - on output -> real number of processes. Can be smaller or larger than array. If larger, the array is truncated
pub fn list(
    array_ptr: usize,
    count_ptr: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    //let thread = thread::current_thread();
    //let process = thread.process();

    let mut writer = ListOutputWriter::<u64>::new(array_ptr, count_ptr)?;

    writer.fill(&thread::list());

    Ok(())
}
