use alloc::alloc;
use core::{
    alloc::Layout,
    ptr::{self, NonNull},
};

/// Align address upwards.
pub fn align_up(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (addr + align - 1) & !(align - 1)
}

/// Align address downwards.
pub fn align_down(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    addr & !(align - 1)
}

/// A helper struct to manage aligned buffers.
#[derive(Debug)]
pub struct AlignedBuffer {
    ptr: NonNull<u8>,
    len: usize,
    layout: Layout,
}

impl AlignedBuffer {
    /// Creates a new aligned buffer with the given size and alignment.
    ///
    /// # Safety
    ///
    /// The buffer is not initialized, and may contain arbitrary data. The caller is responsible for initializing it before use.
    pub unsafe fn new_uninit(size: usize, align: usize) -> Self {
        assert!(size > 0, "Size must be greater than zero");
        assert!(align.is_power_of_two(), "Alignment must be a power of two");
        let layout =
            Layout::from_size_align(size, align).expect("Invalid layout for aligned buffer");

        let ptr = unsafe { alloc::alloc(layout) };
        if ptr.is_null() {
            panic!("Failed to allocate aligned buffer");
        }

        Self {
            // Safety: we checked that the pointer is not null above.
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            len: size,
            layout,
        }
    }

    /// Creates a new aligned buffer with the given size and alignment, and initializes it to zero.
    pub fn new(size: usize, align: usize) -> Self {
        // Safety: we will initialize the buffer after allocation.
        let buffer = unsafe { Self::new_uninit(size, align) };
        unsafe { ptr::write_bytes(buffer.ptr.as_ptr(), 0, size) };
        buffer
    }

    /// Creates a new aligned buffer from a byte slice, copying the data.
    pub fn from_slice(slice: &[u8], align: usize) -> Self {
        // Safety: we will initialize the buffer after allocation.
        let buffer = unsafe { Self::new_uninit(slice.len(), align) };
        unsafe { ptr::copy_nonoverlapping(slice.as_ptr(), buffer.ptr.as_ptr(), slice.len()) };
        buffer
    }

    /// Returns the buffer as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Returns the buffer as a mutable byte slice.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}
