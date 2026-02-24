#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use log::info;

mod framebuffer;

use framebuffer::{BufferShape, FrameBuffer, PixelFormat};

extern crate alloc;
extern crate libruntime;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    info!("Display server started");

    let address: usize = read_usize_arg("framebuffer.address");
    let byte_len: usize = read_usize_arg("framebuffer.byte_len");
    let width: usize = read_usize_arg("framebuffer.width");
    let height: usize = read_usize_arg("framebuffer.height");
    let red_mask: u32 = read_usize_arg("framebuffer.pixel_format.red_mask") as u32;
    let green_mask: u32 = read_usize_arg("framebuffer.pixel_format.green_mask") as u32;
    let blue_mask: u32 = read_usize_arg("framebuffer.pixel_format.blue_mask") as u32;
    let bytes_per_pixel: usize = read_usize_arg("framebuffer.bytes_per_pixel");
    let stride: usize = read_usize_arg("framebuffer.stride");

    let shape = BufferShape::new(byte_len, width, height, stride, bytes_per_pixel);
    let pixel_format = PixelFormat::new(red_mask, green_mask, blue_mask, bytes_per_pixel);
    let mut framebuffer = FrameBuffer::open(address, shape, pixel_format);

    loop {
        libruntime::time::sleep(libruntime::time::Duration::seconds(1));
    }
}

fn read_usize_arg(name: &str) -> usize {
    let value = libruntime::process::SelfProcess::get()
        .arg(name)
        .expect("Failed to read argument");
    value
        .parse::<usize>()
        .expect("Failed to parse argument as usize")
}
