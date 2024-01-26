use crate::{
    align_down, align_up,
    kobject::{Mapping, Process},
};
use core::{cmp, error::Error, mem::size_of, ops::Range, cell::RefCell};
use log::debug;
use std::collections::HashMap;
use xmas_elf::{
    dynamic, header, program,
    sections::{self},
    symbol_table::{self, DynEntry64, Entry, Visibility},
    ElfFile, P64,
};

pub use crate::{wrap_res, LoaderError, Segment};

const R_X86_64_RELATIVE: u32 = 8;
const R_X86_64_IRELATIVE: u32 = 37;

const R_X86_64_JUMP_SLOT: u32 = 7;

pub struct ExportedSymbol<'a> {
    name: &'a str,
    binding: symbol_table::Binding,
    address: usize,
}

pub struct Object<'a> {
    elf_file: ElfFile<'a>,
    mapping: Mapping<'static>,
    segments: Option<Vec<Segment<'a>>>, // Needed to fix permissions after relocations
    is_pie: bool,
    addr_offset: usize,
    needed: Vec<&'a str>,
    exports: HashMap<&'a str, ExportedSymbol<'a>>,
    init: Option<usize>,
    fini: Option<usize>,
}

impl<'a> Object<'a> {
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

        let mut prog = Self {
            elf_file,
            mapping,
            segments: None,
            is_pie,
            addr_offset,
            needed: Vec::new(),
            exports: HashMap::new(),
            init: None,
            fini: None,
        };

        prog.load_segments()?;

        prog.build_needed()?;
        prog.build_exports()?;
/*
        if let Some(dyn_section) = DynamicSection::find(&prog.elf_file)? {
            debug!("Process dynamic section");
            prog.process_relocations(&dyn_section)?;
            prog.process_needed(&dyn_section)?;
            prog.process_exports(&dyn_section)?;
        }

        debug!("deps: {:?}", prog.needed());
        debug!("exports: {:?}", prog.exports());
*/

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

    fn load_segments(&mut self) -> Result<(), Box<dyn Error>> {
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

        self.segments = Some(segments);

        Ok(())
    }

    fn build_needed(&mut self) -> Result<(), Box<dyn Error>> {
        let dyn_section = if let Some(dyn_section) = DynamicSection::find(&self.elf_file)? {
            dyn_section
        } else {
            return Ok(());
        };

        let items = dyn_section.find_all(dynamic::Tag::Needed)?;

        for item in items {
            let index = wrap_res(item.get_val())? as u32;
            let value = self.elf_file.get_dyn_string(index)?;
            self.needed.push(value);
        }

        Ok(())
    }

    fn build_exports(&mut self) -> Result<(), Box<dyn Error>> {
        let dyn_section = if let Some(dyn_section) = DynamicSection::find(&self.elf_file)? {
            dyn_section
        } else {
            return Ok(());
        };

        let symbols = if let Some(symbols) = Symbols::find(self, &dyn_section)? {
            symbols
        } else {
            return Ok(());
        };

        for symbol in symbols.entries() {
            // should use get_name but it has wrong liftime specifier
            let name = self.elf_file.get_dyn_string(symbol.name())?;

            let binding = wrap_res(symbol.get_binding())?;
            let visibility = symbol.get_other();
            let value = symbol.value() as usize;

            if let Visibility::Default = visibility
                && symbol.value() != 0
            {
                let address = self.addr_offset + value;
                self.exports.insert(name, ExportedSymbol { name, binding, address });
            }
        }

        Ok(())
    }

    // relocations : rel, rela, pltrel
    pub fn process_relocations(&self, objects: &HashMap<&str, RefCell<Object>>) -> Result<(), Box<dyn Error>> {

        Ok(())
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn Error>> {
        // Now we can apply permissions on segments
        for segment in self.segments.take().unwrap() {
            segment.finalize()?;
        }

        Ok(())
    }

    fn process_relocationszz(&self, dyn_section: &DynamicSection<'a>) -> Result<(), Box<dyn Error>> {
        // Find the `Rela`, `RelaSize` and `RelaEnt` entries.
        let rela = if let Some(entry) = dyn_section.find_unique(dynamic::Tag::Rela)? {
            Some(entry.get_val()? as usize)
        } else {
            None
        };

        let rela_size = if let Some(entry) = dyn_section.find_unique(dynamic::Tag::RelaSize)? {
            Some(entry.get_val()? as usize)
        } else {
            None
        };

        let rela_ent = if let Some(entry) = dyn_section.find_unique(dynamic::Tag::RelaEnt)? {
            Some(entry.get_val()? as usize)
        } else {
            None
        };

        // dyn_section.find_unique(dynamic::Tag::Relr)

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

    pub fn process_jump_relocations(
        &self,
        objects: &HashMap<&str, Object>,
    ) -> Result<(), Box<dyn Error>> {
        let dyn_section = if let Some(dyn_section) = DynamicSection::find(&self.elf_file)? {
            dyn_section
        } else {
            return Ok(());
        };

        let offset = if let Some(symbols_tag) = dyn_section.find_unique(dynamic::Tag::JmpRel)? {
            wrap_res(symbols_tag.get_ptr())? as usize
        } else {
            debug!("no symbol");
            return Ok(());
        };

        let section = self
            .find_section_by_offset(offset)
            .ok_or(LoaderError::BadDynamicSection)?;

        let entries = if let sections::SectionData::Rela64(entries) =
            wrap_res(section.get_data(&self.elf_file))?
        {
            entries
        } else {
            return Err(Box::new(LoaderError::BadRelocationsSection));
        };

        let symbols = Symbols::find(self, &dyn_section)?;

        for entry in entries {
            match entry.get_type() {
                R_X86_64_JUMP_SLOT => {
                    let symbols = if let Some(symbols) = &symbols {
                        symbols
                    } else {
                        return Err(Box::new(LoaderError::BadRelocationsSection));
                    };

                    self.apply_jump_relocation(objects, symbols, entry)?;
                }
                ty => unimplemented!("jump relocation type {} not supported", ty),
            }
        }

        debug!("Process relocations: {entries:?}");

        Ok(())
    }

    fn apply_jump_relocation(
        &self,
        objects: &HashMap<&str, Object>,
        symbols: &Symbols,
        entry: &sections::Rela<u64>,
    ) -> Result<(), Box<dyn Error>> {
        let sym_index = entry.get_symbol_table_index() as usize;
        let symbol = symbols.entry(sym_index);
        let sym_name = wrap_res(symbol.get_name(&self.elf_file))?;
        let offset = entry.get_offset() as usize;
        debug!("R_X86_64_JUMP_SLOT {sym_name} => 0x{offset:016X}");

        // Walk through needed until we find export
        for needed in self.needed() {
            let dependency = objects
                .get(needed)
                .expect(&format!("dependency not loaded {needed}"));

            if let Some(sym) = dependency.exports().get(sym_name) {
                let addr = sym.address;
                debug!("found match in {needed} at 0x{addr:016X}");
                let relo_addr = self.addr_offset + offset;
                unsafe { core::ptr::write_unaligned(relo_addr as *mut _, addr) };

                return Ok(());
            }
        }

        Err(Box::new(LoaderError::MissingSymbol(String::from(sym_name))))
    }

    fn find_section_by_offset(&self, offset: usize) -> Option<sections::SectionHeader<'a>> {
        for section_header in self.elf_file.section_iter() {
            if section_header.offset() as usize == offset {
                return Some(section_header);
            }
        }

        None
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

    pub fn needed(&self) -> &[&str] {
        &self.needed
    }

    pub fn exports(&self) -> &HashMap<&str, ExportedSymbol> {
        &self.exports
    }

    pub fn entry(&self) -> extern "C" fn() -> ! {
        let entry_addr: usize = self.addr_offset + self.elf_file.header.pt2.entry_point() as usize;

        debug!("entry_addr = 0x{0:016X}", entry_addr);

        unsafe { std::mem::transmute(entry_addr) }
    }
}

struct DynamicSection<'a> {
    data: &'a [dynamic::Dynamic<P64>],
}

impl<'a> DynamicSection<'a> {
    pub fn find(elf_file: &ElfFile<'a>) -> Result<Option<Self>, LoaderError> {
        let section = if let Some(section) = elf_file.find_section_by_name(".dynamic") {
            section
        } else {
            return Ok(None);
        };

        let data = wrap_res(section.get_data(elf_file))?;

        if let sections::SectionData::Dynamic64(data) = data {
            Ok(Some(Self { data }))
        } else {
            Err(LoaderError::BadDynamicSection)
        }
    }

    pub fn find_all(
        &self,
        tag: dynamic::Tag<u64>,
    ) -> Result<Vec<&'a dynamic::Dynamic<u64>>, Box<dyn Error>> {
        let mut res = Vec::new();

        for item in self.data {
            let item_tag = item.get_tag()?;

            if item_tag == tag {
                res.push(item);
            }
        }

        Ok(res)
    }

    pub fn find_unique(
        &self,
        tag: dynamic::Tag<u64>,
    ) -> Result<Option<&'a dynamic::Dynamic<u64>>, Box<dyn Error>> {
        let vec = self.find_all(tag)?;

        match vec.len() {
            0 => Ok(None),
            1 => Ok(Some(vec[0])),
            _ => Err(Box::new(LoaderError::BadDynamicSection)),
        }
    }
}

struct Symbols<'a> {
    entries: &'a [DynEntry64],
}

impl<'a> Symbols<'a> {
    pub fn find(
        prog: &Object<'a>,
        dyn_section: &DynamicSection<'a>,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        let offset = if let Some(symbols_tag) = dyn_section.find_unique(dynamic::Tag::SymTab)? {
            wrap_res(symbols_tag.get_ptr())? as usize
        } else {
            return Ok(None);
        };

        let section = prog
            .find_section_by_offset(offset)
            .ok_or(LoaderError::BadDynamicSection)?;

        let entries = if let sections::SectionData::DynSymbolTable64(entries) =
            wrap_res(section.get_data(&prog.elf_file))?
        {
            entries
        } else {
            return Err(Box::new(LoaderError::BadSymbolsSection));
        };

        Ok(Some(Self { entries }))
    }

    pub fn entries(&self) -> &[DynEntry64] {
        self.entries
    }

    pub fn entry(&self, index: usize) -> &DynEntry64 {
        &self.entries[index]
    }
}
