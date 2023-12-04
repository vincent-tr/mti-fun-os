use alloc::collections::LinkedList;

use crate::memory;

use super::mapping::Mapping;

/// Process
pub struct Process {
  address_space: memory::AddressSpace,
  mappings: LinkedList<Mapping>,
}

impl Process {
  /// Get the address space of the process
  pub fn address_space(&self) -> &memory::AddressSpace {
    &mut self.address_space
  }

  /// Get the address space of the process
  pub fn address_space_mut(&mut self) -> &mut memory::AddressSpace {
    &mut self.address_space
  }
}