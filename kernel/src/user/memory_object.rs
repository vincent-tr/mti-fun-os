use core::slice::Iter;

use crate::memory::{is_page_aligned, phys_allocate, FrameRef, PAGE_SIZE};
use alloc::{sync::Arc, vec::Vec};

use super::{error::*, Error};

/// Represent a area in physical memory, that can be mapped into processes
pub struct MemoryObject {
    pages: Vec<FrameRef>,
}

impl MemoryObject {
    /// Create a new memory object of the given size
    pub fn new(size: usize) -> Result<Arc<Self>, Error> {
        check_page_alignment(size as u64)?;
        check_positive(size as u64)?;

        let page_count = size / PAGE_SIZE;
        let mut object = MemoryObject { pages: Vec::new() };

        for _ in 0..page_count {
            match phys_allocate() {
                Some(frame) => {
                    object.pages.push(frame);
                }

                None => {
                    // Dropping the list of pages will drop all frames created so far
                    return Err(out_of_memory());
                }
            }
        }

        return Ok(Arc::new(object));
    }

    /// Get the size of the memory object
    pub fn size(&self) -> usize {
        self.pages.len() * PAGE_SIZE
    }

    /// Iterates over the physical frames of the memory object
    pub fn frames_iter(&self) -> Iter<'_, FrameRef> {
        self.pages.iter()
    }

    /// Get a particular physical frame of he memory object
    pub fn frame(&self, offset: usize) -> &FrameRef {
        assert!(is_page_aligned(offset));
        assert!(offset < self.size());
        &self.pages[offset / PAGE_SIZE]
    }
}
