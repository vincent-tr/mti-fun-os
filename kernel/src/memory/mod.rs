mod buddy;
mod kalloc;
mod kvm;
mod paging;
mod phys;
mod slab;
mod config;

use bootloader_api::info::MemoryRegions;
use log::info;

use x86_64::structures::paging::{Size4KiB, mapper::MapToError};
pub use x86_64::{align_down, align_up, PhysAddr, VirtAddr};
pub use config::{PAGE_SIZE, KERNEL_START};
pub use paging::{create_adress_space, set_current_address_space, AddressSpace, Permissions};
pub use phys::{FrameRef, AllocatorError};

pub type MapError = MapToError<Size4KiB>;
pub use x86_64::structures::paging::mapper::UnmapError;

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

pub fn phys_allocate() -> Option<FrameRef> {
    match phys::allocate() {
        Ok(frame) => Some(frame),
        Err(err) => {
            // ensure all types are matched
            match err {
                phys::AllocatorError::NoMemory => None,
            }
        }
    }
}

/// Checks whether the address is in userspace.
#[inline]
pub fn is_userspace(addr: VirtAddr) -> bool {
    return addr < config::KERNEL_START;
}

/// Checks whether the address has the demanded alignment.
///
/// Panics if the alignment is not a power of two.
#[inline]
pub fn is_aligned(addr: u64, align: u64) -> bool {
    return align_down(addr, align) == addr;
}

/// Checks whether the address is aligned on PAGE_SIZE.
#[inline]
pub fn is_page_aligned(addr: usize) -> bool {
    return align_down(addr as u64, PAGE_SIZE as u64) == addr as u64;
}

/// Align address upwards.
///
/// Returns the smallest `x` with PAGE_SIZE alignment so that `x >= addr`.
#[inline]
pub fn page_aligned_up(addr: usize) -> usize {
    return align_up(addr as u64, PAGE_SIZE as u64) as usize;
}

/// Align address downwards on PAGE_SIZE.
///
/// Returns the greatest `x` with PAGE_SIZE alignment so that `x <= addr`.
#[inline]
pub fn page_aligned_down(addr: usize) -> usize {
    return align_down(addr as u64, PAGE_SIZE as u64) as usize;
}
