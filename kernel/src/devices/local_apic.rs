use core::ptr::{read_volatile, write_volatile};

use crate::memory::{map_iomem, unmap_phys, Permissions, VirtAddr, PAGE_SIZE};

use super::CPUID;

use bit_field::BitField;

mod registers {
    use crate::memory::{is_page_aligned, PhysAddr, PAGE_SIZE};
    use bit_field::BitField;
    use x86_64::registers::model_specific::Msr;

    /// The ACPI base Register.
    #[derive(Debug)]
    pub struct ApicBase;

    impl ApicBase {
        /// The underlying model specific register.
        pub const MSR: Msr = Msr::new(0x1B);

        /// Read the current ACPI base address register.
        #[inline]
        pub fn read() -> ApicBaseValue {
            let raw = unsafe { Self::MSR.read() };

            ApicBaseValue {
                address: PhysAddr::new(raw.get_bits(12..36) * PAGE_SIZE as u64),
                enabled: raw.get_bit(11),
                is_bootstrap_processor: raw.get_bit(8),
            }
        }

        /// Write a given physical address to the ACPI base register.
        ///
        /// ### Note
        /// Only the address can be changed, the other fields are read-only (unless we want to disable the APIC)
        #[inline]
        pub fn update(addr: PhysAddr) -> PhysAddr {
            assert!(is_page_aligned(addr.as_u64() as usize));
            assert!(addr.as_u64() < 0xFFFF_FFFF);

            let mut raw = unsafe { Self::MSR.read() };

            let old_addr = PhysAddr::new(raw.get_bits(12..36) * PAGE_SIZE as u64);

            raw.set_bits(12..36, addr.as_u64() / PAGE_SIZE as u64);

            let mut msr = Self::MSR;
            unsafe { msr.write(raw) };

            old_addr
        }
    }

    pub struct ApicBaseValue {
        pub address: PhysAddr,
        pub enabled: bool,
        pub is_bootstrap_processor: bool,
    }

    pub const ID: usize = 0x0020; // RW
    pub const VERSION: usize = 0x0030; // RO
    pub const TASK_PRIORITY: usize = 0x0080; // RW
    pub const ARBITRATION_PRIORITY: usize = 0x0090; // RO
    pub const PROCESSOR_PRIORITY: usize = 0x00A0; // RO
    pub const EOI: usize = 0x00B0; // WO
    pub const REMOTE_READ: usize = 0x00C0; // RO
    pub const LOGICAL_DESTINATION: usize = 0x00D0; // RW
    pub const DESTINATION_FORMAT: usize = 0x00E0; // RW
    pub const SPURIOUS_INTERRUPT_VECTOR_REGISTER: usize = 0x00F0; // RW

    /*
    FEE0 0100H In-Service Register (ISR); bits 31:0 Read Only.
    FEE0 0110H In-Service Register (ISR); bits 63:32 Read Only.
    FEE0 0120H In-Service Register (ISR); bits 95:64 Read Only.
    FEE0 0130H In-Service Register (ISR); bits 127:96 Read Only.
    FEE0 0140H In-Service Register (ISR); bits 159:128 Read Only.
    FEE0 0150H In-Service Register (ISR); bits 191:160 Read Only.
    FEE0 0160H In-Service Register (ISR); bits 223:192 Read Only.
    FEE0 0170H In-Service Register (ISR); bits 255:224 Read Only.

    FEE0 0180H Trigger Mode Register (TMR); bits 31:0 Read Only.
    FEE0 0190H Trigger Mode Register (TMR); bits 63:32 Read Only.
    FEE0 01A0H Trigger Mode Register (TMR); bits 95:64 Read Only.
    FEE0 01B0H Trigger Mode Register (TMR); bits 127:96 Read Only.
    FEE0 01C0H Trigger Mode Register (TMR); bits 159:128 Read Only.
    FEE0 01D0H Trigger Mode Register (TMR); bits 191:160 Read Only
    FEE0 01E0H Trigger Mode Register (TMR); bits 223:192 Read Only.
    FEE0 01F0H Trigger Mode Register (TMR); bits 255:224 Read Only.

    FEE0 0200H Interrupt Request Register (IRR); bits 31:0 Read Only.
    FEE0 0210H Interrupt Request Register (IRR); bits 63:32 Read Only.
    FEE0 0220H Interrupt Request Register (IRR); bits 95:64 Read Only.
    FEE0 0230H Interrupt Request Register (IRR); bits 127:96 Read Only.
    FEE0 0240H Interrupt Request Register (IRR); bits 159:128 Read Only.
    FEE0 0250H Interrupt Request Register (IRR); bits 191:160 Read Only.
    FEE0 0260H Interrupt Request Register (IRR); bits 223:192 Read Only.
    FEE0 0270H Interrupt Request Register (IRR); bits 255:224 Read Only.
    */

    pub const ERROR_STATUS: usize = 0x0280; // RO
    pub const LVT_CMCI: usize = 0x02F0; // RW

    /*
    FEE0 0300H Interrupt Command Register (ICR); bits 0-31 Read/Write.
    FEE0 0310H Interrupt Command Register (ICR); bits 32-63 Read/Write.
    */

    pub const LVT_TIMER: usize = 0x0320; // RW
    pub const LVT_THERMAL_SENSOR: usize = 0x0330; // RW
    pub const LVT_PERFORMANCE_MONITORING_COUNTERS: usize = 0x0340; // RW
    pub const LVT_LINT0: usize = 0x0350; // RW
    pub const LVT_LINT1: usize = 0x0360; // RW
    pub const LVT_ERROR: usize = 0x0370; // RW
    pub const TIMER_INITIAL_COUNT: usize = 0x0380; // RW
    pub const TIMER_CURRENT_COUNT: usize = 0x0390; // RO
    pub const TIMER_DIVIDE_CONFIGURATION: usize = 0x03E0; // RW
}

/// Local APIC
#[derive(Debug)]
pub struct LocalApic {
    base_addr: VirtAddr,
}

impl LocalApic {
    pub unsafe fn init() -> Self {
        let features = CPUID.get_feature_info().expect("cpuid: no feature info");
        assert!(features.has_apic());
        assert!(features.has_tsc_deadline());

        let reg_value = registers::ApicBase::read();
        assert!(reg_value.enabled);

        let base_addr = map_iomem(
            reg_value.address..reg_value.address + PAGE_SIZE,
            Permissions::READ | Permissions::WRITE,
        )
        .expect("could not map page into kernel space");

        Self { base_addr }
    }

    unsafe fn read(&self, reg: usize) -> u32 {
        read_volatile((self.base_addr + reg).as_ptr())
    }

    unsafe fn write(&self, reg: usize, value: u32) {
        write_volatile((self.base_addr + reg).as_mut_ptr(), value);
    }

    /// Local APIC ID
    pub fn id(&self) -> LocalApicId {
        LocalApicId(unsafe { self.read(registers::ID) })
    }

    /// Local APIC Version
    pub fn version(&self) -> LocalApicVersion {
        LocalApicVersion(unsafe { self.read(registers::VERSION) })
    }

    /// Signal the end of interrupt
    pub fn end_of_interrupt(&self) {
        unsafe { self.write(registers::EOI, 0) }
    }

    pub fn current_errors(&self) -> LocalApicErrors {
        unsafe {
            self.write(registers::ERROR_STATUS, 0);
            LocalApicErrors(self.read(registers::ERROR_STATUS))
        }
    }

    pub fn lvt_timer(&self) -> LocalApicLVTTimer {
        LocalApicLVTTimer(unsafe { self.read(registers::LVT_TIMER) })
    }

    pub fn set_lvt_timer(&self, value: LocalApicLVTTimer) {
        unsafe { self.write(registers::LVT_TIMER, value.0) };
    }

    pub fn lvt_cmci(&self) -> LocalApicLVTWithDeliveryMode {
        LocalApicLVTWithDeliveryMode(unsafe { self.read(registers::LVT_CMCI) })
    }

    pub fn set_lvt_cmci(&self, value: LocalApicLVTWithDeliveryMode) {
        unsafe { self.write(registers::LVT_CMCI, value.0) };
    }

    pub fn lvt_lint0(&self) -> LocalApicLVTWithDeliveryMode {
        LocalApicLVTWithDeliveryMode(unsafe { self.read(registers::LVT_LINT0) })
    }

    pub fn set_lvt_lint0(&self, value: LocalApicLVTWithDeliveryMode) {
        unsafe { self.write(registers::LVT_LINT0, value.0) };
    }

    pub fn lvt_lint1(&self) -> LocalApicLVTWithDeliveryMode {
        LocalApicLVTWithDeliveryMode(unsafe { self.read(registers::LVT_LINT1) })
    }

    pub fn set_lvt_lint1(&self, value: LocalApicLVTWithDeliveryMode) {
        unsafe { self.write(registers::LVT_LINT1, value.0) };
    }

    pub fn lvt_error(&self) -> LocalApicLVTError {
        LocalApicLVTError(unsafe { self.read(registers::LVT_ERROR) })
    }

    pub fn set_lvt_error(&self, value: LocalApicLVTError) {
        unsafe { self.write(registers::LVT_ERROR, value.0) };
    }

    pub fn lvt_performance_counters(&self) -> LocalApicLVTWithDeliveryMode {
        LocalApicLVTWithDeliveryMode(unsafe {
            self.read(registers::LVT_PERFORMANCE_MONITORING_COUNTERS)
        })
    }

    pub fn set_lvt_performance_counters(&self, value: LocalApicLVTWithDeliveryMode) {
        unsafe { self.write(registers::LVT_PERFORMANCE_MONITORING_COUNTERS, value.0) };
    }

    pub fn lvt_thermal_sensor(&self) -> LocalApicLVTWithDeliveryMode {
        LocalApicLVTWithDeliveryMode(unsafe { self.read(registers::LVT_THERMAL_SENSOR) })
    }

    pub fn set_lvt_thermal_sensor(&self, value: LocalApicLVTWithDeliveryMode) {
        unsafe { self.write(registers::LVT_THERMAL_SENSOR, value.0) };
    }

    pub fn timer(&self) -> Timer {
        Timer { apic: self }
    }
}

impl Drop for LocalApic {
    fn drop(&mut self) {
        unmap_phys(self.base_addr, 1);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalApicId(u32);

impl LocalApicId {
    pub fn value(&self) -> usize {
        self.0.get_bits(24..32) as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalApicVersion(u32);

impl LocalApicVersion {
    pub fn version(&self) -> usize {
        self.0.get_bits(0..8) as usize
    }

    pub fn max_lvt_entries(&self) -> usize {
        self.0.get_bits(16..24) as usize + 1
    }

    pub fn suppress_eoi_broadcasts(&self) -> bool {
        self.0.get_bit(24)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalApicErrors(u32);

impl LocalApicErrors {
    pub fn send_checksum_error(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn receive_checksum_error(&self) -> bool {
        self.0.get_bit(1)
    }

    pub fn send_accept_error(&self) -> bool {
        self.0.get_bit(2)
    }

    pub fn receive_accept_error(&self) -> bool {
        self.0.get_bit(3)
    }

    pub fn redirectable_ipi(&self) -> bool {
        self.0.get_bit(4)
    }

    pub fn send_illegal_vector(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn received_illegal_vector(&self) -> bool {
        self.0.get_bit(6)
    }

    pub fn illegal_register_address(&self) -> bool {
        self.0.get_bit(7)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LocalApicLVTDeliveryStatus {
    Idle,
    SendPending,
}

#[derive(Debug, Clone, Copy)]
pub enum LocalApicLVTDeliveryMode {
    Fixed,
    SMI,
    NMI,
    ExtINT,
    INIT,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LocalApicLVTtimerMode {
    OneShot = 0b00,
    Period = 0b01,
    TscDeadline = 0b10,
}

#[derive(Debug, Clone, Copy)]
pub struct LocalApicLVTTimer(u32);

impl LocalApicLVTTimer {
    pub fn vector(&self) -> u8 {
        self.0.get_bits(0..8) as u8
    }

    pub fn set_vector(&mut self, value: u8) {
        self.0.set_bits(0..8, value as u32);
    }

    pub fn delivery_status(&self) -> LocalApicLVTDeliveryStatus {
        if self.0.get_bit(12) {
            LocalApicLVTDeliveryStatus::SendPending
        } else {
            LocalApicLVTDeliveryStatus::Idle
        }
    }

    pub fn masked(&self) -> bool {
        self.0.get_bit(16)
    }

    pub fn mask(&mut self) {
        self.0.set_bit(16, true);
    }

    pub fn unmask(&mut self) {
        self.0.set_bit(16, false);
    }

    pub fn timer_mode(&self) -> LocalApicLVTtimerMode {
        self.0.get_bits(17..18) as LocalApicLVTtimerMode
    }

    pub fn set_timer_mode(&mut self, value: LocalApicLVTtimerMode) {
        let raw = match value {
            LocalApicLVTtimerMode::OneShot => 0b00,
            LocalApicLVTtimerMode::Period => 0b01,
            LocalApicLVTtimerMode::TscDeadline => 0b10,
        };

        self.0.set_bits(17..18, raw);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalApicLVTWithDeliveryMode(u32);

impl LocalApicLVTWithDeliveryMode {
    pub fn vector(&self) -> u8 {
        self.0.get_bits(0..8) as u8
    }

    pub fn set_vector(&mut self, value: u8) {
        self.0.set_bits(0..8, value as u32);
    }

    pub fn delivery_mode(&self) -> LocalApicLVTDeliveryMode {
        match self.0.get_bits(8..10) {
            0b000 => LocalApicLVTDeliveryMode::Fixed,
            0b010 => LocalApicLVTDeliveryMode::SMI,
            0b100 => LocalApicLVTDeliveryMode::NMI,
            0b111 => LocalApicLVTDeliveryMode::ExtINT,
            0b101 => LocalApicLVTDeliveryMode::INIT,
            _ => panic!("unexpected delivery mode"),
        }
    }

    pub fn set_delivery_mode(&mut self, value: LocalApicLVTDeliveryMode) {
        let raw = match value {
            LocalApicLVTDeliveryMode::Fixed => 0b000,
            LocalApicLVTDeliveryMode::SMI => 0b010,
            LocalApicLVTDeliveryMode::NMI => 0b100,
            LocalApicLVTDeliveryMode::ExtINT => 0b111,
            LocalApicLVTDeliveryMode::INIT => 0b101,
        };

        self.0.set_bits(8..10, raw);
    }

    pub fn delivery_status(&self) -> LocalApicLVTDeliveryStatus {
        if self.0.get_bit(12) {
            LocalApicLVTDeliveryStatus::SendPending
        } else {
            LocalApicLVTDeliveryStatus::Idle
        }
    }

    pub fn masked(&self) -> bool {
        self.0.get_bit(16)
    }

    pub fn mask(&mut self) {
        self.0.set_bit(16, true);
    }

    pub fn unmask(&mut self) {
        self.0.set_bit(16, false);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalApicLVTError(u32);

impl LocalApicLVTError {
    pub fn vector(&self) -> u8 {
        self.0.get_bits(0..8) as u8
    }

    pub fn set_vector(&mut self, value: u8) {
        self.0.set_bits(0..8, value as u32);
    }

    pub fn delivery_status(&self) -> LocalApicLVTDeliveryStatus {
        if self.0.get_bit(12) {
            LocalApicLVTDeliveryStatus::SendPending
        } else {
            LocalApicLVTDeliveryStatus::Idle
        }
    }

    pub fn masked(&self) -> bool {
        self.0.get_bit(16)
    }

    pub fn mask(&mut self) {
        self.0.set_bit(16, true);
    }

    pub fn unmask(&mut self) {
        self.0.set_bit(16, false);
    }
}

#[derive(Debug)]
pub struct Timer<'a> {
    apic: &'a LocalApic,
}

impl Timer<'_> {
    // Get divide configuration
    pub fn divider(&self) -> usize {
        let raw = unsafe { self.apic.read(registers::TIMER_DIVIDE_CONFIGURATION) };
        let bits = (raw.get_bit(3), raw.get_bit(1), raw.get_bit(0));

        match bits {
            (false, false, false) => 2,
            (false, false, true) => 4,
            (false, true, false) => 8,
            (false, true, true) => 16,
            (true, false, false) => 32,
            (true, false, true) => 64,
            (true, true, false) => 128,
            (true, true, true) => 1,
        }
    }

    // Set divide configuration
    pub fn set_divider(&self, value: u32) {
        let bits = match value {
            2 => (false, false, false),
            4 => (false, false, true),
            8 => (false, true, false),
            16 => (false, true, true),
            32 => (true, false, false),
            64 => (true, false, true),
            128 => (true, true, false),
            1 => (true, true, true),
            _ => {
                panic!("invalid divider value");
            }
        };

        let mut raw: u32 = 0;
        raw.set_bit(3, bits.0);
        raw.set_bit(1, bits.1);
        raw.set_bit(0, bits.2);

        unsafe { self.apic.write(registers::TIMER_DIVIDE_CONFIGURATION, raw) };
    }

    // Get initial count
    pub fn initial_count(&self) -> u32 {
        unsafe { self.apic.read(registers::TIMER_INITIAL_COUNT) }
    }

    // Set initial count
    pub fn set_initial_count(&self, value: u32) {
        unsafe { self.apic.write(registers::TIMER_INITIAL_COUNT, value) };
    }

    // Get current count
    pub fn current_count(&self) -> u32 {
        unsafe { self.apic.read(registers::TIMER_CURRENT_COUNT) }
    }
}
