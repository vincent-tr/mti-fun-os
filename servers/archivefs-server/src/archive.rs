use alloc::sync::Arc;
pub use cpio_reader::Mode;
use libruntime::time::DateTime;

/// Represents an archive.
#[derive(Debug)]
pub struct Archive {
    buffer: Arc<[u8]>,
}

impl Archive {
    /// Constructs a new `Archive` from the given byte slice, copying the data into a boxed slice for ownership and immutability.
    ///
    /// Note that if the data are invalid (e.g., not a valid cpio archive, or containing unsupported file types), this function will still succeed,
    /// but the resulting `Archive` will have no entries.
    pub fn new(data: &[u8]) -> Self {
        Archive {
            buffer: Arc::from(data.to_vec().into_boxed_slice()),
        }
    }

    /// Returns a reference to the raw byte buffer of the archive, allowing clients to access the entire archive data if needed.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns an iterator over the entries in the archive, allowing clients to access each file's metadata and content.
    pub fn iter_entries(&self) -> impl Iterator<Item = ArchiveEntry> {
        cpio_reader::iter_files(&self.buffer).map(|entry| {
            // Safety: the cpio reader guarantees that the entry's name and file content are valid slices of the archive's buffer.
            unsafe { ArchiveEntry::new(self.buffer.clone(), entry) }
        })
    }
}

/// An archive entry represents a single file or directory in the archive, containing metadata such as the name, size, and type of the entry, as well as a reference to its content.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    buffer: Arc<[u8]>,
    name: ArchiveString,
    inode: u32,
    mode: Mode,
    mtime: DateTime,
    content: Option<ArchiveBuffer>,
}

impl ArchiveEntry {
    /// Constructs a new `ArchiveEntry` from the given cpio entry, extracting the relevant metadata and content information.
    ///
    /// # Safety
    /// - The caller must ensure that the provided cpio entry is valid and corresponds to a valid portion of the archive's buffer.
    pub unsafe fn new(buffer: Arc<[u8]>, entry: cpio_reader::Entry) -> Self {
        let name = unsafe { ArchiveString::new(buffer.clone(), entry.name()) };
        let mtime =
            DateTime::from_unix_timestamp(entry.mtime() as i64).expect("Invalid unxi timestamp");

        let content = if entry.file().len() > 0 {
            Some(unsafe { ArchiveBuffer::new(buffer.clone(), entry.file()) })
        } else {
            None
        };

        ArchiveEntry {
            buffer,
            name,
            inode: entry.ino(),
            mode: entry.mode(),
            mtime,
            content,
        }
    }

    /// Returns the name of the archive entry.
    pub fn name(&self) -> &ArchiveString {
        &self.name
    }

    /// Returns the inode number of the archive entry.
    pub fn inode(&self) -> u32 {
        self.inode
    }

    /// Returns the mode (type and permissions) of the archive entry.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Returns the modification time of the archive entry.
    pub fn mtime(&self) -> DateTime {
        self.mtime
    }

    /// Returns the content of the archive entry as a byte slice, if it has any content (i.e., if it's a regular file).
    pub fn content(&self) -> Option<&ArchiveBuffer> {
        self.content.as_ref()
    }
}

/// A buffer in the archive, representing a portion of the archive data with a specific offset and length.
#[derive(Debug, Clone)]
pub struct ArchiveBuffer {
    buffer: Arc<[u8]>,
    offset: usize,
    length: usize,
}

impl ArchiveBuffer {
    /// Constructs a new `ArchiveBuffer` from the given slice, calculating the offset and length based on the position of the slice within the archive's buffer.
    ///
    /// # Safety
    /// - The caller must ensure that the provided slice is a valid portion of the archive's buffer.
    pub unsafe fn new(buffer: Arc<[u8]>, slice: &[u8]) -> Self {
        assert!(slice.as_ptr() as usize >= buffer.as_ptr() as usize);
        assert!(slice.as_ptr() as usize + slice.len() <= buffer.as_ptr() as usize + buffer.len());

        let offset = slice.as_ptr() as usize - buffer.as_ptr() as usize;
        let length = slice.len();
        ArchiveBuffer {
            buffer,
            offset,
            length,
        }
    }

    /// Returns a slice of the archive data corresponding to this buffer, allowing clients to access the content of the file or directory represented by this buffer.
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.offset..self.offset + self.length]
    }
}

/// A string in the archive, representing a portion of the archive data that contains a string.
#[derive(Debug, Clone)]
pub struct ArchiveString(ArchiveBuffer);

impl ArchiveString {
    /// Constructs a new `ArchiveString` from the given slice, calculating the offset and length based on the position of the slice within the archive's buffer.
    ///
    /// # Safety
    /// - The caller must ensure that the provided slice is a valid portion of the archive's buffer.
    pub unsafe fn new(buffer: Arc<[u8]>, slice: &str) -> Self {
        Self(unsafe { ArchiveBuffer::new(buffer, slice.as_bytes()) })
    }

    /// Returns the string slice corresponding to this archive string, allowing clients to access the name of the file or directory represented by this string.
    ///
    /// # Safety
    /// - The provided archive must be the same archive from which this string was constructed.
    pub unsafe fn as_str(&self) -> &str {
        // Safety: the constructor has been passed a str, so the content of the buffer is valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.0.as_slice()) }
    }
}
