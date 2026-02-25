use core::cmp::min;
use core::{mem, slice};

use crate::interrupts::SyscallArgs;
use crate::memory::{
    self, FrameRef, PAGE_SIZE, Permissions, drop_initial_kernel_stack, drop_initial_ramdisk,
    page_aligned_up,
};
use crate::user;
use crate::user::process;
use crate::user::syscalls::engine::unregister_syscall;
use crate::{memory::VirtAddr, user::MemoryObject};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use log::info;
use syscalls::{SyscallNumber, ThreadPriority, init};

const INIT_BASE_ADDRESS: VirtAddr = VirtAddr::new_truncate(0x200000);
const INIT_SIZE_OF_HEADERS: usize = PAGE_SIZE;
const INFO_ADDRESS: VirtAddr = VirtAddr::new_truncate(0x100000);
const ARCHIVE_ADDRESS: VirtAddr = VirtAddr::new_truncate(0x1000000);

pub fn setup(context: SyscallArgs) {
    let ramdisk = context.arg1()..context.arg2();
    let init_info = unsafe { Box::from_raw(context.arg3() as *mut init::InitInfo) };

    // Unregister current syscall
    unregister_syscall(SyscallNumber::InitSetup);

    // Drop initial kernel stack (not used anymore, we are on regular interrupt stack)
    drop_initial_kernel_stack();

    // Copy archive to a new memory object, so we can drop the initial ramdisk bootloader provided buffer.
    let archive_mobj = {
        let ramdisk_buffer = unsafe {
            slice::from_raw_parts(ramdisk.start as *const u8, ramdisk.end - ramdisk.start)
        };

        load_mem(ramdisk_buffer)
    };

    drop_initial_ramdisk();

    info!("Loading init binary");

    let (init_info_mobj, init_binary_mobj) = prepare_data(&archive_mobj, init_info);

    create_process(init_binary_mobj, init_info_mobj, archive_mobj);

    user::thread::initial_setup_thread();
}

fn prepare_data(
    archive_mobj: &Arc<MemoryObject>,
    mut init_info: Box<init::InitInfo>,
) -> (Arc<MemoryObject>, Arc<MemoryObject>) {
    let archive_mapping = MemoryObjectMapping::new(archive_mobj.clone());
    let archive_buffer = archive_mapping.as_buffer_mut();

    let init_binary = archive_find_init(archive_buffer);

    let init_info_buffer = unsafe {
        slice::from_raw_parts(
            init_info.as_ref() as *const init::InitInfo as *const u8,
            mem::size_of::<init::InitInfo>(),
        )
    };

    // Fille init_info missing mapping data
    init_info.info_mapping = mapping_info(INFO_ADDRESS, init_info_buffer);
    init_info.init_mapping = mapping_info(INIT_BASE_ADDRESS, init_binary);
    init_info.archive_mapping = mapping_info(ARCHIVE_ADDRESS, archive_buffer);

    // assert that the mapping do not overlap, since else mmap will unmap previous mapping.
    // order is init_info, init_binary and archive
    assert!(
        init_info.info_mapping.address + init_info.info_mapping.size
            <= init_info.init_mapping.address
    );
    assert!(
        init_info.init_mapping.address + init_info.init_mapping.size
            <= init_info.archive_mapping.address
    );

    let init_info_mobj = load_mem(init_info_buffer);
    let init_binary_mobj = load_mem(init_binary);

    (init_info_mobj, init_binary_mobj)
}

fn create_process(
    init_binary_mobj: Arc<MemoryObject>,
    init_info_mobj: Arc<MemoryObject>,
    archive_mobj: Arc<MemoryObject>,
) {
    let process = process::create("init").expect("Failed to create init process");

    process
        .mmap(
            INIT_BASE_ADDRESS,
            init_binary_mobj.size(),
            Permissions::READ | Permissions::WRITE | Permissions::EXECUTE,
            Some(init_binary_mobj),
            0,
        )
        .expect("Failed to map in init process");

    process
        .mmap(
            INFO_ADDRESS,
            init_info_mobj.size(),
            Permissions::READ,
            Some(init_info_mobj),
            0,
        )
        .expect("Failed to map init info in init process");

    process
        .mmap(
            ARCHIVE_ADDRESS,
            archive_mobj.size(),
            Permissions::READ,
            Some(archive_mobj),
            0,
        )
        .expect("Failed to map archive in init process");

    // .text section is right after headers.
    // entry point is laid out at the begining of .text section
    let entry_point = INIT_BASE_ADDRESS + INIT_SIZE_OF_HEADERS as u64;

    // Pass the init info pointer as argument
    let arg: usize = INFO_ADDRESS.as_u64() as usize;

    // Init does setup its stack itself.
    let stack_top = VirtAddr::zero();

    user::thread::create(
        Some("entry"),
        process.clone(),
        false,
        ThreadPriority::Normal,
        entry_point,
        stack_top,
        arg,
        VirtAddr::zero(),
    );
}

fn mapping_info(address: VirtAddr, buffer: &[u8]) -> init::Mapping {
    init::Mapping {
        address: address.as_u64() as usize,
        size: page_aligned_up(buffer.len()),
    }
}

/// Load the given buffer into a new memory object, and return it.
fn load_mem(buffer: &[u8]) -> Arc<MemoryObject> {
    let mem_size = page_aligned_up(buffer.len());
    let mobj = MemoryObject::new(mem_size).expect("Failed to create memory object");

    // Copy page by page
    let buffer_start = buffer.as_ptr() as usize;
    let buffer_end = buffer_start + buffer.len();

    for (index, frame) in mobj.frames_iter().enumerate() {
        let dest = unsafe { memory::access_phys(frame) };

        let source_start = buffer_start + index * PAGE_SIZE;
        let source_end = min(buffer_end, source_start + PAGE_SIZE);
        let size = source_end - source_start;
        let source = unsafe { slice::from_raw_parts(source_start as *const u8, size) };

        dest[0..size].copy_from_slice(source);
    }

    mobj
}

fn archive_find_init(archive: &[u8]) -> &[u8] {
    for entry in cpio_reader::iter_files(archive) {
        if entry.name() == "init" {
            return entry.file();
        }
    }

    panic!("Could not find init binary in archive");
}

/// Mapping of a memory object in the kernel.
#[derive(Debug)]
struct MemoryObjectMapping {
    mobj: Arc<MemoryObject>,
    address: VirtAddr,
}

impl MemoryObjectMapping {
    /// Create a new mapping of the given memory object
    pub fn new(mobj: Arc<MemoryObject>) -> Self {
        let mut frames = Vec::with_capacity(mobj.size() / PAGE_SIZE);

        for (i, frame) in mobj.frames_iter().enumerate() {
            let frame = unsafe {
                // Borrow it in the mobj (physical memory only!!) and unborrow it right after.
                mobj.borrow_frame(i * PAGE_SIZE);
                FrameRef::unborrow(frame)
            };

            frames.push(frame);
        }

        let address = unsafe { memory::map_phys(&mut frames) }.expect("Failed to map memory");

        Self { mobj, address }
    }

    /// Get a buffer to the mapped memory.
    pub fn as_buffer_mut(&self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.address.as_mut_ptr(), self.mobj.size()) }
    }
}

impl Drop for MemoryObjectMapping {
    fn drop(&mut self) {
        memory::unmap_phys(self.address, self.mobj.size() / PAGE_SIZE);
    }
}
