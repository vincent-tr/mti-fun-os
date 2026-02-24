#![no_std]
#![no_main]

use alloc::{boxed::Box, vec::Vec};
use libruntime::kobject;
use log::info;

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

    let memory_object =
        unsafe { libruntime::kobject::MemoryObject::open_iomem(address, byte_len, false, true) }
            .expect("Failed to open framebuffer memory object");

    let proc = kobject::Process::current();
    let mapping = proc
        .map_mem(
            None,
            memory_object
                .size()
                .expect("Failed to get memory object size"),
            libruntime::kobject::Permissions::WRITE,
            &memory_object,
            0,
        )
        .expect("Failed to map framebuffer");
    let framebuffer = unsafe { mapping.as_buffer_mut() }.expect("Failed to obtain framebuffer");

    let pixel_format = PixelFormat::new(red_mask, green_mask, blue_mask, bytes_per_pixel);
    let color = pixel_format.rgb_to_native(255, 0, 0);
    draw_rectangle(
        framebuffer,
        width,
        height,
        stride,
        bytes_per_pixel,
        100,
        100,
        200,
        150,
        &color,
    );

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

fn draw_rectangle(
    framebuffer: &mut [u8],
    fb_width: usize,
    fb_height: usize,
    stride: usize,
    bytes_per_pixel: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: &[u8],
) {
    assert!(
        x < fb_width,
        "Rectangle x={} is out of bounds (width={})",
        x,
        fb_width
    );
    assert!(
        y < fb_height,
        "Rectangle y={} is out of bounds (height={})",
        y,
        fb_height
    );
    assert!(
        x + width <= fb_width,
        "Rectangle extends beyond width: x={}, width={}, fb_width={}",
        x,
        width,
        fb_width
    );
    assert!(
        y + height <= fb_height,
        "Rectangle extends beyond height: y={}, height={}, fb_height={}",
        y,
        height,
        fb_height
    );

    assert!(
        color.len() == bytes_per_pixel,
        "Color length {} does not match bytes per pixel {}",
        color.len(),
        bytes_per_pixel
    );

    for row in 0..height {
        for col in 0..width {
            let pixel_offset = ((y + row) * stride + (x + col)) * bytes_per_pixel;

            // Write each byte of the pixel color
            for (i, &byte) in color.iter().enumerate() {
                unsafe {
                    core::ptr::write_volatile(framebuffer.as_mut_ptr().add(pixel_offset + i), byte);
                }
            }
        }
    }
}

struct PixelFormat {
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
    bytes_per_pixel: usize,
}

impl PixelFormat {
    fn new(red_mask: u32, green_mask: u32, blue_mask: u32, bytes_per_pixel: usize) -> Self {
        // u32 used for internal conversion
        assert!(
            bytes_per_pixel <= size_of::<u32>(),
            "Unsupported bytes per pixel: {}",
            bytes_per_pixel
        );

        Self {
            red_mask,
            green_mask,
            blue_mask,
            bytes_per_pixel,
        }
    }

    /// Convert RGB (0-255 each) to native pixel format
    pub fn rgb_to_native(&self, r: u8, g: u8, b: u8) -> Box<[u8]> {
        let red_shift = self.red_mask.trailing_zeros();
        let green_shift = self.green_mask.trailing_zeros();
        let blue_shift = self.blue_mask.trailing_zeros();

        let red = (r as u32) << red_shift;
        let green = (g as u32) << green_shift;
        let blue = (b as u32) << blue_shift;

        let pixel_value = red | green | blue;

        // Write the pixel value as little-endian bytes
        // Write bytes in native order (no endianness conversion)
        let mut output = Vec::with_capacity(self.bytes_per_pixel);
        for i in 0..self.bytes_per_pixel {
            output.push(((pixel_value >> (i * 8)) & 0xFF) as u8);
        }
        output.into_boxed_slice()
    }
}
