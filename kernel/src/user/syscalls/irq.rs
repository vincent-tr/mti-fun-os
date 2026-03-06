use syscalls::{IrqInfo, Permissions};
use x86_64::VirtAddr;

use crate::{
    devices::local_apic,
    user::{Error, irq::Irq},
};

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

pub async fn info(context: Context) -> Result<(), Error> {
    let irq_handle = context.arg1();
    let info_out_ptr = context.arg2();

    let thread = context.owner();
    let process = thread.process();

    let mut info_user_access = process.vm_access_typed::<IrqInfo>(
        VirtAddr::new(info_out_ptr as u64),
        Permissions::READ | Permissions::WRITE,
    )?;

    let irq = process.handles().get_irq(irq_handle.into())?;

    let info = info_user_access.get_mut();
    info.msi_address = local_apic::get_msi_address().as_u64();
    info.vector = irq.vector() as u8;

    Ok(())
}
