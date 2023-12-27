use crate::user::{thread, Error, MemoryObject};

use super::handle::HandleOutputWriter;

pub fn create(
    size: usize,
    handle_out_ptr: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(handle_out_ptr)?;

    let memory_object = MemoryObject::new(size)?;

    let handle = process.handles().open_memory_object(memory_object);

    handle_out.set(handle);
    Ok(())
}
