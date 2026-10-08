use core::ffi::c_void;

pub type Handle = *mut c_void;
pub type Status = usize;
pub const SUCCESS: Status = 0;
pub const LOAD_ERROR: Status = error(1);
pub const INVALID_PARAMETER: Status = error(2);
pub const UNSUPPORTED: Status = error(3);
pub const BUFFER_TOO_SMALL: Status = error(5);
pub const OUT_OF_RESOURCES: Status = error(9);
pub const EFI_LOADER_CODE: u32 = 1;
pub const EFI_LOADER_DATA: u32 = 2;
pub const EFI_CONVENTIONAL_MEMORY: u32 = 7;
pub const ALLOCATE_ANY_PAGES: u32 = 0;
pub const FILE_MODE_READ: u64 = 1;

const fn error(code: usize) -> Status {
    (1usize << (usize::BITS - 1)) | code
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

pub const LOADED_IMAGE_GUID: Guid = Guid {
    data1: 0x5b1b31a1,
    data2: 0x9562,
    data3: 0x11d2,
    data4: [0x8e, 0x3f, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};
pub const SIMPLE_FILE_SYSTEM_GUID: Guid = Guid {
    data1: 0x964e5b22,
    data2: 0x6459,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};
pub const GRAPHICS_OUTPUT_GUID: Guid = Guid {
    data1: 0x9042a9de,
    data2: 0x23dc,
    data3: 0x4a38,
    data4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[repr(C)]
pub struct TableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct SystemTable {
    pub header: TableHeader,
    pub firmware_vendor: *const u16,
    pub firmware_revision: u32,
    pub _pad: u32,
    pub console_in_handle: Handle,
    pub console_in: *mut c_void,
    pub console_out_handle: Handle,
    pub console_out: *mut SimpleTextOutput,
    pub standard_error_handle: Handle,
    pub standard_error: *mut SimpleTextOutput,
    pub runtime_services: *mut c_void,
    pub boot_services: *mut BootServices,
    pub number_of_table_entries: usize,
    pub configuration_table: *mut c_void,
}

#[repr(C)]
pub struct SimpleTextOutput {
    pub reset: usize,
    pub output_string: extern "efiapi" fn(*mut SimpleTextOutput, *const u16) -> Status,
}

#[repr(C)]
pub struct BootServices {
    pub header: TableHeader,
    pub functions: [usize; 44],
}

impl BootServices {
    pub unsafe fn allocate_pages(&mut self, kind: u32, pages: usize, address: &mut u64) -> Status {
        let f: extern "efiapi" fn(*mut Self, u32, u32, usize, *mut u64) -> Status =
            core::mem::transmute(self.functions[2]);
        f(self, ALLOCATE_ANY_PAGES, kind, pages, address)
    }

    pub unsafe fn free_pages(&mut self, address: u64, pages: usize) -> Status {
        let f: extern "efiapi" fn(*mut Self, u64, usize) -> Status =
            core::mem::transmute(self.functions[3]);
        f(self, address, pages)
    }

    pub unsafe fn get_memory_map(
        &mut self,
        size: &mut usize,
        map: *mut u8,
        key: &mut usize,
        descriptor_size: &mut usize,
        descriptor_version: &mut u32,
    ) -> Status {
        let f: extern "efiapi" fn(
            *mut Self,
            *mut usize,
            *mut u8,
            *mut usize,
            *mut usize,
            *mut u32,
        ) -> Status = core::mem::transmute(self.functions[4]);
        f(self, size, map, key, descriptor_size, descriptor_version)
    }

    pub unsafe fn allocate_pool(&mut self, size: usize, ptr: &mut *mut u8) -> Status {
        let f: extern "efiapi" fn(*mut Self, u32, usize, *mut *mut u8) -> Status =
            core::mem::transmute(self.functions[5]);
        f(self, EFI_LOADER_DATA, size, ptr)
    }

    pub unsafe fn free_pool(&mut self, ptr: *mut u8) -> Status {
        let f: extern "efiapi" fn(*mut Self, *mut u8) -> Status =
            core::mem::transmute(self.functions[6]);
        f(self, ptr)
    }

    pub unsafe fn handle_protocol(
        &mut self,
        handle: Handle,
        guid: &Guid,
        out: &mut *mut c_void,
    ) -> Status {
        let f: extern "efiapi" fn(*mut Self, Handle, *const Guid, *mut *mut c_void) -> Status =
            core::mem::transmute(self.functions[16]);
        f(self, handle, guid, out)
    }

    pub unsafe fn exit_boot_services(&mut self, image: Handle, key: usize) -> Status {
        let f: extern "efiapi" fn(*mut Self, Handle, usize) -> Status =
            core::mem::transmute(self.functions[26]);
        f(self, image, key)
    }

    pub unsafe fn locate_protocol(&mut self, guid: &Guid, out: &mut *mut c_void) -> Status {
        let f: extern "efiapi" fn(*mut Self, *const Guid, *mut c_void, *mut *mut c_void) -> Status =
            core::mem::transmute(self.functions[37]);
        f(self, guid, core::ptr::null_mut(), out)
    }
}

#[repr(C)]
pub struct LoadedImage {
    pub revision: u32,
    pub _pad: u32,
    pub parent_handle: Handle,
    pub system_table: *mut SystemTable,
    pub device_handle: Handle,
    pub file_path: *mut c_void,
    pub reserved: *mut c_void,
    pub load_options_size: u32,
    pub _pad2: u32,
    pub load_options: *mut c_void,
    pub image_base: *mut c_void,
    pub image_size: u64,
}

#[repr(C)]
pub struct SimpleFileSystem {
    pub revision: u64,
    pub open_volume: extern "efiapi" fn(*mut Self, *mut *mut FileProtocol) -> Status,
}

#[repr(C)]
pub struct FileProtocol {
    pub revision: u64,
    pub open: extern "efiapi" fn(*mut Self, *mut *mut FileProtocol, *const u16, u64, u64) -> Status,
    pub close: extern "efiapi" fn(*mut Self) -> Status,
    pub delete: usize,
    pub read: extern "efiapi" fn(*mut Self, *mut usize, *mut u8) -> Status,
    pub write: usize,
    pub get_position: extern "efiapi" fn(*mut Self, *mut u64) -> Status,
    pub set_position: extern "efiapi" fn(*mut Self, u64) -> Status,
}

#[repr(C)]
pub struct GraphicsOutput {
    pub query_mode: usize,
    pub set_mode: usize,
    pub blt: usize,
    pub mode: *const GraphicsMode,
}

#[repr(C)]
pub struct GraphicsMode {
    pub max_mode: u32,
    pub mode: u32,
    pub info: *const GraphicsModeInfo,
    pub size_of_info: usize,
    pub frame_buffer_base: u64,
    pub frame_buffer_size: usize,
}

#[repr(C)]
pub struct GraphicsModeInfo {
    pub version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: u32,
    pub pixel_information: [u32; 4],
    pub pixels_per_scan_line: u32,
}

#[repr(C)]
pub struct MemoryDescriptor {
    pub kind: u32,
    pub _pad: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub pages: u64,
    pub attributes: u64,
}

const _: () = assert!(core::mem::size_of::<MemoryDescriptor>() == 40);
