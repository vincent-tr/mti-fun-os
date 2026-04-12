use core::mem;

use alloc::vec::Vec;
use libruntime::{
    kobject,
    net::types,
    sync::{Mutex, spin::OnceLock},
};

/// Size of each buffer in bytes.
const BUFFER_SIZE: usize = 2048;

/// Number of buffers in the pool.
const BUFFER_COUNT: usize = 2048;

/// Buffer pool for network packet data, shared between the net server and network device drivers.
#[derive(Debug)]
pub struct BufferPool {
    /// The memory object backing the buffer pool.
    mobj: kobject::MemoryObject,

    /// The number of buffers in the pool.
    buffer_count: usize,

    /// The size of each buffer in bytes.
    buffer_size: usize,

    /// The mapping of the buffer pool into the net server's address space.
    mapping: kobject::Mapping<'static>,

    /// The free list of buffer indexes in the pool.
    free_list: Mutex<Vec<usize>>,
}

impl BufferPool {
    /// Creates a new buffer pool.
    pub fn new(buffer_count: usize, buffer_size: usize) -> Self {
        let mobj = kobject::MemoryObject::create(buffer_size * buffer_count)
            .expect("Could not create buffer pool memory object");

        let mapping = kobject::Process::current()
            .map_mem(
                None,
                buffer_size * buffer_count,
                kobject::Permissions::READ | kobject::Permissions::WRITE,
                &mobj,
                0,
            )
            .expect("Could not map buffer pool memory object");

        let free_list = Mutex::new((0..buffer_count).rev().collect());

        Self {
            mobj,
            buffer_count,
            buffer_size,
            mapping,
            free_list,
        }
    }

    /// Returns the number of buffers in the pool.
    pub fn buffer_count(&self) -> usize {
        self.buffer_count
    }

    /// Returns the size of each buffer in bytes.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Allocates a buffer from the pool, returning its index if successful.
    pub fn allocate(&self) -> usize {
        let mut free_list = self.free_list.lock();
        free_list.pop().expect("No free buffers available")
    }

    /// Deallocates a buffer, returning it to the pool.
    pub fn deallocate(&self, buffer_id: usize) {
        let mut free_list = self.free_list.lock();
        free_list.push(buffer_id);
    }

    /// Returns a view of the buffer with the given index.
    pub fn view(&self, buffer_id: usize) -> &[u8] {
        let offset = buffer_id * self.buffer_size;
        unsafe {
            self.mapping
                .as_buffer()
                .expect("could not get mapping buffer")
        }
    }

    /// Returns a mutable view of the buffer with the given index.
    pub fn view_mut(&self, buffer_id: usize) -> &mut [u8] {
        let offset = buffer_id * self.buffer_size;
        unsafe {
            self.mapping
                .as_buffer_mut()
                .expect("could not get mapping buffer")
        }
    }

    /// Shares the buffer pool with a network device driver, returning a `types::BufferPool` that can be sent to the driver.
    pub fn share(&self) -> types::BufferPool {
        types::BufferPool {
            buffer_count: self.buffer_count,
            buffer_size: self.buffer_size,
            mobj: self.mobj.clone(),
        }
    }
}

impl Drop for BufferPool {
    fn drop(&mut self) {
        assert!(
            self.free_list.lock().len() == self.buffer_count,
            "Dropping buffer pool with allocated buffers"
        );
    }
}

/// The global buffer pool instance, initialized at startup.
static BUFFER_POOL: OnceLock<BufferPool> = OnceLock::new();

/// Initializes the global buffer pool instance.
/// This should be called once at startup before any network interfaces are created.
pub fn init() {
    let buffer_pool = BufferPool::new(BUFFER_COUNT, BUFFER_SIZE);
    BUFFER_POOL
        .set(buffer_pool)
        .expect("Could not set global buffer pool");
}

/// Returns a reference to the global buffer pool instance.
pub fn pool() -> &'static BufferPool {
    BUFFER_POOL.get().expect("Buffer pool not initialized")
}

/// A buffer allocated from the buffer pool. Automatically deallocates when dropped.
#[derive(Debug)]
pub struct Buffer {
    // The index of the buffer in the pool.
    buffer_id: usize,
}

impl Buffer {
    /// Allocates a buffer from the pool, returning a `Buffer` that will automatically deallocate when dropped.
    pub fn allocate() -> Self {
        let buffer_id = pool().allocate();
        Self { buffer_id }
    }

    /// Returns a view of the buffer's data.
    pub fn view(&self) -> &[u8] {
        pool().view(self.buffer_id)
    }

    /// Returns a mutable view of the buffer's data.
    pub fn view_mut(&mut self) -> &mut [u8] {
        pool().view_mut(self.buffer_id)
    }

    /// Leaks the buffer, returning its index in the pool. The caller is responsible for deallocating the buffer when it is no longer needed.
    pub unsafe fn leak(self) -> usize {
        let buffer_id = self.buffer_id;
        mem::forget(self);
        buffer_id
    }

    /// Creates a `Buffer` from a buffer index. The caller must ensure that the buffer index is valid and that the buffer is not currently allocated.
    pub unsafe fn from_id(buffer_id: usize) -> Self {
        Self { buffer_id }
    }

    /// Returns the index of the buffer in the pool.
    pub fn id(&self) -> usize {
        self.buffer_id
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        pool().deallocate(self.buffer_id);
    }
}
