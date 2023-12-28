use crate::{
    memory::{Permissions, VirtAddr},
    user::{
        handle::Handle,
        process::{self, Process},
        thread, Error,
    },
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

    let handle = process.handles().open_process(process.clone());

    handle_out.set(handle);
    Ok(())
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

    let mut handle_out = HandleOutputWriter::new(handle_out_ptr)?;

    let new_process = process::create()?;

    let handle = process.handles().open_process(new_process);

    handle_out.set(handle);
    Ok(())
}

pub fn mmap(
    process_handle: usize,
    addr_ptr: usize,
    size: usize,
    perms: usize,
    memory_object_handle: usize,
    offset: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    let memory_object = {
        let handle: Handle = memory_object_handle.into();
        if handle.valid() {
            Some(process.handles().get_memory_object(handle)?)
        } else {
            None
        }
    };

    let mut addr_access = process.vm_access_typed::<VirtAddr>(
        VirtAddr::new(addr_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let addr = target_process.mmap(
        *addr_access.get(),
        size,
        Permissions::from_bits_retain(perms as u64),
        memory_object,
        offset,
    )?;

    *addr_access.get_mut() = addr;
    Ok(())
}

pub fn munmap(
    process_handle: usize,
    addr: usize,
    size: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    target_process.munmap(VirtAddr::new(addr as u64), size)
}

pub fn mprotect(
    process_handle: usize,
    addr: usize,
    size: usize,
    perms: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    target_process.mprotect(
        VirtAddr::new(addr as u64),
        size,
        Permissions::from_bits_retain(perms as u64),
    )
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

    writer.fill(&process::list());

    Ok(())
}
