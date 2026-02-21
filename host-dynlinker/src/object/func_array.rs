use crate::Object;
use core::mem;
use log::debug;
use xmas_elf::sections;

pub use crate::{LoaderError, wrap_res};

#[derive(Debug)]
pub struct FuncArray<'a> {
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
