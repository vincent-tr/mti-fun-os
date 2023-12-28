use syscalls::HandleType;

use crate::{
    memory::{Permissions, VirtAddr},
    user::{thread, Error},
};

use super::helpers::HandleOutputWriter;

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

pub fn r#type(
    handle: usize,
    type_out_ptr: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let mut user_access = process.vm_access_typed::<HandleType>(
        VirtAddr::new(type_out_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let handle_type = process.handles().r#type(handle.into())?;

    *user_access.get_mut() = handle_type;
    Ok(())
}
