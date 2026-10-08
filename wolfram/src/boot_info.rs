//! Versioned, architecture-neutral handoff from firmware-facing boot code.
//!
//! Addresses in this ABI are physical addresses, identity mapped and readable
//! when the kernel enters. The memory map and this structure must remain live
//! until physical allocator initialization has finished.

pub const BOOT_INFO_MAGIC: u64 = 0x574f_4c46_5241_4d21; // "WOLFRAM!"
pub const BOOT_INFO_VERSION: u32 = 1;

pub const MEMORY_USABLE: u32 = 1;
pub const MEMORY_RESERVED: u32 = 2;

pub const PIXEL_FORMAT_NONE: u32 = 0;
#[cfg(target_arch = "x86_64")]
pub const PIXEL_FORMAT_RGB: u32 = 1;
#[cfg(target_arch = "x86_64")]
pub const PIXEL_FORMAT_BGR: u32 = 2;

/// A half-open physical address interval. Reserved intervals take precedence.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub kind: u32,
    pub reserved: u32,
}

impl MemoryRegion {
    pub const EMPTY: Self = Self {
        start: 0,
        end: 0,
        kind: MEMORY_RESERVED,
        reserved: 0,
    };

    pub const fn usable(start: u64, end: u64) -> Self {
        Self {
            start,
            end,
            kind: MEMORY_USABLE,
            reserved: 0,
        }
    }

    pub const fn reserved(start: u64, end: u64) -> Self {
        Self {
            start,
            end,
            kind: MEMORY_RESERVED,
            reserved: 0,
        }
    }
}

/// Optional framebuffer; `base == 0` or `size == 0` means absent.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Framebuffer {
    pub base: u64,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32, // Pixels per scanline.
    pub pixel_format: u32,
}

impl Framebuffer {
    pub const NONE: Self = Self {
        base: 0,
        size: 0,
        width: 0,
        height: 0,
        stride: 0,
        pixel_format: PIXEL_FORMAT_NONE,
    };
}

/// Binary contract shared by the UEFI loader and both kernel architectures.
/// The x86-64 entry signature is `extern "C" fn(*const BootInfo) -> !`.
#[repr(C)]
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

impl BootInfo {
    pub fn new(regions: &[MemoryRegion], kernel_start: u64, kernel_end: u64) -> Self {
        Self {
            magic: BOOT_INFO_MAGIC,
            version: BOOT_INFO_VERSION,
            size: core::mem::size_of::<Self>() as u32,
            memory_regions: regions.as_ptr() as u64,
            memory_region_count: regions.len() as u32,
            memory_region_size: core::mem::size_of::<MemoryRegion>() as u32,
            kernel_start,
            kernel_end,
            framebuffer: Framebuffer::NONE,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.magic != BOOT_INFO_MAGIC || self.version != BOOT_INFO_VERSION {
            return Err("unsupported boot information");
        }
        if self.size as usize != core::mem::size_of::<Self>()
            || self.memory_region_size as usize != core::mem::size_of::<MemoryRegion>()
        {
            return Err("invalid boot information layout");
        }
        if self.memory_region_count == 0 || self.memory_region_count > 4096 {
            return Err("invalid boot memory map count");
        }
        let bytes = (self.memory_region_count as usize)
            .checked_mul(core::mem::size_of::<MemoryRegion>())
            .ok_or("boot memory map overflow")?;
        if self.memory_regions == 0
            || !(self.memory_regions as usize).is_multiple_of(core::mem::align_of::<MemoryRegion>())
            || (self.memory_regions as usize).checked_add(bytes).is_none()
        {
            return Err("invalid boot memory map address");
        }
        if self.kernel_start >= self.kernel_end {
            return Err("invalid kernel image range");
        }
        Ok(())
    }

    /// # Safety
    /// The boot provider must keep the identity-mapped region array readable
    /// for `memory_region_count` entries through allocator initialization.
    pub unsafe fn regions(&self) -> &[MemoryRegion] {
        core::slice::from_raw_parts(
            self.memory_regions as *const MemoryRegion,
            self.memory_region_count as usize,
        )
    }
}

const _: () = assert!(core::mem::size_of::<MemoryRegion>() == 24);
const _: () = assert!(core::mem::size_of::<Framebuffer>() == 32);
const _: () = assert!(core::mem::size_of::<BootInfo>() == 80);
