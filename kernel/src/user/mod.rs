mod id_gen;
mod weak_map;
mod error;
mod memory_object;
pub mod process;
pub mod thread;
mod handle;
mod syscalls;

pub use error::Error;
pub use memory_object::MemoryObject;
pub use syscalls::execute_syscall;

pub fn init() {
    syscalls::init();
}