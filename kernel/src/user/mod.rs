mod error;
mod handle;
mod id_gen;
pub mod ipc;
mod memory_object;
pub mod process;
mod syscalls;
pub mod thread;
mod weak_map;

pub use error::Error;
pub use memory_object::MemoryObject;
pub use syscalls::execute_syscall;

pub fn init() {
    syscalls::init();
}
