use syscalls::HandleType;

use crate::{
    memory::{Permissions, VirtAddr},
    user::Error,
};

use super::{context::Context, helpers::HandleOutputWriter};

pub async fn close(context: Context) -> Result<(), Error> {
    let handle = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    process.handles().close(handle.into())
}

pub async fn duplicate(context: Context) -> Result<(), Error> {
    let handle = context.arg1();
    let handle_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let new_handle = process.handles().duplicate(handle.into())?;

    handle_out.set(new_handle);
    Ok(())
}

pub async fn r#type(context: Context) -> Result<(), Error> {
    let handle = context.arg1();
    let type_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let mut user_access = process.vm_access_typed::<HandleType>(
        VirtAddr::new(type_out_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let handle_type = process.handles().r#type(handle.into())?;

    *user_access.get_mut() = handle_type;
    Ok(())
}
