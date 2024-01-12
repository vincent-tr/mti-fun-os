use syscalls::MemoryStats;

use crate::{
    memory::{self, Permissions, VirtAddr},
    user::Error,
};

use super::context::Context;

pub async fn stats(context: Context) -> Result<(), Error> {
    let stats_ptr = context.arg1();

    let thread = context.owner();
    let process = thread.process();

    let mut user_access = process.vm_access_typed::<MemoryStats>(
        VirtAddr::new(stats_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    *user_access.get_mut() = memory::stats();

    Ok(())
}
