use core::{arch::asm, mem::size_of};

use spin::{Mutex, MutexGuard};

use super::PAGE_SIZE;

pub const TLS_SIZE: usize = PAGE_SIZE;

pub struct TlsAllocator {
    data: Mutex<AllocatorData>,
}

struct AllocatorData {
    id_gen: usize,
    allocation_map: [bool; TlsAllocator::SLOT_COUNT],
}

impl TlsAllocator {
    /// Number of total slots
    pub const SLOT_COUNT: usize = TLS_SIZE / TlsSlot::SLOT_SIZE;

    /// Get the TLS allocator
    fn data() -> MutexGuard<'static, AllocatorData> {
        lazy_static::lazy_static! {
          static ref INSTANCE: TlsAllocator = TlsAllocator {
            data: Mutex::new(AllocatorData {
                id_gen: 0,
                allocation_map: [false; TlsAllocator::SLOT_COUNT],
            })
          };
        }

        INSTANCE.data.lock()
    }

    /// Allocate a TLS slot.
    ///
    /// If the return value is `None`, there is no more slot available
    pub fn allocate() -> Option<TlsSlot> {
        let mut data = Self::data();

        for (index, allocated) in data.allocation_map.iter_mut().enumerate() {
            if !*allocated {
                *allocated = true;
            }

            data.id_gen += 1;
            let seq = data.id_gen;

            return Some(TlsSlot { index, seq });
        }

        None
    }

    fn free_slot(index: usize) {
        let mut data = Self::data();

        data.allocation_map[index] = false;
    }
}

/// TLS slot. Can set or get a value per thread
pub struct TlsSlot {
    index: usize,
    seq: usize,
}

impl Drop for TlsSlot {
    fn drop(&mut self) {
        TlsAllocator::free_slot(self.index);
    }
}

impl TlsSlot {
    /// Get the value of the slot
    ///
    /// Returns None if no value has been set for this slot/thread
    pub fn get(&self) -> Option<usize> {
        let seq = unsafe { Self::get_tls_data(self.seq_offset()) };
        if seq == self.seq {
            let value = unsafe { Self::get_tls_data(self.value_offset()) };
            Some(value)
        } else {
            None
        }
    }

    /// Set the value of the slot
    pub fn set(&self, value: usize) {
        unsafe {
            Self::set_tls_data(self.seq_offset(), self.seq);
            Self::set_tls_data(self.value_offset(), value);
        }
    }

    // TLS slot layout: seq<8>, value<8>
    const SLOT_SIZE: usize = size_of::<(usize, usize)>();

    fn seq_offset(&self) -> usize {
        self.index * Self::SLOT_SIZE
    }

    fn value_offset(&self) -> usize {
        self.index * Self::SLOT_SIZE + size_of::<usize>()
    }

    unsafe fn set_tls_data(offset: usize, value: usize) {
        asm!(
            "mov fs:[{offset}], {value};",
            offset = in(reg)offset,
            value = in(reg)value,
            options(nostack, preserves_flags)
        );
    }

    unsafe fn get_tls_data(offset: usize) -> usize {
        let mut value: usize;

        asm!(
            "mov {value}, fs:[{offset}];",
            offset = in(reg)offset,
            value = out(reg)value,
            options(nostack, preserves_flags)
        );

        value
    }
}
