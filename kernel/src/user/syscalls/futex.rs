use crate::{
    memory::VirtAddr,
    user::{Error, MemoryObject},
};

use super::{context::Context, helpers::HandleOutputWriter};
use syscalls::Permissions;

pub async fn wait(context: Context) -> Result<(), Error> {
    let uaddr_ptr = context.arg1();
    let expected = context.arg2() as u32;

    let thread = context.owner();
    let process = thread.process();

    let uaddr_access =
        process.vm_access_typed::<u32>(VirtAddr::new(uaddr_ptr as u64), Permissions::READ)?;

    // Note: no need to lock, syscalls are not preempted
    if *uaddr_access.get() != expected {
        return Err(Error::ObjectNotReady);
    }

    // TODO: wait

    Ok(())
}

pub async fn wake(context: Context) -> Result<(), Error> {
    let uaddr_ptr = context.arg1();
    let count_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let uaddr_access =
        process.vm_access_typed::<u32>(VirtAddr::new(uaddr_ptr as u64), Permissions::READ)?;

    let mut count_access = process.vm_access_typed::<usize>(
        VirtAddr::new(count_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let max_count = *count_access.get();

    // TODO: wake up to count waiters
    let woken_count = max_count;

    *count_access.get_mut() = woken_count;

    Ok(())
}
