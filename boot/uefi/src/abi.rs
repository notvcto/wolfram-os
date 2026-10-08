pub const BOOT_MAGIC: u64 = 0x574f_4c46_5241_4d21;
pub const BOOT_VERSION: u32 = 1;
pub const REGION_USABLE: u32 = 1;
pub const REGION_RESERVED: u32 = 2;
pub const PIXEL_NONE: u32 = 0;
pub const PIXEL_RGB: u32 = 1;
pub const PIXEL_BGR: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub kind: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Framebuffer {
    pub base: u64,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BootInfo {
    pub magic: u64,
    pub version: u32,
    pub size: u32,
    pub memory_regions: u64,
    pub memory_region_count: u32,
    pub memory_region_size: u32,
    pub kernel_start: u64,
    pub kernel_end: u64,
    pub framebuffer: Framebuffer,
}

const _: () = assert!(core::mem::size_of::<MemoryRegion>() == 24);
const _: () = assert!(core::mem::size_of::<Framebuffer>() == 32);
const _: () = assert!(core::mem::size_of::<BootInfo>() == 80);
