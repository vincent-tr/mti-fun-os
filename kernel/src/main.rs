#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_sub_ptr)]
#![feature(slice_ptr_get)]
#![feature(const_slice_from_raw_parts_mut)]
#![feature(is_sorted)]
#![feature(slice_ptr_len)]

extern crate bootloader_api;
extern crate lazy_static;

mod gdt;
mod interrupts;
mod logging;
mod memory;

use bootloader_api::config::Mapping;
use bootloader_api::info::MemoryRegionKind;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use x86_64::VirtAddr;
use x86_64::structures::paging::page_table::PageTableEntry;
use core::panic::PanicInfo;
use log::{error, info};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable, PageTableFlags};

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.dynamic_range_start = Some(0xFFFF_8000_0000_0000);
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_8080_0000_0000));
    config
};

/*

INFO - Kernel      0xffff800000004883
INFO - Phys mem    0xffff808000000000

INFO - Stack       0xffff810000014b14
INFO - Framebuffer 0xffff818000000000
INFO - Boot info   0xffff820000000000

*/

entry_point!(kernel_main, config = &CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    logging::init();

    let version = &boot_info.api_version;
    info!("Starting kernel with boot info v{}.{}.{}", version.version_major(), version.version_minor(), version.version_patch());

    let physical_memory_offset = VirtAddr::new(*boot_info.physical_memory_offset.as_ref().unwrap());

    gdt::init();
    interrupts::init_idt();
    memory::phys::init(physical_memory_offset, &boot_info.memory_regions);

    let stack_var = 12;

    let framebuffer = boot_info.framebuffer.as_ref().unwrap().buffer();

    info!("Kernel      {:?}", (&kernel_main as *const _));
    info!("Stack       {:?}", (&stack_var as *const _));
    info!("Framebuffer {:?}", framebuffer.as_ptr());
    info!("Phys mem    {:?}", physical_memory_offset.as_ptr::<u8>());
    info!("Boot info   {:?}", boot_info as *const _);

    // print_mem(&boot_info);

    print_pt(physical_memory_offset);

    panic!("End of main!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info}");
    halt()
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

fn print_mem(boot_info: &BootInfo) {
    for region in boot_info.memory_regions.iter() {
        info!("region: {region:?}");
    }

    let mut start: u64 = 0;
    let mut size: u64 = 0;

    for region in boot_info.memory_regions.iter() {
        if let MemoryRegionKind::Usable = region.kind {
            if start > 0 && start + size == region.start {
                size += region.end - region.start;
            } else {
                if size > 0 {
                    info!("usable region: start={start}, size={size}");
                }

                start = region.start;
                size = region.end - region.start;
            }
        }
    }

    if size > 0 {
        info!("usable region: start={start}, size={size}");
    }
}

fn print_pt(phys_offset: VirtAddr) {
    let (level_4_table, _) = Cr3::read();
    info!("Level 4 table at: {:?}", level_4_table.start_address());


    let mut page_table = unsafe {
        let virt = phys_offset + level_4_table.start_address().as_u64();
        let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    
        &mut *page_table_ptr
    };

    print_pt_recurs(page_table, phys_offset, 4);
}

fn print_pt_recurs(page_table: &PageTable, phys_offset: VirtAddr, level: usize) {
    for (index, entry) in page_table.iter().enumerate() {
        if !entry.is_unused() {
            print_pt_entry(index, entry, level);


            if level == 4 {
                info!("Start address : {:#X}", VirtAddr::new(index as u64 * 0x8000000000).as_u64());
            }

            if level >= 2 && !entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                if !memory::phys::used(entry.addr()) {
                    error!("Unused phsical frame : {:#X}", entry.addr().as_u64());
                }
    
                let next_table_addr = phys_offset + entry.addr().as_u64();
                let next_table_ref: &PageTable = unsafe { &(*next_table_addr.as_ptr())};
                print_pt_recurs(next_table_ref, phys_offset, level - 1);
            }
        }
    }
}

fn print_pt_entry(index: usize, entry: &PageTableEntry, level: usize) {
    match level {
        4 => {
            info!("Entry #{index}: {:?}", entry);
        },
        3 => {
            info!("  Entry #{index}: {:?}", entry);
        },
        2 => {
            info!("    Entry #{index}: {:?}", entry);
        },
        1 => {
            info!("      Entry #{index}: {:?}", entry);
        },
        _ => unimplemented!(),
    }
}