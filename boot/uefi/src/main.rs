#![no_std]
#![no_main]

mod abi;
mod elf;
mod paging;
mod uefi;

use abi::{BootInfo, Framebuffer, MemoryRegion};
use core::{
    arch::asm,
    ffi::c_void,
    mem::size_of,
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use elf::load_kernel;
use uefi::{BootServices, Handle, Status, SystemTable};

const PAGE_SIZE: u64 = 4096;
const STACK_PAGES: usize = 16;
static EXIT_ATTEMPTED: AtomicBool = AtomicBool::new(false);
static BOOT_STAGE: AtomicUsize = AtomicUsize::new(0);
const KERNEL_PATH: [u16; 13] = [
    b'\\' as u16,
    b'W' as u16,
    b'O' as u16,
    b'L' as u16,
    b'F' as u16,
    b'R' as u16,
    b'A' as u16,
    b'M' as u16,
    b'.' as u16,
    b'E' as u16,
    b'L' as u16,
    b'F' as u16,
    0,
];

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("cli; hlt", options(nomem, nostack));
        }
    }
}

#[no_mangle]
/// # Safety
///
/// UEFI must invoke this entry point with a valid image handle and system table.
pub unsafe extern "efiapi" fn efi_main(image: Handle, system_table: *mut SystemTable) -> Status {
    if system_table.is_null() {
        return uefi::INVALID_PARAMETER;
    }
    let status = unsafe { boot(image, system_table) };
    if status != uefi::SUCCESS && !EXIT_ATTEMPTED.load(Ordering::Relaxed) {
        unsafe { print((*system_table).console_out, "Wolfram loader stage ") };
        unsafe {
            print_hex(
                (*system_table).console_out,
                BOOT_STAGE.load(Ordering::Relaxed) as u64,
            )
        };
        unsafe { print((*system_table).console_out, " failed: ") };
        unsafe {
            print_hex((*system_table).console_out, status as u64);
        }
        unsafe {
            print((*system_table).console_out, "\r\n");
        }
    }
    status
}

unsafe fn print(out: *mut uefi::SimpleTextOutput, message: &str) {
    if out.is_null() {
        return;
    }
    for byte in message.bytes() {
        let wide = [byte as u16, 0];
        ((*out).output_string)(out, wide.as_ptr());
    }
}

unsafe fn print_hex(out: *mut uefi::SimpleTextOutput, number: u64) {
    print(out, "0x");
    for shift in (0..16).rev() {
        let digit = ((number >> (shift * 4)) & 15) as u8;
        let byte = if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        };
        let wide = [byte as u16, 0];
        if !out.is_null() {
            ((*out).output_string)(out, wide.as_ptr());
        }
    }
}

#[derive(Clone, Copy, Default)]
struct KernelSegment {
    start: u64,
    end: u64,
    flags: u32,
}

struct KernelImage {
    start: u64,
    end: u64,
    entry: u64,
    segments: [KernelSegment; 16],
    segment_count: usize,
}

unsafe fn boot(image: Handle, table: *mut SystemTable) -> Status {
    BOOT_STAGE.store(1, Ordering::Relaxed);
    let services = (*table).boot_services;
    if services.is_null() {
        return uefi::LOAD_ERROR;
    }
    print((*table).console_out, "Wolfram UEFI loader\r\n");

    let mut loaded: *mut c_void = ptr::null_mut();
    let status = (*services).handle_protocol(image, &uefi::LOADED_IMAGE_GUID, &mut loaded);
    if status != uefi::SUCCESS {
        return status;
    }
    if loaded.is_null() {
        return uefi::LOAD_ERROR;
    }

    BOOT_STAGE.store(2, Ordering::Relaxed);
    let mut fs: *mut c_void = ptr::null_mut();
    let status = (*services).handle_protocol(
        (*(loaded as *const uefi::LoadedImage)).device_handle,
        &uefi::SIMPLE_FILE_SYSTEM_GUID,
        &mut fs,
    );
    if status != uefi::SUCCESS {
        return status;
    }
    if fs.is_null() {
        return uefi::LOAD_ERROR;
    }

    BOOT_STAGE.store(3, Ordering::Relaxed);
    let mut root: *mut uefi::FileProtocol = ptr::null_mut();
    let filesystem = fs as *mut uefi::SimpleFileSystem;
    let status = ((*filesystem).open_volume)(filesystem, &mut root);
    if status != uefi::SUCCESS {
        return status;
    }
    if root.is_null() {
        return uefi::LOAD_ERROR;
    }
    BOOT_STAGE.store(4, Ordering::Relaxed);
    let mut file: *mut uefi::FileProtocol = ptr::null_mut();
    let status = ((*root).open)(
        root,
        &mut file,
        KERNEL_PATH.as_ptr(),
        uefi::FILE_MODE_READ,
        0,
    );
    if status != uefi::SUCCESS {
        ((*root).close)(root);
        return status;
    }
    if file.is_null() {
        ((*root).close)(root);
        return uefi::LOAD_ERROR;
    }
    BOOT_STAGE.store(5, Ordering::Relaxed);
    let result = read_file(services, file);
    ((*file).close)(file);
    ((*root).close)(root);
    let (file_bytes, file_size) = match result {
        Ok(value) => value,
        Err(status) => return status,
    };
    BOOT_STAGE.store(6, Ordering::Relaxed);
    let kernel = load_kernel(services, file_bytes, file_size);
    (*services).free_pool(file_bytes);
    let kernel = match kernel {
        Ok(value) => value,
        Err(status) => return status,
    };
    print((*table).console_out, "Kernel loaded at ");
    print_hex((*table).console_out, kernel.start);
    print((*table).console_out, "\r\n");

    BOOT_STAGE.store(7, Ordering::Relaxed);
    let framebuffer = find_framebuffer(services);
    if framebuffer.pixel_format == abi::PIXEL_NONE {
        print((*table).console_out, "No supported GOP framebuffer\r\n");
        return uefi::UNSUPPORTED;
    }

    BOOT_STAGE.store(8, Ordering::Relaxed);
    let mut boot_page = 0u64;
    let status = (*services).allocate_pages(uefi::EFI_LOADER_DATA, 1, &mut boot_page);
    if status != uefi::SUCCESS {
        return status;
    }
    BOOT_STAGE.store(9, Ordering::Relaxed);
    let mut stack = 0u64;
    let status = (*services).allocate_pages(uefi::EFI_LOADER_DATA, STACK_PAGES, &mut stack);
    if status != uefi::SUCCESS {
        return status;
    }

    let boot_info = boot_page as *mut BootInfo;
    ptr::write(
        boot_info,
        BootInfo {
            magic: abi::BOOT_MAGIC,
            version: abi::BOOT_VERSION,
            size: size_of::<BootInfo>() as u32,
            memory_regions: 0,
            memory_region_count: 0,
            memory_region_size: size_of::<MemoryRegion>() as u32,
            kernel_start: kernel.start,
            kernel_end: kernel.end,
            framebuffer,
        },
    );

    BOOT_STAGE.store(10, Ordering::Relaxed);
    let tables = match exit_with_map(image, services, boot_info, &kernel, &framebuffer) {
        Ok(tables) => tables,
        Err(status) => return status,
    };
    jump_to_kernel(
        kernel.entry,
        boot_info,
        stack + (STACK_PAGES as u64 * PAGE_SIZE),
        tables,
    );
}

unsafe fn read_file(
    services: *mut BootServices,
    file: *mut uefi::FileProtocol,
) -> Result<(*mut u8, usize), Status> {
    let status = ((*file).set_position)(file, u64::MAX);
    if status != uefi::SUCCESS {
        return Err(status);
    }
    let mut file_size = 0u64;
    let status = ((*file).get_position)(file, &mut file_size);
    if status != uefi::SUCCESS {
        return Err(status);
    }
    if file_size == 0 || file_size > usize::MAX as u64 {
        return Err(uefi::LOAD_ERROR);
    }
    let status = ((*file).set_position)(file, 0);
    if status != uefi::SUCCESS {
        return Err(status);
    }
    let mut buffer = ptr::null_mut();
    let status = (*services).allocate_pool(file_size as usize, &mut buffer);
    if status != uefi::SUCCESS {
        return Err(status);
    }
    if buffer.is_null() {
        return Err(uefi::OUT_OF_RESOURCES);
    }
    let mut read_total = 0usize;
    while read_total < file_size as usize {
        let mut chunk = file_size as usize - read_total;
        let status = ((*file).read)(file, &mut chunk, buffer.add(read_total));
        if status != uefi::SUCCESS || chunk == 0 {
            (*services).free_pool(buffer);
            return Err(if status == uefi::SUCCESS {
                uefi::LOAD_ERROR
            } else {
                status
            });
        }
        read_total += chunk;
    }
    Ok((buffer, read_total))
}

unsafe fn find_framebuffer(services: *mut BootServices) -> Framebuffer {
    let mut protocol = ptr::null_mut();
    if (*services).locate_protocol(&uefi::GRAPHICS_OUTPUT_GUID, &mut protocol) != uefi::SUCCESS
        || protocol.is_null()
    {
        return Framebuffer::default();
    }
    let gop = protocol as *const uefi::GraphicsOutput;
    if (*gop).mode.is_null() {
        return Framebuffer::default();
    }
    let mode = &*(*gop).mode;
    if mode.info.is_null() {
        return Framebuffer::default();
    }
    let info = &*mode.info;
    let pixel_format = match info.pixel_format {
        0 => abi::PIXEL_RGB,
        1 => abi::PIXEL_BGR,
        _ => abi::PIXEL_NONE,
    };
    let visible_bytes = (info.pixels_per_scan_line as u64)
        .checked_mul(info.vertical_resolution as u64)
        .and_then(|pixels| pixels.checked_mul(4));
    if pixel_format == abi::PIXEL_NONE
        || info.horizontal_resolution == 0
        || info.vertical_resolution == 0
        || info.pixels_per_scan_line < info.horizontal_resolution
        || visible_bytes.is_none_or(|bytes| bytes > mode.frame_buffer_size as u64)
        || mode.frame_buffer_base == 0
        || mode
            .frame_buffer_base
            .checked_add(mode.frame_buffer_size as u64)
            .is_none()
    {
        return Framebuffer::default();
    }
    Framebuffer {
        base: mode.frame_buffer_base,
        size: mode.frame_buffer_size as u64,
        width: info.horizontal_resolution,
        height: info.vertical_resolution,
        stride: info.pixels_per_scan_line,
        pixel_format,
    }
}

unsafe fn exit_with_map(
    image: Handle,
    services: *mut BootServices,
    info: *mut BootInfo,
    kernel: &KernelImage,
    framebuffer: &Framebuffer,
) -> Result<paging::PageTables, Status> {
    let mut required = 0usize;
    let mut key = 0usize;
    let mut descriptor_size = 0usize;
    let mut version = 0u32;
    let probe = (*services).get_memory_map(
        &mut required,
        ptr::null_mut(),
        &mut key,
        &mut descriptor_size,
        &mut version,
    );
    if probe != uefi::BUFFER_TOO_SMALL && probe != uefi::SUCCESS {
        return Err(probe);
    }
    if descriptor_size < size_of::<uefi::MemoryDescriptor>() {
        return Err(uefi::LOAD_ERROR);
    }
    let mut map_capacity = match required.checked_add(descriptor_size.saturating_mul(128)) {
        Some(value) => value.max(16 * 1024),
        None => return Err(uefi::OUT_OF_RESOURCES),
    };
    let mut map: *mut u8 = ptr::null_mut();
    let mut regions_address = 0u64;
    let mut regions_capacity = 0usize;
    let mut tables = None;

    // All allocations are completed before the final GetMemoryMap/ExitBootServices pair.
    for _ in 0..4 {
        if !map.is_null() {
            (*services).free_pool(map);
        }
        let status = (*services).allocate_pool(map_capacity, &mut map);
        if status != uefi::SUCCESS {
            return Err(status);
        }
        if map.is_null() {
            return Err(uefi::OUT_OF_RESOURCES);
        }
        let needed_regions = map_capacity / descriptor_size + 1;
        if needed_regions > regions_capacity {
            if regions_address != 0 {
                (*services).free_pages(
                    regions_address,
                    pages_for_bytes(regions_capacity * size_of::<MemoryRegion>()),
                );
            }
            regions_capacity = needed_regions;
            let mut address = 0u64;
            let bytes = match regions_capacity.checked_mul(size_of::<MemoryRegion>()) {
                Some(value) => value,
                None => return Err(uefi::OUT_OF_RESOURCES),
            };
            let status = (*services).allocate_pages(
                uefi::EFI_LOADER_DATA,
                pages_for_bytes(bytes),
                &mut address,
            );
            if status != uefi::SUCCESS {
                return Err(status);
            }
            regions_address = address;
        }

        for _ in 0..3 {
            let mut map_size = map_capacity;
            let status = (*services).get_memory_map(
                &mut map_size,
                map,
                &mut key,
                &mut descriptor_size,
                &mut version,
            );
            if status == uefi::BUFFER_TOO_SMALL {
                map_capacity = match map_size.checked_add(descriptor_size.saturating_mul(128)) {
                    Some(value) => value,
                    None => return Err(uefi::OUT_OF_RESOURCES),
                };
                break;
            }
            if status != uefi::SUCCESS {
                return Err(status);
            }
            if descriptor_size < size_of::<uefi::MemoryDescriptor>()
                || map_size % descriptor_size != 0
                || map_size / descriptor_size > regions_capacity
                || map_size / descriptor_size > 4096
            {
                return Err(uefi::LOAD_ERROR);
            }
            let count = map_size / descriptor_size;
            let regions = regions_address as *mut MemoryRegion;
            let mut region_count = 0usize;
            for index in 0..count {
                let descriptor = ptr::read_unaligned(
                    map.add(index * descriptor_size) as *const uefi::MemoryDescriptor
                );
                if descriptor.pages == 0 {
                    continue;
                }
                let bytes = match descriptor.pages.checked_mul(PAGE_SIZE) {
                    Some(value) => value,
                    None => return Err(uefi::LOAD_ERROR),
                };
                let end = match descriptor.physical_start.checked_add(bytes) {
                    Some(value) => value,
                    None => return Err(uefi::LOAD_ERROR),
                };
                ptr::write(
                    regions.add(region_count),
                    MemoryRegion {
                        start: descriptor.physical_start,
                        end,
                        kind: if descriptor.kind == uefi::EFI_CONVENTIONAL_MEMORY {
                            abi::REGION_USABLE
                        } else {
                            abi::REGION_RESERVED
                        },
                        reserved: 0,
                    },
                );
                region_count += 1;
            }
            if region_count == 0 {
                return Err(uefi::LOAD_ERROR);
            }
            (*info).memory_regions = regions_address;
            (*info).memory_region_count = region_count as u32;
            if tables.is_none() {
                tables = Some(paging::build(
                    services,
                    map,
                    map_size,
                    descriptor_size,
                    kernel,
                    framebuffer,
                )?);
                // Page-table allocations changed the map key. Re-read it before exit.
                continue;
            }
            EXIT_ATTEMPTED.store(true, Ordering::Relaxed);
            let status = (*services).exit_boot_services(image, key);
            if status == uefi::SUCCESS {
                return Ok(tables.unwrap());
            }
            // A stale key is recoverable without allocating or touching other services.
            if status != uefi::INVALID_PARAMETER {
                return Err(status);
            }
        }
        if EXIT_ATTEMPTED.load(Ordering::Relaxed) {
            return Err(uefi::INVALID_PARAMETER);
        }
    }
    Err(uefi::BUFFER_TOO_SMALL)
}

fn pages_for_bytes(bytes: usize) -> usize {
    bytes.saturating_add(PAGE_SIZE as usize - 1) / PAGE_SIZE as usize
}

unsafe fn jump_to_kernel(
    entry: u64,
    info: *const BootInfo,
    stack_top: u64,
    tables: paging::PageTables,
) -> ! {
    paging::enable_nx_and_switch(tables.root, tables.nx_supported);
    asm!(
        "cli",
        "cld",
        "mov rsp, rcx",
        "and rsp, -16",
        "sub rsp, 8",
        "mov qword ptr [rsp], 0",
        "jmp rax",
        in("rcx") stack_top,
        in("rax") entry,
        in("rdi") info,
        options(noreturn)
    );
}
