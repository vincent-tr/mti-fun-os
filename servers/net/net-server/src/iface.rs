use core::{fmt, mem};

use alloc::{collections::vec_deque::VecDeque, string::String, sync::Arc, vec::Vec};
use futures::select_biased;
use hashbrown::HashSet;
use libruntime::{
    r#async::{self, tools::Worker},
    drivers::pci::PciAddress,
    ipc, kobject,
    net::{
        dev::iface::{
            self, LinkStatusChangedNotification, NetDeviceError, NetDeviceServerCallError,
            RxArrivedNotification, TxFreeNotification,
        },
        iface::NetServerError,
        types::{BufferPool, MacAddress},
    },
    sync::{Mutex, r#async::NotifyOnce},
};
use log::{debug, error};
use smallvec::SmallVec;

use crate::{
    buffer_pool::{self, Buffer},
    packet::{BufferData, Packet},
    proto::ethernet,
};

/// A network intgerface, such as an Ethernet controller.
#[derive(Debug)]
pub struct Interface {
    /// The name of the interface, e.g. "eth0".
    name: String,

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

    /// The queue of received packets that have not yet been processed by the server.
    rx_pending_buffers: Mutex<RxPendingBuffers>,
}

impl Interface {
    /// Create a new network interface.
    pub async fn create(
        name: &str,
        drive_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<Arc<Self>, NetServerError> {
        let ipc_client = iface::Client::new(drive_port_name);
        let handle = ipc_client
            .create(name, pci_address, buffer_pool::pool().share())
            .await
            .into_net_error()?;
        let mac_address = ipc_client.get_mac_address(handle).await.into_net_error()?;

        let link_status_change_port = {
            let (receiver, sender) = kobject::Port::create(None).into_net_error()?;
            ipc_client
                .set_link_status_change_port(handle, Some(sender), 0)
                .await
                .into_net_error()?;

            receiver
        };

        let rx_port = {
            let (receiver, sender) = kobject::Port::create(None).into_net_error()?;
            ipc_client
                .set_rx_port(handle, Some(sender), 0)
                .await
                .into_net_error()?;

            receiver
        };

        let tx_free_port = {
            let (receiver, sender) = kobject::Port::create(None).into_net_error()?;
            ipc_client
                .set_tx_free_port(handle, Some(sender), 0)
                .await
                .into_net_error()?;

            receiver
        };

        let iface = Arc::new(Self {
            name: String::from(name),
            mac_address,
            ipc_client,
            handle,
            buffers: Mutex::new(HashSet::new()),
            worker: Mutex::new(None),
            link_status_change_port,
            rx_port,
            tx_free_port,
            rx_pending_buffers: Mutex::new(RxPendingBuffers::new()),
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

    async fn refill_rx_buffers(&self) -> Result<(), NetServerError> {
        let mut total = 0;

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
                .await
                .into_net_error()?;

            guard.keep(count);

            {
                let mut buffers = self.buffers.lock();
                for &index in &buffer_indexes[0..count] {
                    buffers.insert(index);
                }
            }

            total += count;

            if count < iface::Client::RX_BUFFER_COUNT {
                // The driver cannot accept any additional buffers.
                break;
            }
        }

        debug!("Added {} rx buffers to driver", total);

        Ok(())
    }

    /// Destroy the network device, cleaning up any resources.
    pub async fn destroy(&self) -> Result<(), NetServerError> {
        self.ipc_client
            .destroy(self.handle)
            .await
            .into_net_error()?;

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
        let mut packets = Vec::new();

        loop {
            let msg = match self.rx_port.receive() {
                Ok(msg) => msg,
                Err(kobject::Error::ObjectNotReady) => break,
                Err(e) => {
                    error!("Error receiving from rx port: {:?}", e);
                    break;
                }
            };

            let msg = unsafe { msg.data::<RxArrivedNotification>() };

            {
                let mut buffers = self.buffers.lock();
                let mut rx_pending_buffers = self.rx_pending_buffers.lock();
                for desc in msg.rx_descriptors {
                    if desc.buffer_index() != BufferPool::INVALID_INDEX {
                        let buffer = unsafe { Buffer::from_id(desc.buffer_index()) };
                        buffers.remove(&buffer.id());

                        let buffer_data = BufferData::new(Arc::new(buffer), 0..desc.length());
                        rx_pending_buffers.add(buffer_data, desc.error());

                        if desc.end_of_packet() {
                            if let Some(packet) = rx_pending_buffers.build_packet() {
                                packets.push(packet);
                            }
                        }
                    }
                }
            }

            self.refill_rx_buffers()
                .await
                .expect("Failed to refill rx buffers");
        }

        for packet in packets {
            self.rx_packet(packet).await;
        }
    }

    async fn process_tx_free_notification(&self) {
        loop {
            let msg = match self.tx_free_port.receive() {
                Ok(msg) => msg,
                Err(kobject::Error::ObjectNotReady) => break,
                Err(e) => panic!("Error receiving from tx free port: {:?}", e),
            };

            let msg = unsafe { msg.data::<TxFreeNotification>() };
            let mut buffers = self.buffers.lock();
            for &index in &msg.buffers {
                let index = index as usize;
                if index == BufferPool::INVALID_INDEX {
                    continue;
                }

                if !buffers.remove(&index) {
                    panic!(
                        "Received tx free notification for buffer index {} which is not currently in use",
                        index
                    );
                }

                buffer_pool::pool().deallocate(index);
            }
        }
    }

    async fn process_link_status_change_notification(&self) {
        loop {
            let msg = match self.link_status_change_port.receive() {
                Ok(msg) => msg,
                Err(kobject::Error::ObjectNotReady) => break,
                Err(e) => panic!("Error receiving from link status change port: {:?}", e),
            };

            let msg = unsafe { msg.data::<LinkStatusChangedNotification>() };

            // TODO: do something with message
            log::info!("Link status changed: {}", msg.link_up);
        }
    }

    async fn rx_packet(&self, packet: Packet) {
        ethernet::rx_packet(packet).await;
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

/// Pending buffers for a received packet. Once the end of the packet is reached, these buffers will be combined into a `Packet` and processed by the server.
#[derive(Debug)]
struct RxPendingBuffers {
    buffers: SmallVec<[BufferData; 4]>,
    error: bool,
}

impl RxPendingBuffers {
    pub fn new() -> Self {
        Self {
            buffers: SmallVec::new(),
            error: false,
        }
    }

    /// Add a buffer to the pending buffers, marking if there was an error receiving into this buffer.
    pub fn add(&mut self, buffer: BufferData, error: bool) {
        self.buffers.push(buffer);
        if error {
            self.error = true;
        }
    }

    /// Build the packet, or drop it if there was an error receiving into any of the buffers.
    ///
    /// Also resets the pending buffers for the next packet.
    pub fn build_packet(&mut self) -> Option<Packet> {
        let mut buffers = SmallVec::new();
        mem::swap(&mut buffers, &mut self.buffers);
        let error = self.error;
        self.error = false;

        if error {
            error!("Dropping received packet due to error in one of its buffers");
            None
        } else {
            Some(Packet::new(buffers))
        }
    }
}

trait NetResultExt<T> {
    fn into_net_error(self) -> Result<T, NetServerError>;
}

impl<T> NetResultExt<T> for Result<T, NetDeviceServerCallError> {
    fn into_net_error(self) -> Result<T, NetServerError> {
        self.map_err(|e| match e {
            NetDeviceServerCallError::KernelError(e) => {
                error!("Runtime error during net device server call: {:?}", e);
                NetServerError::RuntimeError
            }
            NetDeviceServerCallError::ReplyError(e) => match e {
                NetDeviceError::InvalidArgument => NetServerError::InvalidArgument,
                NetDeviceError::RuntimeError => NetServerError::RuntimeError,
                NetDeviceError::DeviceError => NetServerError::DeviceError,
            },
        })
    }
}

impl<T> NetResultExt<T> for Result<T, kobject::Error> {
    fn into_net_error(self) -> Result<T, NetServerError> {
        self.map_err(|e| {
            error!("Runtime error in interface management: {:?}", e);
            NetServerError::RuntimeError
        })
    }
}
