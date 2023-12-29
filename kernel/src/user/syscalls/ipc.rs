use core::cmp::min;

use syscalls::{PortInfo, ProcessInfo};

use crate::{
    memory::{Permissions, VirtAddr},
    user::{
        error::{check_arg, check_found},
        ipc, thread, Error,
    },
};

use super::helpers::{HandleOutputWriter, ListOutputWriter, StringReader};

// set one of id or name
pub fn open(
    id: usize,
    name_ptr: usize,
    name_len: usize,
    handle_out_ptr: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(handle_out_ptr)?;

    let is_id = id != 0;
    let is_name = name_ptr != 0 || name_len != 0;
    check_arg((is_id || is_name) && !(is_id && is_name))?;

    let target_port = check_found(if is_id {
        ipc::find_by_id(id as u64)
    } else {
        let name_reader = StringReader::new(name_ptr, name_len)?;
        let name = name_reader.str()?;
        ipc::find_by_name(name)
    })?;

    let handle = process.handles().open_port_sender(target_port);

    handle_out.set(handle);
    Ok(())
}

pub fn create(
    name_ptr: usize,
    name_len: usize,
    handle_receiver_out_ptr: usize,
    handle_sender_out_ptr: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let mut handle_receiver_out = HandleOutputWriter::new(handle_receiver_out_ptr)?;
    let mut handle_sender_out = HandleOutputWriter::new(handle_sender_out_ptr)?;
    let name_reader = StringReader::new(name_ptr, name_len)?;
    let name = name_reader.str()?;

    let (receiver, sender) = ipc::create(name)?;

    let receiver_handle = process.handles().open_port_receiver(receiver);
    let sender_handle = process.handles().open_port_sender(sender);

    handle_receiver_out.set(receiver_handle);
    handle_sender_out.set(sender_handle);
    Ok(())
}

pub fn info(
    port_handle: usize,
    info_ptr: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let target_port = process.handles().get_port(port_handle.into())?;

    let mut user_access = process.vm_access_typed::<PortInfo>(
        VirtAddr::new(info_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let info = &mut *user_access.get_mut();

    *info = PortInfo {
        id: target_port.id(),
        name: [0; PortInfo::NAME_LEN],
        closed: target_port.closed(),
        message_queue_count: target_port.message_queue_count(),
        waiting_receiver_count: target_port.waiting_receiver_count(),
    };

    let src_name = target_port.name().as_bytes();
    let name_len = min(ProcessInfo::NAME_LEN, src_name.len());
    info.name[0..name_len].copy_from_slice(&src_name[0..name_len]);

    Ok(())
}

/// count_ptr:
/// - on input -> element count in array
/// - on output -> real number of ports. Can be smaller or larger than array. If larger, the array is truncated
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

    writer.fill(&ipc::list());

    Ok(())
}
