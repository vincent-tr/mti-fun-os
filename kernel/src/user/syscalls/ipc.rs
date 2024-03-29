use core::cmp::min;

use alloc::vec::Vec;
use bit_field::BitArray;
use hashbrown::HashMap;
use syscalls::{Message, PortInfo, ProcessInfo};

use crate::{
    memory::{align_up, Permissions, VirtAddr},
    user::{
        error::{check_arg, check_found},
        handle::Handle,
        ipc, Error,
    },
};

use super::{
    context::Context,
    helpers::{HandleOutputWriter, ListOutputWriter, StringReader},
};

// set one of id or name
pub async fn open(context: Context) -> Result<(), Error> {
    let id = context.arg1();
    let name_ptr = context.arg2();
    let name_len = context.arg3();
    let handle_out_ptr = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let is_id = id != 0;
    let is_name = name_ptr != 0 || name_len != 0;
    check_arg((is_id || is_name) && !(is_id && is_name))?;

    let target_port = check_found(if is_id {
        ipc::find_by_id(id as u64)
    } else {
        let name_reader = StringReader::new(&context, name_ptr, name_len)?;
        let name = name_reader.str()?;
        ipc::find_by_name(name)
    })?;

    let handle = process.handles().open_port_sender(target_port);

    handle_out.set(handle);
    Ok(())
}

pub async fn create(context: Context) -> Result<(), Error> {
    let name_ptr = context.arg1();
    let name_len = context.arg2();
    let handle_receiver_out_ptr = context.arg3();
    let handle_sender_out_ptr = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_receiver_out = HandleOutputWriter::new(&context, handle_receiver_out_ptr)?;
    let mut handle_sender_out = HandleOutputWriter::new(&context, handle_sender_out_ptr)?;
    let name_reader = StringReader::new(&context, name_ptr, name_len)?;
    let name = name_reader.str()?;

    let name = if name.len() > 0 { Some(name) } else { None };

    let (receiver, sender) = ipc::create(name)?;

    let receiver_handle = process.handles().open_port_receiver(receiver);
    let sender_handle = process.handles().open_port_sender(sender);

    handle_receiver_out.set(receiver_handle);
    handle_sender_out.set(sender_handle);
    Ok(())
}

pub async fn send(context: Context) -> Result<(), Error> {
    let port_handle = context.arg1();
    let message_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let target_port_sender = process.handles().get_port_sender(port_handle.into())?;

    let user_message =
        process.vm_access_typed::<Message>(VirtAddr::new(message_ptr as u64), Permissions::READ)?;

    let message = user_message.get().clone();

    target_port_sender.send(process, message)
}

pub async fn receive(context: Context) -> Result<(), Error> {
    let port_handle = context.arg1();
    let message_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let target_port_receiver = process.handles().get_port_receiver(port_handle.into())?;

    let mut user_message = process.vm_access_typed::<Message>(
        VirtAddr::new(message_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let message = target_port_receiver.receive(process)?;

    *user_message.get_mut() = message;

    Ok(())
}

pub async fn wait(context: Context) -> Result<(), Error> {
    let port_handle_array_ptr = context.arg1();
    let ready_bit_array_ptr = context.arg2();
    let port_count = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let port_handle_array_access = process.vm_access_typed_slice::<Handle>(
        VirtAddr::new(port_handle_array_ptr as u64),
        port_count,
        Permissions::READ,
    )?;

    let mut ready_bit_array_access = process.vm_access_typed_slice::<u8>(
        VirtAddr::new(ready_bit_array_ptr as u64),
        align_up(port_count as u64, u8::BITS as u64) as usize / u8::BITS as usize,
        Permissions::READ | Permissions::WRITE,
    )?;

    let ready_bits = ready_bit_array_access.get_mut();
    ready_bits.fill(0);

    let mut queues = Vec::new();
    let mut queue_map = HashMap::new();
    let mut is_sync = false;

    queues.reserve(port_count);

    for (index, &handle) in port_handle_array_access.get().iter().enumerate() {
        let port = process.handles().get_port_receiver(handle)?;
        if let Some(queue) = port.prepare_wait() {
            queues.push(queue.clone());
            queue_map.insert(queue.as_ref() as *const _, index);
        } else {
            ready_bits.set_bit(index, true);
            is_sync = true;
        }
    }

    if is_sync {
        return Ok(());
    }

    let woken_queue = super::sleep(&context, queues).await;
    let index = *queue_map
        .get(&(woken_queue.as_ref() as *const _))
        .expect("woken queue not found");

    ready_bits.set_bit(index, true);

    Ok(())
}

pub async fn info(context: Context) -> Result<(), Error> {
    let port_handle = context.arg1();
    let info_ptr = context.arg2();

    let thread = context.owner();
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

    let src_name = target_port.name().unwrap_or("").as_bytes();
    let name_len = min(ProcessInfo::NAME_LEN, src_name.len());
    info.name[0..name_len].copy_from_slice(&src_name[0..name_len]);

    Ok(())
}

/// count_ptr:
/// - on input -> element count in array
/// - on output -> real number of ports. Can be smaller or larger than array. If larger, the array is truncated
pub async fn list(context: Context) -> Result<(), Error> {
    let array_ptr = context.arg1();
    let count_ptr = context.arg2();

    //let thread = context.owner();
    //let process = thread.process();

    let mut writer = ListOutputWriter::<u64>::new(&context, array_ptr, count_ptr)?;

    writer.fill(&ipc::list());

    Ok(())
}
