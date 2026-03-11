use core::{marker::PhantomData, mem};

use crate::{drivers::pci, kobject};

/// A helper for managing MMIO regions in device drivers.
#[derive(Debug)]
pub struct MmioRegion<Word> {
    mapping: kobject::Mapping<'static>,
    _word: PhantomData<Word>,
}

impl<Word> MmioRegion<Word> {
    /// Open a new MMIO region from a PCI memory BAR.
    pub fn from_bar(bar: &pci::MemoryBar) -> Result<Self, kobject::Error> {
        unsafe { Self::open(bar.address, bar.size) }
    }

    /// Open a new MMIO region by mapping the given physical address and size.
    pub unsafe fn open(phys_addr: usize, size: usize) -> Result<Self, kobject::Error> {
        let mobj = unsafe { kobject::MemoryObject::open_iomem(phys_addr, size, false, true)? };
        let proc = kobject::Process::current();
        let mapping = proc.map_mem(
            None,
            size,
            kobject::Permissions::READ | kobject::Permissions::WRITE,
            &mobj,
            0,
        )?;

        Ok(Self {
            mapping,
            _word: PhantomData,
        })
    }

    /// Read a word from the MMIO region at the given offset.
    pub fn read(&self, offset: usize) -> Word {
        assert!(
            offset % mem::size_of::<Word>() == 0,
            "Offset must be aligned to word size"
        );
        assert!(
            offset + mem::size_of::<Word>() <= self.mapping.len(),
            "Read out of bounds"
        );
        unsafe { core::ptr::read_volatile((self.mapping.address() + offset) as *const Word) }
    }

    /// Write a word to the MMIO region at the given offset.
    pub fn write(&self, offset: usize, value: Word) {
        assert!(
            offset % mem::size_of::<Word>() == 0,
            "Offset must be aligned to word size"
        );
        assert!(
            offset + mem::size_of::<Word>() <= self.mapping.len(),
            "Write out of bounds"
        );
        unsafe { core::ptr::write_volatile((self.mapping.address() + offset) as *mut Word, value) }
    }
}
