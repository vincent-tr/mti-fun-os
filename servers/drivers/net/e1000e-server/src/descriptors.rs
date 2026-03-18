use core::fmt;

use bit_field::BitField;
use libruntime::net::types::PhysAddr;

#[derive(Copy, Clone, Default)]
#[repr(C, align(16))]
pub struct TxDescriptor {
    address: PhysAddr,
    flags: u64,
}

impl fmt::Debug for TxDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxDescriptor")
            .field("address", &format_args!("{:#x}", self.address.as_u64()))
            .field("length", &format_args!("{:#x}", self.length()))
            .field(
                "checksum_offset",
                &format_args!("{:#x}", self.checksum_offset()),
            )
            .field("command", &self.command())
            .field("status", &self.status())
            .field(
                "checksum_start",
                &format_args!("{:#x}", self.checksum_start()),
            )
            .field("special", &format_args!("{:#x}", self.special()))
            .finish()
    }
}

#[allow(dead_code)]
impl TxDescriptor {
    pub fn address(&self) -> PhysAddr {
        self.address
    }

    pub fn set_address(&mut self, address: PhysAddr) {
        self.address = address;
    }

    pub fn length(&self) -> usize {
        self.flags.get_bits(0..16) as usize
    }

    pub fn set_length(&mut self, length: usize) {
        assert!(length <= u16::MAX as usize, "Length must fit in 16 bits");

        self.flags.set_bits(0..16, length as u64);
    }

    pub fn checksum_offset(&self) -> u8 {
        self.flags.get_bits(16..24) as u8
    }

    pub fn set_checksum_offset(&mut self, offset: usize) {
        assert!(
            offset <= u8::MAX as usize,
            "Checksum offset must fit in 8 bits"
        );
        self.flags.set_bits(16..24, offset as u64);
    }

    pub fn command(&self) -> TxDescriptorCommand {
        TxDescriptorCommand(self.flags.get_bits(24..32) as u8)
    }

    pub fn set_command(&mut self, command: TxDescriptorCommand) {
        self.flags.set_bits(24..32, command.0 as u64);
    }

    pub fn status(&self) -> TxDescriptorStatus {
        TxDescriptorStatus(self.flags.get_bits(32..40) as u8)
    }

    pub fn set_status(&mut self, status: TxDescriptorStatus) {
        self.flags.set_bits(32..40, status.0 as u64);
    }

    pub fn checksum_start(&self) -> usize {
        self.flags.get_bits(40..48) as usize
    }

    pub fn set_checksum_start(&mut self, start: usize) {
        assert!(
            start <= u8::MAX as usize,
            "Checksum start must fit in 8 bits"
        );
        self.flags.set_bits(40..48, start as u64);
    }

    pub fn special(&self) -> Special {
        Special(self.flags.get_bits(48..64) as u16)
    }

    pub fn set_special(&mut self, special: Special) {
        self.flags.set_bits(48..64, special.0 as u64);
    }
}

#[derive(Copy, Clone, Default)]
pub struct TxDescriptorCommand(u8);

impl fmt::Debug for TxDescriptorCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxDescriptorCommand")
            .field("end_of_packet", &self.end_of_packet())
            .field("insert_fcs", &self.insert_fcs())
            .field("insert_checksum", &self.insert_checksum())
            .field("report_status", &self.report_status())
            .field("vlan_packet", &self.vlan_packet())
            .field("interrupt_delay_enable", &self.interrupt_delay_enable())
            .finish()
    }
}

#[allow(dead_code)]
impl TxDescriptorCommand {
    pub fn end_of_packet(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_end_of_packet(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    pub fn insert_fcs(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn set_insert_fcs(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    pub fn insert_checksum(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn set_insert_checksum(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    pub fn report_status(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn set_report_status(&mut self, value: bool) {
        self.0.set_bit(3, value);
    }

    pub fn vlan_packet(&self) -> bool {
        self.0.get_bit(4)
    }

    pub fn set_vlan_packet(&mut self, value: bool) {
        self.0.set_bit(4, value);
    }

    pub fn interrupt_delay_enable(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn set_interrupt_delay_enable(&mut self, value: bool) {
        self.0.set_bit(5, value);
    }
}

#[derive(Copy, Clone, Default)]
pub struct TxDescriptorStatus(u8);

impl fmt::Debug for TxDescriptorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxDescriptorStatus")
            .field("descriptor_done", &self.descriptor_done())
            .finish()
    }
}

#[allow(dead_code)]
impl TxDescriptorStatus {
    pub fn descriptor_done(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_descriptor_done(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    pub fn excess_collisions(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn set_excess_collisions(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    pub fn late_collision(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn set_late_collision(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    pub fn transmit_underrun(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn set_transmit_underrun(&mut self, value: bool) {
        self.0.set_bit(3, value);
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C, align(16))]
pub struct RxDescriptor {
    address: PhysAddr,
    flags: u64,
}

impl fmt::Debug for RxDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RxDescriptor")
            .field("address", &format_args!("{:#x}", self.address.as_u64()))
            .field("length", &format_args!("{:#x}", self.length()))
            .field(
                "checksum_offset",
                &format_args!("{:#x}", self.checksum_offset()),
            )
            .field("command", &self.command())
            .field("status", &self.status())
            .field(
                "checksum_start",
                &format_args!("{:#x}", self.checksum_start()),
            )
            .field("special", &format_args!("{:#x}", self.special()))
            .finish()
    }
}

#[allow(dead_code)]
impl RxDescriptor {
    pub fn address(&self) -> PhysAddr {
        self.address
    }

    pub fn set_address(&mut self, address: PhysAddr) {
        self.address = address;
    }

    pub fn length(&self) -> usize {
        self.flags.get_bits(0..16) as usize
    }

    pub fn set_length(&mut self, length: usize) {
        assert!(length <= u16::MAX as usize, "Length must fit in 16 bits");

        self.flags.set_bits(0..16, length as u64);
    }

    pub fn packet_checksum(&self) -> u16 {
        self.flags.get_bits(16..32) as u16
    }

    pub fn set_packet_checksum(&mut self, checksum: u16) {
        self.flags.set_bits(16..32, checksum as u64);
    }

    pub fn status(&self) -> RxDescriptorStatus {
        RxDescriptorStatus(self.flags.get_bits(32..40) as u8)
    }

    pub fn set_status(&mut self, status: RxDescriptorStatus) {
        self.flags.set_bits(32..40, status.0 as u64);
    }

    pub fn errors(&self) -> RxDescriptorErrors {
        RxDescriptorErrors(self.flags.get_bits(40..48) as u8)
    }

    pub fn set_errors(&mut self, errors: RxDescriptorErrors) {
        self.flags.set_bits(40..48, errors.0 as u64);
    }

    pub fn special(&self) -> Special {
        Special(self.flags.get_bits(48..64) as u16)
    }

    pub fn set_special(&mut self, special: Special) {
        self.flags.set_bits(48..64, special.0 as u64);
    }
}

#[derive(Copy, Clone, Default)]
pub struct RxDescriptorStatus(u8);

impl fmt::Debug for RxDescriptorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RxDescriptorStatus")
            .field("descriptor_done", &self.descriptor_done())
            .field("end_of_packet", &self.end_of_packet())
            .field("ignore_checksum", &self.ignore_checksum())
            .field("vet_packet", &self.vet_packet())
            .field("tcp_checksum_calculated", &self.tcp_checksum_calculated())
            .field("ip_checksum_calculated", &self.ip_checksum_calculated())
            .field("passed_inexact_filter", &self.passed_inexact_filter())
            .finish()
    }
}

#[allow(dead_code)]
impl RxDescriptorStatus {
    pub fn descriptor_done(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_descriptor_done(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    pub fn end_of_packet(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn set_end_of_packet(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    pub fn ignore_checksum(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn set_ignore_checksum(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    pub fn vet_packet(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn set_vet_packet(&mut self, value: bool) {
        self.0.set_bit(3, value);
    }

    pub fn tcp_checksum_calculated(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn set_tcp_checksum_calculated(&mut self, value: bool) {
        self.0.set_bit(5, value);
    }

    pub fn ip_checksum_calculated(&self) -> bool {
        self.0.get_bit(6)
    }

    pub fn set_ip_checksum_calculated(&mut self, value: bool) {
        self.0.set_bit(6, value);
    }

    pub fn passed_inexact_filter(&self) -> bool {
        self.0.get_bit(7)
    }

    pub fn set_passed_inexact_filter(&mut self, value: bool) {
        self.0.set_bit(7, value);
    }
}

#[derive(Copy, Clone, Default)]
pub struct RxDescriptorErrors(u8);

impl fmt::Debug for RxDescriptorErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RxDescriptorErrors")
            .field("crc_error", &self.crc_error())
            .field("symbol_error", &self.symbol_error())
            .field("sequence_error", &self.sequence_error())
            .field("carrier_extension_error", &self.carrier_extension_error())
            .field("tcp_checksum_error", &self.tcp_checksum_error())
            .field("ip_checksum_error", &self.ip_checksum_error())
            .field("rx_data_error", &self.rx_data_error())
            .finish()
    }
}

#[allow(dead_code)]
impl RxDescriptorErrors {
    pub fn crc_error(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn set_crc_error(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    pub fn symbol_error(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn set_symbol_error(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    pub fn sequence_error(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn set_sequence_error(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    pub fn carrier_extension_error(&self) -> bool {
        self.0.get_bit(4)
    }

    pub fn set_carrier_extension_error(&mut self, value: bool) {
        self.0.set_bit(4, value);
    }

    pub fn tcp_checksum_error(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn set_tcp_checksum_error(&mut self, value: bool) {
        self.0.set_bit(5, value);
    }

    pub fn ip_checksum_error(&self) -> bool {
        self.0.get_bit(6)
    }

    pub fn set_ip_checksum_error(&mut self, value: bool) {
        self.0.set_bit(6, value);
    }

    pub fn rx_data_error(&self) -> bool {
        self.0.get_bit(7)
    }

    pub fn set_rx_data_error(&mut self, value: bool) {
        self.0.set_bit(7, value);
    }
}
#[derive(Copy, Clone, Default)]
pub struct Special(u16);

impl fmt::Debug for Special {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Special").field("value", &self.0).finish()
    }
}

#[allow(dead_code)]
impl Special {
    pub fn vlan(&self) -> u16 {
        self.0.get_bits(0..12)
    }

    pub fn set_vlan(&mut self, vlan: u16) {
        assert!(vlan < 2_u16.pow(12), "VLAN must fit in 12 bits");
        self.0.set_bits(0..12, vlan);
    }

    pub fn canonical_form_indicator(&self) -> bool {
        self.0.get_bit(12)
    }

    pub fn set_canonical_form_indicator(&mut self, value: bool) {
        self.0.set_bit(12, value);
    }

    pub fn user_priority(&self) -> u8 {
        self.0.get_bits(13..16) as u8
    }

    pub fn set_user_priority(&mut self, priority: u8) {
        assert!(priority < 2_u8.pow(3), "User priority must fit in 3 bits");
        self.0.set_bits(13..16, priority as u16);
    }
}
