use alloc::sync::Arc;

use super::{ipc::PortSender, Error};

/// Represent a timer object
#[derive(Debug)]
pub struct Timer {}

impl Timer {
    /// Create a new timer
    pub fn new(port: Arc<PortSender>, id: u64) -> Result<Arc<Self>, Error> {
        Ok(Arc::new(Self {}))
    }

    pub fn arm(&self, deadline: u64) -> Result<(), Error> {
        Ok(())
    }

    pub fn cancel(&self) -> Result<(), Error> {
        Ok(())
    }
}
