#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_sub_ptr)]
#![feature(slice_ptr_get)]
#![feature(const_slice_from_raw_parts_mut)]
#![feature(is_sorted)]
#![feature(slice_ptr_len)]
#![feature(allocator_api)]
#![feature(const_mut_refs)]
#![feature(btree_cursors)]
#![feature(let_chains)]

extern crate bootloader_api;
extern crate lazy_static;
extern crate alloc;

mod gdt;
mod interrupts;
mod logging;
mod memory;

mod user;
use alloc::boxed::Box;

use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use log::{error, info};
use x86_64::VirtAddr;

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.dynamic_range_start = Some(0xFFFF_8000_0000_0000);
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_8080_0000_0000));
    config
};

entry_point!(kernel_main, config = &CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    logging::init();

    let version = &boot_info.api_version;
    info!(
        "Starting kernel with boot info v{}.{}.{}",
        version.version_major(),
        version.version_minor(),
        version.version_patch()
    );

    let physical_memory_offset = VirtAddr::new(*boot_info.physical_memory_offset.as_ref().unwrap());

    gdt::init();
    interrupts::init_idt();
    memory::init(physical_memory_offset, &boot_info.memory_regions);
    
    // Note:
    // boot_info is unmapped from here.
    // Do not used it.

    // From here we can use normal allocations in the kernel.

    let toto = Box::new(42);

    
  let stats = memory::stats();
  const MEGA: usize = 1 * 1024 * 1024;
  info!("kalloc: slabs: user={} ({}MB), allocated={} ({}MB)", stats.kalloc.slabs_user, stats.kalloc.slabs_user / MEGA, stats.kalloc.slabs_allocated, stats.kalloc.slabs_allocated / MEGA);
  info!("kalloc: kvm: user={} ({}MB), allocated={} ({}MB)", stats.kalloc.kvm_user, stats.kalloc.kvm_user / MEGA, stats.kalloc.kvm_allocated, stats.kalloc.kvm_allocated / MEGA);

    core::mem::drop(toto);

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

/*


/// Performs the actual context switch.
unsafe fn context_switch(addresses: Addresses) -> ! {
    unsafe {
        asm!(
            r#"
            xor rbp, rbp
            mov cr3, {}
            mov rsp, {}
            push 0
            jmp {}
            "#,
            in(reg) addresses.page_table.start_address().as_u64(),
            in(reg) addresses.stack_top.as_u64(),
            in(reg) addresses.entry_point.as_u64(),
            in("rdi") addresses.boot_info as *const _ as usize,
        );
    }
    unreachable!();
}

*/
