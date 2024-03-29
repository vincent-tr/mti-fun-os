use syscalls::{Error, Permissions};

use crate::{
    memory::VirtAddr,
    user::listener::{ProcessListener, ThreadListener},
};

use super::{context::Context, helpers::HandleOutputWriter};

pub async fn create_process(context: Context) -> Result<(), Error> {
    let port_handle = context.arg1();
    let pid_list_ptr = context.arg2();
    let pid_list_size = context.arg3();
    let handle_out_ptr = context.arg4();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let port = process.handles().get_port_sender(port_handle.into())?;

    let pid_list_access = if pid_list_size == 0 {
        None
    } else {
        Some(process.vm_access_typed_slice::<u64>(
            VirtAddr::new(pid_list_ptr as u64),
            pid_list_size,
            Permissions::READ,
        )?)
    };

    let pids = if let Some(access) = &pid_list_access {
        Some(access.get())
    } else {
        None
    };

    let process_listener = ProcessListener::new(port, pids);

    let handle = process.handles().open_process_listener(process_listener);

    handle_out.set(handle);
    Ok(())
}

pub async fn create_thread(context: Context) -> Result<(), Error> {
    let port_handle = context.arg1();
    let id_list_ptr = context.arg2();
    let id_list_size = context.arg3();
    let is_pids = context.arg4() > 0;
    let handle_out_ptr = context.arg5();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let port = process.handles().get_port_sender(port_handle.into())?;

    let id_list_access = if id_list_size == 0 {
        None
    } else {
        Some(process.vm_access_typed_slice::<u64>(
            VirtAddr::new(id_list_ptr as u64),
            id_list_size,
            Permissions::READ,
        )?)
    };

    let ids = if let Some(access) = &id_list_access {
        Some(access.get())
    } else {
        None
    };

    let thread_listener = ThreadListener::new(port, ids, is_pids);

    let handle = process.handles().open_thread_listener(thread_listener);

    handle_out.set(handle);
    Ok(())
}
