use core::{
    fmt::Debug,
    mem,
    ptr::{read_volatile, write_volatile},
};

use crate::{
    interrupts::Irq,
    memory::{PAGE_SIZE, Permissions, VirtAddr, map_iomem, unmap_phys},
};

use super::cpu::CPUID;

use super::pit;
use bit_field::BitField;
use log::{debug, info};
use spin::Mutex;

const FS_IN_SEC: usize = 1_000_000_000_000_000;

mod registers {
    use crate::memory::{PAGE_SIZE, PhysAddr, is_page_aligned};
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
struct LocalApic {
    base_addr: VirtAddr,
    timer_period_fs: Mutex<usize>,
}

impl LocalApic {
    pub const fn new() -> Self {
        Self {
            base_addr: VirtAddr::zero(),
            timer_period_fs: Mutex::new(0),
        }
    }

    pub unsafe fn init(&mut self) {
        let features = CPUID.get_feature_info().expect("cpuid: no feature info");
        assert!(features.has_apic());

        let reg_value = registers::ApicBase::read();
        assert!(reg_value.enabled);

        self.base_addr = unsafe {
            map_iomem(
                reg_value.address..reg_value.address + PAGE_SIZE as u64,
                Permissions::READ | Permissions::WRITE,
            )
        }
        .expect("could not map page into kernel space");
    }

    unsafe fn read(&self, reg: usize) -> u32 {
        unsafe { read_volatile((self.base_addr + reg as u64).as_ptr()) }
    }

    unsafe fn write(&self, reg: usize, value: u32) {
        unsafe { write_volatile((self.base_addr + reg as u64).as_mut_ptr(), value) };
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

    pub fn spurious_interrupt_vector(&self) -> LocalApicSpuriousInterruptVector {
        LocalApicSpuriousInterruptVector(unsafe {
            self.read(registers::SPURIOUS_INTERRUPT_VECTOR_REGISTER)
        })
    }

    pub fn set_spurious_interrupt_vector(&self, value: LocalApicSpuriousInterruptVector) {
        unsafe { self.write(registers::SPURIOUS_INTERRUPT_VECTOR_REGISTER, value.0) };
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

    pub fn timer(&self) -> Timer<'_> {
        Timer { apic: self }
    }
}

impl Drop for LocalApic {
    fn drop(&mut self) {
        unmap_phys(self.base_addr, 1);
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalApicId(u32);

impl LocalApicId {
    pub fn value(&self) -> usize {
        self.0.get_bits(24..32) as usize
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalApicVersion(u32);

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

#[derive(Clone, Copy)]
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

impl Debug for LocalApicErrors {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut set = f.debug_tuple("LocalApicErrors");

        if self.send_checksum_error() {
            set.field(&"send_checksum_error");
        }

        if self.receive_checksum_error() {
            set.field(&"receive_checksum_error");
        }

        if self.send_accept_error() {
            set.field(&"send_accept_error");
        }

        if self.receive_accept_error() {
            set.field(&"receive_accept_error");
        }

        if self.redirectable_ipi() {
            set.field(&"redirectable_ipi");
        }

        if self.send_illegal_vector() {
            set.field(&"send_illegal_vector");
        }

        if self.received_illegal_vector() {
            set.field(&"received_illegal_vector");
        }

        if self.illegal_register_address() {
            set.field(&"illegal_register_address");
        }

        set.finish()
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
struct LocalApicLVTTimer(u32);

impl LocalApicLVTTimer {
    pub const fn new() -> Self {
        Self(0)
    }

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
        let ivalue = self.0.get_bits(17..18) as u8;
        unsafe { mem::transmute(ivalue) }
    }

    pub fn set_timer_mode(&mut self, value: LocalApicLVTtimerMode) {
        self.0.set_bits(17..18, value as u8 as u32);
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalApicLVTWithDeliveryMode(u32);

impl LocalApicLVTWithDeliveryMode {
    pub const fn new() -> Self {
        Self(0)
    }

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
struct LocalApicLVTError(u32);

impl LocalApicLVTError {
    pub const fn new() -> Self {
        Self(0)
    }

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

#[derive(Debug, Clone, Copy)]
struct LocalApicSpuriousInterruptVector(u32);

impl LocalApicSpuriousInterruptVector {
    pub fn vector(&self) -> u8 {
        self.0.get_bits(0..8) as u8
    }

    /// # Safety
    ///
    /// Bits 0 through 3 may be unsupported
    pub fn set_vector(&mut self, value: u8) {
        self.0.set_bits(0..8, value as u32);
    }

    pub fn software_enabled(&self) -> bool {
        self.0.get_bit(8)
    }

    pub fn software_enable(&mut self) {
        self.0.set_bit(8, true);
    }

    pub fn software_disable(&mut self) {
        self.0.set_bit(8, false);
    }

    pub fn focus_processor_checking(&self) -> bool {
        self.0.get_bit(9)
    }

    /// # Safety
    ///
    /// Not supported on all processors
    pub unsafe fn set_focus_processor_checking(&mut self, value: bool) {
        self.0.set_bit(9, value);
    }

    pub fn eoi_broadcast_suppression(&self) -> bool {
        self.0.get_bit(12)
    }

    /// # Safety
    ///
    /// Not supported on all processors
    pub unsafe fn set_eoi_broadcast_suppression(&mut self, value: bool) {
        self.0.set_bit(12, value);
    }
}

#[derive(Debug)]
struct Timer<'a> {
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

    pub fn calibrate(&self) {
        // Do it 5 times to be consistent
        let mut freq_array: [usize; 5] = [0; 5];

        for freq in freq_array.iter_mut() {
            *freq = self.calibrate_once();
        }

        let timer_freq = freq_array.iter().sum::<usize>() / freq_array.len();

        let period_fs = FS_IN_SEC / timer_freq; // Hz -> fs

        *self.apic.timer_period_fs.lock() = period_fs;

        info!(
            "APIC Timer frequency: {}Mhz, period={}fs",
            timer_freq / 1_000_000,
            period_fs
        );
    }

    fn calibrate_once(&self) -> usize {
        // Use the PIT to calibrate APIC timer
        const DIVIDER: u32 = 16;
        const INITIAL_COUNT: u32 = 0xFFFFFFFF;

        self.set_divider(16);

        pit::start(pit::DIVISOR_10MS);
        self.set_initial_count(INITIAL_COUNT);

        let mut value = pit::read();
        let mut prev = value;

        // Else we wrapped around
        while prev >= value {
            prev = value;
            value = pit::read();
        }

        let ticks_in_10ms = INITIAL_COUNT - self.current_count();

        ticks_in_10ms as usize * 100 * DIVIDER as usize
    }

    pub fn configure(&self, delay_fs: usize) {
        let period_fs = *self.apic.timer_period_fs.lock();
        info!("period_fs = {period_fs}");
        assert!(period_fs > 0);
        assert!(delay_fs >= period_fs);

        let mut divider = 1;
        let mut init_count = delay_fs / period_fs;

        while init_count > u32::MAX as usize {
            divider *= 2;
            init_count /= 2;
        }

        debug!(
            "Configure Local APIC timer: timer period={period_fs}fs delay={delay_fs}fs, initial count={init_count}, divider={divider}"
        );

        self.set_divider(divider as u32);
        self.set_initial_count(init_count as u32);
    }
}

static LOCAL_APIC: Mutex<LocalApic> = Mutex::new(LocalApic::new());

pub fn init() {
    let mut apic = LOCAL_APIC.lock();

    unsafe { apic.init() };
    apic.timer().calibrate();

    // Enable Local APIC
    let mut siv = apic.spurious_interrupt_vector();
    siv.software_enable();
    apic.set_spurious_interrupt_vector(siv);

    // Set error interrupt
    let mut lvt = LocalApicLVTError::new();
    lvt.set_vector(Irq::LocalApicError as u8);
    apic.set_lvt_error(lvt)
}

pub fn configure_timer() {
    let apic = LOCAL_APIC.lock();

    // Configure timer
    let mut lvt = LocalApicLVTTimer::new();
    lvt.set_timer_mode(LocalApicLVTtimerMode::Period);
    lvt.set_vector(Irq::LocalApicTimer as u8);
    apic.set_lvt_timer(lvt);

    // Interrupt period=10ms
    apic.timer().configure(FS_IN_SEC / 100);
}

/// Signal end of interrupt for Local APIC
pub fn end_of_interrupt() {
    let apic = LOCAL_APIC.lock();

    apic.end_of_interrupt();
}

/// Get the current errors on Local APIC
pub fn current_errors() -> LocalApicErrors {
    let apic = LOCAL_APIC.lock();

    apic.current_errors()
}
