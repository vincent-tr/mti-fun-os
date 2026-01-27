use core::{error::Error, fmt};
use libruntime::kobject::{self, KObject};
use log::debug;
use xmas_elf::{header, program, ElfFile};

// Very simple loader.
// It only loads static binaries for now, with no checks
pub fn load(name: &str, binary: &[u8]) -> Result<(), LoaderError> {
    let elf_file = xmas_elf::ElfFile::new(binary)?;

    for program_header in elf_file.program_iter() {
        program::sanity_check(program_header, &elf_file)?;
    }

    match elf_file.header.pt2.type_().as_type() {
        header::Type::None => Err(LoaderError::BadObjectType("none"))?,
        header::Type::Relocatable => Err(LoaderError::BadObjectType("relocatable"))?,
        header::Type::Executable => (),
        header::Type::SharedObject => Err(LoaderError::BadObjectType("shared-object"))?,
        header::Type::Core => Err(LoaderError::BadObjectType("core"))?,
        header::Type::ProcessorSpecific(_) => {
            Err(LoaderError::BadObjectType("processor-specific"))?
        }
    };

    let process = kobject::Process::create(name)?;

    load_segments(&process, &elf_file)?;

    let stack_size = kobject::helpers::STACK_SIZE;
    let entry_point = unsafe {
        *((&elf_file.header.pt2.entry_point()) as *const u64 as *const extern "C" fn(usize) -> !)
    };

    let stack = kobject::helpers::AllocWithGuards::new_remote(stack_size, &process)?;
    let tls = kobject::helpers::AllocWithGuards::new_remote(kobject::helpers::TLS_SIZE, &process)?;

    let stack_top_addr = stack.address() + stack_size;
    let tls_addr = tls.address();

    debug!(
        "Creating main thread: entry_point={:#x}, stack_top={:#x}, tls={:#x}",
        entry_point as usize, stack_top_addr, tls_addr
    );

    // Use syscall directory to create remote thread
    let thread_handle = libsyscalls::thread::create(
        Some("main"),
        unsafe { process.handle() },
        false,
        kobject::ThreadPriority::Normal,
        entry_point,
        stack_top_addr,
        0, // arg not used
        tls_addr,
    )?;

    // Everything is ready, we can leak the allocations and close the handles
    stack.leak();
    tls.leak();
    drop(thread_handle);
    drop(process);

    Ok(())
}

fn load_segments(process: &kobject::Process, elf_file: &ElfFile) -> Result<(), LoaderError> {
    for program_header in elf_file.program_iter() {
        if !matches!(program_header.get_type()?, program::Type::Load) {
            continue;
        }

        let virtual_addr = program_header.virtual_addr() as usize;
        let file_addr = program_header.offset() as usize;
        let virtual_size = program_header.mem_size() as usize;
        let file_size = program_header.file_size() as usize;
        let align = program_header.align() as usize;

        debug!(
            "Loading segment: virtual_addr={:#x}, virtual_size={:#x}, file_addr={:#x}, file_size={:#x}, align={:#x}, flags={:?}",
            virtual_addr,
            virtual_size,
            file_addr,
            file_size,
            align,
            program_header.flags()
        );

        assert!(program_header.align() as usize >= kobject::PAGE_SIZE);

        let start = align_down(virtual_addr, align);
        let end = align_up(virtual_addr + virtual_size, align);
        let size = end - start;

        let offset = virtual_addr - start;
        let segment_data = &elf_file.input[file_addr..(file_addr + file_size)];
        let mem_obj = create_segment_data(segment_data, offset, size)?;

        let mut perms = kobject::Permissions::empty();
        let flags = program_header.flags();
        if flags.is_read() {
            perms |= kobject::Permissions::READ;
        }
        if flags.is_write() {
            perms |= kobject::Permissions::WRITE;
        }
        if flags.is_execute() {
            perms |= kobject::Permissions::EXECUTE;
        }

        let mapping = process.map_mem(Some(start), size, perms, &mem_obj, 0)?;
        mapping.leak();
    }

    Ok(())
}

fn create_segment_data(
    data: &[u8],
    offset: usize,
    size: usize,
) -> Result<kobject::MemoryObject, LoaderError> {
    let mem_obj = kobject::MemoryObject::create(size)?;

    let process = kobject::Process::current();
    let mapping = process.map_mem(
        None,
        size,
        kobject::Permissions::READ | kobject::Permissions::WRITE,
        &mem_obj,
        0,
    )?;

    let mem_data = unsafe { mapping.as_buffer_mut().expect("Buffer not writable") };

    mem_data[offset..(offset + data.len())].copy_from_slice(data);

    Ok(mem_obj)
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

#[derive(Debug, Clone, Copy)]
pub enum LoaderError {
    ElfReaderError(&'static str),
    BadObjectType(&'static str),
    Error(kobject::Error),
}

impl From<kobject::Error> for LoaderError {
    fn from(err: kobject::Error) -> LoaderError {
        LoaderError::Error(err)
    }
}

impl From<&'static str> for LoaderError {
    fn from(err: &'static str) -> LoaderError {
        LoaderError::ElfReaderError(err)
    }
}

impl fmt::Display for LoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LoaderError::ElfReaderError(str) => {
                write!(formatter, "elf reader error: {}", str)
            }
            LoaderError::BadObjectType(typ) => {
                write!(formatter, "bad object type: '{}'", typ)
            }
            LoaderError::Error(ref err) => write!(formatter, "loader error: {:?}", err),
        }
    }
}

impl Error for LoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
