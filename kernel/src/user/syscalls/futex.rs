use crate::{
    memory::VirtAddr,
    user::{futex, Error},
};

use super::context::Context;
use alloc::vec;
use syscalls::Permissions;

pub async fn wait(context: Context) -> Result<(), Error> {
    let uaddr_ptr = context.arg1();
    let expected = context.arg2() as u32;

    let thread = context.owner();
    let process = thread.process();

    let vuaddr = VirtAddr::new(uaddr_ptr as u64);

    let value = *process
        .vm_access_typed::<u32>(vuaddr, Permissions::READ)?
        .get();
    if value != expected {
        return Err(Error::ObjectNotReady);
    }

    let uaddr_info = process.minfo(vuaddr);
    let wait_queue = futex::get_waitqueue(uaddr_info);
    super::sleep(&context, vec![wait_queue]).await;

    // Note: we can have been woken by wake, but also because address has been unmapped.

    Ok(())
}

pub async fn wake(context: Context) -> Result<(), Error> {
    let uaddr_ptr = context.arg1();
    let count_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let vuaddr = VirtAddr::new(uaddr_ptr as u64);

    // Check access only
    process.vm_access_typed::<u32>(vuaddr, Permissions::READ)?;

    let uaddr_info = process.minfo(vuaddr);

    let mut count_access = process.vm_access_typed::<usize>(
        VirtAddr::new(count_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let max_count = *count_access.get();

    let woken_count = futex::wake(uaddr_info, max_count);

    *count_access.get_mut() = woken_count;

    Ok(())
}
