use core::alloc::{GlobalAlloc, Layout};

use spin::Mutex;

use super::Dlmalloc;

#[global_allocator]
pub static ALLOC: GlobalDlmalloc = GlobalDlmalloc::new();

/// An instance of a "global allocator" backed by `Dlmalloc`
///
/// This API requires the `global` feature is activated, and this type
/// implements the `GlobalAlloc` trait in the standard library.
pub struct GlobalDlmalloc(Mutex<Dlmalloc>);

impl GlobalDlmalloc {
    pub const fn new() -> Self {
        Self(Mutex::new(Dlmalloc::new()))
    }
}

unsafe impl GlobalAlloc for GlobalDlmalloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.0.lock();
        allocator.malloc(layout.size(), layout.align())
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.0.lock();
        allocator.free(ptr, layout.size(), layout.align())
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.0.lock();
        allocator.calloc(layout.size(), layout.align())
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let mut allocator = self.0.lock();
        allocator.realloc(ptr, layout.size(), layout.align(), new_size)
    }
}
