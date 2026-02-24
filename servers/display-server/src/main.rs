#![no_std]
#![no_main]

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

    let pixel_format = PixelFormat::new(red_mask, green_mask, blue_mask);
    let color = pixel_format.rgb_to_native(255, 0, 0);
    draw_rectangle(
        framebuffer,
        stride,
        bytes_per_pixel,
        100,
        100,
        200,
        150,
        color,
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
    stride: usize,
    bytes_per_pixel: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
) {
    let color_bytes = color.to_ne_bytes();

    for row in 0..height {
        for col in 0..width {
            let pixel_offset = ((y + row) * stride + (x + col)) * bytes_per_pixel;

            // Write each byte of the pixel color
            for (i, &byte) in color_bytes.iter().take(bytes_per_pixel).enumerate() {
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
}

impl PixelFormat {
    fn new(red_mask: u32, green_mask: u32, blue_mask: u32) -> Self {
        Self {
            red_mask,
            green_mask,
            blue_mask,
        }
    }

    /// Convert RGB (0-255 each) to native pixel format
    pub fn rgb_to_native(&self, r: u8, g: u8, b: u8) -> u32 {
        let red_shift = self.red_mask.trailing_zeros();
        let green_shift = self.green_mask.trailing_zeros();
        let blue_shift = self.blue_mask.trailing_zeros();

        let red = (r as u32) << red_shift;
        let green = (g as u32) << green_shift;
        let blue = (b as u32) << blue_shift;

        red | green | blue
    }
}
