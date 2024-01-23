use core::{error::Error, fmt, mem::size_of};
use log::debug;
use xmas_elf::{
    header, program,
    sections::{SectionData, ShType},
    symbol_table::{DynEntry32, DynEntry64},
    ElfFile, P64,
};
use zero::read;

pub fn load(binary: &[u8]) -> Result<(), LoaderError> {
    let elf_file = wrap_res(xmas_elf::ElfFile::new(binary))?;

    for program_header in elf_file.program_iter() {
        wrap_res(program::sanity_check(program_header, &elf_file))?;
    }

    match elf_file.header.pt2.type_().as_type() {
        header::Type::None => Err(LoaderError::BadObjectType("none"))?,
        header::Type::Relocatable => Err(LoaderError::BadObjectType("relocatable"))?,
        header::Type::Executable => Err(LoaderError::BadObjectType("executable"))?,
        header::Type::SharedObject => (),
        header::Type::Core => Err(LoaderError::BadObjectType("core"))?,
        header::Type::ProcessorSpecific(_) => {
            Err(LoaderError::BadObjectType("processor-specific"))?
        }
    };

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

    resolve_dependencies(&elf_file);

    Ok(())
}

fn resolve_dependencies(elf_file: &ElfFile) -> Result<bool, LoaderError> {
    let section = {
        if let Some(section) = elf_file.find_section_by_name(".dynamic") {
            section
        } else {
            return Ok(false);
        }
    };

    let data = {
        if let SectionData::Dynamic64(data) = wrap_res(section.get_data(&elf_file))? {
            data
        } else {
            return Err(LoaderError::BadDynamicSection);
        }
    };

    for entry in data {
        match wrap_res(entry.get_tag())? {
            xmas_elf::dynamic::Tag::Null => {
                // this mark the end of the section
                break;
            }

            xmas_elf::dynamic::Tag::Needed => {
                // dynamic library to load
                let str = wrap_res(elf_file.get_dyn_string(wrap_res(entry.get_val())? as u32))?;
                debug!("NEEDED: {str}");
            }
            _ => {
                // process later
                debug!(".dynamic entry: {:?}", entry);
            }
        };
    }
    Ok(true)
}

fn load_segments(elf_file: &ElfFile) {}

fn wrap_res<T>(res: Result<T, &'static str>) -> Result<T, LoaderError> {
    res.map_err(|str| LoaderError::ElfReaderError(str))
}

#[derive(Debug, Clone, Copy)]
pub enum LoaderError {
    ElfReaderError(&'static str),
    BadObjectType(&'static str),
    BadDynamicSection,
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
            LoaderError::BadDynamicSection => {
                write!(formatter, "bad dynamic section")
            }
        }
    }
}

impl Error for LoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
