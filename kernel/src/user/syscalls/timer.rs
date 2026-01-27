use syscalls::Error;

use crate::{
    memory::VirtAddr,
    user::timer::{now as timer_now, Timer},
};

use super::{context::Context, helpers::HandleOutputWriter};
use syscalls::Permissions;

pub async fn create(context: Context) -> Result<(), Error> {
    let port_handle = context.arg1();
    let id = context.arg2();
    let handle_out_ptr = context.arg3();

    let thread = context.owner();
    let process = thread.process();
    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;
    let port = process.handles().get_port_sender(port_handle.into())?;

    let timer = Timer::new(port, id as u64)?;

    let handle = process.handles().open_timer(timer);

    handle_out.set(handle);
    Ok(())
}

pub async fn arm(context: Context) -> Result<(), Error> {
    let handle = context.arg1();
    let deadline = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let timer = process.handles().get_timer(handle.into())?;

    timer.arm(deadline as u64);

    Ok(())
}

pub async fn cancel(context: Context) -> Result<(), Error> {
    let handle = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    let timer = process.handles().get_timer(handle.into())?;

    timer.cancel();

    Ok(())
}

pub async fn now(context: Context) -> Result<(), Error> {
    let now_out_ptr = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    let mut user_access = process.vm_access_typed::<u64>(
        VirtAddr::new(now_out_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    *user_access.get_mut() = timer_now();

    Ok(())
}
