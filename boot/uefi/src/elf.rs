use crate::{uefi, KernelImage, PAGE_SIZE};
use core::{mem::size_of, ptr};

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PF_X: u32 = 1;
const ET_DYN: u16 = 3;
const EM_X86_64: u16 = 62;
const DT_NULL: i64 = 0;
const DT_NEEDED: i64 = 1;
const DT_RELA: i64 = 7;
const DT_RELASZ: i64 = 8;
const DT_RELAENT: i64 = 9;
const DT_PLTRELSZ: i64 = 2;
const DT_JMPREL: i64 = 23;
const DT_REL: i64 = 17;
const DT_RELR: i64 = 36;
const R_X86_64_RELATIVE: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy)]
struct ElfHeader {
    ident: [u8; 16],
    kind: u16,
    machine: u16,
    version: u32,
    entry: u64,
    phoff: u64,
    shoff: u64,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ProgramHeader {
    kind: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    paddr: u64,
    file_size: u64,
    memory_size: u64,
    align: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Dynamic {
    tag: i64,
    value: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rela {
    offset: u64,
    info: u64,
    addend: i64,
}

const _: () = assert!(size_of::<ElfHeader>() == 64);
const _: () = assert!(size_of::<ProgramHeader>() == 56);
const _: () = assert!(size_of::<Dynamic>() == 16);
const _: () = assert!(size_of::<Rela>() == 24);

fn checked_range(offset: u64, len: u64, total: usize) -> Option<usize> {
    let end = offset.checked_add(len)?;
    if end > total as u64 {
        None
    } else {
        usize::try_from(offset).ok()
    }
}

fn page_up(address: u64) -> Option<u64> {
    address
        .checked_add(PAGE_SIZE - 1)
        .map(|value| value & !(PAGE_SIZE - 1))
}

unsafe fn header_at<T: Copy>(base: *const u8, len: usize, offset: u64) -> Option<T> {
    let offset = checked_range(offset, size_of::<T>() as u64, len)?;
    Some(ptr::read_unaligned(base.add(offset) as *const T))
}

unsafe fn loaded_address(
    image: u64,
    min_vaddr: u64,
    span: u64,
    vaddr: u64,
    size: u64,
) -> Option<*mut u8> {
    let offset = vaddr.checked_sub(min_vaddr)?;
    if offset.checked_add(size)? > span {
        return None;
    }
    image.checked_add(offset).map(|address| address as *mut u8)
}

pub unsafe fn load_kernel(
    services: *mut uefi::BootServices,
    file: *const u8,
    len: usize,
) -> Result<KernelImage, uefi::Status> {
    let header: ElfHeader = header_at(file, len, 0).ok_or(uefi::LOAD_ERROR)?;
    if header.ident[0..4] != *b"\x7fELF"
        || header.ident[4] != 2 // ELFCLASS64
        || header.ident[5] != 1 // little endian
        || header.ident[6] != 1 // ELF version
        || header.kind != ET_DYN
        || header.machine != EM_X86_64
        || header.version != 1
        || header.ehsize as usize != size_of::<ElfHeader>()
        || header.phentsize as usize != size_of::<ProgramHeader>()
        || header.phnum == 0
    {
        return Err(uefi::UNSUPPORTED);
    }
    let headers_len = (header.phnum as u64)
        .checked_mul(size_of::<ProgramHeader>() as u64)
        .ok_or(uefi::LOAD_ERROR)?;
    checked_range(header.phoff, headers_len, len).ok_or(uefi::LOAD_ERROR)?;

    let mut minimum = u64::MAX;
    let mut maximum = 0u64;
    let mut executable_entry = false;
    for index in 0..header.phnum {
        let ph = program_header(file, len, &header, index)?;
        if ph.kind == 3 {
            return Err(uefi::UNSUPPORTED);
        } // PT_INTERP
        if ph.kind != PT_LOAD {
            continue;
        }
        if ph.memory_size < ph.file_size {
            return Err(uefi::LOAD_ERROR);
        }
        checked_range(ph.offset, ph.file_size, len).ok_or(uefi::LOAD_ERROR)?;
        if ph.memory_size == 0 {
            continue;
        }
        let end = ph
            .vaddr
            .checked_add(ph.memory_size)
            .ok_or(uefi::LOAD_ERROR)?;
        minimum = minimum.min(ph.vaddr & !(PAGE_SIZE - 1));
        maximum = maximum.max(page_up(end).ok_or(uefi::LOAD_ERROR)?);
        if ph.flags & PF_X != 0 && header.entry >= ph.vaddr && header.entry < end {
            executable_entry = true;
        }
    }
    if minimum == u64::MAX || maximum <= minimum || !executable_entry {
        return Err(uefi::LOAD_ERROR);
    }
    let span = maximum - minimum;
    if span > usize::MAX as u64 {
        return Err(uefi::OUT_OF_RESOURCES);
    }
    let pages = usize::try_from(span / PAGE_SIZE).map_err(|_| uefi::OUT_OF_RESOURCES)?;
    let mut image = 0u64;
    let status = (*services).allocate_pages(uefi::EFI_LOADER_CODE, pages, &mut image);
    if status != uefi::SUCCESS {
        return Err(status);
    }
    ptr::write_bytes(image as *mut u8, 0, span as usize);

    let result = (|| -> Result<(), uefi::Status> {
        for index in 0..header.phnum {
            let ph = program_header(file, len, &header, index)?;
            if ph.kind != PT_LOAD || ph.file_size == 0 {
                continue;
            }
            let destination = loaded_address(image, minimum, span, ph.vaddr, ph.file_size)
                .ok_or(uefi::LOAD_ERROR)?;
            ptr::copy_nonoverlapping(
                file.add(ph.offset as usize),
                destination,
                ph.file_size as usize,
            );
        }
        relocate(file, len, &header, image, minimum, span)
    })();
    if let Err(status) = result {
        (*services).free_pages(image, pages);
        return Err(status);
    }
    let entry =
        loaded_address(image, minimum, span, header.entry, 1).ok_or(uefi::LOAD_ERROR)? as u64;
    Ok(KernelImage {
        start: image,
        end: image + span,
        entry,
    })
}

unsafe fn program_header(
    file: *const u8,
    len: usize,
    header: &ElfHeader,
    index: u16,
) -> Result<ProgramHeader, uefi::Status> {
    let offset = header
        .phoff
        .checked_add(index as u64 * size_of::<ProgramHeader>() as u64)
        .ok_or(uefi::LOAD_ERROR)?;
    header_at(file, len, offset).ok_or(uefi::LOAD_ERROR)
}

unsafe fn relocate(
    file: *const u8,
    len: usize,
    header: &ElfHeader,
    image: u64,
    minimum: u64,
    span: u64,
) -> Result<(), uefi::Status> {
    let mut rela_address = 0u64;
    let mut rela_size = 0u64;
    let mut rela_entry = 0u64;
    for index in 0..header.phnum {
        let ph = program_header(file, len, header, index)?;
        if ph.kind != PT_DYNAMIC {
            continue;
        }
        if ph.file_size % size_of::<Dynamic>() as u64 != 0 {
            return Err(uefi::LOAD_ERROR);
        }
        checked_range(ph.offset, ph.file_size, len).ok_or(uefi::LOAD_ERROR)?;
        let count = ph.file_size / size_of::<Dynamic>() as u64;
        for entry in 0..count {
            let item: Dynamic =
                header_at(file, len, ph.offset + entry * 16).ok_or(uefi::LOAD_ERROR)?;
            match item.tag {
                DT_NULL => break,
                DT_RELA => rela_address = item.value,
                DT_RELASZ => rela_size = item.value,
                DT_RELAENT => rela_entry = item.value,
                DT_NEEDED | DT_REL | DT_RELR => return Err(uefi::UNSUPPORTED),
                DT_PLTRELSZ | DT_JMPREL if item.value != 0 => return Err(uefi::UNSUPPORTED),
                _ => {}
            }
        }
    }
    if rela_size == 0 {
        return Ok(());
    }
    if rela_address == 0
        || rela_entry != size_of::<Rela>() as u64
        || !rela_size.is_multiple_of(rela_entry)
    {
        return Err(uefi::UNSUPPORTED);
    }
    let table =
        loaded_address(image, minimum, span, rela_address, rela_size).ok_or(uefi::LOAD_ERROR)?;
    let bias = i128::from(image) - i128::from(minimum);
    for index in 0..(rela_size / rela_entry) {
        let rela = ptr::read_unaligned(table.add((index * rela_entry) as usize) as *const Rela);
        if rela.info >> 32 != 0 || rela.info as u32 != R_X86_64_RELATIVE {
            return Err(uefi::UNSUPPORTED);
        }
        let target = loaded_address(image, minimum, span, rela.offset, 8).ok_or(uefi::LOAD_ERROR)?
            as *mut u64;
        let value = bias + i128::from(rela.addend);
        if value < 0 || value > u64::MAX as i128 {
            return Err(uefi::LOAD_ERROR);
        }
        ptr::write_unaligned(target, value as u64);
    }
    Ok(())
}
