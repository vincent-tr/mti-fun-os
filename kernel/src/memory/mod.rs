mod paging;
mod phys;
mod slab;
mod buddy;
mod kvm;
//mod kalloc;

mod config;
use bootloader_api::info::MemoryRegions;
pub use config::*;
pub use paging::{AddressSpace, set_current_address_space};
use x86_64::VirtAddr;

pub fn init(phys_mapping: VirtAddr, memory_regions: &MemoryRegions) {
  phys::init(phys_mapping, memory_regions);
  paging::init(phys_mapping);

  // Note:
  // boot_info is unmapped from here.
  // Do not used it. (memory_regions)

  kvm::init();
}
