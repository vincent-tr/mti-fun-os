use alloc::vec::Vec;
use core::{error::Error, fmt};
use libruntime::{
    kobject::{self, KObject},
    memory,
    process::messages::ProcessServerError,
};
use log::{debug, error};
use xmas_elf::{header, program, ElfFile};

/// Loader error, which converts to ProcessServerError
///
/// Internally, it outputs to the log and converts to ProcessServerError::InvalidBinaryFormat
#[derive(Debug)]
pub struct LoaderError();

impl From<&'static str> for LoaderError {
    fn from(err: &'static str) -> LoaderError {
        error!("Loader error: {}", err);
        LoaderError()
    }
}

impl From<kobject::Error> for LoaderError {
    fn from(err: kobject::Error) -> LoaderError {
        error!("Kernel error: {}", err);
        LoaderError()
    }
}

impl Into<ProcessServerError> for LoaderError {
    fn into(self) -> ProcessServerError {
        ProcessServerError::InvalidBinaryFormat
    }
}

/// Binary loader for ELF files
///
/// Note: This only supports static binaries for now
pub struct Loader<'a> {
    name: &'a str,
    elf_file: ElfFile<'a>,
}

impl<'a> Loader<'a> {
    /// Create a new loader from the given binary data, and validate it
    pub fn new(name: &'a str, binary: &'a [u8]) -> Result<Self, LoaderError> {
        let loader = Loader {
            name,
            elf_file: ElfFile::new(binary)?,
        };

        match loader.elf_file.header.pt2.type_().as_type() {
            header::Type::None => Err("Bad binary type: none")?,
            header::Type::Relocatable => Err("Bad binary type: relocatable")?,
            header::Type::Executable => (),
            header::Type::SharedObject => Err("Bad binary type: shared-object")?,
            header::Type::Core => Err("Bad binary type: core")?,
            header::Type::ProcessorSpecific(_) => Err("Bad binary type: processor-specific")?,
        };

        let mut has_loadable_segment = false;

        for program_header in loader.elf_file.program_iter() {
            program::sanity_check(program_header, &loader.elf_file)?;

            // Ensure that there is at least one loadable segment, and it has compatible alignment
            if matches!(program_header.get_type()?, program::Type::Load) {
                has_loadable_segment = true;

                if (program_header.align() as usize) < kobject::PAGE_SIZE {
                    Err("Segment alignment less than page size")?;
                }
            }
        }

        if !has_loadable_segment {
            Err("No loadable segments found")?;
        }

        Ok(loader)
    }

    /// Map the ELF segments into the given process's address space
    pub fn map(
        &self,
        process: &'a kobject::Process,
    ) -> Result<Vec<kobject::Mapping<'a>>, LoaderError> {
        let mut mappings = Vec::new();

        for program_header in self.elf_file.program_iter() {
            if !matches!(program_header.get_type()?, program::Type::Load) {
                continue;
            }

            let mapping = self.load_segment(process, &program_header)?;
            mappings.push(mapping);
        }

        Ok(mappings)
    }

    fn load_segment(
        &self,
        process: &'a kobject::Process,
        program_header: &program::ProgramHeader,
    ) -> Result<kobject::Mapping<'a>, LoaderError> {
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

        let start = memory::align_down(virtual_addr, align);
        let end = memory::align_up(virtual_addr + virtual_size, align);
        let size = end - start;

        let offset = virtual_addr - start;
        let segment_data = &self.elf_file.input[file_addr..(file_addr + file_size)];
        let mem_obj = Self::create_segment_data(segment_data, offset, size)?;

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

        Ok(mapping)
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

    /// Get the entry point of the loaded binary
    pub fn entry_point(&self) -> extern "C" fn(usize) -> ! {
        unsafe {
            *((&self.elf_file.header.pt2.entry_point()) as *const u64
                as *const extern "C" fn(usize) -> !)
        }
    }
}
