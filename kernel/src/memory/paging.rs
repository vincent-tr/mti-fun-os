use spin::Mutex;
use x86_64::{
    structures::paging::{Page, PageTable, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

use super::{frame_allocator, phys_view};
use crate::{error::Error, memory::PAGE_SIZE, println};

pub fn init() {
    // Nothing for now
}

pub fn translate(addr: VirtAddr) -> Option<PhysAddr> {
    let p4 = active_level_4_table();
    let e4 = &p4[addr.p4_index()];
    if !e4.flags().contains(PageTableFlags::PRESENT) {
        return None;
    }

    let p3 = frame_to_page_table(PhysFrame::from_start_address(e4.addr()).unwrap());
    let e3 = &p3[addr.p3_index()];
    if !e3.flags().contains(PageTableFlags::PRESENT) {
        return None;
    }

    if e3.flags().contains(PageTableFlags::HUGE_PAGE) {
        let offset = addr.as_u64() & 0o_777_777_7777;
        return Some(e3.addr() + offset);
    }

    let p2 = frame_to_page_table(PhysFrame::from_start_address(e3.addr()).unwrap());
    let e2 = &p2[addr.p2_index()];
    if !e2.flags().contains(PageTableFlags::PRESENT) {
        return None;
    }

    if e2.flags().contains(PageTableFlags::HUGE_PAGE) {
        let offset = addr.as_u64() & 0o_777_7777;
        return Some(e2.addr() + offset);
    }

    let p1 = frame_to_page_table(PhysFrame::from_start_address(e2.addr()).unwrap());
    let e1 = &p1[addr.p1_index()];
    if !e1.flags().contains(PageTableFlags::PRESENT) {
        return None;
    }

    let offset = u64::from(addr.page_offset());
    return Some(e1.addr() + offset);
}

// Note: no huge pages for now
pub fn map(page: Page, frame: PhysFrame, flags: PageTableFlags) -> Result<(), Error> {
    unimplemented!();
}

// Note: no huge pages for now
pub fn unmap(page: Page) -> Result<PhysFrame, Error> {
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
