use core::mem;
use core::marker::PhantomData;

use super::Object;

#[derive(Debug)]
pub struct RelocationTable<'a, Relocation> {
    base_address: usize,
    count: usize,
    _marker1: PhantomData<&'a ()>, // Note: this way we ensure that address remain valid
    _marker2: PhantomData<Relocation>,
}

impl<'a, Relocation> RelocationTable<'a, Relocation> {
    pub fn new(object: &Object<'a>, table_offset: usize, table_size: usize) -> Self {
        let base_address = object.addr_offset + table_offset;
        let count = table_size / mem::size_of::<Relocation>();

        Self {
            base_address,
            count,
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }

    pub fn size(&self) -> usize {
        self.count
    }

    pub fn entry(&self, index: usize) -> Relocation {
        assert!(index < self.count);

        let address = self.base_address + index * mem::size_of::<Relocation>();

        unsafe { core::ptr::read_unaligned(address as *const Relocation) }
    }

    pub fn iter(&self) -> RelocationTableIter<'a, Relocation> {
        RelocationTableIter {
            base_address: self.base_address,
            end_address: self.base_address + self.count * mem::size_of::<Relocation>(),
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct RelocationTableIter<'a, Relocation> {
    base_address: usize,
    end_address: usize,
    _marker1: PhantomData<&'a ()>, // Note: this way we ensure that address remain valid
    _marker2: PhantomData<Relocation>,
}

impl<'a, Relocation> Iterator for RelocationTableIter<'a, Relocation> {
    type Item = Relocation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.base_address == self.end_address {
            return None;
        }

        let entry = unsafe { core::ptr::read_unaligned(self.base_address as *const Relocation) };

        self.base_address += mem::size_of::<Relocation>();
        assert!(self.base_address <= self.end_address);

        Some(entry)
    }
}
