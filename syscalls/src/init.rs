/// Structure passed from the kernel to init process on startup, containing information about the system state and resources.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct InitInfo {
    /// Information about the mapping of the init info structure itself.
    pub info_mapping: Mapping,

    /// Information about the mapping of the init process binary, used for init to complete its own mapping.
    pub init_mapping: Mapping,

    /// Information about the mapping of the archive containing the servers used for bootstrapping the system.
    pub archive_mapping: Mapping,

    /// Information about the framebuffer.
    pub framebuffer: Framebuffer,
}

/// Information about a mapping area.
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct Mapping {
    /// The virtual address of the mapping.
    pub address: usize,

    /// The size of the mapping in bytes.
    pub size: usize,
}

/// Information about the framebuffer, used for init to setup its own framebuffer mapping and provide framebuffer info to userland.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Framebuffer {
    /// The physical address of the framebuffer.
    pub address: usize,

    /// The total size in bytes.
    pub byte_len: usize,

    /// The width in pixels.
    pub width: usize,

    /// The height in pixels.
    pub height: usize,

    /// The color format of each pixel.
    pub pixel_format: PixelFormat,

    /// The number of bytes per pixel.
    pub bytes_per_pixel: usize,

    /// Number of pixels between the start of a line and the start of the next.
    pub stride: usize,
}

/// Color format of each pixel in the framebuffer.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct PixelFormat {
    /// Bit mask for the red component of a pixel.
    pub red_mask: u32,

    /// Bit mask for the green component of a pixel.
    pub green_mask: u32,

    /// Bit mask for the blue component of a pixel.
    pub blue_mask: u32,
}
