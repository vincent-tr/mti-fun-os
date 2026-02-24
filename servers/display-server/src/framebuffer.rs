use core::intrinsics::volatile_copy_nonoverlapping_memory;

use alloc::{boxed::Box, vec::Vec};
use embedded_graphics::{pixelcolor::Rgb888, prelude::DrawTarget, prelude::*};
use libruntime::kobject;

/// A simple framebuffer implementation for the display server.
#[derive(Debug)]
pub struct FrameBuffer {
    pixel_format: PixelFormat,
    shape: BufferShape,
    front_buffer: kobject::Mapping<'static>,
    back_buffer: kobject::Mapping<'static>,
}

impl FrameBuffer {
    /// Open the framebuffer using the provided parameters from the init process.
    pub fn open(address: usize, shape: BufferShape, pixel_format: PixelFormat) -> Self {
        // front buffer: backed by device memory, used for display
        // back buffer: anonymous memory, used for drawing operations before copying to front buffer

        let process = kobject::Process::current();

        let front_buffer = {
            let memory_object =
                unsafe { kobject::MemoryObject::open_iomem(address, shape.byte_len, false, true) }
                    .expect("Failed to open framebuffer memory object");

            process
                .map_mem(
                    None,
                    memory_object
                        .size()
                        .expect("Failed to get memory object size"),
                    kobject::Permissions::WRITE,
                    &memory_object,
                    0,
                )
                .expect("Failed to map framebuffer")
        };

        let back_buffer = {
            let memory_object = kobject::MemoryObject::create(shape.byte_len)
                .expect("Failed to create back buffer memory object");

            process
                .map_mem(
                    None,
                    memory_object
                        .size()
                        .expect("Failed to get back buffer size"),
                    kobject::Permissions::READ | kobject::Permissions::WRITE,
                    &memory_object,
                    0,
                )
                .expect("Failed to map back buffer")
        };

        Self {
            pixel_format,
            shape,
            front_buffer,
            back_buffer,
        }
    }

    /// Flip the back buffer to the front buffer, making the drawn content visible on the display.
    pub fn flip(&mut self) {
        // Copy back buffer to front buffer
        let front_buffer =
            unsafe { self.front_buffer.as_buffer_mut() }.expect("Failed to get front buffer slice");
        let back_buffer = self.back_buffer();

        unsafe {
            volatile_copy_nonoverlapping_memory(
                front_buffer.as_mut_ptr(),
                back_buffer.as_ptr(),
                back_buffer.len(),
            );
        }
    }

    fn back_buffer(&mut self) -> &mut [u8] {
        unsafe { self.back_buffer.as_buffer_mut() }.expect("Failed to get back buffer slice")
    }

    fn draw_pixel(&mut self, point: Point, color: Rgb888) {
        debug_assert!(
            point.x >= 0,
            "X coordinate must be non-negative: {}",
            point.x
        );
        debug_assert!(
            point.y >= 0,
            "Y coordinate must be non-negative: {}",
            point.y
        );
        debug_assert!(
            (point.x as usize) < self.shape.width,
            "X coordinate out of bounds: {}",
            point.x
        );
        debug_assert!(
            (point.y as usize) < self.shape.height,
            "Y coordinate out of bounds: {}",
            point.y
        );

        let pixel_offset = (point.y as usize) * self.shape.stride
            + (point.x as usize) * self.shape.bytes_per_pixel;
        let pixel_data = self.pixel_format.rgb_to_native(color);

        let range = pixel_offset..pixel_offset + self.shape.bytes_per_pixel;
        let back_slice = self.back_buffer();
        back_slice[range].copy_from_slice(&pixel_data);
    }
}

impl DrawTarget for FrameBuffer {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for pixel in pixels {
            let embedded_graphics::Pixel(point, color) = pixel;
            self.draw_pixel(point, color);

            let color_value = self.pixel_format.rgb_to_native(color);
        }
        Ok(())
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(self.shape.width as u32, self.shape.height as u32)
    }
}

/// A simple shape struct for drawing operations.
#[derive(Debug, Clone)]
pub struct BufferShape {
    byte_len: usize,
    width: usize,
    height: usize,
    stride: usize,
    bytes_per_pixel: usize,
}

impl BufferShape {
    /// Create a new shape with the given position, size, and pixel format information.
    pub fn new(
        byte_len: usize,
        width: usize,
        height: usize,
        stride: usize,
        bytes_per_pixel: usize,
    ) -> Self {
        assert!(
            bytes_per_pixel <= size_of::<u32>(),
            "Unsupported bytes per pixel: {}",
            bytes_per_pixel
        );

        assert!(
            byte_len >= stride * height * bytes_per_pixel,
            "Byte length {} is too small for the given dimensions and pixel format ({} bytes needed)",
            byte_len,
            stride * height * bytes_per_pixel
        );

        Self {
            byte_len,
            width,
            height,
            stride,
            bytes_per_pixel,
        }
    }
}

/// Pixel format information for the framebuffer, used for converting RGB values to the native format.
#[derive(Debug, Clone)]
pub struct PixelFormat {
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
    bytes_per_pixel: usize,
}

impl PixelFormat {
    /// Create a new PixelFormat with the given color masks and bytes per pixel.
    pub fn new(red_mask: u32, green_mask: u32, blue_mask: u32, bytes_per_pixel: usize) -> Self {
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
    pub fn rgb_to_native(&self, color: Rgb888) -> Box<[u8]> {
        let red_shift = self.red_mask.trailing_zeros();
        let green_shift = self.green_mask.trailing_zeros();
        let blue_shift = self.blue_mask.trailing_zeros();

        let red = (color.r() as u32) << red_shift;
        let green = (color.g() as u32) << green_shift;
        let blue = (color.b() as u32) << blue_shift;

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
