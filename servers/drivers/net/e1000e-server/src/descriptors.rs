use core::fmt;

use bit_field::BitField;

#[derive(Copy, Clone, Default)]
#[repr(C, align(16))]
pub struct TxDescriptor {
    /// The physical address of the buffer for this descriptor.
    pub address: u64,

    pub flags: u64,
}

impl fmt::Debug for TxDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxDescriptor")
            .field("address", &format_args!("{:#x}", self.address))
            .field("flags", &format_args!("{:#x}", self.flags))
            .finish()
    }
}

impl TxDescriptor {
    pub fn address(&self) -> u64 {
        self.address
    }

    pub fn set_address(&mut self, address: u64) {
        self.address = address;
    }

    pub fn length(&self) -> u16 {
        self.flags.get_bits(0..16) as u16
    }

    pub fn set_length(&mut self, length: u16) {
        self.flags.set_bits(0..16, length as u64);
    }

    pub fn checksum_offset(&self) -> u8 {
        self.flags.get_bits(16..24) as u8
    }

    pub fn set_checksum_offset(&mut self, offset: u8) {
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

    pub fn checksum_start(&self) -> u8 {
        self.flags.get_bits(40..48) as u8
    }

    pub fn set_checksum_start(&mut self, start: u8) {
        self.flags.set_bits(40..48, start as u64);
    }

    pub fn special(&self) -> u16 {
        self.flags.get_bits(48..64) as u16
    }

    pub fn set_special(&mut self, special: u16) {
        self.flags.set_bits(48..64, special as u64);
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
