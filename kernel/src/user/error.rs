use crate::memory::VirtAddr;

pub enum Error {

}

pub fn check_arg(condition: bool) -> Result<(), Error> {
}

pub fn check_is_userspace(addr: VirtAddr) -> Result<(), Error> {

}

pub fn check_page_alignment(addr: usize) -> Result<(), Error> {

}

pub fn check_positive(value: usize) -> Result<(), Error> {
  
}


pub fn out_of_memory() -> Error {

}