use core::slice::Iter;

use crate::memory::{is_page_aligned, phys_allocate, view_phys, FrameRef, PAGE_SIZE, access_phys};
use alloc::{sync::Arc, vec::Vec};

use super::{error::*, Error};

/// Represent a area in physical memory, that can be mapped into processes
#[derive(Debug)]
pub struct MemoryObject {
    pages: Vec<FrameRef>,
}

impl MemoryObject {
    /// Create a new memory object of the given size
    pub fn new(size: usize) -> Result<Arc<Self>, Error> {
        check_page_alignment(size)?;
        check_positive(size)?;

        let page_count = size / PAGE_SIZE;
        let mut object = Self { pages: Vec::new() };

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

        for page in object.pages.iter() {
            Self::zero_page(page);
        }

        return Ok(Arc::new(object));
    }

    /// Create a new memory object from a list of frames
    ///
    /// Note: frames will not be zeroed
    ///
    pub fn from_frames(frames: Vec<FrameRef>) -> Arc<Self> {
        Arc::new(Self { pages: frames })
    }

    fn zero_page(page: &FrameRef) {
        let page_data = unsafe { access_phys(page) };
        page_data.fill(0);
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
