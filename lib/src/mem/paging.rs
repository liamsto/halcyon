//! Sv39 page tables.

use super::{PAGE_SIZE, frame::FrameAlloc};
use core::{arch::asm, ops::Range};

const LEVELS: usize = 3;
const ENTRIES: usize = 512;
/// Sv39 addresses with bit 38 set must have bits 63..39 set too, so identity mappings end here.
const IDENTITY_END: usize = 1 << 38;

const SATP_SV39: usize = 8 << 60;

const PTE_VALID: usize = 1 << 0;
const PTE_READ: usize = 1 << 1;
const PTE_WRITE: usize = 1 << 2;
const PTE_EXEC: usize = 1 << 3;
const PTE_GLOBAL: usize = 1 << 5;
const PTE_ACCESSED: usize = 1 << 6;
const PTE_DIRTY: usize = 1 << 7;
/// An entry with none of these set points to the next table.
const PTE_LEAF: usize = PTE_READ | PTE_WRITE | PTE_EXEC;
const PTE_PPN_SHIFT: usize = 10;

#[repr(C, align(4096))]
pub(super) struct Table([usize; ENTRIES]);

const _: () = assert!(size_of::<Table>() == PAGE_SIZE);

/// What kernel code may do with a mapping.
#[derive(Copy, Clone)]
pub(super) enum Access {
    Read,
    ReadWrite,
    ReadExec,
}

impl Access {
    const fn bits(self) -> usize {
        match self {
            Self::Read => PTE_READ,
            Self::ReadWrite => PTE_READ | PTE_WRITE,
            Self::ReadExec => PTE_READ | PTE_EXEC,
        }
    }
}

/// Allocates an empty page table.
///
/// # Panics
/// Panics if `frames` is out of memory.
pub(super) fn new_table(frames: &mut FrameAlloc) -> *mut Table {
    let table = frames
        .alloc(1, 1)
        .expect("out of memory for kernel page tables") as *mut Table;
    // SAFETY: The frame was just allocated, so nothing else uses it.
    unsafe { table.write_bytes(0, 1) };
    table
}

/// Maps the addresses in `range` to themselves, using the largest pages that fit.
///
/// # Safety
/// `root` must come from [`new_table`] and must not be in use by any hart. Paging must be off or
/// identity map the free frames of `frames`.
///
/// # Panics
/// Panics if `range` isn't page aligned, is already mapped, or can't be identity mapped by Sv39.
pub(super) unsafe fn map(
    root: *mut Table,
    range: Range<usize>,
    access: Access,
    frames: &mut FrameAlloc,
) {
    assert!(
        range.end <= IDENTITY_END,
        "{:#x}..{:#x} is outside the lower half of Sv39",
        range.start,
        range.end
    );
    // Hardware may fault on clear accessed and dirty bits rather than set them.
    let leaf = PTE_VALID | access.bits() | PTE_GLOBAL | PTE_ACCESSED | PTE_DIRTY;

    let mut addr = range.start;
    while addr < range.end {
        let level = (0..LEVELS)
            .rev()
            .find(|&level| {
                addr.is_multiple_of(page_size(level)) && range.end - addr >= page_size(level)
            })
            .expect("mapped ranges must be page aligned");

        let mut table = root;
        for level in (level + 1..LEVELS).rev() {
            // SAFETY: `table` is `root` or a table linked from it, which only this code touches.
            let entry = unsafe { &mut (*table).0[index(addr, level)] };
            if *entry & PTE_VALID == 0 {
                let next = new_table(frames);
                *entry = (next.addr() / PAGE_SIZE) << PTE_PPN_SHIFT | PTE_VALID;
            }
            assert_eq!(*entry & PTE_LEAF, 0, "{addr:#x} is already mapped");
            table = ((*entry >> PTE_PPN_SHIFT) * PAGE_SIZE) as *mut Table;
        }

        // SAFETY: As above.
        let entry = unsafe { &mut (*table).0[index(addr, level)] };
        assert_eq!(*entry & PTE_VALID, 0, "{addr:#x} is already mapped");
        *entry = (addr / PAGE_SIZE) << PTE_PPN_SHIFT | leaf;

        addr += page_size(level);
    }
}

/// Returns the `satp` value that selects the table at `root`.
pub(super) fn satp(root: *mut Table) -> usize {
    SATP_SV39 | (root.addr() / PAGE_SIZE)
}

/// Switches the current hart to the page table in `satp`.
///
/// # Safety
/// The table must identity map everything this hart is using with the access it needs: code,
/// stack, and data.
///
/// # Panics
/// Panics if the hart doesn't support Sv39.
pub(super) unsafe fn activate(satp: usize) {
    let actual: usize;
    // SAFETY: The caller guarantees execution can continue under the new table.
    unsafe {
        asm!(
            "csrw satp, {satp}",
            "sfence.vma",
            "csrr {actual}, satp",
            satp = in(reg) satp,
            actual = out(reg) actual,
            options(nostack),
        );
    }
    // A hart ignores writes that select a mode it doesn't implement.
    assert_eq!(actual, satp, "hart doesn't support Sv39 paging");
}

const fn page_size(level: usize) -> usize {
    PAGE_SIZE << (9 * level)
}

const fn index(addr: usize, level: usize) -> usize {
    (addr / page_size(level)) % ENTRIES
}
