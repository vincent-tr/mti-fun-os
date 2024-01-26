use xmas_elf::sections;

pub use crate::LoaderError;
use crate::Object;

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
#[allow(non_camel_case_types)]
pub enum RelocationType {
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
pub struct Relocation {
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

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn addend(&self) -> Option<usize> {
        self.addend
    }

    pub fn symbol_table_index(&self) -> usize {
        self.symbol_table_index
    }

    pub fn r#type(&self) -> RelocationType {
        self.r#type
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
