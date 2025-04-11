mod buddy;
mod config;
mod kalloc;
mod kvm;
mod paging;
mod phys;
mod slab;

use core::cell::RefCell;
use core::fmt::{self, Debug};
use core::ops::Range;
use core::{mem, slice};

use alloc::format;
use bootloader_api::info::MemoryRegions;
use log::info;

pub use config::{KERNEL_START, PAGE_SIZE};
pub use paging::{
    create_adress_space, drop_initial_kernel_stack, drop_initial_ramdisk,
    set_current_address_space, AdditionalFlags, AddressSpace, Permissions,
};
pub use phys::{AllocatorError, FrameRef};
use x86_64::structures::paging::{mapper::MapToError, Size4KiB};
pub use x86_64::{align_down, align_up, PhysAddr, VirtAddr};

pub type MapError = MapToError<Size4KiB>;
pub use syscalls::{KallocStats, KvmStats, MemoryStats, PhysStats};
pub use x86_64::structures::paging::mapper::UnmapError;

use config::KERNEL_STACK_SIZE;
use paging::phys_to_virt;

pub fn init(phys_mapping: VirtAddr, memory_regions: &MemoryRegions, ramdisk: &Range<usize>) {
    phys::init(phys_mapping, memory_regions);
    paging::init(phys_mapping, ramdisk);

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

pub fn stats() -> MemoryStats {
    MemoryStats {
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

/// Helper to permit to view a physical page.
///
/// Return a virtual address that corresponds to a view of the physical address
#[inline]
pub fn view_phys<'a>(frame: &'a FrameRef) -> &'a [u8] {
    let addr = phys_to_virt(frame.frame());
    unsafe { slice::from_raw_parts(addr.as_ptr(), PAGE_SIZE) }
}

/// Helper to permit to access a physical page.
///
/// Return a virtual address that corresponds to a view of the physical address
///
/// # Safety
/// - Concurrent mutable accesses are not checked.
/// - Caller must ensure that no other access or view on the page occurs at the same time.
///
pub unsafe fn access_phys<'a>(frame: &'a FrameRef) -> &'a mut [u8] {
    let addr = phys_to_virt(frame.frame());
    slice::from_raw_parts_mut(addr.as_mut_ptr(), PAGE_SIZE)
}

/// Helper to permit map a set of physical pages into kernel space.
///
/// This is different from `access_phys` because this create a contigous kernel VM space with all phys frames.
///
/// This permits to access structures which can be laid out across a page boundary.
///
/// Return a virtual address that corresponds to the start of the mapping.
///
/// # Safety
/// - Concurrent mutable accesses are not checked.
/// - Must be freed with unmap_phys
///
pub unsafe fn map_phys(frames: &mut [FrameRef]) -> Option<VirtAddr> {
    match kvm::allocate_with_frames(frames) {
        Ok(addr) => Some(addr),
        Err(err) => {
            // Ensure all arms are matched
            match err {
                kvm::AllocatorError::NoMemory => None,
                kvm::AllocatorError::NoVirtualSpace => None,
            }
        }
    }
}

/// unmap kernel VM space previously mapped with `map_phys`
pub fn unmap_phys(addr: VirtAddr, frame_count: usize) {
    kvm::deallocate(addr, frame_count);
}

/// Helper to permit to map a set of physical iomem pages into kernel space
///
/// iomem is the area of physical address space that is not backup by RAM, and not managed by the physical memory allocator.
///
/// # Safety
/// The iomem has currently no allocator, no concurrent reservations are unchecked.
pub unsafe fn map_iomem(phys_frames: Range<PhysAddr>, perms: Permissions) -> Option<VirtAddr> {
    assert!(phys_frames.start.is_aligned(PAGE_SIZE as u64));
    let len = (phys_frames.end - phys_frames.start) as usize;
    assert!(is_page_aligned(len));

    match kvm::allocate_iomem(phys_frames.start, len / PAGE_SIZE, perms) {
        Ok(addr) => Some(addr),
        Err(err) => {
            // Ensure all arms are matched
            match err {
                kvm::AllocatorError::NoMemory => None,
                kvm::AllocatorError::NoVirtualSpace => None,
            }
        }
    }
}

/// unmap kernel VM space previously mapped with `map_iomem`
pub fn unmap_iomem(addr: VirtAddr, frame_count: usize) {
    kvm::deallocate_iomem(addr, frame_count);
}

/// Structure that defines a kernel stack
///
/// Note:
/// - align(16) to be able to use it as interrupt stack
///
/// TODO: guards
#[repr(align(16))]
pub struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
    end: [u8; 0],
}

impl KernelStack {
    pub const fn new() -> Self {
        Self {
            data: [0; KERNEL_STACK_SIZE],
            end: [0; 0],
        }
    }

    pub fn address(&self) -> VirtAddr {
        VirtAddr::new_truncate(self.data.as_ptr() as u64)
    }

    pub fn stack_top(&self) -> VirtAddr {
        VirtAddr::new_truncate(self.end.as_ptr() as u64)
    }
}

impl Debug for KernelStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut writer = f.debug_struct("KernelStack");

        for counter in 0..50 {
            let addr = self.stack_top() - (((counter + 1) * mem::size_of::<u64>()) as u64);
            let value = unsafe { *addr.as_ptr::<u64>() };

            writer.field(&format!("{counter}"), &format_args!("{value:#016x}"));
        }

        writer.finish()
    }
}

/// This forces allocation in kernel binary in RW data section
#[derive(Debug)]
pub struct StaticKernelStack(RefCell<KernelStack>);

unsafe impl Sync for StaticKernelStack {}
unsafe impl Send for StaticKernelStack {}

impl StaticKernelStack {
    pub const fn new() -> Self {
        StaticKernelStack(RefCell::new(KernelStack::new()))
    }

    pub fn address(&self) -> VirtAddr {
        self.0.borrow().address()
    }

    pub fn stack_top(&self) -> VirtAddr {
        self.0.borrow().stack_top()
    }
}
