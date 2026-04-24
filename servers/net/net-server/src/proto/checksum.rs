use core::{mem, slice};

/// IP checksum computer
#[derive(Debug)]
pub struct Checksum {
    sum: u32,
    carry: Option<u8>, // leftover byte
}

impl Checksum {
    /// Create a new Checksum computer instance
    pub fn new() -> Self {
        Self {
            sum: 0,
            carry: None,
        }
    }

    /// Add the data of the provided object
    pub fn update<T>(&mut self, data: &T) {
        let buffer =
            unsafe { slice::from_raw_parts((data as *const T) as *const u8, mem::size_of::<T>()) };
    }

    /// Add the data of the packet
    pub fn update_packet_view<'a>(&mut self, packet: impl Iterator<Item = &'a [u8]>) {
        for buffer in packet {
            self.update_data(buffer);
        }
    }

    /// Add data
    pub fn update_data(&mut self, buf: &[u8]) {
        if buf.is_empty() {
            return;
        }

        let mut i = 0;

        // Handle carry from previous chunk
        if let Some(prev) = self.carry.take() {
            let word = u16::from_be_bytes([prev, buf[0]]) as u32;
            self.sum += word;
            i = 1;
        }

        // Process aligned 16-bit words
        while i + 1 < buf.len() {
            let word = u16::from_be_bytes([buf[i], buf[i + 1]]) as u32;
            self.sum += word;
            i += 2;
        }

        // Save leftover byte
        if i < buf.len() {
            self.carry = Some(buf[i]);
        }
    }

    /// Finalize the computation and get the result
    pub fn finalize(mut self) -> u16 {
        // Handle trailing byte
        if let Some(last) = self.carry {
            let word = u16::from_be_bytes([last, 0]) as u32;
            self.sum += word;
        }

        // Fold 32 → 16
        while (self.sum >> 16) != 0 {
            self.sum = (self.sum & 0xFFFF) + (self.sum >> 16);
        }

        !(self.sum as u16)
    }
}
