use core::cmp::min;

use alloc::{format, sync::Arc};
use syscalls::{AddressInfo, ProcessInfo};

use crate::{
    memory::{Permissions, VirtAddr},
    user::{
        error::{check_arg, check_found},
        handle::Handle,
        process, thread, Error,
    },
};

use super::{
    context::Context,
    helpers::{HandleOutputWriter, ListOutputWriter, StringReader},
};

pub async fn open_self(context: Context) -> Result<(), Error> {
    let handle_out_ptr = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let handle = process.handles().open_process(process.clone());

    handle_out.set(handle);
    Ok(())
}

pub async fn open(context: Context) -> Result<(), Error> {
    let pid = context.arg1();
    let handle_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let target_process = check_found(process::find(pid as u64))?;
    let handle = process.handles().open_process(target_process);

    handle_out.set(handle);
    Ok(())
}

pub async fn create(context: Context) -> Result<(), Error> {
    let name_ptr = context.arg1();
    let name_len = context.arg2();
    let handle_out_ptr = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;
    let name_reader = StringReader::new(&context, name_ptr, name_len)?;
    let name = name_reader.str()?;
    check_arg(name.len() > 0)?;

    let new_process = process::create(name)?;

    let handle = process.handles().open_process(new_process);

    handle_out.set(handle);
    Ok(())
}

pub async fn mmap(context: Context) -> Result<(), Error> {
    let process_handle = context.arg1();
    let addr_ptr = context.arg2();
    let size = context.arg3();
    let perms = context.arg4();
    let memory_object_handle = context.arg5();
    let offset = context.arg6();

    let thread = context.owner();
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

pub async fn munmap(context: Context) -> Result<(), Error> {
    let process_handle = context.arg1();
    let addr = context.arg2();
    let size = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    target_process.munmap(VirtAddr::new(addr as u64), size)
}

pub async fn mprotect(context: Context) -> Result<(), Error> {
    let process_handle = context.arg1();
    let addr = context.arg2();
    let size = context.arg3();
    let perms = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    target_process.mprotect(
        VirtAddr::new(addr as u64),
        size,
        Permissions::from_bits_retain(perms as u64),
    )
}

pub async fn minfo(context: Context) -> Result<(), Error> {
    let process_handle = context.arg1();
    let addr = context.arg2();
    let info_out_ptr = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    let mut info_user_access = process.vm_access_typed::<AddressInfo>(
        VirtAddr::new(info_out_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let info = target_process.minfo(VirtAddr::new(addr as u64));
    let handle = if let Some(memory_object) = info.mobj {
        process.handles().open_memory_object(memory_object)
    } else {
        Handle::invalid()
    };

    *info_user_access.get_mut() = AddressInfo {
        perms: info.perms,
        mobj: handle.as_u64(),
        offset: info.offset,
    };

    Ok(())
}

pub async fn exit(context: Context) -> Result<(), Error> {
    let thread = context.owner();
    let process = thread.process();

    // TODO: must be atomic (no thread must be created in the process while doing this)
    // Kill other threads
    for tid in process.threads() {
        if tid != thread.id() {
            let thread =
                crate::user::thread::find(tid).expect(&format!("Thread does not exist: {tid}"));

            if !thread.state().is_terminated() {
                thread::thread_terminate(&thread);
            }
        }
    }

    super::exit(&context).await
}

pub async fn kill(context: Context) -> Result<(), Error> {
    let process_handle = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    // Forbid to kill self
    check_arg(!Arc::ptr_eq(&process, &target_process))?;

    // TODO: must be atomic (no thread must be created in the process while doing this)
    for tid in target_process.threads() {
        let thread = crate::user::thread::find(tid).expect("Thread does not exist");

        if !thread.state().is_terminated() {
            thread::thread_terminate(&thread);
        }
    }

    Ok(())
}

pub async fn info(context: Context) -> Result<(), Error> {
    let process_handle = context.arg1();
    let info_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    let mut user_access = process.vm_access_typed::<ProcessInfo>(
        VirtAddr::new(info_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let info = &mut *user_access.get_mut();

    *info = ProcessInfo {
        pid: target_process.id(),
        name: [0; ProcessInfo::NAME_LEN],
        thread_count: target_process.thread_count(),
        mapping_count: target_process.mapping_count(),
        handle_count: target_process.handles().len(),
        terminated: target_process.terminated(),
    };

    let process_name = target_process.name();
    let src_name = process_name.as_bytes();
    let name_len = min(ProcessInfo::NAME_LEN, src_name.len());
    info.name[0..name_len].copy_from_slice(&src_name[0..name_len]);

    Ok(())
}

/// count_ptr:
/// - on input -> element count in array
/// - on output -> real number of processes. Can be smaller or larger than array. If larger, the array is truncated
pub async fn list(context: Context) -> Result<(), Error> {
    let array_ptr = context.arg1();
    let count_ptr = context.arg2();

    //let thread = context.owner();
    //let process = thread.process();

    let mut writer = ListOutputWriter::<u64>::new(&context, array_ptr, count_ptr)?;

    writer.fill(&process::list());

    Ok(())
}

pub async fn set_name(context: Context) -> Result<(), Error> {
    let process_handle = context.arg1();
    let name_ptr = context.arg2();
    let name_len = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    let name_reader = StringReader::new(&context, name_ptr, name_len)?;
    let name = name_reader.str()?;
    check_arg(name.len() > 0)?;

    target_process.set_name(name);

    Ok(())
}

pub async fn get_name(context: Context) -> Result<(), Error> {
    let process_handle = context.arg1();
    let name_ptr = context.arg2();
    let name_len = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let target_process = process.handles().get_process(process_handle.into())?;

    let mut writer = ListOutputWriter::<u8>::new(&context, name_ptr, name_len)?;

    let name = target_process.name();

    writer.fill(name.as_bytes());

    Ok(())
}
