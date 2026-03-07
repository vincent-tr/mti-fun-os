use core::{mem, ops::Range};
use libruntime::{drivers::pci::types::PciAddress, kobject, sync::spin::OnceLock};

use crate::pci::PCI_CONFIG_SPACE_SIZE;

/// PCI configuration space access
#[derive(Debug)]
pub struct ConfigurationSpace {
    address: kobject::PortRange,
    data: kobject::PortRange,
}

impl ConfigurationSpace {
    fn cell() -> &'static OnceLock<ConfigurationSpace> {
        static INSTANCE: OnceLock<ConfigurationSpace> = OnceLock::new();

        &INSTANCE
    }

    /// Get the current configuration space
    pub fn get() -> &'static Self {
        Self::cell()
            .get()
            .expect("configuration space not initialized")
    }

    /// Initializes the PCI configuration space access ports.
    ///
    /// # Safety
    /// - This must be called before any other methods on `ConfigurationSpace` are used.
    /// - It should only be called once.
    pub unsafe fn init() {
        const CONFIG_ADDRESS: u16 = 0xCF8;
        const CONFIG_DATA: u16 = 0xCFC;

        let address = kobject::PortRange::open(
            CONFIG_ADDRESS,
            1,
            kobject::PortAccess::READ | kobject::PortAccess::WRITE,
        )
        .expect("Failed to open CONFIG_ADDRESS port range");
        let data = kobject::PortRange::open(
            CONFIG_DATA,
            4,
            kobject::PortAccess::READ | kobject::PortAccess::WRITE,
        )
        .expect("Failed to open CONFIG_DATA port range");

        Self::cell()
            .set(Self { address, data })
            .expect("Failed to initialize configuration space");
    }

    /// Reads a value of type `T` from the PCI configuration space of the specified address and offset.
    pub fn read_data<T>(
        &self,
        address: PciAddress,
        offset: usize,
        data: &mut T,
        partial: Option<Range<usize>>,
    ) {
        const {
            assert!(
                mem::size_of::<T>() % mem::size_of::<u32>() == 0,
                "Type must be a multiple of 32 bits"
            );

            assert!(
                mem::align_of::<T>() % mem::size_of::<u32>() == 0,
                "Type must be aligned to size of u32"
            );
        }

        assert!(
            offset as usize + mem::size_of::<T>() <= PCI_CONFIG_SPACE_SIZE,
            "Offset + size of type must be within 256 bytes"
        );

        assert!(
            offset % mem::size_of::<u32>() == 0,
            "Offset must be aligned to size of u32"
        );

        if let Some(partial) = &partial {
            assert!(
                partial.start < partial.end && partial.end <= mem::size_of::<T>(),
                "Invalid partial range"
            );

            assert!(
                partial.start % mem::size_of::<u32>() == 0
                    && partial.end % mem::size_of::<u32>() == 0,
                "Partial range must be aligned to size of u32"
            );
        }

        let data_buffer = unsafe {
            core::slice::from_raw_parts_mut(
                data as *mut T as *mut u32,
                mem::size_of::<T>() / mem::size_of::<u32>(),
            )
        };

        let indexes = if let Some(range) = &partial {
            let start = range.start / mem::size_of::<u32>();
            let end = range.end / mem::size_of::<u32>();
            start..end
        } else {
            0..data_buffer.len()
        };

        for index in indexes {
            let offset = offset + index * mem::size_of::<u32>();
            data_buffer[index] = self.read_u32(address, offset);
        }
    }

    /// Reads a 32-bit value from the PCI configuration space of the specified address and offset.
    pub fn read_u32(&self, address: PciAddress, offset: usize) -> u32 {
        assert!(
            offset % mem::size_of::<u32>() == 0,
            "Offset must be aligned to size of u32"
        );

        assert!(
            offset as usize + mem::size_of::<u32>() <= PCI_CONFIG_SPACE_SIZE,
            "Offset + size of type must be within 256 bytes"
        );

        // https://wiki.osdev.org/PCI#Configuration_Space_Access_Mechanism_#1
        let address = (1 << 31)
            | ((address.bus as u32) << 16)
            | ((address.device as u32) << 11)
            | ((address.function as u32) << 8)
            | (offset as u32);

        self.address
            .write32(0, address)
            .expect("Failed to write to PCI configuration address port");
        self.data
            .read32(0)
            .expect("Failed to read from PCI configuration space")
    }

    /// Writes a 32-bit value to the PCI configuration space of the specified address and offset.
    pub fn write_u32(&self, address: PciAddress, offset: usize, value: u32) {
        assert!(
            offset % mem::size_of::<u32>() == 0,
            "Offset must be aligned to size of u32"
        );

        assert!(
            offset as usize + mem::size_of::<u32>() <= PCI_CONFIG_SPACE_SIZE,
            "Offset + size of type must be within 256 bytes"
        );

        let address = (1 << 31)
            | ((address.bus as u32) << 16)
            | ((address.device as u32) << 11)
            | ((address.function as u32) << 8)
            | (offset as u32); // & 0xFC => unnecessary since we assert that offset is aligned to 4 bytes

        self.address
            .write32(0, address)
            .expect("Failed to write to PCI configuration address port");
        self.data
            .write32(0, value)
            .expect("Failed to write to PCI configuration space");
    }
}
