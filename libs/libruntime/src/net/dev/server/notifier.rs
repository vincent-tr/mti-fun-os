use log::{debug, error};

use crate::{ipc, kobject, net::dev::iface, sync::RwLock};

/// Notifier for network device events (e.g., link up/down).
#[derive(Debug)]
pub struct Notifier(RwLock<Option<NotifierData>>);

#[derive(Debug)]
struct NotifierData {
    correlation: u64,
    port: kobject::PortSender,
}

impl Notifier {
    /// Creates a new notifier instance.
    pub fn new() -> Self {
        Self(RwLock::new(None))
    }

    /// Registers or unregisters a notifier for the specified device.
    pub fn set(
        &self,
        dev_name: &str,
        correlation: u64,
        port: Option<kobject::PortSender>,
    ) -> Result<(), iface::NetDeviceError> {
        let mut notifier = self.0.write();

        // forbid to overwrite existing value
        if notifier.is_some() && port.is_some() {
            error!("Notifier already set for device {}", dev_name);
            return Err(iface::NetDeviceError::InvalidArgument);
        }

        if let Some(port) = port {
            debug!("Registering change notifier for device {}", dev_name);
            *notifier = Some(NotifierData { correlation, port });
        } else {
            debug!("Unregistering change notifier for device {}", dev_name);
            *notifier = None;
        }

        Ok(())
    }

    /// Notifies the registered notifier about a device event.
    pub fn notify<T: Copy>(&self, dev_name: &str, creator: impl Fn(u64) -> (T, ipc::KHandles)) {
        let notifier_access = self.0.read();
        let Some(notifier) = &*notifier_access else {
            return;
        };

        let (data, handles) = creator(notifier.correlation);
        let mut message = unsafe { kobject::Message::new(&data, handles.into()) };

        if let Err(err) = notifier.port.send(&mut message) {
            error!(
                "Failed to send notification for device {}: {:?}",
                dev_name, err
            );
        }
    }
}
