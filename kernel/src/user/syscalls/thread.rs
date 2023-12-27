use crate::{
    memory::VirtAddr,
    user::{thread, Error},
};

use super::handle::HandleOutputWriter;

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
    entry_point: usize,
    stack_top: usize,
    handle_out_ptr: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(handle_out_ptr)?;

    let target_process = process.handles().get_process(process_handle.into())?;

    let new_thread = thread::create(
        target_process.clone(),
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
