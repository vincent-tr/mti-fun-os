use crate::{process::iface::SymBlock, sync::spin::OnceLock};

/// Indicate information on a location
#[derive(Debug, Clone)]
pub struct LocationInfo<'a> {
    function_name: &'a str,
    function_offset: usize,
}

impl<'a> LocationInfo<'a> {
    /// Get the name of the function
    pub const fn function_name(&self) -> &'a str {
        self.function_name
    }

    /// Get the offset in the function
    pub const fn function_offset(&self) -> usize {
        self.function_offset
    }

    const fn new(function_name: &'a str, function_offset: usize) -> Self {
        Self {
            function_name,
            function_offset,
        }
    }
}

/// Global symbol information, used for debugging
static SYMBOLS: OnceLock<SymBlock> = OnceLock::new();

/// Init the global symbol information
pub fn init_symbols(sym_block: SymBlock) {
    SYMBOLS
        .set(sym_block)
        .expect("failed to set global symbol information");
}

/// Find the location information of the given address
pub fn find_location_info(addr: usize) -> Option<LocationInfo<'static>> {
    if let Some(symbols) = SYMBOLS.get() {
        if let Some((sym_addr, sym_name)) = symbols.lookup(addr as u64) {
            let function_offset = addr - sym_addr as usize;
            return Some(LocationInfo::new(sym_name, function_offset));
        }
    }

    None
}
