use crate::{
    align_down, align_up,
    kobject::{Mapping, Process},
};
use core::{cmp, error::Error, mem::size_of, ops::Range};
use log::debug;
use xmas_elf::{
    dynamic, header, program,
    sections::{self},
    ElfFile,
};

pub use crate::{wrap_res, LoaderError, Segment};

const R_X86_64_RELATIVE: u32 = 8;
const R_X86_64_IRELATIVE: u32 = 37;

pub struct Program<'a> {
    elf_file: ElfFile<'a>,
    mapping: Mapping<'static>,
    is_pie: bool,
    addr_offset: usize,
}

impl<'a> Program<'a> {
    pub fn load(binary: &'a [u8]) -> Result<Self, Box<dyn Error>> {
        let elf_file = wrap_res(xmas_elf::ElfFile::new(binary))?;

        for program_header in elf_file.program_iter() {
            wrap_res(program::sanity_check(program_header, &elf_file))?;
        }

        match elf_file.header.pt2.type_().as_type() {
            header::Type::None => Err(LoaderError::BadObjectType("none"))?,
            header::Type::Relocatable => Err(LoaderError::BadObjectType("relocatable"))?,
            header::Type::Executable => (),
            header::Type::SharedObject => (),
            header::Type::Core => Err(LoaderError::BadObjectType("core"))?,
            header::Type::ProcessorSpecific(_) => {
                Err(LoaderError::BadObjectType("processor-specific"))?
            }
        };

        /*
            for program_header in elf_file.program_iter() {
                debug!("SEGMENT {:?}", program_header);
            }

            for section in elf_file.section_iter() {
                if wrap_res(section.get_type())? != ShType::Null {
                    debug!(
                        "SECTION name={} {:?}",
                        section.get_name(&elf_file).unwrap(),
                        section
                    );
                }
            }
        */

        let is_pie = if let header::Type::SharedObject = elf_file.header.pt2.type_().as_type() {
            true
        } else {
            false
        };

        let vm_range = Self::get_vm_range(&elf_file)?;
        debug!("vm_range = 0x{0:016X}", vm_range.start);
        debug!("vm_range = 0x{0:016X}", vm_range.end);

        if is_pie {
            assert!(vm_range.start == 0);
        } else {
            assert!(vm_range.start != 0);
        }

        let reserv_addr = if is_pie { None } else { Some(vm_range.start) };

        // Reserve a region anywhere there is vmspace
        let mapping = Process::current().map_reserve(reserv_addr, vm_range.len())?;

        let addr_offset = if is_pie { mapping.address() } else { 0 };

        debug!("addr base = 0x{0:016X}", mapping.address());

        let prog = Self {
            elf_file,
            mapping,
            is_pie,
            addr_offset,
        };

        let segments = prog.load_segments()?;

        for program_header in prog.elf_file.program_iter() {
            if let program::Type::Dynamic = wrap_res(program_header.get_type())? {
                debug!("Process dynamic segment");
                prog.process_dynamic_segment(program_header)?;
            }
        }

        for segment in segments {
            segment.finalize()?;
        }

        Ok(prog)
    }

    fn get_vm_range(elf_file: &ElfFile) -> Result<Range<usize>, LoaderError> {
        let mut min = usize::MAX;
        let mut max = usize::MIN;
        for program_header in elf_file.program_iter() {
            if let program::Type::Load = wrap_res(program_header.get_type())? {
                debug!("PROGRAM {:?}", program_header);

                let start = align_down(
                    program_header.virtual_addr() as usize,
                    program_header.align() as usize,
                );
                let end = align_up(
                    (program_header.virtual_addr() + program_header.mem_size()) as usize,
                    program_header.align() as usize,
                );

                min = cmp::min(min, start);
                max = cmp::max(max, end);
            }
        }

        Ok(min..max)
    }

    fn load_segments(&self) -> Result<Vec<Segment>, Box<dyn Error>> {
        let mut segments = Vec::new();

        for program_header in self.elf_file.program_iter() {
            if let program::Type::Load = wrap_res(program_header.get_type())? {
                let file_rel_segment = program_header.offset() as usize
                    ..(program_header.offset() + program_header.file_size()) as usize;

                let mut segment =
                    Segment::new(Process::current(), &program_header, self.addr_offset)?;

                // copy data
                let dest_slice = &mut segment.buffer_mut()[0..file_rel_segment.len()];
                let source_slice = &self.elf_file.input[file_rel_segment];

                dest_slice.copy_from_slice(source_slice);

                segments.push(segment);
            }
        }

        Ok(segments)
    }

    fn process_dynamic_segment(
        &self,
        segment: program::ProgramHeader,
    ) -> Result<(), Box<dyn Error>> {
        let data = if let program::SegmentData::Dynamic64(data) =
            wrap_res(segment.get_data(&self.elf_file))?
        {
            data
        } else {
            return Err(Box::new(LoaderError::BadDynamicSection));
        };

        self.process_relocations(data)?;

        Ok(())
    }

    fn process_relocations(&self, data: &[dynamic::Dynamic<u64>]) -> Result<(), Box<dyn Error>> {
        // Find the `Rela`, `RelaSize` and `RelaEnt` entries.
        let mut rela = None;
        let mut rela_size = None;
        let mut rela_ent = None;

        for rel in data {
            let tag = rel.get_tag()?;
            match tag {
                dynamic::Tag::Rela => {
                    let ptr = wrap_res(rel.get_ptr())? as usize;
                    let prev = rela.replace(ptr);
                    if prev.is_some() {
                        return Err(Box::new(LoaderError::BadDynamicSection));
                    }
                }
                dynamic::Tag::RelaSize => {
                    let val = wrap_res(rel.get_val())? as usize;
                    let prev = rela_size.replace(val);
                    if prev.is_some() {
                        return Err(Box::new(LoaderError::BadDynamicSection));
                    }
                }
                dynamic::Tag::RelaEnt => {
                    let val = wrap_res(rel.get_val())? as usize;
                    let prev = rela_ent.replace(val);
                    if prev.is_some() {
                        return Err(Box::new(LoaderError::BadDynamicSection));
                    }
                }
                _ => {}
            }
        }

        let offset = if let Some(offset) = rela {
            offset
        } else {
            // The section doesn't contain any relocations.

            if rela_size.is_some() || rela_ent.is_some() {
                return Err(Box::new(LoaderError::BadDynamicSection));
            }

            return Ok(());
        };

        // We should have relocation only on PIE
        assert!(self.is_pie);

        let base_addr = self.mapping.address();

        let total_size = rela_size.ok_or(Box::new(LoaderError::BadDynamicSection))?;
        let entry_size = rela_ent.ok_or(Box::new(LoaderError::BadDynamicSection))?;

        // Make sure that the reported size matches our `Rela<u64>`.
        if entry_size != size_of::<sections::Rela<u64>>() {
            return Err(Box::new(LoaderError::BadDynamicSection));
        }

        // Apply the relocations.
        let num_entries = total_size / entry_size;
        debug!("Process {num_entries} relocations");
        for idx in 0..num_entries {
            let rela = Self::read_relocation(offset, idx, base_addr);
            self.apply_relocation(rela)?;
        }

        Ok(())
    }

    /// Reads a relocation from a relocation table.
    fn read_relocation(
        relocation_table: usize,
        idx: usize,
        base_addr: usize,
    ) -> sections::Rela<u64> {
        // Calculate the address of the entry in the relocation table.
        let offset = relocation_table + size_of::<sections::Rela<u64>>() * idx;
        let addr = base_addr + offset;

        unsafe { core::ptr::read_unaligned(addr as *const sections::Rela<u64>) }
    }

    fn apply_relocation(&self, rela: sections::Rela<u64>) -> Result<(), LoaderError> {
        let base_addr = self.mapping.address();
        let symbol_idx = rela.get_symbol_table_index();
        assert_eq!(
            symbol_idx, 0,
            "relocations using the symbol table are not supported"
        );

        match rela.get_type() {
            R_X86_64_RELATIVE | R_X86_64_IRELATIVE => {
                // Make sure that the relocation happens in memory mapped
                // by a Load segment.
                self.check_is_in_load(rela.get_offset())?;

                // Calculate the destination of the relocation.
                let addr = (base_addr + rela.get_offset() as usize) as *mut _;

                // Calculate the relocated value.
                let value = base_addr + rela.get_addend() as usize;

                // Write the relocated value to memory.
                // SAFETY: We just verified that the address is in a Load segment.
                unsafe { core::ptr::write_unaligned(addr, value) };

                let typestr = match rela.get_type() {
                    R_X86_64_RELATIVE => "R_X86_64_RELATIVE",
                    R_X86_64_IRELATIVE => "R_X86_64_IRELATIVE",
                    _ => "??",
                };

                debug!(
                    "Relocation {} : 0x{:016X} => 0x{:016X}",
                    typestr, addr as usize, value
                );
            }

            ty => unimplemented!("relocation type {} not supported", ty),
        }

        Ok(())
    }

    fn check_is_in_load(&self, virt_offset: u64) -> Result<(), LoaderError> {
        for program_header in self.elf_file.program_iter() {
            if let program::Type::Load = wrap_res(program_header.get_type())? {
                if program_header.virtual_addr() <= virt_offset {
                    let offset_in_segment = virt_offset - program_header.virtual_addr();
                    if offset_in_segment < program_header.mem_size() {
                        return Ok(());
                    }
                }
            }
        }
        Err(LoaderError::BadDynamicSection)
    }

    pub fn entry(&self) -> extern "C" fn() -> ! {
        let entry_addr: usize = self.addr_offset + self.elf_file.header.pt2.entry_point() as usize;

        debug!("entry_addr = 0x{0:016X}", entry_addr);

        unsafe { std::mem::transmute(entry_addr) }
    }
}
