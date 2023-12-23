use core::cmp::{max, min};
use core::ops::Range;

use crate::memory::{self, page_aligned_down, page_aligned_up, Permissions, PAGE_SIZE};
use crate::user;
use crate::user::process::{self, Process};
use crate::user::thread::Thread;
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

pub fn run() -> ! {
    info!("Loading init binary");
    let (process, entry_point) = load();

    info!("Starting init binary");
    let thread = create_thread(process, entry_point);

    // Note: all locals must be dropped here
    unsafe { user::thread::initial_setup_thread(thread) };
}

fn load() -> (Arc<Process>, VirtAddr) {
    // TODO: try to make it part of the build
    let binary = include_bytes_aligned!(8, "../../target/x86_64-mti_fun_os/debug/init");

    let loader = Loader::new(binary);
    loader.sanity_check();
    loader.load_segments();
    let entry_point = loader.entry_point();
    (loader.process, entry_point)
}

fn create_thread(process: Arc<Process>, entry_point: VirtAddr) -> Arc<Thread> {
    const INIT_STACK_SIZE: usize = 5 * PAGE_SIZE;

    let user_stack_mobj =
        MemoryObject::new(INIT_STACK_SIZE).expect("Failed to allocate user stack");
    let user_stack = process
        .map(
            VirtAddr::zero(),
            user_stack_mobj.size(),
            Permissions::READ | Permissions::WRITE,
            Some(user_stack_mobj),
            0,
        )
        .expect("Failed to map user stack");
    let user_stack_top = user_stack + INIT_STACK_SIZE;

    user::thread::create(process.clone(), entry_point, user_stack_top)
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
                    panic!("Init binary Relocations not supported");
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

        let mem_start = VirtAddr::new(page_aligned_down(segment.virtual_addr() as usize) as u64);
        let mobj_start = (segment.virtual_addr() - mem_start.as_u64()) as usize;
        let mem_size = page_aligned_up(mobj_start + segment.mem_size() as usize);
        let mobj_range = mobj_start..mobj_start + segment.file_size() as usize;

        let mobj = MemoryObject::new(mem_size).expect("Could not create MemoryObject");

        self.load_data(
            &mobj,
            mobj_range,
            segment.offset() as usize..(segment.offset() + segment.file_size()) as usize,
        );

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

        self.process
            .map(mem_start, mem_size, perms, Some(mobj), 0)
            .expect("map failed");
    }

    fn load_data(
        &self,
        mobj: &Arc<MemoryObject>,
        mobj_range: Range<usize>,
        mut binary_range: Range<usize>,
    ) {
        let mobj_range_aligned =
            page_aligned_down(mobj_range.start)..page_aligned_up(mobj_range.end);

        for frame_offset in mobj_range_aligned.step_by(PAGE_SIZE) {
            if mobj_range.start >= frame_offset + PAGE_SIZE {
                continue;
            }

            let dest_range =
                max(frame_offset, mobj_range.start)..min(frame_offset + PAGE_SIZE, mobj_range.end);
            let frame_range = dest_range.start - frame_offset..dest_range.end - frame_offset;
            let source_range = binary_range.start..binary_range.start + frame_range.len();

            assert!(binary_range.len() >= frame_range.len());
            binary_range.start += frame_range.len();

            let frame_data = unsafe { memory::access_phys(mobj.frame(frame_offset)) };
            frame_data[frame_range].copy_from_slice(&self.elf_file.input[source_range]);
        }
    }

    fn entry_point(&self) -> VirtAddr {
        VirtAddr::new(self.elf_file.header.pt2.entry_point())
    }
}
