use core::fmt;

use bit_field::BitField;

/// Control register for the e1000e network device.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct Control(u32);

impl From<u32> for Control {
    fn from(value: u32) -> Self {
        Control(value)
    }
}

impl From<Control> for u32 {
    fn from(control: Control) -> Self {
        control.0
    }
}

impl fmt::Debug for Control {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Control")
            .field("raw", &format_args!("{:#010x}", self.0))
            .field("full_duplex", &self.full_duplex())
            .field("link_reset", &self.link_reset())
            .field("auto_speed_detection", &self.auto_speed_detection())
            .field("link_up", &self.link_up())
            .field("invert_loss_of_signal", &self.invert_loss_of_signal())
            .field("speed", &self.speed())
            .field("speed_forced", &self.speed_forced())
            .field("duplex_forced", &self.duplex_forced())
            .field("spd0_data_rate", &self.spd0_data_rate())
            .field("spd1_data", &self.spd1_data())
            .field("reset", &self.reset())
            .field("rx_enabled", &self.rx_enabled())
            .field("tx_enabled", &self.tx_enabled())
            .field("phy_reset", &self.phy_reset())
            .finish()
    }
}

impl Control {
    pub const OFFSET: usize = 0x00000;

    pub fn full_duplex(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_full_duplex(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    pub fn link_reset(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn set_link_reset(&mut self, value: bool) {
        self.0.set_bit(3, value);
    }

    pub fn auto_speed_detection(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn set_auto_speed_detection(&mut self, value: bool) {
        self.0.set_bit(5, value);
    }

    pub fn link_up(&self) -> bool {
        self.0.get_bit(6)
    }

    pub fn set_link_up(&mut self, value: bool) {
        self.0.set_bit(6, value);
    }

    pub fn invert_loss_of_signal(&self) -> bool {
        self.0.get_bit(7)
    }

    pub fn set_invert_loss_of_signal(&mut self, value: bool) {
        self.0.set_bit(7, value);
    }

    pub fn speed(&self) -> Speed {
        Speed::from(self.0.get_bits(8..10) as u8)
    }

    pub fn set_speed(&mut self, value: Speed) {
        self.0.set_bits(8..10, value as u8 as u32);
    }

    pub fn speed_forced(&self) -> bool {
        self.0.get_bit(11)
    }

    pub fn force_speed(&mut self, value: bool) {
        self.0.set_bit(11, value);
    }

    pub fn duplex_forced(&self) -> bool {
        self.0.get_bit(12)
    }

    pub fn force_duplex(&mut self, value: bool) {
        self.0.set_bit(12, value);
    }

    pub fn spd0_data_rate(&self) -> bool {
        self.0.get_bit(18)
    }

    pub fn set_spd0_data(&mut self, value: bool) {
        self.0.set_bit(18, value);
    }

    pub fn spd1_data(&self) -> bool {
        self.0.get_bit(19)
    }

    pub fn set_spd1_data(&mut self, value: bool) {
        self.0.set_bit(19, value);
    }

    // ADVD3WUC
    // EN_PHY_PWR_MGMT
    // SDP0_IODIR
    // SDP1_IODIR

    pub fn reset(&self) -> bool {
        self.0.get_bit(26)
    }

    pub fn set_reset(&mut self, value: bool) {
        self.0.set_bit(26, value);
    }

    pub fn rx_enabled(&self) -> bool {
        self.0.get_bit(27)
    }

    pub fn enable_rx(&mut self, value: bool) {
        self.0.set_bit(27, value);
    }

    pub fn tx_enabled(&self) -> bool {
        self.0.get_bit(28)
    }

    pub fn enable_tx(&mut self, value: bool) {
        self.0.set_bit(28, value);
    }

    // VME

    pub fn phy_reset(&self) -> bool {
        self.0.get_bit(30)
    }

    pub fn set_phy_reset(&mut self, value: bool) {
        self.0.set_bit(30, value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Speed {
    Speed10Mbps = 0,
    Speed100Mbps = 1,
    Speed1Gbps = 2,
}

impl From<u8> for Speed {
    fn from(value: u8) -> Self {
        match value {
            0 => Speed::Speed10Mbps,
            1 => Speed::Speed100Mbps,
            2 => Speed::Speed1Gbps,
            _ => panic!("Invalid speed value: {}", value),
        }
    }
}

/// Status register for the e1000e network device.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct Status(u32);

impl From<u32> for Status {
    fn from(value: u32) -> Self {
        Status(value)
    }
}

impl From<Status> for u32 {
    fn from(status: Status) -> Self {
        status.0
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Status")
            .field("raw", &format_args!("{:#010x}", self.0))
            .field("full_duplex", &self.full_duplex())
            .field("link_up", &self.link_up())
            .field("tx_paused", &self.tx_paused())
            .field("tbi_mode", &self.tbi_mode())
            .field("speed", &self.speed())
            .field(
                "auto_speed_detected_value",
                &self.auto_speed_detected_value(),
            )
            .field("bus_speed", &self.bus_speed())
            .field("bus_width", &self.bus_width())
            .field("pcix_mode", &self.pcix_mode())
            .field("pcix_speed", &self.pcix_speed())
            .finish()
    }
}

impl Status {
    pub const OFFSET: usize = 0x00008;

    pub fn full_duplex(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn link_up(&self) -> bool {
        self.0.get_bit(1)
    }

    // Function ID

    pub fn tx_paused(&self) -> bool {
        self.0.get_bit(4)
    }

    pub fn tbi_mode(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn speed(&self) -> Speed {
        Speed::from(self.0.get_bits(6..8) as u8)
    }

    pub fn auto_speed_detected_value(&self) -> Speed {
        Speed::from(self.0.get_bits(8..10) as u8)
    }

    pub fn bus_speed(&self) -> BusSpeed {
        if self.0.get_bit(11) {
            BusSpeed::Speed66MHz
        } else {
            BusSpeed::Speed33MHz
        }
    }

    pub fn bus_width(&self) -> BusWidth {
        if self.0.get_bit(12) {
            BusWidth::Bus64Bits
        } else {
            BusWidth::Bus32Bits
        }
    }

    pub fn pcix_mode(&self) -> bool {
        self.0.get_bit(13)
    }

    pub fn pcix_speed(&self) -> PcixSpeed {
        PcixSpeed::from(self.0.get_bits(14..16) as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusSpeed {
    Speed33MHz,
    Speed66MHz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusWidth {
    Bus32Bits,
    Bus64Bits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcixSpeed {
    Speed50_66MHz = 0,
    Speed66_100Mhz = 1,
    Speed100_133MHz = 2,
}

impl From<u8> for PcixSpeed {
    fn from(value: u8) -> Self {
        match value {
            0 => PcixSpeed::Speed50_66MHz,
            1 => PcixSpeed::Speed66_100Mhz,
            2 => PcixSpeed::Speed100_133MHz,
            _ => panic!("Invalid speed value: {}", value),
        }
    }
}

/// EEPROM Control/Data register for the e1000e network device.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct EepromControlData(u32);

impl From<u32> for EepromControlData {
    fn from(value: u32) -> Self {
        EepromControlData(value)
    }
}

impl From<EepromControlData> for u32 {
    fn from(control: EepromControlData) -> Self {
        control.0
    }
}

impl fmt::Debug for EepromControlData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EepromControlData")
            .field("raw", &format_args!("{:#010x}", self.0))
            .field("clock_input", &self.clock_input())
            .field("chip_select", &self.chip_select())
            .field("data_input", &self.data_input())
            .field("data_output", &self.data_output())
            .field("flash_write_enabled", &self.flash_write_enabled())
            .field("access_request", &self.access_request())
            .field("access_grant", &self.access_grant())
            .field("present", &self.present())
            .field("size", &self.size())
            .field("type", &self.r#type())
            .finish()
    }
}

impl EepromControlData {
    pub const OFFSET: usize = 0x00010;

    pub fn clock_input(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_clock_input(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    pub fn chip_select(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn set_chip_select(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    pub fn data_input(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn set_data_input(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    pub fn data_output(&self) -> bool {
        self.0.get_bit(3)
    }

    // No setter for data output, as it's read-only

    pub fn flash_write_enabled(&self) -> bool {
        let value = self.0.get_bits(4..6);
        match value {
            1 => false,
            2 => true,
            _ => panic!("Invalid flash write control value: {}", value),
        }
    }

    pub fn enable_flash_write(&mut self, value: bool) {
        let value = if value { 2 } else { 1 };
        self.0.set_bits(4..6, value);
    }

    pub fn access_request(&self) -> bool {
        self.0.get_bit(6)
    }

    pub fn set_access_request(&mut self, value: bool) {
        self.0.set_bit(6, value);
    }

    pub fn access_grant(&self) -> bool {
        self.0.get_bit(7)
    }

    // No setter for access grant, as it's read-only

    pub fn present(&self) -> bool {
        self.0.get_bit(8)
    }

    // No setter for present, as it's read-only

    pub fn size(&self) -> usize {
        if self.0.get_bit(9) {
            // 4096 bits
            512
        } else {
            // 1024 bits
            128
        }
    }

    // No setter for size, as it's read-only

    // TODO size bit 10 ??

    pub fn r#type(&self) -> EEpromType {
        EEpromType::from(self.0.get_bit(13) as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EEpromType {
    Microwire = 0,
    Spi = 1,
}

impl From<u8> for EEpromType {
    fn from(value: u8) -> Self {
        match value {
            0 => EEpromType::Microwire,
            1 => EEpromType::Spi,
            _ => panic!("Invalid eeprom type: {}", value),
        }
    }
}

/// EEPROM Read register for the e1000e network device.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct EepromRead(u32);

impl From<u32> for EepromRead {
    fn from(value: u32) -> Self {
        EepromRead(value)
    }
}

impl From<EepromRead> for u32 {
    fn from(eerd: EepromRead) -> Self {
        eerd.0
    }
}

impl fmt::Debug for EepromRead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EepromRead")
            .field("raw", &format_args!("{:#010x}", self.0))
            .field("start", &self.start())
            .field("done", &self.done())
            .field("address", &self.address())
            .field("data", &self.data())
            .finish()
    }
}

impl EepromRead {
    pub const OFFSET: usize = 0x00014;

    pub fn start(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_start(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    pub fn done(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn address(&self) -> u16 {
        self.0.get_bits(2..16) as u16
    }

    pub fn set_address(&mut self, address: u16) {
        self.0.set_bits(2..16, address as u32);
    }

    pub fn data(&self) -> u16 {
        self.0.get_bits(16..32) as u16
    }
}

#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct RxControl(u32);

impl From<u32> for RxControl {
    fn from(value: u32) -> Self {
        RxControl(value)
    }
}

impl From<RxControl> for u32 {
    fn from(control: RxControl) -> Self {
        control.0
    }
}

impl fmt::Debug for RxControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RxControl")
            .field("raw", &format_args!("{:#010x}", self.0))
            .field("enabled", &self.enabled())
            .field("store_bad_packets", &self.store_bad_packets())
            .field(
                "unicast_promiscuous_enabled",
                &self.unicast_promiscuous_enabled(),
            )
            .field(
                "multicast_promiscuous_enabled",
                &self.multicast_promiscuous_enabled(),
            )
            .field(
                "long_packet_reception_enabled",
                &self.long_packet_reception_enabled(),
            )
            .field("loopback_mode", &self.loopback_mode())
            .field(
                "receive_descriptor_minimum_threshold_size",
                &self.receive_descriptor_minimum_threshold_size(),
            )
            .field("multicast_offset", &self.multicast_offset())
            .field("broadcast_accepted", &self.broadcast_accepted())
            .field("receive_buffer_size", &self.receive_buffer_size())
            .field("vlan_filter_enabled", &self.vlan_filter_enabled())
            .field(
                "canonical_form_indicator_enabled",
                &self.canonical_form_indicator_enabled(),
            )
            .field(
                "canonical_form_indicator_bit_value",
                &self.canonical_form_indicator_bit_value(),
            )
            .field("discard_pause_frames", &self.discard_pause_frames())
            .field("pass_mac_control_frames", &self.pass_mac_control_frames())
            .field("buffer_size_extension", &self.buffer_size_extension())
            .field("strip_ethernet_crc", &self.strip_ethernet_crc())
            .finish()
    }
}

impl RxControl {
    pub const OFFSET: usize = 0x00100;

    pub fn enabled(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn enable(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    pub fn store_bad_packets(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn set_store_bad_packets(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    pub fn unicast_promiscuous_enabled(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn enable_unicast_promiscuous(&mut self, value: bool) {
        self.0.set_bit(3, value);
    }

    pub fn multicast_promiscuous_enabled(&self) -> bool {
        self.0.get_bit(4)
    }

    pub fn enable_multicast_promiscuous(&mut self, value: bool) {
        self.0.set_bit(4, value);
    }

    pub fn long_packet_reception_enabled(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn enable_long_packet_reception(&mut self, value: bool) {
        self.0.set_bit(5, value);
    }

    pub fn loopback_mode(&self) -> LoopbackMode {
        LoopbackMode::from(self.0.get_bits(6..8) as u8)
    }

    pub fn set_loopback_mode(&mut self, value: LoopbackMode) {
        self.0.set_bits(6..8, value as u8 as u32);
    }

    pub fn receive_descriptor_minimum_threshold_size(&self) -> u8 {
        self.0.get_bits(8..10) as u8
    }

    pub fn set_receive_descriptor_minimum_threshold_size(&mut self, value: u8) {
        self.0.set_bits(8..10, value as u32);
    }

    pub fn multicast_offset(&self) -> u8 {
        self.0.get_bits(12..14) as u8
    }

    pub fn set_multicast_offset(&mut self, value: u8) {
        self.0.set_bits(12..14, value as u32);
    }

    pub fn broadcast_accepted(&self) -> bool {
        self.0.get_bit(15)
    }

    pub fn set_broadcast_accepted(&mut self, value: bool) {
        self.0.set_bit(15, value);
    }

    pub fn receive_buffer_size(&self) -> u8 {
        self.0.get_bits(16..18) as u8
    }

    pub fn set_receive_buffer_size(&mut self, value: u8) {
        self.0.set_bits(16..18, value as u32);
    }

    pub fn vlan_filter_enabled(&self) -> bool {
        self.0.get_bit(18)
    }

    pub fn enable_vlan_filter(&mut self, value: bool) {
        self.0.set_bit(18, value);
    }

    pub fn canonical_form_indicator_enabled(&self) -> bool {
        self.0.get_bit(19)
    }

    pub fn enable_canonical_form_indicator(&mut self, value: bool) {
        self.0.set_bit(19, value);
    }

    pub fn canonical_form_indicator_bit_value(&self) -> bool {
        self.0.get_bit(20)
    }

    pub fn set_canonical_form_indicator_bit_value(&mut self, value: bool) {
        self.0.set_bit(20, value);
    }

    pub fn discard_pause_frames(&self) -> bool {
        self.0.get_bit(22)
    }

    pub fn set_discard_pause_frames(&mut self, value: bool) {
        self.0.set_bit(22, value);
    }

    pub fn pass_mac_control_frames(&self) -> bool {
        self.0.get_bit(23)
    }

    pub fn set_pass_mac_control_frames(&mut self, value: bool) {
        self.0.set_bit(23, value);
    }

    pub fn buffer_size_extension(&self) -> bool {
        self.0.get_bit(25)
    }

    pub fn set_buffer_size_extension(&mut self, value: bool) {
        self.0.set_bit(25, value);
    }

    pub fn strip_ethernet_crc(&self) -> bool {
        self.0.get_bit(26)
    }

    pub fn set_strip_ethernet_crc(&mut self, value: bool) {
        self.0.set_bit(26, value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LoopbackMode {
    NoLoopback = 0,
    PhyOrExternal = 3,
}

impl From<u8> for LoopbackMode {
    fn from(value: u8) -> Self {
        match value {
            0 => LoopbackMode::NoLoopback,
            3 => LoopbackMode::PhyOrExternal,
            _ => panic!("Invalid loopback mode value: {}", value),
        }
    }
}

/// Transmit Control register for the e1000e network device.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct TxControl(u32);

impl From<u32> for TxControl {
    fn from(value: u32) -> Self {
        TxControl(value)
    }
}

impl From<TxControl> for u32 {
    fn from(control: TxControl) -> Self {
        control.0
    }
}

impl fmt::Debug for TxControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxControl")
            .field("raw", &format_args!("{:#010x}", self.0))
            .field("enabled", &self.enabled())
            .field("pad_short_packets", &self.pad_short_packets())
            .field("collision_threshold", &self.collision_threshold())
            .field("collision_distance", &self.collision_distance())
            .field(
                "software_xoff_transmission",
                &self.software_xoff_transmission(),
            )
            .field(
                "retransmit_on_late_collision",
                &self.retransmit_on_late_collision(),
            )
            .field(
                "no_retransmit_on_underrun",
                &self.no_retransmit_on_underrun(),
            )
            .finish()
    }
}

impl TxControl {
    pub const OFFSET: usize = 0x00400;

    pub fn enabled(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn enable(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    pub fn pad_short_packets(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn set_pad_short_packets(&mut self, value: bool) {
        self.0.set_bit(3, value);
    }

    pub fn collision_threshold(&self) -> u8 {
        self.0.get_bits(4..12) as u8
    }

    pub fn set_collision_threshold(&mut self, value: u8) {
        self.0.set_bits(4..12, value as u32);
    }

    pub fn collision_distance(&self) -> u8 {
        self.0.get_bits(12..22) as u8
    }

    pub fn set_collision_distance(&mut self, value: u8) {
        self.0.set_bits(12..22, value as u32);
    }

    pub fn software_xoff_transmission(&self) -> bool {
        self.0.get_bit(22)
    }

    pub fn set_software_xoff_transmission(&mut self, value: bool) {
        self.0.set_bit(22, value);
    }

    pub fn retransmit_on_late_collision(&self) -> bool {
        self.0.get_bit(24)
    }

    pub fn set_retransmit_on_late_collision(&mut self, value: bool) {
        self.0.set_bit(24, value);
    }

    pub fn no_retransmit_on_underrun(&self) -> bool {
        self.0.get_bit(25)
    }

    pub fn set_no_retransmit_on_underrun(&mut self, value: bool) {
        self.0.set_bit(25, value);
    }
}

/// Interrupt Mask register for the e1000e network device.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct InterruptMask(u32);

impl From<u32> for InterruptMask {
    fn from(value: u32) -> Self {
        InterruptMask(value)
    }
}

impl From<InterruptMask> for u32 {
    fn from(mask: InterruptMask) -> Self {
        mask.0
    }
}

impl fmt::Debug for InterruptMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptMask")
            .field("raw", &format_args!("{:#010x}", self.0))
            .field(
                "transmit_descriptor_written_back",
                &self.transmit_descriptor_written_back(),
            )
            .field("transmit_queue_empty", &self.transmit_queue_empty())
            .field("link_status_change", &self.link_status_change())
            .field("receive_sequence_error", &self.receive_sequence_error())
            .field(
                "receive_descriptor_minimum_threshold",
                &self.receive_descriptor_minimum_threshold(),
            )
            .field("receive_fifo_overrun", &self.receive_fifo_overrun())
            .field("receive_timer_interrupt", &self.receive_timer_interrupt())
            .field(
                "mdio_access_complete_interrupt",
                &self.mdio_access_complete_interrupt(),
            )
            .field("receiving_config", &self.receiving_config())
            .field("phy_interrupt", &self.phy_interrupt())
            .field(
                "transmit_descriptor_low_threshold",
                &self.transmit_descriptor_low_threshold(),
            )
            .field(
                "small_receive_packet_detection",
                &self.small_receive_packet_detection(),
            )
            .finish()
    }
}

impl InterruptMask {
    pub const OFFSET: usize = 0x00D0;

    pub fn transmit_descriptor_written_back(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_transmit_descriptor_written_back(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    pub fn transmit_queue_empty(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn set_transmit_queue_empty(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    pub fn link_status_change(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn set_link_status_change(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    pub fn receive_sequence_error(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn set_receive_sequence_error(&mut self, value: bool) {
        self.0.set_bit(3, value);
    }

    pub fn receive_descriptor_minimum_threshold(&self) -> bool {
        self.0.get_bit(4)
    }

    pub fn set_receive_descriptor_minimum_threshold(&mut self, value: bool) {
        self.0.set_bit(4, value);
    }

    pub fn receive_fifo_overrun(&self) -> bool {
        self.0.get_bit(6)
    }

    pub fn set_receive_fifo_overrun(&mut self, value: bool) {
        self.0.set_bit(6, value);
    }

    pub fn receive_timer_interrupt(&self) -> bool {
        self.0.get_bit(7)
    }

    pub fn set_receive_timer_interrupt(&mut self, value: bool) {
        self.0.set_bit(7, value);
    }

    pub fn mdio_access_complete_interrupt(&self) -> bool {
        self.0.get_bit(9)
    }

    pub fn set_mdio_access_complete_interrupt(&mut self, value: bool) {
        self.0.set_bit(9, value);
    }

    pub fn receiving_config(&self) -> bool {
        self.0.get_bit(10)
    }

    pub fn set_receiving_config(&mut self, value: bool) {
        self.0.set_bit(10, value);
    }

    pub fn phy_interrupt(&self) -> bool {
        self.0.get_bit(12)
    }

    pub fn set_phy_interrupt(&mut self, value: bool) {
        self.0.set_bit(12, value);
    }

    // GPI - General Purpose Interrupts

    pub fn transmit_descriptor_low_threshold(&self) -> bool {
        self.0.get_bit(15)
    }

    pub fn set_transmit_descriptor_low_threshold(&mut self, value: bool) {
        self.0.set_bit(15, value);
    }

    pub fn small_receive_packet_detection(&self) -> bool {
        self.0.get_bit(16)
    }

    pub fn set_small_receive_packet_detection(&mut self, value: bool) {
        self.0.set_bit(16, value);
    }
}
