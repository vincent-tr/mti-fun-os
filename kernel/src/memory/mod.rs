mod buddy;
mod kalloc;
mod kvm;
mod paging;
mod phys;
mod slab;

mod config;
use bootloader_api::info::MemoryRegions;
pub use config::PAGE_SIZE;
use log::info;
pub use paging::{set_current_address_space, AddressSpace};
use x86_64::VirtAddr;

pub fn init(phys_mapping: VirtAddr, memory_regions: &MemoryRegions) {
    phys::init(phys_mapping, memory_regions);
    paging::init(phys_mapping);

    // Note:
    // boot_info is unmapped from here.
    // Do not used it. (memory_regions)

    kvm::init();

    let stats = stats();
    const MEGA: usize = 1 * 1024 * 1024;
    info!("Memory allocator initialized. Initial stats:");
    info!(
        "phys: total={} ({}MB), free={} ({}MB)",
        stats.phys.total,
        stats.phys.total / MEGA,
        stats.phys.free,
        stats.phys.free / MEGA
    );
    info!(
        "kvm: total={} ({:#X}), used={} ({:#X})",
        stats.kvm.total, stats.kvm.total, stats.kvm.used, stats.kvm.used
    );
    info!(
        "kalloc: slabs: user={} ({}MB), allocated={} ({}MB)",
        stats.kalloc.slabs_user,
        stats.kalloc.slabs_user / MEGA,
        stats.kalloc.slabs_allocated,
        stats.kalloc.slabs_allocated / MEGA
    );
    info!(
        "kalloc: kvm: user={} ({}MB), allocated={} ({}MB)",
        stats.kalloc.kvm_user,
        stats.kalloc.kvm_user / MEGA,
        stats.kalloc.kvm_allocated,
        stats.kalloc.kvm_allocated / MEGA
    );
}

#[derive(Debug)]
pub struct PhysStats {
    pub total: usize,
    pub free: usize,
}

#[derive(Debug)]
pub struct KvmStats {
    /// KVM virtual space used
    pub used: usize,

    /// KVM total virtual space
    pub total: usize,
}

#[derive(Debug)]
pub struct KallocStats {
    /// Size of all requests currently served by the slabs allocator
    pub slabs_user: usize,

    /// Size actually allocated to serve requests by the slabs allocator
    pub slabs_allocated: usize,

    /// Size of all requests currently served by the kvm allocator
    pub kvm_user: usize,

    /// Size actually allocated to serve requests by the kvm allocator
    pub kvm_allocated: usize,
}

pub struct Stats {
    /// Physical allocator stats
    pub phys: PhysStats,

    /// KVM space allocator stats
    pub kvm: KvmStats,

    /// Kernel allocator stats
    pub kalloc: KallocStats,
}

pub fn stats() -> Stats {
    Stats {
        phys: phys::stats(),
        kvm: kvm::stats(),
        kalloc: kalloc::ALLOC.stats(),
    }
}
