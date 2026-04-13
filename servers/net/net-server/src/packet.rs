use core::{mem, ops::Range, slice};

use alloc::{sync::Arc, vec::Vec};
use libruntime::net::dev::iface::RxBufferDescriptor;
use smallvec::SmallVec;

use crate::buffer_pool::Buffer;

/// A packet is a collection of buffers that together represent a network packet.
/// It may consist of multiple buffers if the packet is large or if it is fragmented.
/// The `Packet` struct provides methods to access the underlying buffers and their data.
#[derive(Debug)]
pub struct Packet {
    buffers: SmallVec<[BufferData; 4]>,
    len: usize,
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

    /// Returns a slice of the buffer data for the specified range.
    pub fn slice(&self, range: Range<usize>) -> BufferData {
        let start = self.range.start + range.start;
        let end = self.range.start + range.end;
        BufferData::new(self.buffer.clone(), start..end)
    }

    /// Returns a slice of the buffer data for the entire range.
    pub fn view(&self) -> &[u8] {
        &self.buffer.view()[self.range.clone()]
    }
}

impl Packet {
    /// Creates a new `Packet` from a vector of `BufferData`.
    pub fn new(buffers: SmallVec<[BufferData; 4]>) -> Self {
        let len = buffers.iter().map(|b| b.range.len()).sum();
        Self { buffers, len }
    }

    /// Returns the length of the packet.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns a slice of the packet data for the specified range.
    pub fn slice(&self, range: Range<usize>) -> Packet {
        let mut remaining = range;
        let mut new_buffers = SmallVec::new();

        for buffer in &self.buffers {
            if remaining.is_empty() {
                break;
            }

            let buffer_len = buffer.range.len();
            if remaining.start >= buffer_len {
                remaining.start -= buffer_len;
                remaining.end -= buffer_len;
                continue;
            }

            let slice_start = remaining.start;
            let slice_end = usize::min(remaining.end, buffer_len);
            new_buffers.push(buffer.slice(slice_start..slice_end));
            remaining.start = 0;
            remaining.end -= slice_end - slice_start;
        }

        Packet::new(new_buffers)
    }
}

/// A `PacketCursor` allows iterating over data in a `Packet`.
#[derive(Debug)]
pub struct PacketCursor<'a> {
    packet: &'a Packet,
    buffer_index: usize,
    buffer_offset: usize,
}

impl<'a> PacketCursor<'a> {
    /// Creates a new `PacketCursor` for the given packet.
    pub fn new(packet: &'a Packet) -> Self {
        Self {
            packet,
            buffer_index: 0,
            buffer_offset: 0,
        }
    }

    /// Reads data of type `T` from the packet, returning `None` if there is not enough data left.
    pub fn read<T>(&mut self) -> Option<T> {
        let size = mem::size_of::<T>();
        let mut result = mem::MaybeUninit::<T>::uninit();
        // SAFETY: `result` has the correct size and alignment for `T`; we write exactly `size` bytes before `assume_init`.
        let dst = unsafe { slice::from_raw_parts_mut(result.as_mut_ptr() as *mut u8, size) };
        let mut written = 0;

        while written < size {
            // Advance past exhausted buffers.
            loop {
                if self.buffer_index >= self.packet.buffers.len() {
                    return None;
                }
                if self.buffer_offset < self.packet.buffers[self.buffer_index].range.len() {
                    break;
                }
                self.buffer_index += 1;
                self.buffer_offset = 0;
            }

            let buffer = &self.packet.buffers[self.buffer_index];
            let available = buffer.range.len() - self.buffer_offset;
            let to_copy = available.min(size - written);

            dst[written..written + to_copy]
                .copy_from_slice(&buffer.view()[self.buffer_offset..self.buffer_offset + to_copy]);

            written += to_copy;
            self.buffer_offset += to_copy;
        }

        // SAFETY: All `size` bytes have been initialised above.
        Some(unsafe { result.assume_init() })
    }

    /// Reads `len` bytes of data from the packet, returning `None` if there is not enough data left.
    pub fn read_data(&mut self, len: usize) -> Option<Packet> {
        let mut remaining = len;
        let mut buffers = SmallVec::new();

        while remaining > 0 {
            // Advance past exhausted buffers.
            loop {
                if self.buffer_index >= self.packet.buffers.len() {
                    return None;
                }
                if self.buffer_offset < self.packet.buffers[self.buffer_index].range.len() {
                    break;
                }
                self.buffer_index += 1;
                self.buffer_offset = 0;
            }

            let buffer = &self.packet.buffers[self.buffer_index];
            let available = buffer.range.len() - self.buffer_offset;
            let to_take = available.min(remaining);

            buffers.push(buffer.slice(self.buffer_offset..self.buffer_offset + to_take));

            remaining -= to_take;
            self.buffer_offset += to_take;
        }

        Some(Packet::new(buffers))
    }

    /// Reads the next chunk of data from the packet, where a chunk is defined as the remaining data in the current buffer.
    pub fn read_chunk(&mut self) -> &[u8] {
        // Advance past exhausted buffers.
        loop {
            if self.buffer_index >= self.packet.buffers.len() {
                return &[];
            }
            if self.buffer_offset < self.packet.buffers[self.buffer_index].range.len() {
                break;
            }
            self.buffer_index += 1;
            self.buffer_offset = 0;
        }

        let buffer = &self.packet.buffers[self.buffer_index];
        let chunk = &buffer.view()[self.buffer_offset..];

        self.buffer_index += 1;
        self.buffer_offset = 0;

        chunk
    }

    pub fn is_end(&mut self) -> bool {
        // Advance past exhausted buffers.
        loop {
            if self.buffer_index >= self.packet.buffers.len() {
                return true;
            }
            if self.buffer_offset < self.packet.buffers[self.buffer_index].range.len() {
                return false;
            }
            self.buffer_index += 1;
            self.buffer_offset = 0;
        }
    }
}
