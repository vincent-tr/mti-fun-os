use crate::{
    memory::{Permissions, VirtAddr},
    user::{
        Error,
        ioport::{PortAccess, PortRange},
    },
};

use super::{context::Context, helpers::HandleOutputWriter};

pub async fn open(context: Context) -> Result<(), Error> {
    let from = context.arg1();
    let count = context.arg2();
    let access = context.arg3();
    let handle_out_ptr = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let port_range = PortRange::new(from, count, PortAccess::from_bits_retain(access as u64))?;

    let handle = process.handles().open_port_range(port_range);

    handle_out.set(handle);
    Ok(())
}

pub async fn read(context: Context) -> Result<(), Error> {
    let port_range_handle = context.arg1();
    let index = context.arg2();
    let word_size = context.arg3();
    let value_ptr = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    let port_range = process.handles().get_port_range(port_range_handle.into())?;

    // must have a u64 ptr to write the value, even for smaller word sizes, to simplify the implementation
    let mut value_user_access = process.vm_access_typed::<u64>(
        VirtAddr::new(value_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let value = port_range.read(index as u16, word_size as u8)?;

    *value_user_access.get_mut() = value as u64;

    Ok(())
}

pub async fn write(context: Context) -> Result<(), Error> {
    let port_range_handle = context.arg1();
    let index = context.arg2();
    let word_size = context.arg3();
    let value = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    let port_range = process.handles().get_port_range(port_range_handle.into())?;

    port_range.write(index as u16, word_size as u8, value)?;

    Ok(())
}
