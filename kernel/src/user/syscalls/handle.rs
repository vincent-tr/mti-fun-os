use core::mem::size_of;

use crate::{
    memory::{Permissions, VirtAddr},
    user::{handle::Handle, thread, Error},
};

pub fn close(
    handle: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    process.handles().close(handle.into())
}

pub fn as_output(handle_out_ptr: usize, handle: Handle) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let handle_out_addr = VirtAddr::new(handle_out_ptr as u64);
    let mut user_access = process.vm_access(
        handle_out_addr..handle_out_addr + size_of::<Handle>(),
        Permissions::READ | Permissions::WRITE,
    )?;

    *user_access.get_mut::<Handle>() = handle;

    Ok(())
}
