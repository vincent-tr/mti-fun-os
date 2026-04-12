use core::ops::Range;

use alloc::{sync::Arc, vec::Vec};
use libruntime::net::dev::iface::RxBufferDescriptor;

use crate::buffer_pool::Buffer;

/// A packet is a collection of buffers that together represent a network packet.
/// It may consist of multiple buffers if the packet is large or if it is fragmented.
/// The `Packet` struct provides methods to access the underlying buffers and their data.
#[derive(Debug)]
pub struct Packet {
    buffers: Vec<BufferData>,
}

/// A `BufferData` represents a single buffer and the range of data within that buffer that is part of the packet.
#[derive(Debug)]
pub struct BufferData {
    buffer: Arc<Buffer>,
    range: Range<usize>,
}

impl BufferData {
    /// Creates a new `BufferData` with the given buffer and range.
    pub fn new(buffer: Arc<Buffer>, range: Range<usize>) -> Self {
        Self { buffer, range }
    }
}

impl Packet {
    /// Creates a new `Packet` from a vector of `BufferData`.
    pub fn new(buffers: Vec<BufferData>) -> Self {
        Self { buffers }
    }

    /// Returns the length of the packet.
    pub fn len(&self) -> usize {
        self.buffers.iter().map(|b| b.range.len()).sum()
    }
}
