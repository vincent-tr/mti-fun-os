use crate::{
    align_down, align_up,
    kobject::{Mapping, Process},
};
use core::{cell::RefCell, cmp, error::Error, mem, ops::Range};
use log::debug;
use std::{collections::HashMap, marker::PhantomData};
use xmas_elf::{
    dynamic, header, program,
    sections::{self},
    symbol_table::{self, DynEntry64, Entry, Visibility},
    ElfFile, P64,
};

pub use crate::{wrap_res, LoaderError, Segment};

#[derive(Debug)]
pub struct ExportedSymbol<'a> {
    name: &'a str,
    binding: symbol_table::Binding,
    address: usize,
}

#[derive(Debug)]
pub struct Object<'a> {
    name: &'a str,
    elf_file: ElfFile<'a>,
    mapping: Mapping<'static>,
    segments: Option<Vec<Segment<'a>>>, // Needed to fix permissions after relocations
    is_pie: bool,
    addr_offset: usize,
    needed: Vec<&'a str>,
    exports: HashMap<&'a str, ExportedSymbol<'a>>,
    init: Option<FuncArray<'a>>,
    fini: Option<FuncArray<'a>>,
}

impl<'a> Object<'a> {
    pub fn load(name: &'a str, binary: &'a [u8]) -> Result<Self, Box<dyn Error>> {
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

        if is_pie {
            assert!(vm_range.start == 0);
        } else {
            assert!(vm_range.start != 0);
        }

        let reserv_addr = if is_pie { None } else { Some(vm_range.start) };

        // Reserve a region anywhere there is vmspace
        let mapping = Process::current().map_reserve(reserv_addr, vm_range.len())?;
        let addr_offset = if is_pie { mapping.address() } else { 0 };

        debug!(
            "mapping 0x{:016X} -> 0x{:016X}",
            mapping.range().start,
            mapping.range().end
        );

        let mut prog = Self {
            name,
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
        prog.build_init_fini()?;

        Ok(prog)
    }

    fn get_vm_range(elf_file: &ElfFile) -> Result<Range<usize>, LoaderError> {
        let mut min = usize::MAX;
        let mut max = usize::MIN;
        for program_header in elf_file.program_iter() {
            if let program::Type::Load = wrap_res(program_header.get_type())? {
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
                self.exports.insert(
                    name,
                    ExportedSymbol {
                        name,
                        binding,
                        address,
                    },
                );
            }
        }

        Ok(())
    }

    fn build_init_fini(&mut self) -> Result<(), LoaderError> {
        let init = if let Some(init_array_section) =
            self.find_section_by_type(sections::ShType::InitArray)?
        {
            Some(FuncArray::from_section(self, init_array_section)?)
        } else {
            None
        };

        let fini = if let Some(fini_array_section) =
            self.find_section_by_type(sections::ShType::FiniArray)?
        {
            Some(FuncArray::from_section(self, fini_array_section)?)
        } else {
            None
        };

        self.init = init;
        self.fini = fini;

        Ok(())
    }

    // relocations : rel, rela, pltrel
    pub fn relocate(&self, objects: &HashMap<&str, RefCell<Object>>) -> Result<(), LoaderError> {
        let dyn_section = if let Some(dyn_section) = DynamicSection::find(&self.elf_file)? {
            dyn_section
        } else {
            return Ok(());
        };

        let symbols = Symbols::find(self, &dyn_section)?;

        if let Some(table) = self.get_relocation_table::<sections::Rel<u64>>(
            &dyn_section,
            dynamic::Tag::Rel,
            dynamic::Tag::RelSize,
            Some(dynamic::Tag::RelEnt),
        )? {
            for entry in table.iter() {
                let relocation = Relocation::try_from(entry)?;
                debug!("rel {relocation:?}");
                self.process_relocation(objects, &symbols, relocation)?;
            }
        }

        if let Some(table) = self.get_relocation_table::<sections::Rela<u64>>(
            &dyn_section,
            dynamic::Tag::Rela,
            dynamic::Tag::RelaSize,
            Some(dynamic::Tag::RelaEnt),
        )? {
            for entry in table.iter() {
                let relocation = Relocation::try_from(entry)?;
                debug!("rela {relocation:?}");
                self.process_relocation(objects, &symbols, relocation)?;
            }
        }

        // DT_JMPREL => offset of table
        // DT_PLTREL => type of jump_rel (rel or rela)
        // DT_PLTRELSZ => size of table

        if let Some(plt_rel_type) = dyn_section.find_unique(dynamic::Tag::PltRel)? {
            // we have a PLT relocation table
            let plt_rel_type = wrap_res(plt_rel_type.get_val())?;

            const DT_RELA: u64 = 7;
            const DT_REL: u64 = 17;

            match plt_rel_type {
                DT_REL => {
                    if let Some(table) = self.get_relocation_table::<sections::Rel<u64>>(
                        &dyn_section,
                        dynamic::Tag::JmpRel,
                        dynamic::Tag::PltRelSize,
                        None,
                    )? {
                        for entry in table.iter() {
                            let relocation = Relocation::try_from(entry)?;
                            debug!("plt rel {relocation:?}");
                            self.process_relocation(objects, &symbols, relocation)?;
                        }
                    }
                }
                DT_RELA => {
                    if let Some(table) = self.get_relocation_table::<sections::Rela<u64>>(
                        &dyn_section,
                        dynamic::Tag::JmpRel,
                        dynamic::Tag::PltRelSize,
                        None,
                    )? {
                        for entry in table.iter() {
                            let relocation = Relocation::try_from(entry)?;
                            debug!("plt rela {relocation:?}");
                            self.process_relocation(objects, &symbols, relocation)?;
                        }
                    }
                }
                _ => {
                    return Err(LoaderError::BadDynamicSection);
                }
            }
        }

        if let Some(table) = self.get_relocation_table::<sections::Rel<u64>>(
            &dyn_section,
            dynamic::Tag::Rel,
            dynamic::Tag::RelSize,
            None,
        )? {
            for entry in table.iter() {
                debug!("REL {entry:?}");
            }
        }

        Ok(())
    }

    fn get_relocation_table<Relocation>(
        &self,
        dyn_section: &DynamicSection<'_>,
        offset: dynamic::Tag<u64>,
        size: dynamic::Tag<u64>,
        ent: Option<dynamic::Tag<u64>>,
    ) -> Result<Option<RelocationTable<'a, Relocation>>, LoaderError> {
        let table_offset = if let Some(entry) = dyn_section.find_unique(offset)? {
            wrap_res(entry.get_ptr())? as usize
        } else {
            return Ok(None);
        };

        let table_size = if let Some(entry) = dyn_section.find_unique(size)? {
            wrap_res(entry.get_val())? as usize
        } else {
            return Err(LoaderError::BadDynamicSection);
        };

        if let Some(ent) = ent {
            if let Some(entry) = dyn_section.find_unique(ent)? {
                let entry_size = wrap_res(entry.get_val())? as usize;

                // Make sure that the reported size matches our `Rela<u64>`.
                if entry_size != mem::size_of::<Relocation>() {
                    return Err(LoaderError::BadDynamicSection);
                }
            } else {
                return Err(LoaderError::BadDynamicSection);
            };
        }

        if table_size % mem::size_of::<Relocation>() > 0 {
            return Err(LoaderError::BadDynamicSection);
        }

        let base_address = self.addr_offset + table_offset;
        let count = table_size / mem::size_of::<Relocation>();

        Ok(Some(RelocationTable {
            base_address,
            count,
            _marker1: PhantomData,
            _marker2: PhantomData,
        }))
    }

    fn process_relocation(
        &self,
        objects: &HashMap<&str, RefCell<Object>>,
        symbols: &Option<Symbols>,
        relocation: Relocation,
    ) -> Result<(), LoaderError> {
        match relocation.r#type {
            RelocationType::R_X86_64_NONE => Ok(()),
            //RelocationType::R_X86_64_64 => todo!(),
            //RelocationType::R_X86_64_PC32 => todo!(),
            //RelocationType::R_X86_64_GOT32 => todo!(),
            //RelocationType::R_X86_64_PLT32 => todo!(),
            //RelocationType::R_X86_64_COPY => todo!(),
            //RelocationType::R_X86_64_GLOB_DAT => todo!(),
            RelocationType::R_X86_64_JUMP_SLOT => {
                let symbols = symbols.as_ref().ok_or(LoaderError::BadRelocation)?;

                let symbol = symbols.entry(relocation.symbol_table_index);
                let sym_name = wrap_res(symbol.get_name(&self.elf_file))?;

                let resolve = |object: &Object| -> Result<bool, LoaderError> {
                    if let Some(sym) = object.exports().get(sym_name) {
                        debug!(
                            "found match for symbol '{}' in '{}' at 0x{:016X}",
                            sym_name,
                            object.name(),
                            sym.address
                        );

                        relocation.apply(self, sym.address)?;

                        Ok(true)
                    } else {
                        Ok(false)
                    }
                };

                // First try to find in self (some missing symbols seems to be self-resolved..)
                if resolve(self)? {
                    return Ok(());
                }

                // Walk through needed until we find export
                for obj_name in self.needed() {
                    let dependency = objects
                        .get(obj_name)
                        .expect(&format!("dependency not loaded {obj_name}"));

                    if resolve(&dependency.borrow())? {
                        return Ok(());
                    }
                }

                Err(LoaderError::MissingSymbol(String::from(sym_name)))
            }
            RelocationType::R_X86_64_RELATIVE => {
                // Calculate the relocated value.
                let value =
                    self.addr_offset + relocation.addend.ok_or(LoaderError::BadRelocation)?;

                relocation.apply(self, value)?;

                Ok(())
            }
            //RelocationType::R_X86_64_GOTPCREL => todo!(),
            //RelocationType::R_X86_64_32 => todo!(),
            //RelocationType::R_X86_64_32S => todo!(),
            //RelocationType::R_X86_64_16 => todo!(),
            //RelocationType::R_X86_64_PC16 => todo!(),
            //RelocationType::R_X86_64_8 => todo!(),
            //RelocationType::R_X86_64_PC8 => todo!(),
            //RelocationType::R_X86_64_PC64 => todo!(),
            r#type => {
                unimplemented!("Unimplemented relocation of type {type:?}");
            }
        }
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn Error>> {
        // Now we can apply permissions on segments
        for segment in self.segments.take().unwrap() {
            segment.finalize()?;
        }

        Ok(())
    }

    fn find_section_by_offset(&self, offset: usize) -> Option<sections::SectionHeader<'a>> {
        for section_header in self.elf_file.section_iter() {
            if section_header.offset() as usize == offset {
                return Some(section_header);
            }
        }

        None
    }

    fn find_section_by_type(
        &self,
        r#type: sections::ShType,
    ) -> Result<Option<sections::SectionHeader<'a>>, LoaderError> {
        for section_header in self.elf_file.section_iter() {
            if wrap_res(section_header.get_type())? == r#type {
                return Ok(Some(section_header));
            }
        }

        Ok(None)
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

    pub fn name(&self) -> &str {
        self.name
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

    pub fn run_init(&self) {
        if let Some(funcs) = &self.init {
            funcs.run();
        }
    }

    pub fn run_fini(&self) {
        if let Some(funcs) = &self.fini {
            funcs.run();
        }
    }
}

#[derive(Debug)]
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
    ) -> Result<Vec<&'a dynamic::Dynamic<u64>>, LoaderError> {
        let mut res = Vec::new();

        for item in self.data {
            let item_tag = wrap_res(item.get_tag())?;

            if item_tag == tag {
                res.push(item);
            }
        }

        Ok(res)
    }

    pub fn find_unique(
        &self,
        tag: dynamic::Tag<u64>,
    ) -> Result<Option<&'a dynamic::Dynamic<u64>>, LoaderError> {
        let vec = self.find_all(tag)?;

        match vec.len() {
            0 => Ok(None),
            1 => Ok(Some(vec[0])),
            _ => Err(LoaderError::BadDynamicSection),
        }
    }
}

#[derive(Debug)]
struct Symbols<'a> {
    entries: &'a [DynEntry64],
}

impl<'a> Symbols<'a> {
    pub fn find(
        prog: &Object<'a>,
        dyn_section: &DynamicSection<'a>,
    ) -> Result<Option<Self>, LoaderError> {
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
            return Err(LoaderError::BadSymbolsSection);
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

#[derive(Debug)]
struct RelocationTable<'a, Relocation> {
    base_address: usize,
    count: usize,
    _marker1: PhantomData<&'a ()>, // Note: this way we ensure that address remain valid
    _marker2: PhantomData<Relocation>,
}

impl<'a, Relocation> RelocationTable<'a, Relocation> {
    pub fn size(&self) -> usize {
        self.count
    }

    pub fn entry(&self, index: usize) -> Relocation {
        assert!(index < self.count);

        let address = self.base_address + index * mem::size_of::<Relocation>();

        unsafe { core::ptr::read_unaligned(address as *const Relocation) }
    }

    pub fn iter(&self) -> RelocationTableIter<'a, Relocation> {
        RelocationTableIter {
            base_address: self.base_address,
            end_address: self.base_address + self.count * mem::size_of::<Relocation>(),
            _marker1: PhantomData,
            _marker2: PhantomData,
        }
    }
}

#[derive(Debug)]
struct RelocationTableIter<'a, Relocation> {
    base_address: usize,
    end_address: usize,
    _marker1: PhantomData<&'a ()>, // Note: this way we ensure that address remain valid
    _marker2: PhantomData<Relocation>,
}

impl<'a, Relocation> Iterator for RelocationTableIter<'a, Relocation> {
    type Item = Relocation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.base_address == self.end_address {
            return None;
        }

        let entry = unsafe { core::ptr::read_unaligned(self.base_address as *const Relocation) };

        self.base_address += mem::size_of::<Relocation>();
        assert!(self.base_address <= self.end_address);

        Some(entry)
    }
}

#[derive(Debug)]
#[repr(u32)]
#[allow(non_camel_case_types)]
enum RelocationType {
    R_X86_64_NONE = 0,      // No reloc
    R_X86_64_64 = 1,        // Direct 64 bit
    R_X86_64_PC32 = 2,      // PC relative 32 bit signed
    R_X86_64_GOT32 = 3,     // 32 bit GOT entry
    R_X86_64_PLT32 = 4,     // 32 bit PLT address
    R_X86_64_COPY = 5,      // Copy symbol at runtime
    R_X86_64_GLOB_DAT = 6,  // Create GOT entry
    R_X86_64_JUMP_SLOT = 7, // Create PLT entry
    R_X86_64_RELATIVE = 8,  // Adjust by program base
    R_X86_64_GOTPCREL = 9,  // 32 bit signed pc relative offset to GOT
    R_X86_64_32 = 10,       // Direct 32 bit zero extended
    R_X86_64_32S = 11,      // Direct 32 bit sign extended
    R_X86_64_16 = 12,       // Direct 16 bit zero extended
    R_X86_64_PC16 = 13,     // 16 bit sign extended pc relative
    R_X86_64_8 = 14,        // Direct 8 bit sign extended
    R_X86_64_PC8 = 15,      // 8 bit sign extended pc relative
    R_X86_64_PC64 = 24,     // Place relative 64-bit signed
}

// https://stackoverflow.com/questions/28028854/how-do-i-match-enum-values-with-an-integer
impl TryFrom<u32> for RelocationType {
    type Error = LoaderError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            x if x == RelocationType::R_X86_64_NONE as u32 => Ok(RelocationType::R_X86_64_NONE),
            x if x == RelocationType::R_X86_64_64 as u32 => Ok(RelocationType::R_X86_64_64),
            x if x == RelocationType::R_X86_64_PC32 as u32 => Ok(RelocationType::R_X86_64_PC32),
            x if x == RelocationType::R_X86_64_GOT32 as u32 => Ok(RelocationType::R_X86_64_GOT32),
            x if x == RelocationType::R_X86_64_PLT32 as u32 => Ok(RelocationType::R_X86_64_PLT32),
            x if x == RelocationType::R_X86_64_COPY as u32 => Ok(RelocationType::R_X86_64_COPY),
            x if x == RelocationType::R_X86_64_GLOB_DAT as u32 => {
                Ok(RelocationType::R_X86_64_GLOB_DAT)
            }
            x if x == RelocationType::R_X86_64_JUMP_SLOT as u32 => {
                Ok(RelocationType::R_X86_64_JUMP_SLOT)
            }
            x if x == RelocationType::R_X86_64_RELATIVE as u32 => {
                Ok(RelocationType::R_X86_64_RELATIVE)
            }
            x if x == RelocationType::R_X86_64_GOTPCREL as u32 => {
                Ok(RelocationType::R_X86_64_GOTPCREL)
            }
            x if x == RelocationType::R_X86_64_32 as u32 => Ok(RelocationType::R_X86_64_32),
            x if x == RelocationType::R_X86_64_32S as u32 => Ok(RelocationType::R_X86_64_32S),
            x if x == RelocationType::R_X86_64_16 as u32 => Ok(RelocationType::R_X86_64_16),
            x if x == RelocationType::R_X86_64_PC16 as u32 => Ok(RelocationType::R_X86_64_PC16),
            x if x == RelocationType::R_X86_64_8 as u32 => Ok(RelocationType::R_X86_64_8),
            x if x == RelocationType::R_X86_64_PC8 as u32 => Ok(RelocationType::R_X86_64_PC8),
            x if x == RelocationType::R_X86_64_PC64 as u32 => Ok(RelocationType::R_X86_64_PC64),
            _ => Err(LoaderError::BadRelocation),
        }
    }
}

#[derive(Debug)]
struct Relocation {
    offset: usize,
    addend: Option<usize>,
    symbol_table_index: usize,
    r#type: RelocationType,
}

impl Relocation {
    pub fn apply(&self, object: &Object, value: usize) -> Result<(), LoaderError> {
        object.check_is_in_load(self.offset as u64)?;

        let address = object.addr_offset + self.offset;
        unsafe { core::ptr::write_unaligned(address as *mut _, value) };
        Ok(())
    }
}

impl TryFrom<sections::Rela<u64>> for Relocation {
    type Error = LoaderError;

    fn try_from(value: sections::Rela<u64>) -> Result<Self, Self::Error> {
        Ok(Self {
            offset: value.get_offset() as usize,
            addend: Some(value.get_addend() as usize),
            symbol_table_index: value.get_symbol_table_index() as usize,
            r#type: value.get_type().try_into()?,
        })
    }
}

impl TryFrom<sections::Rel<u64>> for Relocation {
    type Error = LoaderError;

    fn try_from(value: sections::Rel<u64>) -> Result<Self, Self::Error> {
        Ok(Self {
            offset: value.get_offset() as usize,
            addend: None,
            symbol_table_index: value.get_symbol_table_index() as usize,
            r#type: value.get_type().try_into()?,
        })
    }
}

#[derive(Debug)]
struct FuncArray<'a> {
    addr_offset: usize,
    array: &'a [u64],
}

impl<'a> FuncArray<'a> {
    pub fn from_section(
        object: &Object<'a>,
        header: sections::SectionHeader<'a>,
    ) -> Result<Self, LoaderError> {
        let array = if let sections::SectionData::FnArray64(array) =
            wrap_res(header.get_data(&object.elf_file))?
        {
            array
        } else {
            return Err(LoaderError::BadInitFiniSection);
        };

        Ok(Self {
            addr_offset: object.addr_offset,
            array,
        })
    }

    pub fn run(&self) {
        for &entry in self.array {
            if entry != 0 {
                let addr = self.addr_offset + entry as usize;
                debug!("self.addr_offset = 0x{:016X}", self.addr_offset);
                debug!("entry = 0x{:016X}", entry);
                debug!("addr = 0x{:016X}", addr);
                let func: extern "C" fn() = unsafe { mem::transmute(addr) };

                func();
            }
        }
    }
}
