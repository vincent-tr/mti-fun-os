use core::fmt;
use spin::Mutex;
use x86_64::{
    structures::paging::{Page, PageTable, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

use super::{frame_allocator, phys_view, VM_SPLIT};
use crate::{error::Error, memory::PAGE_SIZE, println};

pub struct Protection {
    read: bool,
    write: bool,
    execute: bool,
}

impl Protection {
    pub fn can_read(self) -> bool {
        self.read
    }

    pub fn can_write(self) -> bool {
        self.write
    }

    pub fn can_execute(self) -> bool {
        self.execute
    }

    pub fn read_only() -> Protection {
        Protection {
            read: true,
            write: false,
            execute: false,
        }
    }

    pub fn read_write() -> Protection {
        Protection {
            read: true,
            write: true,
            execute: false,
        }
    }

    pub fn read_execute() -> Protection {
        Protection {
            read: true,
            write: false,
            execute: true,
        }
    }
}

impl fmt::Debug for Protection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug_tuple = f.debug_tuple("Protection");

        if self.read {
            debug_tuple.field(&format_args!("R"));
        }

        if self.write {
            debug_tuple.field(&format_args!("W"));
        }

        if self.execute {
            debug_tuple.field(&format_args!("X"));
        }

        debug_tuple.finish()
    }
}

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

/// Map a virtual page to a physical frame
/// Notes:
/// - no huge pages for now
/// - if page addr > VM_SPLIT we use it for kernel, else for userland
pub fn map(page: Page, frame: PhysFrame, protection: Protection) -> Result<(), Error> {
    unimplemented!();
}

/// Unmap a virtual page from a physical frame
/// Note: no huge pages for now
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
