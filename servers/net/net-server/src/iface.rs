use alloc::{string::String, sync::Arc, vec::Vec};
use hashbrown::HashSet;
use libruntime::{
    r#async,
    drivers::pci::PciAddress,
    ipc, kobject,
    net::{dev::iface, types::MacAddress},
    sync::Mutex,
};

use crate::buffer_pool;

enum TodoError {}

impl From<iface::NetDeviceServerCallError> for TodoError {
    fn from(_: iface::NetDeviceServerCallError) -> Self {
        todo!()
    }
}

impl From<kobject::Error> for TodoError {
    fn from(_: kobject::Error) -> Self {
        todo!()
    }
}

/// A network intgerface, such as an Ethernet controller.
#[derive(Debug)]
pub struct Interface {
    /// The name of the network device, e.g. "eth0".
    dev_name: String,

    /// The MAC address of the network device.
    mac_address: MacAddress,

    /// The IPC client for communicating with the network device driver.
    ipc_client: iface::Client<'static>,

    /// The IPC handle for this interface.
    handle: ipc::Handle,

    /// The set of buffer indexes currently in use for this interface.
    /// On destroy, the interface will free any buffers in this set back to the buffer pool.
    buffers: Mutex<HashSet<usize>>,
}

impl Interface {
    /// Create a new network interface.
    pub async fn create(
        dev_name: &str,
        drive_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<Arc<Self>, TodoError> {
        let ipc_client = iface::Client::new(drive_port_name);
        let handle = ipc_client
            .create(dev_name, pci_address, buffer_pool::pool().share())
            .await?;
        let mac_address = ipc_client.get_mac_address(handle).await?;

        let iface = Arc::new(Self {
            dev_name: String::from(dev_name),
            mac_address,
            ipc_client,
            handle,
            buffers: Mutex::new(HashSet::new()),
        });

        let link_status_change_port = {
            let (receiver, sender) = kobject::Port::create(None)?;
            iface
                .ipc_client
                .set_link_status_change_port(iface.handle, Some(sender), 0)
                .await?;

            receiver
        };

        let rx_port = {
            let (receiver, sender) = kobject::Port::create(None)?;
            iface
                .ipc_client
                .set_rx_port(iface.handle, Some(sender), 0)
                .await?;

            receiver
        };

        let tx_free_port = {
            let (receiver, sender) = kobject::Port::create(None)?;
            iface
                .ipc_client
                .set_tx_free_port(iface.handle, Some(sender), 0)
                .await?;

            receiver
        };

        // Fill rx buffers initially, so the driver can start receiving packets immediately.
        iface.refill_rx_buffers().await?;

        r#async::spawn(
            iface
                .clone()
                .worker(link_status_change_port, rx_port, tx_free_port),
        );

        Ok(iface)
    }

    async fn refill_rx_buffers(&self) -> Result<(), iface::NetDeviceServerCallError> {
        loop {
            let mut buffer_indexes = Vec::new();
            for _ in 0..iface::Client::RX_BUFFER_COUNT {
                let index = buffer_pool::pool().allocate();
                buffer_indexes.push(index);
                self.buffers.lock().insert(index);
            }

            // TODO: what if the future is dropped?
            // We should use RAII for temp buffers

            match self
                .ipc_client
                .add_rx_buffers(self.handle, &buffer_indexes)
                .await
            {
                Ok(count) => {
                    // Release any buffers that were not added.
                    for &index in &buffer_indexes[count..] {
                        self.buffers.lock().remove(&index);
                        buffer_pool::pool().deallocate(index);
                    }

                    if count == 0 {
                        // The driver cannot accept any additional buffers.
                        break;
                    }
                }

                Err(e) => {
                    // On error, deallocate all buffers.
                    for &index in &buffer_indexes {
                        self.buffers.lock().remove(&index);
                        buffer_pool::pool().deallocate(index);
                    }

                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Destroy the network device, cleaning up any resources.
    pub async fn destroy(self) -> Result<(), TodoError> {
        self.ipc_client.destroy(self.handle).await?;

        // TODO: stop worker task

        for index in self.buffers.lock().drain() {
            buffer_pool::pool().deallocate(index);
        }

        Ok(())
    }

    /// Interface worker task, which listens for notifications from the driver and handles them.
    async fn worker(
        self: Arc<Self>,
        link_status_change_port: kobject::PortReceiver,
        rx_port: kobject::PortReceiver,
        tx_free_port: kobject::PortReceiver,
    ) {
        // TODO: select + exit signal
    }
}

// TODO: wait for async task
// TODO: oneshot signal (for exit)
