use crate::user::{Process, PROCESSES};
use crate::{memory::VirtAddr, user::MemoryObject};
use alloc::sync::Arc;
use log::{debug, info};
use xmas_elf::{
    header,
    program::{self, ProgramHeader},
    ElfFile,
};

pub fn load() -> VirtAddr {
    info!("Loading init binary");

    // FIXME
    let binary = include_bytes!("../../target/x86_64-mti_fun_os/debug/init");

    let mut loader = Loader::new(binary);
    loader.sanity_check();
    loader.load_segments();

    //entry_point()
    todo!();
}

struct Loader<'a> {
    elf_file: ElfFile<'a>,
    process: Arc<Process>,
}

impl<'a> Loader<'a> {
    pub fn new(binary: &'a [u8]) -> Self {
        Self {
            elf_file: ElfFile::new(binary).expect("Could not read init binary"),
            process: PROCESSES.create().expect("could not create process"),
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

        let mobj =
            MemoryObject::new(segment.mem_size() as usize).expect("Could not create MemoryObject");
        let data = &self.elf_file.input
            [segment.offset() as usize..(segment.offset() + segment.file_size()) as usize];
        /*
        let phys_start_addr = self.kernel_offset + segment.offset();
        let start_frame: PhysFrame = PhysFrame::containing_address(phys_start_addr);
        let end_frame: PhysFrame =
            PhysFrame::containing_address(phys_start_addr + segment.file_size() - 1u64);

        let virt_start_addr = VirtAddr::new(self.virtual_address_offset + segment.virtual_addr());
        let start_page: Page = Page::containing_address(virt_start_addr);

        let mut segment_flags = Flags::PRESENT;
        if !segment.flags().is_execute() {
            segment_flags |= Flags::NO_EXECUTE;
        }
        if segment.flags().is_write() {
            segment_flags |= Flags::WRITABLE;
        }

        // map all frames of the segment at the desired virtual address
        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let offset = frame - start_frame;
            let page = start_page + offset;
            let flusher = unsafe {
                self.page_table
                    .map_to(page, frame, segment_flags, self.frame_allocator)
                    .map_err(|_err| "map_to failed")?
            };
            // we operate on an inactive page table, so there's no need to flush anything
            flusher.ignore();
        }

        // Handle .bss section (mem_size > file_size)
        if segment.mem_size() > segment.file_size() {
            // .bss section (or similar), which needs to be mapped and zeroed
            self.handle_bss_section(&segment, segment_flags)?;
        }
        */
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
