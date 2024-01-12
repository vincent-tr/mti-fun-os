#[derive(Debug)]
#[repr(C)]
pub struct PhysStats {
    pub total: usize,
    pub free: usize,
}

#[derive(Debug)]
#[repr(C)]
pub struct KvmStats {
    /// KVM virtual space used
    pub used: usize,

    /// KVM total virtual space
    pub total: usize,
}

#[derive(Debug)]
#[repr(C)]
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

#[derive(Debug)]
#[repr(C)]
pub struct MemoryStats {
    /// Physical allocator stats
    pub phys: PhysStats,

    /// KVM space allocator stats
    pub kvm: KvmStats,

    /// Kernel allocator stats
    pub kalloc: KallocStats,
}
