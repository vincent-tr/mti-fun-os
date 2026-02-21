#![no_std]
#![no_main]

use libruntime::kobject;
use log::{debug, info};

extern crate alloc;
extern crate libruntime;

#[no_mangle]
pub fn main() -> i32 {
    info!("Time server started");

    // https://wiki.osdev.org/CMOS#The_Real-Time_Clock
    //
    // The second alternative is to be prepared for dodgy/inconsistent values and cope with them if they occur.
    // To do this, make sure the "Update in progress" flag is clear (e.g. "while(update_in_progress_flag != clear)") then read all the time and date registers;
    // then make sure the "Update in progress" flag is clear again (e.g. "while(update_in_progress_flag != clear)") and read all the time and date registers again.
    // If the values that were read the first time are the same as the value that were read the second time then the values must be correct.
    // If any of the values are different you need to do it again, and keep doing it again until the newest values are the same as the previous values.

    let rtc_ports = RtcPorts::new();

    while rtc_ports.read_status_register_a().update_in_progress() {}
    let mut data = rtc_ports.read_rtc_data();

    loop {
        let last_data = data;

        while rtc_ports.read_status_register_a().update_in_progress() {}
        data = rtc_ports.read_rtc_data();

        if data == last_data {
            break;
        }
    }

    let metadata = rtc_ports.read_status_register_b();

    data.decode(metadata.is_binary_mode(), metadata.is_hour_format_24());

    debug!("binary mode: {}", metadata.is_binary_mode());
    debug!("24-hour format: {}", metadata.is_hour_format_24());
    debug!("Current RTC data: {:?}", data);

    loop {
        libruntime::timer::sleep(libruntime::timer::Duration::from_seconds(1));
    }
}

#[derive(Debug)]
struct RtcPorts {
    ports: kobject::PortRange,
}

impl RtcPorts {
    const NMI_DISABLED: bool = false;

    const CONTROL_PORT: u16 = 0;
    const DATA_PORT: u16 = 1;

    const RTC_REG_SECONDS: u8 = 0x00;
    const RTC_REG_MINUTES: u8 = 0x02;
    const RTC_REG_HOURS: u8 = 0x04;
    const RTC_REG_DAY: u8 = 0x07;
    const RTC_REG_MONTH: u8 = 0x08;
    const RTC_REG_YEAR: u8 = 0x09;
    const RTC_REG_STATUS_A: u8 = 0x0A;
    const RTC_REG_STATUS_B: u8 = 0x0B;

    /// Creates a new `RtcPorts` instance by opening the necessary I/O ports.
    pub fn new() -> Self {
        let ports = kobject::PortRange::open(
            0x70_u16,
            2,
            kobject::PortAccess::READ | kobject::PortAccess::WRITE,
        )
        .expect("Failed to open I/O ports for RTC");

        Self { ports }
    }

    /// Reads a value from the specified RTC register.
    fn read_data(&self, reg: u8) -> u8 {
        let value = if Self::NMI_DISABLED { reg | 0x80 } else { reg };

        self.ports
            .write8(Self::CONTROL_PORT, value)
            .expect("Failed to write RTC control port");
        self.ports
            .read8(Self::DATA_PORT)
            .expect("Failed to read RTC data")
    }

    /// Read status register A
    pub fn read_status_register_a(&self) -> StatusRegisterA {
        let value = self.read_data(Self::RTC_REG_STATUS_A);
        StatusRegisterA(value)
    }

    /// Read status register B
    pub fn read_status_register_b(&self) -> StatusRegisterB {
        let value = self.read_data(Self::RTC_REG_STATUS_B);
        StatusRegisterB(value)
    }

    /// Reads the current date and time from the RTC.
    pub fn read_rtc_data(&self) -> RtcData {
        RtcData {
            seconds: self.read_data(Self::RTC_REG_SECONDS),
            minutes: self.read_data(Self::RTC_REG_MINUTES),
            hours: self.read_data(Self::RTC_REG_HOURS),
            day: self.read_data(Self::RTC_REG_DAY),
            month: self.read_data(Self::RTC_REG_MONTH),
            year: self.read_data(Self::RTC_REG_YEAR),
        }
    }
}

/// RTC Status Register A
#[derive(Debug, Clone, Copy)]
struct StatusRegisterA(u8);

impl StatusRegisterA {
    /// Check if an update is in progress.
    pub fn update_in_progress(&self) -> bool {
        self.0 & 0x80 != 0
    }
}

/// RTC Status Register B
#[derive(Debug, Clone, Copy)]
struct StatusRegisterB(u8);

impl StatusRegisterB {
    /// Check if the RTC is in binary mode (as opposed to BCD).
    pub fn is_binary_mode(&self) -> bool {
        self.0 & 0x04 != 0
    }

    /// Check if the RTC is in 24-hour mode (as opposed to 12-hour mode).
    pub fn is_hour_format_24(&self) -> bool {
        self.0 & 0x02 != 0
    }
}

/// RTC data structure representing the current date and time.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RtcData {
    seconds: u8,
    minutes: u8,
    hours: u8,
    day: u8,
    month: u8,
    year: u8,
}

impl RtcData {
    /// Decodes the RTC data based on the provided metadata (binary mode and hour format).
    pub fn decode(&mut self, is_binary_mode: bool, is_24_hour_format: bool) {
        if !is_binary_mode {
            self.seconds = Self::bcd_to_binary(self.seconds);
            self.minutes = Self::bcd_to_binary(self.minutes);
            self.hours = Self::bcd_to_binary(self.hours & 0x7F) + (self.hours & 0x80); // Mask out PM bit for conversion
            self.day = Self::bcd_to_binary(self.day);
            self.month = Self::bcd_to_binary(self.month);
            self.year = Self::bcd_to_binary(self.year);
        }

        if !is_24_hour_format && (self.hours & 0x80 != 0) {
            self.hours = ((self.hours & 0x7F) + 12) % 24;
        }
    }

    fn bcd_to_binary(value: u8) -> u8 {
        ((value / 16) * 10) + (value & 0x0F)
    }
}
