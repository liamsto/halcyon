use crate::cpu::MAX_CPUS;
use core::{ptr, slice, str};

const FDT_MAGIC: u32 = 0xD00D_FEED;
const HEADER_LEN: usize = 40;
const MIN_VERSION: u32 = 17;

const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_NOP: u32 = 4;
const FDT_END: u32 = 9;

const MAX_DEPTH: usize = 32;
const MAX_RANGES: usize = 32;

const DEFAULT_ADDR_CELLS: usize = 2;
const DEFAULT_SIZE_CELLS: usize = 1;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Null,
    BadMagic(u32),
    BadVersion(u32),
    BadHeader,
    BadReserve,
    BadStruct,
    BadString,
    BadCells,
    BadToken(u32),
    TooDeep,
    TooManyMem,
    TooManyMemreserve,
    TooManyHarts,
    HartTooLarge,
}

#[derive(Copy, Clone)]
pub struct Range {
    pub base: u64,
    pub size: u64,
}

impl Range {
    const EMPTY: Self = Self { base: 0, size: 0 };
}

pub struct Info<'a> {
    pub model: Option<&'a str>,
    pub boot_cpuid: u32,
    mem: [Range; MAX_RANGES],
    mem_len: usize,
    memreserve: [Range; MAX_RANGES],
    memreserve_len: usize,
    harts: [usize; MAX_CPUS],
    hart_len: usize,
}

impl<'a> Info<'a> {
    const fn empty(boot_cpuid: u32) -> Self {
        Self {
            model: None,
            boot_cpuid,
            mem: [Range::EMPTY; MAX_RANGES],
            mem_len: 0,
            memreserve: [Range::EMPTY; MAX_RANGES],
            memreserve_len: 0,
            harts: [0; MAX_CPUS],
            hart_len: 0,
        }
    }

    pub fn mem(&self) -> &[Range] {
        &self.mem[..self.mem_len]
    }

    pub fn memreserve(&self) -> &[Range] {
        &self.memreserve[..self.memreserve_len]
    }

    pub fn harts(&self) -> &[usize] {
        &self.harts[..self.hart_len]
    }

    const fn push_mem(&mut self, range: Range) -> Result<(), Error> {
        if self.mem_len == self.mem.len() {
            return Err(Error::TooManyMem);
        }

        self.mem[self.mem_len] = range;
        self.mem_len += 1;
        Ok(())
    }

    const fn push_memreserve(&mut self, range: Range) -> Result<(), Error> {
        if self.memreserve_len == self.memreserve.len() {
            return Err(Error::TooManyMemreserve);
        }

        self.memreserve[self.memreserve_len] = range;
        self.memreserve_len += 1;
        Ok(())
    }

    fn push_hart(&mut self, hart: usize) -> Result<(), Error> {
        if self.harts[..self.hart_len].contains(&hart) {
            return Ok(());
        }

        if self.hart_len == self.harts.len() {
            return Err(Error::TooManyHarts);
        }

        self.harts[self.hart_len] = hart;
        self.hart_len += 1;
        Ok(())
    }
}

struct Header {
    total: usize,
    struct_off: usize,
    strings_off: usize,
    reserve_off: usize,
    strings_len: usize,
    struct_len: usize,
    boot_cpuid: u32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Kind {
    Other,
    Memory,
    Cpu,
}

#[derive(Copy, Clone)]
struct Node<'a> {
    name: &'a str,
    parent_addr: usize,
    parent_size: usize,
    addr_cells: usize,
    size_cells: usize,
    kind: Kind,
    reg: Option<&'a [u8]>,
    status: Option<&'a str>,
}

impl<'a> Node<'a> {
    const EMPTY: Self = Self {
        name: "",
        parent_addr: DEFAULT_ADDR_CELLS,
        parent_size: DEFAULT_SIZE_CELLS,
        addr_cells: DEFAULT_ADDR_CELLS,
        size_cells: DEFAULT_SIZE_CELLS,
        kind: Kind::Other,
        reg: None,
        status: None,
    };
}

struct Parser<'a> {
    structure: &'a [u8],
    strings: &'a [u8],
    pos: usize,
    stack: [Node<'a>; MAX_DEPTH],
    depth: usize,
    info: Info<'a>,
}

/// Parses a flattened device tree passed by the SBI in `a1`.
///
/// # Safety
/// `addr` must point to a valid, mapped FDT blob that lives as long as the
/// returned references are used.
pub unsafe fn parse<'a>(addr: usize) -> Result<Info<'a>, Error> {
    if addr == 0 {
        return Err(Error::Null);
    }

    let base = addr as *const u8;
    let header = unsafe { read_header(base)? };
    let blob = unsafe { slice::from_raw_parts(base, header.total) };
    let mut info = Info::empty(header.boot_cpuid);

    parse_reserve(blob, header.reserve_off, &mut info)?;

    let structure = block(blob, header.struct_off, header.struct_len)?;
    let strings = block(blob, header.strings_off, header.strings_len)?;

    Parser {
        structure,
        strings,
        pos: 0,
        stack: [Node::EMPTY; MAX_DEPTH],
        depth: 0,
        info,
    }
    .parse()
}

unsafe fn read_header(base: *const u8) -> Result<Header, Error> {
    let magic = unsafe { read_be32(base, 0) };
    if magic != FDT_MAGIC {
        return Err(Error::BadMagic(magic));
    }

    let total = unsafe { read_be32(base, 4) } as usize;
    if total < HEADER_LEN {
        return Err(Error::BadHeader);
    }

    let version = unsafe { read_be32(base, 20) };
    if version < MIN_VERSION {
        return Err(Error::BadVersion(version));
    }

    let header = Header {
        total,
        struct_off: unsafe { read_be32(base, 8) } as usize,
        strings_off: unsafe { read_be32(base, 12) } as usize,
        reserve_off: unsafe { read_be32(base, 16) } as usize,
        boot_cpuid: unsafe { read_be32(base, 28) },
        strings_len: unsafe { read_be32(base, 32) } as usize,
        struct_len: unsafe { read_be32(base, 36) } as usize,
    };

    block_bounds(header.struct_off, header.struct_len, total)?;
    block_bounds(header.strings_off, header.strings_len, total)?;

    if header.reserve_off >= total {
        return Err(Error::BadHeader);
    }

    Ok(header)
}

const unsafe fn read_be32(base: *const u8, off: usize) -> u32 {
    unsafe { u32::from_be(ptr::read_unaligned(base.add(off).cast::<u32>())) }
}

fn parse_reserve<'a>(blob: &[u8], mut off: usize, info: &mut Info<'a>) -> Result<(), Error> {
    loop {
        if off.checked_add(16).is_none_or(|end| end > blob.len()) {
            return Err(Error::BadReserve);
        }

        let base = be64(blob, off)?;
        let size = be64(blob, off + 8)?;
        off += 16;

        if base == 0 && size == 0 {
            return Ok(());
        }

        if size != 0 {
            info.push_memreserve(Range { base, size })?;
        }
    }
}

impl<'a> Parser<'a> {
    fn parse(mut self) -> Result<Info<'a>, Error> {
        while self.pos < self.structure.len() {
            match self.take_be32()? {
                FDT_BEGIN_NODE => self.begin_node()?,
                FDT_END_NODE => self.end_node()?,
                FDT_PROP => self.prop()?,
                FDT_NOP => {}
                FDT_END => {
                    if self.depth != 0 {
                        return Err(Error::BadStruct);
                    }

                    return Ok(self.info);
                }
                token => return Err(Error::BadToken(token)),
            }
        }

        Err(Error::BadStruct)
    }

    fn begin_node(&mut self) -> Result<(), Error> {
        if self.depth == self.stack.len() {
            return Err(Error::TooDeep);
        }

        let start = self.pos;
        let len = self.structure[start..]
            .iter()
            .position(|&byte| byte == 0)
            .ok_or(Error::BadStruct)?;
        let end = start + len;
        let name = str::from_utf8(&self.structure[start..end]).map_err(|_| Error::BadString)?;
        self.pos = align4(end + 1)?;

        let (parent_addr, parent_size) = match self.depth {
            0 => (DEFAULT_ADDR_CELLS, DEFAULT_SIZE_CELLS),
            depth => {
                let parent = self.stack[depth - 1];
                (parent.addr_cells, parent.size_cells)
            }
        };

        self.stack[self.depth] = Node {
            name,
            parent_addr,
            parent_size,
            addr_cells: DEFAULT_ADDR_CELLS,
            size_cells: DEFAULT_SIZE_CELLS,
            kind: kind_from_name(name),
            reg: None,
            status: None,
        };
        self.depth += 1;
        Ok(())
    }

    fn end_node(&mut self) -> Result<(), Error> {
        if self.depth == 0 {
            return Err(Error::BadStruct);
        }

        self.depth -= 1;
        let node = self.stack[self.depth];
        if !enabled(node.status) {
            return Ok(());
        }

        match node.kind {
            Kind::Memory => self.parse_mem(node),
            Kind::Cpu => self.parse_cpu(node),
            Kind::Other => Ok(()),
        }
    }

    fn prop(&mut self) -> Result<(), Error> {
        if self.depth == 0 {
            return Err(Error::BadStruct);
        }

        let len = self.take_be32()? as usize;
        let name_off = self.take_be32()? as usize;
        let start = self.pos;
        let end = start.checked_add(len).ok_or(Error::BadStruct)?;
        if end > self.structure.len() {
            return Err(Error::BadStruct);
        }

        let value = &self.structure[start..end];
        self.pos = align4(end)?;

        let name = str_at(self.strings, name_off)?;
        let is_root = self.depth == 1 && self.stack[0].name.is_empty();
        let node = &mut self.stack[self.depth - 1];

        match name {
            "#address-cells" => node.addr_cells = one_cell(value)?,
            "#size-cells" => node.size_cells = one_cell(value)?,
            "device_type" => match first_str(value)? {
                Some("memory") => node.kind = Kind::Memory,
                Some("cpu") => node.kind = Kind::Cpu,
                _ => {}
            },
            "model" if is_root => set_model(value, &mut self.info)?,
            "reg" => node.reg = Some(value),
            "status" => node.status = first_str(value)?,
            _ => {}
        }

        Ok(())
    }

    fn parse_mem(&mut self, node: Node<'a>) -> Result<(), Error> {
        let Some(reg) = node.reg else {
            return Ok(());
        };

        if node.parent_size == 0 {
            return Err(Error::BadCells);
        }

        let mut off = 0;
        while off < reg.len() {
            let base = read_cells(reg, &mut off, node.parent_addr)?;
            let size = read_cells(reg, &mut off, node.parent_size)?;
            if size != 0 {
                self.info.push_mem(Range { base, size })?;
            }
        }

        Ok(())
    }

    fn parse_cpu(&mut self, node: Node<'a>) -> Result<(), Error> {
        let Some(reg) = node.reg else {
            return Ok(());
        };

        if node.parent_addr == 0 {
            return Err(Error::BadCells);
        }

        let mut off = 0;
        while off < reg.len() {
            let hart = read_cells(reg, &mut off, node.parent_addr)?;
            let _ = read_cells(reg, &mut off, node.parent_size)?;

            let hart = usize::try_from(hart).map_err(|_| Error::HartTooLarge)?;
            self.info.push_hart(hart)?;
        }

        Ok(())
    }

    fn take_be32(&mut self) -> Result<u32, Error> {
        let value = be32(self.structure, self.pos)?;
        self.pos += 4;
        Ok(value)
    }
}

fn set_model<'a>(value: &'a [u8], info: &mut Info<'a>) -> Result<(), Error> {
    info.model = first_str(value)?;
    Ok(())
}

fn enabled(status: Option<&str>) -> bool {
    !matches!(status, Some("disabled"))
}

fn kind_from_name(name: &str) -> Kind {
    match base_name(name) {
        "memory" => Kind::Memory,
        "cpu" => Kind::Cpu,
        _ => Kind::Other,
    }
}

fn base_name(name: &str) -> &str {
    name.as_bytes()
        .iter()
        .position(|&byte| byte == b'@')
        .map_or(name, |at| &name[..at])
}

fn one_cell(value: &[u8]) -> Result<usize, Error> {
    if value.len() != 4 {
        return Err(Error::BadCells);
    }

    Ok(be32(value, 0)? as usize)
}

fn read_cells(buf: &[u8], off: &mut usize, cells: usize) -> Result<u64, Error> {
    if cells > 2 {
        return Err(Error::BadCells);
    }

    let mut value = 0;
    for _ in 0..cells {
        value = (value << 32) | u64::from(be32(buf, *off)?);
        *off += 4;
    }

    Ok(value)
}

fn first_str(value: &[u8]) -> Result<Option<&str>, Error> {
    if value.is_empty() {
        return Ok(None);
    }

    let end = value
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(value.len());

    str::from_utf8(&value[..end])
        .map(Some)
        .map_err(|_| Error::BadString)
}

fn str_at(strings: &[u8], off: usize) -> Result<&str, Error> {
    if off >= strings.len() {
        return Err(Error::BadString);
    }

    let end = strings[off..]
        .iter()
        .position(|&byte| byte == 0)
        .ok_or(Error::BadString)?
        + off;

    str::from_utf8(&strings[off..end]).map_err(|_| Error::BadString)
}

fn block(blob: &[u8], off: usize, len: usize) -> Result<&[u8], Error> {
    let end = block_bounds(off, len, blob.len())?;
    Ok(&blob[off..end])
}

fn block_bounds(off: usize, len: usize, total: usize) -> Result<usize, Error> {
    let end = off.checked_add(len).ok_or(Error::BadHeader)?;
    if end > total {
        return Err(Error::BadHeader);
    }

    Ok(end)
}

fn align4(value: usize) -> Result<usize, Error> {
    value
        .checked_add(3)
        .map(|value| value & !3)
        .ok_or(Error::BadStruct)
}

fn be32(buf: &[u8], off: usize) -> Result<u32, Error> {
    if off.checked_add(4).is_none_or(|end| end > buf.len()) {
        return Err(Error::BadStruct);
    }

    Ok(u32::from_be_bytes([
        buf[off],
        buf[off + 1],
        buf[off + 2],
        buf[off + 3],
    ]))
}

fn be64(buf: &[u8], off: usize) -> Result<u64, Error> {
    if off.checked_add(8).is_none_or(|end| end > buf.len()) {
        return Err(Error::BadStruct);
    }

    Ok(u64::from_be_bytes([
        buf[off],
        buf[off + 1],
        buf[off + 2],
        buf[off + 3],
        buf[off + 4],
        buf[off + 5],
        buf[off + 6],
        buf[off + 7],
    ]))
}
