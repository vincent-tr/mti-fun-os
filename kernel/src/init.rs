use core::cmp::min;
use core::ops::Range;

use crate::memory::{PAGE_SIZE, page_aligned_up, self, Permissions};
use crate::user::process::{self, Process};
use crate::{memory::VirtAddr, user::MemoryObject};
use alloc::sync::Arc;
use log::{debug, info};
use xmas_elf::{
    header,
    program::{self, ProgramHeader},
    ElfFile,
};

// https://docs.rs/include_bytes_aligned/latest/src/include_bytes_aligned/lib.rs.html#1-37
macro_rules! include_bytes_aligned {
    ($align_to:expr, $path:expr) => {{
        #[repr(C, align($align_to))]
        struct __Aligned<T: ?Sized>(T);

        static __DATA: &'static __Aligned<[u8]> = &__Aligned(*include_bytes!($path));

        &__DATA.0
    }};
}

pub fn load() -> VirtAddr {
    info!("Loading init binary");

    // FIXME
    let binary = include_bytes_aligned!(8, "../../target/x86_64-mti_fun_os/debug/init");

    let loader = Loader::new(binary);
    loader.sanity_check();
    loader.load_segments();
    loader.entry_point()
}

struct Loader<'a> {
    elf_file: ElfFile<'a>,
    process: Arc<Process>,
}

impl<'a> Loader<'a> {
    pub fn new(binary: &'a [u8]) -> Self {
        Self {
            elf_file: ElfFile::new(binary).expect("Could not read init binary"),
            process: process::create().expect("could not create process"),
        }
    }

    pub fn sanity_check(&self) {
        header::sanity_check(&self.elf_file).expect("Init binary sanity check failed");

        for program_header in self.elf_file.program_iter() {
            program::sanity_check(program_header, &self.elf_file)
                .expect("Init binary sanity check failed");
        }

        match self.elf_file.header.pt2.type_().as_type() {
            header::Type::None => panic!("Init binary unexpected type: None"),
            header::Type::Relocatable => panic!("Init binary unexpected type: Relocatable"),
            header::Type::Executable => {}
            header::Type::SharedObject => panic!("Init binary unexpected type: SharedObject"),
            header::Type::Core => panic!("Init binary unexpected type: Core"),
            header::Type::ProcessorSpecific(_) => {
                panic!("Init binary unexpected type: ProcessorSpecific")
            }
        };
    }

    pub fn load_segments(&self) {
        // Load the segments into virtual memory.
        for program_header in self.elf_file.program_iter() {
            match program_header
                .get_type()
                .expect("Could not get program header type")
            {
                program::Type::Load => self.load_segment(program_header),
                program::Type::Tls => {
                    panic!("Init binary TLS not supported");
                }
                program::Type::Dynamic | program::Type::GnuRelro => {
                    panic!("Init binary Relactions not supported");
                }
                program::Type::Null
                | program::Type::Interp
                | program::Type::Note
                | program::Type::ShLib
                | program::Type::Phdr
                | program::Type::OsSpecific(_)
                | program::Type::ProcessorSpecific(_) => {}
            }
        }

        // self.inner.remove_copied_flags(&elf_file).unwrap();
    }

    fn load_segment(&self, segment: ProgramHeader) {
        debug!("Loading Segment: {:x?}", segment);

        assert!(segment.align() as usize == PAGE_SIZE, "wrong alignment");

        let mem_size = page_aligned_up(segment.mem_size() as usize);
        
        let mobj =
            MemoryObject::new(mem_size).expect("Could not create MemoryObject");

        self.load_data(&mobj, segment.offset() as usize..(segment.offset() + segment.file_size()) as usize);

        if segment.mem_size() > segment.file_size() {
            // .bss section (or similar), which needs to be mapped and zeroed
            todo!("handle bss section");
            //self.handle_bss_section(&segment, segment_flags)?;
        }

        let mut perms = Permissions::NONE;

        if segment.flags().is_read() {
            perms |= Permissions::READ;
        }

        if segment.flags().is_write() {
            perms |= Permissions::WRITE;
        }

        if segment.flags().is_execute() {
            perms |= Permissions::EXECUTE;
        }

        self.process.map(VirtAddr::new(segment.virtual_addr()), mem_size, perms, Some(mobj), 0).expect("map failed");
    }

    fn load_data(&self, mobj: &Arc<MemoryObject>, mut range: Range<usize>) {
        for frame in mobj.frames_iter() {
            let frame_data = unsafe { memory::access_phys(frame) };
            let copy_size = min(range.len(), PAGE_SIZE);
            let source_data = &self.elf_file.input[range.start..range.start + copy_size];
            frame_data[..copy_size].copy_from_slice(source_data);

            range.start += copy_size;

            if range.len() == 0 {
                break;
            }
        }
    }

    fn entry_point(&self) -> VirtAddr {
        VirtAddr::new(self.elf_file.header.pt2.entry_point())
    }
}
/*
fn handle_bss_section(segment: &ProgramHeader, segment_flags: Flags) -> Result<(), &'static str> {
    log::info!("Mapping bss section");

    let virt_start_addr = VirtAddr::new(self.virtual_address_offset + segment.virtual_addr());
    let mem_size = segment.mem_size();
    let file_size = segment.file_size();

    // calculate virtual memory region that must be zeroed
    let zero_start = virt_start_addr + file_size;
    let zero_end = virt_start_addr + mem_size;

    // a type alias that helps in efficiently clearing a page
    type PageArray = [u64; Size4KiB::SIZE as usize / 8];
    const ZERO_ARRAY: PageArray = [0; Size4KiB::SIZE as usize / 8];

    // In some cases, `zero_start` might not be page-aligned. This requires some
    // special treatment because we can't safely zero a frame of the original file.
    let data_bytes_before_zero = zero_start.as_u64() & 0xfff;
    if data_bytes_before_zero != 0 {
        // The last non-bss frame of the segment consists partly of data and partly of bss
        // memory, which must be zeroed. Unfortunately, the file representation might have
        // reused the part of the frame that should be zeroed to store the next segment. This
        // means that we can't simply overwrite that part with zeroes, as we might overwrite
        // other data this way.
        //
        // Example:
        //
        //   XXXXXXXXXXXXXXX000000YYYYYYY000ZZZZZZZZZZZ     virtual memory (XYZ are data)
        //   |·············|     /·····/   /·········/
        //   |·············| ___/·····/   /·········/
        //   |·············|/·····/‾‾‾   /·········/
        //   |·············||·····|/·̅·̅·̅·̅·̅·····/‾‾‾‾
        //   XXXXXXXXXXXXXXXYYYYYYYZZZZZZZZZZZ              file memory (zeros are not saved)
        //   '       '       '       '        '
        //   The areas filled with dots (`·`) indicate a mapping between virtual and file
        //   memory. We see that the data regions `X`, `Y`, `Z` have a valid mapping, while
        //   the regions that are initialized with 0 have not.
        //
        //   The ticks (`'`) below the file memory line indicate the start of a new frame. We
        //   see that the last frames of the `X` and `Y` regions in the file are followed
        //   by the bytes of the next region. So we can't zero these parts of the frame
        //   because they are needed by other memory regions.
        //
        // To solve this problem, we need to allocate a new frame for the last segment page
        // and copy all data content of the original frame over. Afterwards, we can zero
        // the remaining part of the frame since the frame is no longer shared with other
        // segments now.

        let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
        let new_frame = unsafe { self.make_mut(last_page) };
        let new_bytes_ptr = new_frame.start_address().as_u64() as *mut u8;
        unsafe {
            core::ptr::write_bytes(
                new_bytes_ptr.add(data_bytes_before_zero as usize),
                0,
                (Size4KiB::SIZE - data_bytes_before_zero) as usize,
            );
        }
    }

    // map additional frames for `.bss` memory that is not present in source file
    let start_page: Page =
        Page::containing_address(VirtAddr::new(align_up(zero_start.as_u64(), Size4KiB::SIZE)));
    let end_page = Page::containing_address(zero_end - 1u64);
    for page in Page::range_inclusive(start_page, end_page) {
        // allocate a new unused frame
        let frame = self.frame_allocator.allocate_frame().unwrap();

        // zero frame, utilizing identity-mapping
        let frame_ptr = frame.start_address().as_u64() as *mut PageArray;
        unsafe { frame_ptr.write(ZERO_ARRAY) };

        // map frame
        let flusher = unsafe {
            self.page_table
                .map_to(page, frame, segment_flags, self.frame_allocator)
                .map_err(|_err| "Failed to map new frame for bss memory")?
        };
        // we operate on an inactive page table, so we don't need to flush our changes
        flusher.ignore();
    }

    Ok(())
}

fn entry_point() -> VirtAddr {
    VirtAddr::new(self.inner.virtual_address_offset + elf_file.header.pt2.entry_point())
}
*/
