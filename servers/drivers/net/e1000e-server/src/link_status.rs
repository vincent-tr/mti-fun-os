use core::fmt;

use alloc::boxed::Box;

/// Represents the link status of the device.
pub struct LinkStatus {
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
    pub fn new(change_callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        Self {
            is_up: false,
            change: Box::new(change_callback),
        }
    }

    pub fn update(&mut self, new_status: bool) {
        let old_status = self.is_up;
        self.is_up = new_status;
        if old_status != new_status {
            (self.change)(new_status);
        }
    }

    pub fn is_up(&self) -> bool {
        self.is_up
    }
}
