use xmas_elf::{ElfFile, P64, dynamic, sections};

pub use crate::{LoaderError, wrap_res};

#[derive(Debug)]
pub struct DynamicSection<'a> {
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
