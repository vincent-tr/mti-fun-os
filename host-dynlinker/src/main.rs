#![feature(error_in_core)]
#![feature(error_generic_member_access)]
#![feature(let_chains)]
#![allow(dead_code)]

// https://github.com/rust-osdev/bootloader/blob/main/common/src/load_kernel.rs

mod helpers;
mod kobject;
mod object;

use core::{cell::RefCell, error::Error};
use log::debug;
use std::collections::HashMap;

pub use helpers::*;
pub use object::Object;

const BINARY_PATH: &str = "static/hello";
const SHARED_PATH: &str = "static/shared.so";

pub struct Program<'a> {
    entry: &'a str,
    objects: HashMap<&'a str, RefCell<Object<'a>>>,
}

impl<'a> Program<'a> {
    pub fn new(entry_name: &'a str) -> Self {
        Self {
            entry: entry_name,
            objects: HashMap::new(),
        }
    }

    pub fn load_object(&mut self, name: &'a str, binary: &'a [u8]) -> Result<(), Box<dyn Error>> {
        debug!("loading {name}");

        self.objects
            .insert(name, RefCell::new(Object::load(name, &binary)?));

        Ok(())
    }

    pub fn relocate(&mut self) -> Result<(), Box<dyn Error>> {
        for (name, obj) in self.objects.iter() {
            let mut obj = obj.borrow_mut();
            debug!("relocate {name}");
            obj.relocate(&self.objects)?;
            obj.finalize()?;
        }

        Ok(())
    }

    pub fn run_init(&self) {
        // TODO: order
        for obj in self.objects.values() {
            obj.borrow().run_init();
        }
    }

    pub fn run_fini(&self) {
        // TODO: order
        for obj in self.objects.values() {
            obj.borrow().run_init();
        }
    }

    pub fn run_entry(&self) -> ! {
        let entry = self
            .objects
            .get(self.entry)
            .expect("entry object not found!");

        let entry_func = entry.borrow().entry();
        debug!("Let go!");

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
}

pub fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let hello_content = read_file(BINARY_PATH);
    let shared_content = read_file(SHARED_PATH);

    let mut program = Program::new("hello");

    // TODO: recursive
    program.load_object("hello", &hello_content)?;
    program.load_object("shared.so", &shared_content)?;

    program.relocate()?;

    program.run_init();

    program.run_entry();

    // program.run_init();
}

fn read_file(path: &str) -> Vec<u8> {
    use std::{fs::File, io::Read};
    let mut file = File::open(path).unwrap();
    let mut buff = Vec::new();
    file.read_to_end(&mut buff).unwrap();

    buff
}
