use core::mem;

use bit_field::BitField;
use spin::Mutex;
use x86_64::instructions::port::Port;

use super::pic8259::{set_irq_masked, IRQ0};

pub const TIMER_INTERRUPT_INDEX: usize = IRQ0;

/*
from http://www.brokenthorn.com/Resources/OSDevPit.html

Bit 0: (BCP) Binary Counter
    0: Binary
    1: Binary Coded Decimal (BCD)
Bit 1-3: (M0, M1, M2) Operating Mode.
    000: Mode 0: Interrupt or Terminal Count
    001: Mode 1: Programmable one-shot
    010: Mode 2: Rate Generator
    011: Mode 3: Square Wave Generator
    100: Mode 4: Software Triggered Strobe
    101: Mode 5: Hardware Triggered Strobe
    110: Undefined; Don't use
    111: Undefined; Don't use
Bits 4-5: (RL0, RL1) Read/Load Mode. We are going to read or send data to a counter register
    00: Counter value is latched into an internal control register at the time of the I/O write operation.
    01: Read or Load Least Significant Byte (LSB) only
    10: Read or Load Most Significant Byte (MSB) only
    11: Read or Load LSB first then MSB
Bits 6-7: (SC0-SC1) Select Counter.
    00: Counter 0
    01: Counter 1
    10: Counter 2
    11: Illegal value
*/

/// (BCP) Binary Counter
#[repr(u8)]
enum BinaryCounter {
    /// Binary
    Binary = 0,

    ///Binary Coded Decimal (BCD)
    BinaryCodedDecimal = 1,
}

/// (M0, M1, M2) Operating Mode.
#[repr(u8)]
enum OperatingMode {
    /// Mode 0: Interrupt or Terminal Count
    Mode0 = 0,

    ///  Mode 1: Programmable one-shot
    Mode1 = 1,

    /// Mode 2: Rate Generator
    Mode2 = 2,

    /// Mode 3: Square Wave Generator
    Mode3 = 3,

    /// Mode 4: Software Triggered Strobe
    Mode4 = 4,

    /// Mode 5: Hardware Triggered Strobe
    Mode5 = 5,
}

/// (RL0, RL1) Read/Load Mode. We are going to read or send data to a counter register
#[repr(u8)]
enum ReadLoadMode {
    /// Counter value is latched into an internal control register at the time of the I/O write operation.
    Latch = 0,

    /// Read or Load Least Significant Byte (LSB) only
    LsbOnly = 1,

    /// Read or Load Most Significant Byte (MSB) only
    MsbOnly = 2,

    /// Read or Load LSB first then MSB
    LsbThenMsB = 3,
}

/// PIT channel
#[repr(u8)]
pub enum Channel {
    /// Channel 0
    Channel0 = 0,

    /// Channel 1
    Channel1 = 1,

    /// Channel 2
    Channel2 = 2,
}


#[derive(Debug, Clone, Copy)]
struct Command(u8);

impl Command {
    pub const fn new() -> Self{
        Self(0)
    }

    /// get (BCP) Binary Counter
    pub fn binary_counter(&self) -> BinaryCounter {
        unsafe { mem::transmute(self.0.get_bits(0..1)) }
    }

    /// set (BCP) Binary Counter
    pub fn set_binary_counter(&mut self, value: BinaryCounter) {
        self.0.set_bits(0..1, value as u8);
    }

    /// get Operating Mode.
    pub fn operating_mode(&self) -> OperatingMode {
        unsafe { mem::transmute(self.0.get_bits(1..4)) }
    }

    /// set Operating Mode.
    pub fn set_operating_mode(&mut self, value: OperatingMode) {
        self.0.set_bits(1..4, value as u8);
    }

    /// get Read/Load Mode.
    pub fn read_load_mode(&self) -> ReadLoadMode {
        unsafe { mem::transmute(self.0.get_bits(4..6)) }
    }

    /// set Read/Load Mode.
    pub fn set_read_load_mode(&mut self, value: ReadLoadMode) {
        self.0.set_bits(4..6, value as u8);
    }

    /// get channel.
    pub fn channel(&self) -> Channel {
        unsafe { mem::transmute(self.0.get_bits(6..8)) }
    }

    /// set channel.
    pub fn set_channel(&mut self, value: Channel) {
        self.0.set_bits(6..8, value as u8);
    }
}

struct Pit {
    command: Port<u8>,
    channels: [Port<u8>; 3],
}

impl Pit {
    pub const fn new() -> Self {
        Self {
            command: Port::new(0x43),
            channels: [
                Port::new(0x40),
                Port::new(0x41),
                Port::new(0x42),
            ]
        }
    }

    /// # Safety
    /// Command must match read operation
    unsafe fn read(&mut self, command: Command, channel: Channel) -> u16 {
        let channel = &mut self.channels[channel as usize];

        self.command.write(command.0);
        let lsb = channel.read();
        let msb = channel.read();

        (msb as u16) << 8 | lsb as u16
    }

    /// # Safety
    /// Command must match write operation
    unsafe fn write(&mut self, command: Command, channel: Channel, value: u16) {
        let lsb = (value & (u8::MAX as u16)) as u8;
        let msb = (value >> 8) as u8;

        let channel = &mut self.channels[channel as usize];

        self.command.write(command.0);
        channel.write(lsb);
        channel.write(msb);
    }
}

// from https://gitlab.redox-os.org/redox-os/kernel/-/blob/master/src/arch/x86_64/device/pit.rs

// 1 / (1.193182 MHz) = 838,095,110 femtoseconds ~= 838.095 ns
const PERIOD_FS: u128 = 838_095_110;

// 4847 / (1.193182 MHz) = 4,062,247 ns ~= 4.1 ms or 246 Hz
const CHAN0_DIVISOR: u16 = 4847;

static PIT: Mutex<Pit> = Mutex::new(Pit::new());

pub fn init() {
    set_irq_masked(TIMER_INTERRUPT_INDEX, false);

    let mut command = Command::new();
    command.set_channel(Channel::Channel0);
    command.set_operating_mode(OperatingMode::Mode2);
    command.set_read_load_mode(ReadLoadMode::LsbThenMsB);

    let mut pit = PIT.lock();
    unsafe { pit.write(command, Channel::Channel0, CHAN0_DIVISOR) };
}
