use libruntime::net::dev::iface::NetDeviceError;
use log::{error, info};

use crate::{device::DeviceData, registers};

const MAX_LOCK_ATTEMPTS: usize = 1000;
const MAX_READ_ATTEMPTS: usize = 1000;

/// Access to the EEPROM
#[derive(Debug)]
pub struct EepromAccess<'a> {
    dev_data: &'a DeviceData,
}

impl Drop for EepromAccess<'_> {
    fn drop(&mut self) {
        let mut control: registers::EepromControlData = self
            .dev_data
            .mmio_read(registers::EepromControlData::OFFSET);
        control.set_access_request(false);
        self.dev_data
            .mmio_write(registers::EepromControlData::OFFSET, control);
    }
}

impl<'a> EepromAccess<'a> {
    pub fn acquire(dev_data: &'a DeviceData) -> Result<Self, NetDeviceError> {
        let mut control: registers::EepromControlData =
            dev_data.mmio_read(registers::EepromControlData::OFFSET);

        if !control.present() {
            error!("EEPROM not present");
            return Err(NetDeviceError::DeviceError);
        }

        control.set_access_request(true);
        dev_data.mmio_write(registers::EepromControlData::OFFSET, control);

        let mut granted = false;
        for _ in 0..MAX_LOCK_ATTEMPTS {
            let control: registers::EepromControlData =
                dev_data.mmio_read(registers::EepromControlData::OFFSET);
            if control.access_grant() {
                granted = true;
                break;
            }

            core::hint::spin_loop();
        }

        if !granted {
            info!("Could not acquire EEPROM lock, consider it is not implemented");
        }

        Ok(Self { dev_data })
    }

    pub fn read(&self, address: u16) -> Result<u16, NetDeviceError> {
        let mut eerd = registers::EepromRead::default();
        eerd.set_address(address);
        eerd.set_start(true);
        self.dev_data
            .mmio_write(registers::EepromRead::OFFSET, eerd);

        for _ in 0..MAX_READ_ATTEMPTS {
            let eerd: registers::EepromRead =
                self.dev_data.mmio_read(registers::EepromRead::OFFSET);

            if eerd.done() {
                return Ok(eerd.data());
            }

            core::hint::spin_loop();
        }

        error!("EEPROM read timeout for address {:#x}", address);
        Err(NetDeviceError::DeviceError)
    }
}
