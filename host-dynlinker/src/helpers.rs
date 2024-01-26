use core::{error::Error, fmt};

pub const PAGE_SIZE: usize = 0x1000;

pub fn align_down(value: usize, align: usize) -> usize {
    value / align * align
}

pub fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) / align * align
}

pub fn wrap_res<T>(res: Result<T, &'static str>) -> Result<T, LoaderError> {
    res.map_err(|str| LoaderError::ElfReaderError(str))
}

#[derive(Debug, Clone)]
pub enum LoaderError {
    ElfReaderError(&'static str),
    BadObjectType(&'static str),
    BadDynamicSection,
    BadSymbolsSection,
    BadRelocation,
    BadInitFiniSection,
    MissingSymbol(String),
}

impl fmt::Display for LoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self {
            LoaderError::ElfReaderError(str) => {
                write!(formatter, "elf reader error: {}", str)
            }
            LoaderError::BadObjectType(typ) => {
                write!(formatter, "bad object type: '{}'", typ)
            }
            LoaderError::BadDynamicSection => {
                write!(formatter, "bad dynamic section")
            }
            LoaderError::BadSymbolsSection => {
                write!(formatter, "bad symbols section")
            }
            LoaderError::BadRelocation => {
                write!(formatter, "bad relocation")
            }
            LoaderError::BadInitFiniSection => {
                write!(formatter, "bad init/fini section")
            }
            LoaderError::MissingSymbol(name) => {
                write!(formatter, "missing symbol '{name}'")
            }
        }
    }
}

impl Error for LoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
