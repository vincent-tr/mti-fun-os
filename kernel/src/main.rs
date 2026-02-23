#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(abi_x86_interrupt)]
#![feature(slice_ptr_get)]
#![feature(allocator_api)]
#![feature(btree_cursors)]
#![feature(const_trait_impl)]
#![feature(linked_list_cursors)]
#![feature(trait_alias)]
#![feature(never_type)]
#![feature(step_trait)]
#![feature(naked_functions_rustic_abi)]

extern crate alloc;
extern crate bootloader_api;
extern crate lazy_static;

mod util;

mod devices;
mod gdt;
mod interrupts;
mod logging;
mod memory;

mod user;

use crate::memory::VirtAddr;
use alloc::boxed::Box;
use bootloader_api::{
    BootInfo, BootloaderConfig, config::Mapping, entry_point, info::FrameBufferInfo,
};
use core::{ops::Range, panic::PanicInfo};
use log::{error, info};
use syscalls::SyscallNumber;
use x86_64::registers::model_specific::{Efer, EferFlags};

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.dynamic_range_start = Some(0xFFFF_8000_0000_0000);
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_8080_0000_0000));
    config
};

entry_point!(kernel_main, config = &CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    logging::init();

    let efer_flags = Efer::read();
    assert!(efer_flags.contains(EferFlags::LONG_MODE_ENABLE));
    assert!(efer_flags.contains(EferFlags::LONG_MODE_ACTIVE));

    let version = &boot_info.api_version;
    info!(
        "Starting kernel with boot info v{}.{}.{}",
        version.version_major(),
        version.version_minor(),
        version.version_patch()
    );

    let physical_memory_offset = VirtAddr::new(*boot_info.physical_memory_offset.as_ref().unwrap());

    let ramdisk_start = *boot_info.ramdisk_addr.as_ref().expect("No ramdisk defined") as usize;
    let ramdisk = ramdisk_start..(ramdisk_start + boot_info.ramdisk_len as usize);

    let fb_info = boot_info
        .framebuffer
        .as_ref()
        .expect("No framebuffer defined");
    let fb_ptr = fb_info.buffer().as_ptr() as u64;
    let fb_info = fb_info.info();

    gdt::init();
    interrupts::init_base();
    memory::init(physical_memory_offset, &boot_info.memory_regions, &ramdisk);

    // Note:
    // boot_info is unmapped from here.
    // Do not used it.

    // From here we can use normal allocations in the kernel.

    devices::init();
    interrupts::init_userland();
    user::init();

    let init_info = build_init_info(&ramdisk, fb_info, fb_ptr);

    interrupts::syscall_switch(
        SyscallNumber::InitSetup as usize,
        ramdisk.start,
        ramdisk.end,
        Box::leak(init_info) as *const _ as usize,
        0,
        0,
        0,
    );
}

fn build_init_info(
    ramdisk: &Range<usize>,
    fb_info: FrameBufferInfo,
    fb_ptr: u64,
) -> Box<syscalls::init::InitInfo> {
    let pixel_format = match fb_info.pixel_format {
        bootloader_api::info::PixelFormat::Rgb => syscalls::init::PixelFormat {
            red_mask: 0x00FF_0000,
            green_mask: 0x0000_FF00,
            blue_mask: 0x0000_00FF,
        },
        bootloader_api::info::PixelFormat::Bgr => syscalls::init::PixelFormat {
            red_mask: 0x0000_00FF,
            green_mask: 0x0000_FF00,
            blue_mask: 0x00FF_0000,
        },
        bootloader_api::info::PixelFormat::Unknown {
            red_position,
            green_position,
            blue_position,
        } => syscalls::init::PixelFormat {
            red_mask: 0xFF << red_position,
            green_mask: 0xFF << green_position,
            blue_mask: 0xFF << blue_position,
        },
        _ => panic!("Unsupported pixel format"),
    };

    Box::new(syscalls::init::InitInfo {
        init_mapping: syscalls::init::InitMapping {
            mapping_size: ramdisk.len(),
        },
        framebuffer: syscalls::init::Framebuffer {
            address: fb_ptr as usize,
            byte_len: fb_info.byte_len,
            width: fb_info.width,
            height: fb_info.height,
            pixel_format,
            bytes_per_pixel: fb_info.bytes_per_pixel,
            stride: fb_info.stride,
        },
    })
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();

    error!("PANIC: {info}");
    halt()
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
