use crate::{
    memory::VirtAddr,
    user::{Error, MemoryObject},
};

use super::{context::Context, helpers::HandleOutputWriter};
use syscalls::{IoMemFlags, Permissions};
use x86_64::PhysAddr;

pub async fn create(context: Context) -> Result<(), Error> {
    let size = context.arg1();
    let handle_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let memory_object = MemoryObject::new(size)?;

    let handle = process.handles().open_memory_object(memory_object);

    handle_out.set(handle);
    Ok(())
}

pub async fn open_iomem(context: Context) -> Result<(), Error> {
    let phys_addr = context.arg1();
    let size = context.arg2();
    let flags = context.arg3();
    let handle_out_ptr = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    let phys_addr = PhysAddr::try_new(phys_addr as u64).map_err(|_| Error::InvalidArgument)?;
    let flags = IoMemFlags::from_bits_retain(flags as u64);

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let memory_object = MemoryObject::new_iomem(
        phys_addr,
        size,
        flags.contains(IoMemFlags::WRITE_THROUGH),
        flags.contains(IoMemFlags::NO_CACHE),
    )?;

    let handle = process.handles().open_memory_object(memory_object);

    handle_out.set(handle);
    Ok(())
}

pub async fn size(context: Context) -> Result<(), Error> {
    let handle = context.arg1();
    let size_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let memory_object = process.handles().get_memory_object(handle.into())?;

    let mut user_access = process.vm_access_typed::<usize>(
        VirtAddr::new(size_out_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    *user_access.get_mut() = memory_object.size();

    Ok(())
}
