use syscalls::{Message, PortInfo, SyscallNumber};

use super::{
    ref_ptr, syscalls::*, sysret_to_result, Handle, SyscallInStr, SyscallList, SyscallOutPtr,
    SyscallResult,
};

pub enum NameOrId<'a> {
    Id(u64),
    Name(&'a str),
}

pub fn open(name_or_id: NameOrId) -> SyscallResult<Handle> {
    let mut new_handle = Handle::invalid();

    let mut arg_id: usize = 0;
    let mut arg_name_ptr: usize = 0;
    let mut arg_name_len: usize = 0;

    match name_or_id {
        NameOrId::Id(id) => {
            arg_id = id as usize;
        }
        NameOrId::Name(name) => {
            let name_reader = SyscallInStr::new(name);
            unsafe {
                arg_name_ptr = name_reader.ptr_arg();
                arg_name_len = name_reader.len_arg();
            }
        }
    }

    let ret = unsafe {
        syscall4(
            SyscallNumber::PortOpen,
            arg_id,
            arg_name_ptr,
            arg_name_len,
            new_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok(new_handle)
}

// return (receiver, sender)
pub fn create(name: &str) -> SyscallResult<(Handle, Handle)> {
    let mut new_receiver_handle = Handle::invalid();
    let mut new_sender_handle = Handle::invalid();
    let name_reader = SyscallInStr::new(name);

    let ret = unsafe {
        syscall4(
            SyscallNumber::PortCreate,
            name_reader.ptr_arg(),
            name_reader.len_arg(),
            new_receiver_handle.as_syscall_ptr(),
            new_sender_handle.as_syscall_ptr(),
        )
    };

    sysret_to_result(ret)?;

    Ok((new_receiver_handle, new_sender_handle))
}

/// Send a message to a port
pub fn send(port: &Handle, msg: &Message) -> SyscallResult<()> {
    let ret = unsafe {
        syscall2(
            SyscallNumber::PortSend,
            port.as_syscall_value(),
            ref_ptr(msg),
        )
    };

    sysret_to_result(ret)?;

    Ok(())
}

/// Receive a message from a port
pub fn receive(port: &Handle) -> SyscallResult<Message> {
    let msg = SyscallOutPtr::new();

    let ret = unsafe {
        syscall2(
            SyscallNumber::PortReceive,
            port.as_syscall_value(),
            msg.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(msg.take())
}

/// Wait for a port to be ready to receive a message
///
/// Note: `ready_buffer` must be at least `align_up(ports.len() / 8, 8)` size
pub fn wait(ports: &[&Handle], ready_buffer: &mut [u8]) -> SyscallResult<()> {
    assert!(ports.len() <= ready_buffer.len() * 8);
    let size = ports.len();

    let ret = unsafe {
        syscall3(
            SyscallNumber::PortWait,
            ports.as_ptr() as usize,
            ready_buffer.as_ptr() as usize,
            size,
        )
    };

    sysret_to_result(ret)?;

    Ok(())
}

/// Get info about the port (can use sender or receiver)
pub fn info(port: &Handle) -> SyscallResult<PortInfo> {
    let info = SyscallOutPtr::new();

    let ret = unsafe {
        syscall2(
            SyscallNumber::PortInfo,
            port.as_syscall_value(),
            info.ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(info.take())
}

/// Get list of port ids living in the system
pub fn list<'a>(array: &'a mut [u64]) -> SyscallResult<(&'a [u64], usize)> {
    let mut list = unsafe { SyscallList::new(array) };

    let ret = unsafe {
        syscall2(
            SyscallNumber::PortList,
            list.array_ptr_arg(),
            list.count_ptr_arg(),
        )
    };

    sysret_to_result(ret)?;

    Ok(list.finalize())
}
