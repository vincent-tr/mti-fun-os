use alloc::vec::Vec;
use libruntime::{kobject, memory, process::iface::ProcessServerError};
use log::debug;
use xmas_elf::{header, program, ElfFile};

use crate::error::{invalid_binary, ResultExt};

/// Binary loader for ELF files
///
/// Note: This only supports static binaries for now
pub struct Loader<'a> {
    elf_file: ElfFile<'a>,
}

impl<'a> Loader<'a> {
    /// Create a new loader from the given binary data, and validate it
    pub fn new(binary: &'a [u8]) -> Result<Self, ProcessServerError> {
        let loader = Loader {
            elf_file: ElfFile::new(binary).invalid_binary("Failed to parse ELF binary")?,
        };

        match loader.elf_file.header.pt2.type_().as_type() {
            header::Type::Executable => (),
            header::Type::None => {
                return Err(invalid_binary("ELF type is None"));
            }
            header::Type::Relocatable => {
                return Err(invalid_binary("Relocatable binaries not supported"));
            }
            header::Type::SharedObject => {
                return Err(invalid_binary("Shared objects not supported"));
            }
            header::Type::Core => {
                return Err(invalid_binary("Core dumps not supported"));
            }
            header::Type::ProcessorSpecific(_) => {
                return Err(invalid_binary("Processor-specific type not supported"));
            }
        };

        let mut has_loadable_segment = false;

        for program_header in loader.elf_file.program_iter() {
            program::sanity_check(program_header, &loader.elf_file)
                .invalid_binary("ELF sanity check failed")?;

            // Ensure that there is at least one loadable segment, and it has compatible alignment
            let r#type = program_header
                .get_type()
                .invalid_binary("Failed to get program header type")?;

            if matches!(r#type, program::Type::Load) {
                has_loadable_segment = true;

                if (program_header.align() as usize) < kobject::PAGE_SIZE {
                    Err(invalid_binary("Segment alignment less than page size"))?;
                }
            }
        }

        if !has_loadable_segment {
            Err(invalid_binary("No loadable segments found"))?;
        }

        Ok(loader)
    }

    /// Map the ELF segments into the given process's address space
    pub fn map(
        &self,
        process: &'a kobject::Process,
    ) -> Result<Vec<kobject::Mapping<'a>>, ProcessServerError> {
        let mut mappings = Vec::new();

        for program_header in self.elf_file.program_iter() {
            if !matches!(
                program_header
                    .get_type()
                    .expect("could not match already-checked type"),
                program::Type::Load
            ) {
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
    ) -> Result<kobject::Mapping<'a>, ProcessServerError> {
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

        let mapping = process
            .map_mem(Some(start), size, perms, &mem_obj, 0)
            .runtime_err("Failed to map memory")?;

        Ok(mapping)
    }

    fn create_segment_data(
        data: &[u8],
        offset: usize,
        size: usize,
    ) -> Result<kobject::MemoryObject, ProcessServerError> {
        let mem_obj =
            kobject::MemoryObject::create(size).runtime_err("Failed to create memory object")?;

        let process = kobject::Process::current();
        let mapping = process
            .map_mem(
                None,
                size,
                kobject::Permissions::READ | kobject::Permissions::WRITE,
                &mem_obj,
                0,
            )
            .runtime_err("Failed to map memory")?;

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
