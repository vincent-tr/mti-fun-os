#![feature(error_in_core)]

mod mapping;

use core::{fmt, error::Error, mem::size_of, cmp::{self, min}, ops::Range};
use log::debug;
use xmas_elf::{
    header, program,
    sections::{SectionData, ShType},
    symbol_table::{DynEntry32, DynEntry64},
    ElfFile, P64,
};
use zero::read;

pub fn main() -> Result<(), Box<dyn Error>> {
    
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    
    use std::{io::Read, fs::File};
    let mut file = File::open("/bin/ls").unwrap();
    let mut buff = Vec::new();
    file.read_to_end(&mut buff).unwrap();

    load(&buff)?;

    Ok(())
}

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
    let range = get_vm_range(&elf_file)?;
    debug!("range = 0x{0:016X}", range.start);
    debug!("range = 0x{0:016X}", range.end);

    

    //libc::mmap()

    assert!(range.start == 0);

    resolve_dependencies(&elf_file);

    Ok(())
}

fn get_vm_range(elf_file: &ElfFile) -> Result<Range<u64>, LoaderError> {
    let mut min = u64::MAX;
    let mut max = u64::MIN;
    for program_header in elf_file.program_iter() {
        if let program::Type::Load = wrap_res(program_header.get_type())? {
            debug!("PROGRAM {:?}", program_header);

            let start = align_down(program_header.virtual_addr(), program_header.align());
            let end = align_up(program_header.virtual_addr() + program_header.mem_size(), program_header.align());
            
            min = cmp::min(min, start);
            max = cmp::max(max, end);
        }
    }

    Ok(min..max)
}

fn align_down(value: u64, align: u64) -> u64 {
    value / align * align
}

fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) / align * align
}

fn load_program(elf_file: &ElfFile, base_addr: usize) {

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
