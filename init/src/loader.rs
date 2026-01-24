use core::{error::Error, fmt, panic};
use libruntime::kobject;
use log::debug;
use xmas_elf::{header, program, ElfFile};

// Very simple loader.
// It only loads static binaries for now, with no checks
pub fn load(binary: &[u8]) -> Result<(), LoaderError> {
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

    load_segments(&elf_file)?;

    Ok(())
}

fn load_segments(elf_file: &ElfFile) -> Result<(), LoaderError> {
    for program_header in elf_file.program_iter() {
        if !matches!(program_header.get_type()?, program::Type::Load) {
            continue;
        }

        let vaddr = program_header.virtual_addr() as usize;
        let paddr = program_header.physical_addr() as usize;
        let memsz = program_header.mem_size() as usize;
        let filesz = program_header.file_size() as usize;

        debug!(
            "Loading segment: paddr={:#x}, memsz={:#x}, filesz={:#x}, flags={:?}",
            paddr,
            memsz,
            filesz,
            program_header.flags()
        );

        assert!(program_header.align() as usize >= kobject::PAGE_SIZE);

        
        // TODO: align addresses

        assert!(vaddr % kobject::PAGE_SIZE == 0);
        assert!(memsz % kobject::PAGE_SIZE == 0);

        let mem_obj = kobject::MemoryObject::create(memsz)?;
        let process = kobject::Process::current();
        let mapping = process.map_mem(
            None,
            memsz,
            kobject::Permissions::READ | kobject::Permissions::WRITE,
            &mem_obj,
            0,
        )?;
        let data = unsafe { mapping.as_buffer_mut().expect("Buffer not writable") };
    }

    panic!("unimplemented");
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
