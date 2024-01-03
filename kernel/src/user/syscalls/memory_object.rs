use crate::user::{Error, MemoryObject};

use super::{context::Context, helpers::HandleOutputWriter};

pub async fn create(context: Context) -> Result<(), Error> {
    let size = context.arg1();
    let handle_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let memory_object = MemoryObject::new(size)?;

    let handle = process.handles().open_memory_object(memory_object);

    handle_out.set(handle);
    Ok(())
}
