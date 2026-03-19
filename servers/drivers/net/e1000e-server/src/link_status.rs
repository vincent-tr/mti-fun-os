use core::fmt;

use alloc::{boxed::Box, sync::Arc};

use crate::{device::DeviceData, registers};

/// Represents the link status of the device.
pub struct LinkStatus {
    dev_data: Arc<DeviceData>,
    is_up: bool,
    change: Box<dyn Fn(bool) + Send + Sync + 'static>,
}

impl fmt::Debug for LinkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinkStatus")
            .field("is_up", &self.is_up())
            .finish()
    }
}

impl LinkStatus {
    pub fn new(
        dev_data: Arc<DeviceData>,
        change_callback: impl Fn(bool) + Send + Sync + 'static,
    ) -> Self {
        let mut status = Self {
            dev_data,
            is_up: false,
            change: Box::new(change_callback),
        };

        status.is_up = status.read_status();

        status
    }

    pub fn handle_interrupt(&mut self) {
        let new_status = self.read_status();
        if new_status == self.is_up {
            return;
        }

        self.is_up = new_status;
        (self.change)(new_status);
    }

    pub fn is_up(&self) -> bool {
        self.is_up
    }

    fn read_status(&self) -> bool {
        let status: registers::Status = self.dev_data.mmio_read(registers::Status::OFFSET);
        status.link_up()
    }
}
