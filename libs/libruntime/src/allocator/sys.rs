use libsyscalls::{memory_object, process, Error, Permissions};
use log::{error, trace};

use crate::kobject;

use super::Allocator;
use core::ptr;

/// System setting for mti-fun-os
pub struct System {}

impl System {
    pub const fn new() -> System {
        System {}
    }

    fn mmap(&self, size: usize) -> Result<*mut u8, Error> {
        trace!("mmap size={size}");

        let self_proc = process::open_self()?;

        let mobj = memory_object::create(size)?;
        let addr = process::mmap(
            &self_proc,
            None,
            size,
            Permissions::READ | Permissions::WRITE,
            Some(&mobj),
            0,
        )?;

        Ok(addr as *mut u8)
    }

    fn munmap(&self, addr: *mut u8, size: usize) -> Result<(), Error> {
        trace!("munmap addr={addr:?} size={size}");

        let self_proc = process::open_self()?;

        // memory object will be dropped when all mapping is removed
        let addr = addr as usize;
        process::munmap(&self_proc, &(addr..(addr + size)))
    }
}

unsafe impl Allocator for System {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        match self.mmap(size) {
            Ok(addr) => (addr, size, 0),
            Err(err) => {
                error!("Allocation failed: {:?}", err);
                (ptr::null_mut(), 0, 0)
            }
        }
    }

    fn remap(&self, _ptr: *mut u8, _oldsize: usize, _newsize: usize, _can_move: bool) -> *mut u8 {
        ptr::null_mut()
    }

    fn free_part(&self, ptr: *mut u8, oldsize: usize, newsize: usize) -> bool {
        match self.munmap(unsafe { ptr.offset(newsize as isize) }, oldsize - newsize) {
            Ok(()) => true,
            Err(err) => {
                error!("Deallocation failed: {:?}", err);
                false
            }
        }
    }

    fn free(&self, ptr: *mut u8, size: usize) -> bool {
        match self.munmap(ptr, size) {
            Ok(()) => true,
            Err(err) => {
                error!("Deallocation failed: {:?}", err);
                false
            }
        }
    }

    fn can_release_part(&self, _flags: u32) -> bool {
        true
    }

    fn allocates_zeros(&self) -> bool {
        true
    }

    fn page_size(&self) -> usize {
        kobject::PAGE_SIZE
    }
}
