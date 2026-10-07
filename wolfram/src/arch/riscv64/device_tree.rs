//! Minimal flattened device tree reader for boot-time physical memory discovery.
//! Firmware passes the FDT address in a1; no allocation is available yet.

const MAGIC: u32 = 0xd00d_feed;
const MAX_SIZE: usize = 2 * 1024 * 1024;
const MAX_DEPTH: usize = 16;

#[derive(Clone, Copy)]
pub struct Region {
    pub start: u64,
    pub end: u64,
}

pub struct DeviceTree<'a> {
    bytes: &'a [u8],
    structure: &'a [u8],
    strings: &'a [u8],
    reserve_offset: usize,
}

#[derive(Clone, Copy)]
struct Node {
    kind: u8,
    reg_address_cells: u32,
    reg_size_cells: u32,
    child_address_cells: u32,
    child_size_cells: u32,
    reg_offset: usize,
    reg_len: usize,
}

impl Node {
    const EMPTY: Self = Self {
        kind: 0, reg_address_cells: 2, reg_size_cells: 1,
        child_address_cells: 2, child_size_cells: 1,
        reg_offset: 0, reg_len: 0,
    };
}

fn be32(bytes: &[u8], offset: usize) -> Result<u32, &'static str> {
    let b = bytes.get(offset..offset.checked_add(4).ok_or("FDT offset overflow")?)
        .ok_or("truncated FDT")?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn be64(bytes: &[u8], offset: usize) -> Result<u64, &'static str> {
    Ok(((be32(bytes, offset)? as u64) << 32) | be32(bytes, offset + 4)? as u64)
}

fn section(bytes: &[u8], offset: usize, len: usize) -> Result<&[u8], &'static str> {
    bytes.get(offset..offset.checked_add(len).ok_or("FDT section overflow")?)
        .ok_or("FDT section out of bounds")
}

fn cstr(bytes: &[u8], offset: usize) -> Result<&[u8], &'static str> {
    let tail = bytes.get(offset..).ok_or("FDT string out of bounds")?;
    let len = tail.iter().position(|&b| b == 0).ok_or("unterminated FDT string")?;
    Ok(&tail[..len])
}

fn aligned4(n: usize) -> Result<usize, &'static str> {
    Ok(n.checked_add(3).ok_or("FDT offset overflow")? & !3)
}

impl<'a> DeviceTree<'a> {
    /// # Safety
    /// `address` must point to the firmware-provided, mapped FDT header and
    /// the full `totalsize` bytes advertised by that header must be readable.
    pub unsafe fn from_ptr(address: usize) -> Result<Self, &'static str> {
        if address == 0 || address & 3 != 0 {
            return Err("invalid FDT address");
        }
        let header = core::slice::from_raw_parts(address as *const u8, 40);
        if be32(header, 0)? != MAGIC {
            return Err("invalid FDT magic");
        }
        let size = be32(header, 4)? as usize;
        if !(40..=MAX_SIZE).contains(&size) || address.checked_add(size).is_none() {
            return Err("invalid FDT size");
        }
        let bytes = core::slice::from_raw_parts(address as *const u8, size);
        let structure = section(bytes, be32(bytes, 8)? as usize, be32(bytes, 36)? as usize)?;
        let strings = section(bytes, be32(bytes, 12)? as usize, be32(bytes, 32)? as usize)?;
        let reserve_offset = be32(bytes, 16)? as usize;
        section(bytes, reserve_offset, 16)?;
        Ok(Self { bytes, structure, strings, reserve_offset })
    }

    pub fn size(&self) -> usize { self.bytes.len() }

    pub fn memory_regions(&self, mut visit: impl FnMut(Region)) -> Result<(), &'static str> {
        self.walk(1, &mut visit)
    }

    pub fn reserved_regions(&self, mut visit: impl FnMut(Region)) -> Result<(), &'static str> {
        let mut offset = self.reserve_offset;
        loop {
            let start = be64(self.bytes, offset)?;
            let size = be64(self.bytes, offset + 8)?;
            if start == 0 && size == 0 { break; }
            let end = start.checked_add(size).ok_or("FDT reserved range overflow")?;
            if size != 0 { visit(Region { start, end }); }
            offset = offset.checked_add(16).ok_or("FDT reserve offset overflow")?;
        }
        self.walk(3, &mut visit)
    }

    fn walk(&self, wanted: u8, visit: &mut impl FnMut(Region)) -> Result<(), &'static str> {
        let mut stack = [Node::EMPTY; MAX_DEPTH];
        let mut depth = 0usize;
        let mut offset = 0usize;
        loop {
            let token = be32(self.structure, offset)?;
            offset += 4;
            match token {
                1 => { // FDT_BEGIN_NODE
                    if depth == MAX_DEPTH { return Err("FDT nesting too deep"); }
                    let name = cstr(self.structure, offset)?;
                    offset = aligned4(offset + name.len() + 1)?;
                    let parent = if depth == 0 { Node::EMPTY } else { stack[depth - 1] };
                    let kind = if depth == 1 && (name == b"memory" || name.starts_with(b"memory@")) {
                        1
                    } else if depth == 1 && name == b"reserved-memory" {
                        2
                    } else if depth == 2 && parent.kind == 2 {
                        3
                    } else { 0 };
                    stack[depth] = Node {
                        kind,
                        reg_address_cells: parent.child_address_cells,
                        reg_size_cells: parent.child_size_cells,
                        child_address_cells: parent.child_address_cells,
                        child_size_cells: parent.child_size_cells,
                        reg_offset: 0, reg_len: 0,
                    };
                    depth += 1;
                }
                2 => { // FDT_END_NODE
                    if depth == 0 { return Err("unbalanced FDT nodes"); }
                    depth -= 1;
                    let node = stack[depth];
                    if node.kind == wanted && node.reg_len != 0 {
                        self.regions(node, visit)?;
                    }
                }
                3 => { // FDT_PROP
                    if depth == 0 { return Err("FDT property outside node"); }
                    let len = be32(self.structure, offset)? as usize;
                    let name_offset = be32(self.structure, offset + 4)? as usize;
                    offset += 8;
                    section(self.structure, offset, len)?;
                    let name = cstr(self.strings, name_offset)?;
                    let node = &mut stack[depth - 1];
                    match name {
                        b"#address-cells" if len == 4 => node.child_address_cells = be32(self.structure, offset)?,
                        b"#size-cells" if len == 4 => node.child_size_cells = be32(self.structure, offset)?,
                        b"reg" => { node.reg_offset = offset; node.reg_len = len; }
                        _ => {}
                    }
                    offset = aligned4(offset.checked_add(len).ok_or("FDT offset overflow")?)?;
                }
                4 => {} // FDT_NOP
                9 if depth == 0 => return Ok(()), // FDT_END
                _ => return Err("invalid FDT token"),
            }
        }
    }

    fn regions(&self, node: Node, visit: &mut impl FnMut(Region)) -> Result<(), &'static str> {
        let a = node.reg_address_cells as usize;
        let s = node.reg_size_cells as usize;
        if !(1..=2).contains(&a) || !(1..=2).contains(&s) {
            return Err("unsupported FDT address or size cells");
        }
        let stride = (a + s) * 4;
        if !node.reg_len.is_multiple_of(stride) { return Err("invalid FDT reg length"); }
        let end = node.reg_offset + node.reg_len;
        let mut offset = node.reg_offset;
        while offset < end {
            let start = self.cells(offset, a)?;
            let size = self.cells(offset + a * 4, s)?;
            if size != 0 {
                visit(Region { start, end: start.checked_add(size).ok_or("FDT region overflow")? });
            }
            offset += stride;
        }
        Ok(())
    }

    fn cells(&self, offset: usize, count: usize) -> Result<u64, &'static str> {
        let mut value = 0u64;
        for i in 0..count { value = (value << 32) | be32(self.structure, offset + i * 4)? as u64; }
        Ok(value)
    }
}
