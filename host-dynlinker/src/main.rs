#![feature(error_in_core)]
#![feature(error_generic_member_access)]
#![feature(let_chains)]
#![allow(dead_code)]

// https://github.com/rust-osdev/bootloader/blob/main/common/src/load_kernel.rs

mod helpers;
mod kobject;
mod object;
mod segment;

use core::error::Error;
use std::collections::HashMap;
use log::debug;

pub use helpers::*;
pub use object::Object;
pub use segment::Segment;

const BINARY_PATH: &str = "static/hello";
const SHARED_PATH: &str = "static/shared.so";

pub fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let hello_content = read_file(BINARY_PATH);
    let shared_content = read_file(SHARED_PATH);

    let mut objects = HashMap::new();

    debug!("loading hello");
    objects.insert("hello", Object::load(&hello_content)?);
    debug!("loaded hello");

    // TODO: recursive
    debug!("loading shared.so");
    objects.insert("shared.so", Object::load(&shared_content)?);
    debug!("loaded shared.so");

    for obj in objects.values() {
        obj.process_jump_relocations(&objects)?;
    }

    // TODO: DT_INIT, DT_FINI

    let entry = objects.get("hello").unwrap();

    let entry_func = entry.entry();
    debug!("Let go!");
    start(entry_func);
}

fn read_file(path: &str) -> Vec<u8> {
    use std::{fs::File, io::Read};
    let mut file = File::open(path).unwrap();
    let mut buff = Vec::new();
    file.read_to_end(&mut buff).unwrap();

    buff
}
// strlen@got.plt
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
        int 3
        call rax
        ", 
        in("rax") entry_func,

        options(noreturn)
        );
    }
}
