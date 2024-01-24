#![feature(error_in_core)]
#![feature(error_generic_member_access)]
#![allow(dead_code)]

// https://github.com/rust-osdev/bootloader/blob/main/common/src/load_kernel.rs

mod helpers;
mod kobject;
mod program;
mod segment;

use core::error::Error;
use log::debug;
use xmas_elf::{sections::SectionData, ElfFile};

pub use helpers::*;
pub use program::Program;
pub use segment::Segment;

const BINARY_PATH: &str = "static/hello";

pub fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    use std::{fs::File, io::Read};
    let mut file = File::open(BINARY_PATH).unwrap();
    let mut buff = Vec::new();
    file.read_to_end(&mut buff).unwrap();

    load(&buff)?;

    Ok(())
}

pub fn load(binary: &[u8]) -> Result<(), Box<dyn Error>> {
    let program = Program::load(binary)?;

    //resolve_dependencies(&elf_file);

    let entry_func = program.entry();

    debug!("Let go!");

    //let args = &["hello"];

    start(entry_func);
}

fn start(entry_func: extern "C" fn() -> !) -> ! {
    unsafe {
        core::arch::asm!("
        // All this registers seems to be = 0 at startup
        // mov rax, 0
        mov rbx, 0
        mov rcx, 0
        mov rdx, 0
        mov rsi, 0
        mov rdi, 0
        mov rbp, 0
        mov r8 , 0
        mov r9 , 0
        mov r10, 0
        mov r11, 0
        mov r12, 0
        mov r13, 0
        mov r14, 0
        mov r15, 0
        // int 3
        call rax
        ", 
        in("rax") entry_func,

        options(noreturn)
        );
    }
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
