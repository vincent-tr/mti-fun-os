#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_sub_ptr)]
#![feature(slice_ptr_get)]
#![feature(const_slice_from_raw_parts_mut)]
#![feature(is_sorted)]

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
use core::panic::PanicInfo;
use log::{error, info};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable};

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.dynamic_range_start = Some(0xFFFF_8000_0000_0000);
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

/*

INFO - Kernel      0xffff800000003111
INFO - Stack       0xffff808000014b54
INFO - Framebuffer 0xffff810000000000
INFO - Phys mem    0xffff818000000000
INFO - Boot info   0xffff820000000000

*/

entry_point!(kernel_main, config = &CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    logging::init();

    info!("Entered kernel with boot info: {boot_info:?}");

    let stack_var = 12;

    let physical_memory_offset = *boot_info.physical_memory_offset.as_ref().unwrap();
    let framebuffer = boot_info.framebuffer.as_ref().unwrap().buffer();

    info!("Kernel      {:?}", (&kernel_main as *const _));
    info!("Stack       {:?}", (&stack_var as *const _));
    info!("Framebuffer {:?}", framebuffer.as_ptr());
    info!("Phys mem    {:#x?}", physical_memory_offset);
    info!("Boot info   {:?}", boot_info as *const _);

    print_mem(&boot_info);

    gdt::init();
    interrupts::init_idt();

    let (level_4_table, _) = Cr3::read();
    info!("Level 4 table at: {:?}", level_4_table.start_address());

    let phys_offset = VirtAddr::new(physical_memory_offset);

    let mut page_table = unsafe {
        let virt = phys_offset + level_4_table.start_address().as_u64();
        let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    
        OffsetPageTable::new(&mut *page_table_ptr, phys_offset)
    };

    memory::phys::init(phys_offset, &boot_info.memory_regions);

    //page_table.

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
        } else {
            if size > 0 {
                info!("usable region: start={start}, size={size}");
            }

            start = 0;
            size = 0;
        }
    }

    if size > 0 {
        info!("usable region: start={start}, size={size}");
    }
}
