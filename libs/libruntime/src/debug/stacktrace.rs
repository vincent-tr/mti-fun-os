use alloc::{boxed::Box, vec::Vec};
use core::{arch::asm, ops::Index};

use super::{LocationInfo, find_location_info};

// from https://wiki.osdev.org/Stack_Trace

// Note: this only work if "force-frame-pointers" compiler option is set
#[derive(Debug)]
struct FrameWalker {
    rbp: *mut FrameWalker,
    rip: usize,
}

impl FrameWalker {
    pub fn from_current() -> &'static FrameWalker {
        let mut frame_ptr: *mut FrameWalker;
        unsafe {
            asm!(
                "mov {frame_ptr}, rbp",
                frame_ptr = out(reg) frame_ptr,
                options(nomem, preserves_flags, nostack)
            );
        }

        unsafe { &*frame_ptr }
    }

    pub fn valid(&self) -> bool {
        // This is our limit.
        // Note: if we access rip now, it will point out of the stack, and the access will PageFault
        !self.rbp.is_null()
    }

    pub fn next(&self) -> &'static Self {
        assert!(self.valid());

        unsafe { &*self.rbp }
    }

    pub fn rip(&self) -> usize {
        assert!(self.valid());

        self.rip
    }
}

/// Stacktrace of a thread
#[derive(Debug)]
pub struct StackTrace(Box<[StackFrame]>);

impl StackTrace {
    /// Capture the stacktrace of the current thread
    pub fn capture() -> Self {
        let mut frames = Vec::new();
        let mut walker = FrameWalker::from_current();

        while walker.valid() {
            // RIP points to the following instruction.
            // We want to point to the call itself, so - 1
            let ip = walker.rip() - 1;

            frames.push(StackFrame(ip));

            walker = walker.next()
        }

        Self(frames.into_boxed_slice())
    }

    // TODO: capture from thread context (thread in error state)

    /// Iterate over frames
    pub fn iter(&self) -> core::slice::Iter<'_, StackFrame> {
        self.0.iter()
    }

    /// Get the number of frames in the stacktrace
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Skip the X first frames
    ///
    /// This construct a new StackTrace object
    pub fn skip(&self, count: usize) -> StackTrace {
        StackTrace(self.0[count..].to_vec().into_boxed_slice())
    }
}

impl Index<usize> for StackTrace {
    type Output = StackFrame;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

/// Represent a frame of a stacktrace
#[derive(Debug, Clone, Copy)]
pub struct StackFrame(usize);

impl StackFrame {
    /// Get the address this frame represents
    pub fn address(&self) -> usize {
        self.0
    }

    /// Get the location information of the frame
    pub fn location(&self) -> Option<LocationInfo<'_>> {
        find_location_info(self.address())
    }
}
