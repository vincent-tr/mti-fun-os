mod error;
mod process;
mod memory_object;
mod id_gen;

pub use error::Error;
pub use memory_object::MemoryObject;
pub use process::{PROCESSES, Processes, Process};