use crate::user::{process, thread, Error};

use super::handle;

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

    let handle = process.handles().open_process(process.clone());

    handle::as_output(handle_out_ptr, handle)
}

pub fn create(
    handle_out_ptr: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let new_process = process::create()?;

    let handle = process.handles().open_process(new_process);

    handle::as_output(handle_out_ptr, handle)
}
