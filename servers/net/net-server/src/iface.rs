use core::{
    cell::UnsafeCell,
    mem::{self, MaybeUninit},
};

use alloc::{string::String, sync::Arc, vec::Vec};
use futures::{FutureExt, select_biased};
use hashbrown::HashMap;
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
        types::{BufferPool, IpAddress, MacAddress},
    },
    sync::{Mutex, r#async::NotifyOnce},
    time,
};
use log::{debug, error};
use smallvec::SmallVec;

use crate::{
    buffer_pool::{self, Buffer},
    packet::{BufferData, Packet, PacketBuilder},
    proto::InterfaceProtocols,
};

/// A network intgerface, such as an Ethernet controller.
#[derive(Debug)]
pub struct Interface {
    /// The name of the interface, e.g. "eth0".
    name: String,

    /// The MAC address of the network device.
    mac_address: MacAddress,

    /// The IP configuration for this interface, if it has been configured with an IP address.
    ip_config: Mutex<Option<Arc<IpConfiguration>>>,

    /// The IPC client for communicating with the network device driver.
    ipc_client: iface::Client<'static>,

    /// The IPC handle for this interface.
    handle: ipc::Handle,

    /// The set of buffer currently in use by the NIC side of this interface.
    /// On destroy, the interface will free any buffers in this set back to the buffer pool.
    buffers: Mutex<HashMap<usize, Arc<Buffer>>>,

    /// The worker task that listens for notifications from the driver and handles them.
    worker: Mutex<Option<Worker>>,

    /// The worker task that performs periodic maintenance.
    tick_worker: Mutex<Option<Worker>>,

    /// The ports for receiving notifications of link status changes from the driver.
    link_status_change_port: kobject::PortReceiver,

    /// The port for receiving notifications of received packets from the driver.
    rx_port: kobject::PortReceiver,

    /// The port for receiving notifications of transmitted packet buffers being freed by the driver.
    tx_free_port: kobject::PortReceiver,

    /// The queue of received packets that have not yet been processed by the server.
    rx_pending_buffers: Mutex<RxPendingBuffers>,

    /// The protocol stack for this interface.
    protocols: DelayedInitCell<InterfaceProtocols>,
}

impl Interface {
    /// Get the name of the interface, e.g. "eth0".
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the MAC address of the network device.
    pub fn mac_address(&self) -> MacAddress {
        self.mac_address
    }

    /// Get the IP configuration for this interface, if it has been configured with an IP address.
    pub fn ip_config(&self) -> Option<Arc<IpConfiguration>> {
        self.ip_config.lock().clone()
    }

    /// Set the IP configuration for this interface.
    pub fn set_ip_config(&self, ip_config: Option<Arc<IpConfiguration>>) {
        *self.ip_config.lock() = ip_config;
    }

    /// Get a reference to the protocol stack for this interface.
    pub fn protocols(&self) -> &InterfaceProtocols {
        // SAFETY: This is InterfaceProtocols late initialization management.
        unsafe { self.protocols.get() }
    }

    /// Create a new network interface.
    pub async fn create(
        name: &str,
        drive_port_name: &str,
        pci_address: PciAddress,
    ) -> Result<Arc<Self>, NetServerError> {
        let ipc_client = iface::Client::new(drive_port_name);
        let handle = ipc_client
            .create(name, pci_address, buffer_pool::shared_pool())
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
            buffers: Mutex::new(HashMap::new()),
            worker: Mutex::new(None),
            tick_worker: Mutex::new(None),
            link_status_change_port,
            rx_port,
            tx_free_port,
            rx_pending_buffers: Mutex::new(RxPendingBuffers::new()),
            protocols: DelayedInitCell::uninit(),
            ip_config: Mutex::new(None),
        });

        // SAFETY: This is InterfaceProtocols late initialization management.
        unsafe {
            iface.protocols.init(InterfaceProtocols::new(&iface));
        }

        // Fill rx buffers initially, so the driver can start receiving packets immediately.
        iface.refill_rx_buffers().await?;

        let worker = Worker::spawn({
            let iface = iface.clone();
            async move |exit_signal| {
                iface.worker(exit_signal).await;
            }
        });

        let tick_worker = Worker::spawn({
            let iface = iface.clone();
            async move |exit_signal| {
                iface.tick_worker(exit_signal).await;
            }
        });

        *iface.worker.lock() = Some(worker);
        *iface.tick_worker.lock() = Some(tick_worker);

        Ok(iface)
    }

    /// Destroy the network device, cleaning up any resources.
    pub async fn destroy(&self) -> Result<(), NetServerError> {
        self.ipc_client
            .destroy(self.handle)
            .await
            .into_net_error()?;

        if let Some(tick_worker) = self.tick_worker.lock().take() {
            tick_worker.terminate().await;
        }

        if let Some(worker) = self.worker.lock().take() {
            worker.terminate().await;
        }

        // SAFETY: This is InterfaceProtocols late initialization management.
        unsafe {
            self.protocols.assume_init_drop();
        }

        self.buffers.lock().clear();

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

    /// Interface worker task, that performs periodic maintenance.
    async fn tick_worker(self: Arc<Self>, exit_signal: NotifyOnce) {
        loop {
            select_biased! {
                _ = exit_signal.wait() => {
                    return;
                }

                _ = time::async_sleep(time::Duration::milliseconds(100)).fuse() => {
                    self.protocols().tick();
                }
            }
        }
    }

    async fn refill_rx_buffers(&self) -> Result<(), NetServerError> {
        let mut total = 0;

        loop {
            let mut buffers = Vec::new();
            let mut buffer_indexes = Vec::new();
            for _ in 0..iface::Client::RX_BUFFER_COUNT {
                let buffer = Arc::new(buffer_pool::Buffer::allocate());
                buffer_indexes.push(buffer.id());
                buffers.push(buffer);
            }

            let count = self
                .ipc_client
                .add_rx_buffers(self.handle, &buffer_indexes)
                .await
                .into_net_error()?;

            self.keep_buffers(buffers.into_iter().take(count));

            total += count;

            if count < iface::Client::RX_BUFFER_COUNT {
                // The driver cannot accept any additional buffers.
                break;
            }
        }

        debug!("[{}] Added {} rx buffers to driver", self.name(), total);

        Ok(())
    }

    async fn process_rx_notification(&self) {
        let mut packets = Vec::new();

        loop {
            let msg = match self.rx_port.receive() {
                Ok(msg) => msg,
                Err(kobject::Error::ObjectNotReady) => break,
                Err(e) => {
                    error!("[{}] Error receiving from rx port: {:?}", self.name(), e);
                    break;
                }
            };

            let msg = unsafe { msg.data::<RxArrivedNotification>() };

            {
                let mut buffers = self.buffers.lock();
                let mut rx_pending_buffers = self.rx_pending_buffers.lock();
                for desc in msg.rx_descriptors {
                    if desc.buffer_index() != BufferPool::INVALID_INDEX {
                        let Some(buffer) = buffers.remove(&desc.buffer_index()) else {
                            panic!(
                                "[{}] Received rx notification for buffer index {} which is not currently in use",
                                self.name(),
                                desc.buffer_index(),
                            );
                        };

                        let buffer_data = BufferData::new(buffer, 0..desc.length());
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
            self.protocols().receive(packet);
        }
    }

    async fn process_tx_free_notification(&self) {
        loop {
            let msg = match self.tx_free_port.receive() {
                Ok(msg) => msg,
                Err(kobject::Error::ObjectNotReady) => break,
                Err(e) => panic!(
                    "[{}] Error receiving from tx free port: {:?}",
                    self.name(),
                    e
                ),
            };

            let msg = unsafe { msg.data::<TxFreeNotification>() };
            let mut buffers = self.buffers.lock();
            for &index in &msg.buffers {
                let index = index as usize;
                if index == BufferPool::INVALID_INDEX {
                    continue;
                }

                if buffers.remove(&index).is_none() {
                    panic!(
                        "[{}] Received tx free notification for buffer index {} which is not currently in use",
                        self.name(),
                        index
                    );
                }
            }
        }
    }

    async fn process_link_status_change_notification(&self) {
        loop {
            let msg = match self.link_status_change_port.receive() {
                Ok(msg) => msg,
                Err(kobject::Error::ObjectNotReady) => break,
                Err(e) => panic!(
                    "[{}] Error receiving from link status change port: {:?}",
                    self.name(),
                    e
                ),
            };

            let msg = unsafe { msg.data::<LinkStatusChangedNotification>() };

            // TODO: do something with message
            log::info!("[{}] Link status changed: {}", self.name(), msg.link_up);
        }
    }

    /// Transmit a packet on this interface.
    pub async fn transmit(&self, packet: PacketBuilder) {
        // TODO: properly handle TX full, wait for tx_free notifications, retry, manage a send queue, etc.
        // For now we will just try to send the packet, and drop it if the driver cannot accept it

        let mut desc = Vec::new();
        let mut buffers = Vec::new();

        let packet_data = packet.build().into_buffers();
        let len = packet_data.len();

        for (index, buffer_data) in packet_data.into_iter().enumerate() {
            let eop = index + 1 == len;
            let (buffer, range) = buffer_data.into_inner();

            desc.push(iface::TxBufferDescriptor::new(
                buffer.id(),
                range.start,
                range.len(),
                eop,
            ));

            buffers.push(buffer);
        }

        let count = match self.ipc_client.tx(self.handle, &desc).await {
            Ok(count) => count,
            Err(e) => {
                error!(
                    "[{}] Driver rejected packet for transmission (dropped): {:?}",
                    self.name(),
                    e
                );
                return;
            }
        };

        if count < desc.len() {
            // Note: as we did not include last packet, this will also probably mess next packet
            error!(
                "[{}] Driver could not accept all buffers for transmission (accepted {}, total {})",
                self.name(),
                count,
                desc.len(),
            );
        }

        self.keep_buffers(buffers.into_iter().take(count));
    }

    fn keep_buffers(&self, iter: impl Iterator<Item = Arc<Buffer>>) {
        let mut buffers = self.buffers.lock();

        for buffer in iter {
            buffers.insert(buffer.id(), buffer);
        }
    }
}

/// IP configuration for an interface, including IP address and subnet mask.
#[derive(Debug)]
pub struct IpConfiguration {
    ip_address: IpAddress,
    subnet_mask: IpAddress,
}

impl IpConfiguration {
    /// Create a new IP configuration with the given IP address and subnet mask.
    pub fn new(ip_address: IpAddress, subnet_mask: IpAddress) -> Self {
        Self {
            ip_address,
            subnet_mask,
        }
    }

    /// Get the IP address in this configuration.
    pub fn ip_address(&self) -> IpAddress {
        self.ip_address
    }

    /// Get the subnet mask in this configuration.
    pub fn subnet_mask(&self) -> IpAddress {
        self.subnet_mask
    }

    /// Check if this IP configuration is in the same subnet as the given IP address.
    fn is_same_subnet(&self, ip_address: IpAddress) -> bool {
        self.ip_address.as_u32() & self.subnet_mask.as_u32()
            == ip_address.as_u32() & self.subnet_mask.as_u32()
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

/// Helper structure for managing delayed initialization of the `InterfaceProtocols` for an `Interface`.
#[derive(Debug)]
struct DelayedInitCell<T> {
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Sync for DelayedInitCell<T> {}
unsafe impl<T> Send for DelayedInitCell<T> {}

impl<T> DelayedInitCell<T> {
    pub const fn uninit() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Initialize the value.
    ///
    /// Safety: The caller must ensure that the value is only initialized once.
    pub unsafe fn init(&self, value: T) {
        unsafe { (*self.value.get()).write(value) };
    }

    /// Get a reference to the value.
    ///
    /// Safety: The caller must ensure that the value has been initialized before calling this method.
    pub unsafe fn get(&self) -> &T {
        unsafe { (*self.value.get()).assume_init_ref() }
    }

    /// Drop the value, without dropping the `DelayedInitCell` itself.
    ///
    /// Safety: The caller must ensure that the value has been initialized before calling this method, and that it will not be used after this method is called.
    pub unsafe fn assume_init_drop(&self) {
        unsafe { (*self.value.get()).assume_init_drop() };
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
