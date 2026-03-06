use crate::user::{Error, irq::Irq};

use super::{context::Context, helpers::HandleOutputWriter};

pub async fn open(context: Context) -> Result<(), Error> {
    let port_handle = context.arg1();
    let handle_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let mut handle_out = HandleOutputWriter::new(&context, handle_out_ptr)?;

    let port = process.handles().get_port_sender(port_handle.into())?;

    let irq = Irq::new(port)?;

    let handle = process.handles().open_irq(irq);

    handle_out.set(handle);
    Ok(())
}
