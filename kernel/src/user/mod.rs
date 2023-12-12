mod error;
mod id_gen;
mod memory_object;
pub mod process;
mod syscalls;

pub use error::Error;
use log::info;
pub use memory_object::MemoryObject;
pub use syscalls::execute_syscall;

use self::syscalls::register_syscall;

pub fn init() {
    register_syscall(1, syscall_noop);
    register_syscall(2, syscall_panic);
}

fn syscall_noop(
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    info!("syscall noop");

    Ok(())
}

fn syscall_panic(
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    panic!("syscall panic");
}
