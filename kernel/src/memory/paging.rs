use spin::Mutex;
use x86_64::{
    structures::paging::{PageTable, PhysFrame},
    VirtAddr,
};

use super::{frame_allocator, phys_view};
use crate::{error::Error, memory::PAGE_SIZE, println};

pub fn init() {
    unimplemented!();
}

pub fn map(addr: VirtAddr, frame: PhysFrame) -> Result<(), Error> {
    unimplemented!();
}

pub fn unmap(addr: VirtAddr) -> Result<PhysFrame, Error> {
    unimplemented!();
}

/// Returns a mutable reference to the active level 4 table.
fn active_level_4_table() -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (frame, _) = Cr3::read();
    frame_to_page_table(frame)
}

/// This function must be only called once to avoid aliasing `&mut` references (which is undefined behavior).
fn frame_to_page_table(frame: PhysFrame) -> &'static mut PageTable {
    let address = phys_view::to_virt_view(frame.start_address());
    let page_table_ptr: *mut PageTable = address.as_mut_ptr();
    unsafe { &mut *page_table_ptr }
}

fn allocate_page_table() -> Result<&'static mut PageTable, Error> {
    let frame = frame_allocator::allocate()?;
    let page_table = frame_to_page_table(frame);

    page_table.zero();

    Ok(page_table)
}

fn deallocate_page_table(page_table: &'static mut PageTable) {
    let phys_address = phys_view::to_phys(VirtAddr::from_ptr(page_table));
    let frame = PhysFrame::from_start_address(phys_address).unwrap();
    frame_allocator::deallocate(frame);
}
