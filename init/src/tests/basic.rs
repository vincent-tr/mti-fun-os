use libruntime::kobject;
use log::debug;

/// Simple unwind test to verify panic handling
#[allow(dead_code)]
#[inline(never)]
pub fn test_unwind() {
    test_unwind2();
}

#[inline(never)]
fn test_unwind2() {
    test_unwind3();
}

#[inline(never)]
fn test_unwind3() {
    panic!("test unwind");
}

/// Display kernel memory allocator statistics
#[allow(dead_code)]
pub fn kmem_stats() {
    let stats = kobject::Memory::stats();
    const MEGA: usize = 1 * 1024 * 1024;
    debug!("Kernel memory allocator stats:");

    debug!(
        "phys: total={} ({}MB), free={} ({}MB)",
        stats.phys.total,
        stats.phys.total / MEGA,
        stats.phys.free,
        stats.phys.free / MEGA
    );
    debug!(
        "kvm: total={} ({:#X}), used={} ({:#X})",
        stats.kvm.total, stats.kvm.total, stats.kvm.used, stats.kvm.used
    );
    debug!(
        "kalloc: slabs: user={} ({}MB), allocated={} ({}MB)",
        stats.kalloc.slabs_user,
        stats.kalloc.slabs_user / MEGA,
        stats.kalloc.slabs_allocated,
        stats.kalloc.slabs_allocated / MEGA
    );
    debug!(
        "kalloc: kvm: user={} ({}MB), allocated={} ({}MB)",
        stats.kalloc.kvm_user,
        stats.kalloc.kvm_user / MEGA,
        stats.kalloc.kvm_allocated,
        stats.kalloc.kvm_allocated / MEGA
    );
}
