use addr2line::{
    gimli::{Dwarf, EndianSlice, RunTimeEndian, SectionId},
    object::{File, Object, ObjectSection},
    Context, Frame,
};
use alloc::borrow::Cow;
use core::panic::Location;
use typed_arena::Arena;

use crate::sync::OnceLock;

/// Indicate information on a location
pub struct LocationInfo<'a> {
    function_name: Option<&'a str>,
    source_location: Option<Location<'a>>,
}

impl<'a> LocationInfo<'a> {
    /// Get the name of the function
    pub const fn function_name(&self) -> Option<&str> {
        self.function_name
    }

    /// Get the location in source code
    pub const fn source_location(&self) -> Option<&Location> {
        self.source_location.as_ref()
    }

    const fn new(function_name: Option<&'a str>, source_location: Option<Location<'a>>) -> Self {
        Self {
            function_name,
            source_location,
        }
    }
}

static BINARY: OnceLock<&'static [u8]> = OnceLock::new();

static FINDER: OnceLock<Finder> = OnceLock::new();

/// Init the debugging engine using a binary file fully loaded into a chunk of memory
pub fn init_memory_binary(binary: &'static [u8]) {
    let _ = BINARY.set(binary);
}

/// Find the location information of the given address
pub fn find_location_info(addr: usize) -> Option<LocationInfo<'static>> {
    let finder: &Finder<'static> = FINDER.get_or_init(|| {
        let binary = BINARY.get().expect("Binary not set");
        Finder::load(binary)
    });

    if let Some(frame) = finder.find_frame(addr) {
        let source_location = if let Some(location) = frame.location {
            if let (Some(file), Some(line), Some(column)) =
                (location.file, location.line, location.column)
            {
                Some(Location::internal_constructor(file, line, column))
            } else {
                None
            }
        } else {
            None
        };

        let function_name = if let Some(function) = &frame.function {
            let function_name: &'static str = finder
                .strings
                .alloc_str(&function.demangle().expect("Could not demangle name"));

            Some(function_name)
        } else {
            None
        };

        Some(LocationInfo::new(function_name, source_location))
    } else {
        None
    }
}

pub struct Finder<'a> {
    strings: Arena<u8>,
    context: Context<EndianSlice<'a, RunTimeEndian>>,
}

unsafe impl<'a> Sync for Finder<'a> {}
unsafe impl<'a> Send for Finder<'a> {}

impl<'a> Finder<'a> {
    pub fn load(binary: &'a [u8]) -> Self {
        let object = &addr2line::object::File::parse(binary).expect("Could not load binary");

        let endian = if object.is_little_endian() {
            RunTimeEndian::Little
        } else {
            RunTimeEndian::Big
        };

        let strings = Arena::new();

        let mut load_section = |id: SectionId| -> Result<EndianSlice<'a, RunTimeEndian>, ()> {
            // `load_section`` only lives while `Dwarf::load`, so we can safely take strings ref
            // TODO: how to makes this understandable to the borrow checker?
            let strings = unsafe { &*(&strings as *const _) };

            load_file_section(id, object, endian, &strings)
        };

        let dwarf = Dwarf::load(&mut load_section).expect("Could not load DWARF");

        let context = Context::from_dwarf(dwarf).expect("Could not create context");

        Self { strings, context }
    }

    pub fn find_frame(&self, addr: usize) -> Option<Frame<'_, EndianSlice<'a, RunTimeEndian>>> {
        self.context
            .find_frames(addr as u64)
            .skip_all_loads()
            .expect("Could not get frame (1)")
            .next()
            .expect("Could not get frame (2)")
    }
}

fn load_file_section<'input, 'arena>(
    id: SectionId,
    file: &File<'input>,
    endian: RunTimeEndian,
    strings: &'arena Arena<u8>,
) -> Result<EndianSlice<'arena, RunTimeEndian>, ()>
where
    'input: 'arena,
{
    // TODO: Unify with dwarfdump.rs in gimli.
    let name = id.name();
    match file.section_by_name(name) {
        Some(section) => match section.uncompressed_data().unwrap() {
            Cow::Borrowed(b) => Ok(EndianSlice::new(b, endian)),
            Cow::Owned(b) => Ok(EndianSlice::new(
                strings.alloc_extend(b.into_iter()),
                endian,
            )),
        },
        None => Ok(EndianSlice::new(&[][..], endian)),
    }
}
