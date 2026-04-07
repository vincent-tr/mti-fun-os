use alloc::{string::String, sync::Arc, vec::Vec};
use futures::select_biased;
use hashbrown::HashSet;
use libruntime::{
    r#async::{self, tools::Worker},
    drivers::pci::PciAddress,
    ipc, kobject,
    net::{dev::iface, types::MacAddress},
    sync::{Mutex, r#async::NotifyOnce},
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

    /// The worker task that listens for notifications from the driver and handles them.
    worker: Mutex<Option<Worker>>,

    /// The ports for receiving notifications of link status changes from the driver.
    link_status_change_port: kobject::PortReceiver,

    /// The port for receiving notifications of received packets from the driver.
    rx_port: kobject::PortReceiver,

    /// The port for receiving notifications of transmitted packet buffers being freed by the driver.
    tx_free_port: kobject::PortReceiver,
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

        let link_status_change_port = {
            let (receiver, sender) = kobject::Port::create(None)?;
            ipc_client
                .set_link_status_change_port(handle, Some(sender), 0)
                .await?;

            receiver
        };

        let rx_port = {
            let (receiver, sender) = kobject::Port::create(None)?;
            ipc_client.set_rx_port(handle, Some(sender), 0).await?;

            receiver
        };

        let tx_free_port = {
            let (receiver, sender) = kobject::Port::create(None)?;
            ipc_client.set_tx_free_port(handle, Some(sender), 0).await?;

            receiver
        };

        let iface = Arc::new(Self {
            dev_name: String::from(dev_name),
            mac_address,
            ipc_client,
            handle,
            buffers: Mutex::new(HashSet::new()),
            worker: Mutex::new(None),
            link_status_change_port,
            rx_port,
            tx_free_port,
        });

        // Fill rx buffers initially, so the driver can start receiving packets immediately.
        iface.refill_rx_buffers().await?;

        let worker = Worker::spawn({
            let iface = iface.clone();
            async move |exit_signal| {
                iface.worker(exit_signal).await;
            }
        });

        *iface.worker.lock() = Some(worker);

        Ok(iface)
    }

    async fn refill_rx_buffers(&self) -> Result<(), iface::NetDeviceServerCallError> {
        loop {
            let mut buffer_indexes = Vec::new();
            for _ in 0..iface::Client::RX_BUFFER_COUNT {
                let index = buffer_pool::pool().allocate();
                buffer_indexes.push(index);
            }

            let mut guard = BufferGuard::new(buffer_indexes.iter().copied());

            let count = self
                .ipc_client
                .add_rx_buffers(self.handle, &buffer_indexes)
                .await?;

            guard.keep(count);

            {
                let mut buffers = self.buffers.lock();
                for &index in &buffer_indexes[0..count] {
                    buffers.insert(index);
                }
            }

            if count == 0 {
                // The driver cannot accept any additional buffers.
                break;
            }
        }

        Ok(())
    }

    /// Destroy the network device, cleaning up any resources.
    pub async fn destroy(self) -> Result<(), TodoError> {
        self.ipc_client.destroy(self.handle).await?;

        if let Some(worker) = self.worker.lock().take() {
            worker.terminate().await;
        }

        for index in self.buffers.lock().drain() {
            buffer_pool::pool().deallocate(index);
        }

        Ok(())
    }

    /// Interface worker task, which listens for notifications from the driver and handles them.
    async fn worker(self: Arc<Self>, exit_signal: NotifyOnce) {
        loop {
            select_biased! {
                // Note: important to check the exit signal first.
                _ = exit_signal.wait() => {
                    return;
                }

                _ = r#async::wait(&self.rx_port) => {
                    self.process_rx_notification().await;
                }

                _ = r#async::wait(&self.tx_free_port) => {
                    self.process_tx_free_notification().await;
                }

                _ = r#async::wait(&self.link_status_change_port) => {
                    self.process_link_status_change_notification().await;
                }
            }
        }
    }

    async fn process_rx_notification(&self) {
        // TODO
    }

    async fn process_tx_free_notification(&self) {
        // TODO
    }

    async fn process_link_status_change_notification(&self) {
        // TODO
    }
}

/// A guard for a set of buffer indexes allocated for an interface.
///
/// On drop, the guard will deallocate all of the buffers back to the buffer pool, unless `keep()` is called to keep some of them.
#[derive(Debug)]
struct BufferGuard {
    indexes: Vec<usize>,
}

impl BufferGuard {
    /// Create a new BufferGuard for the given buffer indexes.
    ///
    /// The guard will deallocate all of the buffers on drop, unless `keep()` is called to keep some of them.
    pub fn new(indexes: impl IntoIterator<Item = usize>) -> Self {
        Self {
            indexes: indexes.into_iter().collect(),
        }
    }

    /// Keep the first `count` indexes, and deallocate the rest when dropped.
    pub fn keep(&mut self, count: usize) {
        // Remove the indexes that we want to keep from the guard, so they won't be deallocated on drop.
        self.indexes.drain(0..count);
    }
}

impl Drop for BufferGuard {
    fn drop(&mut self) {
        for &index in &self.indexes {
            buffer_pool::pool().deallocate(index);
        }
    }
}
