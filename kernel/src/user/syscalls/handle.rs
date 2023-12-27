use core::mem::size_of;

use crate::{
    memory::{Permissions, VirtAddr},
    user::{handle::Handle, process::MemoryAccess, thread, Error},
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

pub fn duplicate(
    handle: usize,
    handle_out_ptr: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(handle_out_ptr)?;

    let new_handle = process.handles().duplicate(handle.into())?;

    handle_out.set(new_handle);
    Ok(())
}

/// Helper object to operate output in 2 steps:
/// - fallible step: prepare pointer view
/// - infallible step: write output pointer
pub struct HandleOutputWriter {
    user_access: MemoryAccess,
}

impl HandleOutputWriter {
    // Create a new writer
    pub fn new(handle_out_ptr: usize) -> Result<Self, Error> {
        let thread = thread::current_thread();
        let process = thread.process();

        let handle_out_addr = VirtAddr::new(handle_out_ptr as u64);
        let user_access = process.vm_access(
            handle_out_addr..handle_out_addr + size_of::<Handle>(),
            Permissions::READ | Permissions::WRITE,
        )?;

        Ok(Self { user_access })
    }

    /// Write the handle output value
    pub fn set(&mut self, handle: Handle) {
        *self.user_access.get_mut::<Handle>() = handle;
    }
}
