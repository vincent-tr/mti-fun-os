use xmas_elf::{dynamic, sections, symbol_table::DynEntry64};

use super::{DynamicSection, Object};
pub use crate::{LoaderError, wrap_res};

#[derive(Debug)]
pub struct Symbols<'a> {
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
