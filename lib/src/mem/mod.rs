//! Memory management.

mod frame;
mod heap;
mod paging;

use crate::{dtb, sync::Spinlock};
use core::{
    alloc::{GlobalAlloc, Layout},
    ops::Range,
    ptr::{self, NonNull},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use frame::FrameAlloc;
use heap::KernelHeap;
use paging::Access;

/// Size of a page in bytes.
pub const PAGE_SIZE: usize = 4096;

// Set by the linker script. All of them are page aligned.
unsafe extern "C" {
    static __kernel_start: u8;
    static __text_end: u8;
    static __rodata_end: u8;
    static __kernel_end: u8;
}

/// Free physical memory. `None` until [`init`] runs.
static FRAMES: Spinlock<Option<FrameAlloc>> = Spinlock::new(None);

/// `satp` value of the kernel page table. Zero until [`init`] runs.
static KERNEL_SATP: AtomicUsize = AtomicUsize::new(0);

/// Sets up kernel memory and turns on paging for the boot hart.
///
/// All RAM gets identity mapped:
/// - kernel code is rwx
/// - kernel constants are read only,
/// - everything else as rw
/// Nothing else is mapped, so device memory faults. RAM that isn't the kernel image, the device tree,
/// or reserved by the device tree then backs the kernel heap.
///
/// # Safety
/// - Must be called once, on the boot hart, after
///   [`init_current_cpu`](crate::cpu::init_current_cpu) and before any other hart runs Rust code
///   that uses memory outside the kernel image.
/// - `info` must describe the memory of this machine.
/// - Nothing may use RAM outside the kernel image, the device tree, and the ranges reserved by
///   the device tree.
///
/// # Panics
/// Panics if the device tree has no usable memory, if memory doesn't fit the lower half of the
/// Sv39 address space, or if the hart doesn't support Sv39.
pub unsafe fn init(info: &dtb::Info<'_>) {
    let kernel = (&raw const __kernel_start).addr()..(&raw const __kernel_end).addr();
    let text_end = (&raw const __text_end).addr();
    let rodata_end = (&raw const __rodata_end).addr();

    let dtb = info.blob.as_ptr_range();
    let reserved = [kernel.clone(), dtb.start.addr()..dtb.end.addr()]
        .into_iter()
        .chain(info.memreserve().iter().map(addr_range));
    // Partial pages at either end of a range are unusable.
    let ram = info
        .mem()
        .iter()
        .map(addr_range)
        .map(|range| range.start.next_multiple_of(PAGE_SIZE)..range.end / PAGE_SIZE * PAGE_SIZE)
        .filter(|range| !range.is_empty());

    let start = ram.clone().map(|range| range.start).min();
    let end = ram.clone().map(|range| range.end).max();
    let (Some(start), Some(end)) = (start, end) else {
        panic!("device tree has no usable memory");
    };
    let span = start..end;

    let bitmap_size = FrameAlloc::bitmap_size(&span).next_multiple_of(PAGE_SIZE);
    let bitmap = ram
        .clone()
        .find_map(|range| find_unreserved(range, bitmap_size, reserved.clone()))
        .expect("no free memory for the frame bitmap");

    // SAFETY: `bitmap` is page aligned RAM that is big enough and that nothing else claims.
    let mut frames = unsafe { FrameAlloc::new(&span, bitmap) };
    for range in ram.clone() {
        frames.release(range);
    }
    for range in reserved {
        frames.reserve(range);
    }
    frames.reserve(bitmap..bitmap + bitmap_size);
    // The heap can't hand out a null pointer.
    frames.reserve(0..PAGE_SIZE);

    let segments = [
        (0..kernel.start, Access::ReadWrite),
        (kernel.start..text_end, Access::ReadExec),
        (text_end..rodata_end, Access::Read),
        (rodata_end..usize::MAX, Access::ReadWrite),
    ];
    let root = paging::new_table(&mut frames);
    for range in ram {
        for (segment, access) in &segments {
            let part = range.start.max(segment.start)..range.end.min(segment.end);
            if !part.is_empty() {
                // SAFETY: `root` is a new table that no hart uses yet, and paging is off.
                unsafe { paging::map(root, part, *access, &mut frames) };
            }
        }
    }

    let satp = paging::satp(root);
    KERNEL_SATP.store(satp, Ordering::Release);
    // SAFETY: The kernel image, including the boot stacks, is in RAM, and all RAM is identity
    // mapped with the access the linker script lays out.
    unsafe { paging::activate(satp) };

    *FRAMES.lock() = Some(frames);
    init_heap(&FRAMES);
}

/// Turns on paging for a secondary hart.
///
/// # Safety
/// Must be called after [`init`] has returned on the boot hart.
///
/// # Panics
/// Panics if the hart doesn't support Sv39.
pub unsafe fn init_hart() {
    let satp = KERNEL_SATP.load(Ordering::Acquire);
    assert_ne!(satp, 0, "mem::init hasn't run on the boot hart");
    // SAFETY: The kernel page table identity maps the kernel image, including the boot stacks.
    unsafe { paging::activate(satp) };
}

fn addr_range(range: &dtb::Range) -> Range<usize> {
    let base = usize::try_from(range.base).expect("physical addresses fit in usize on RV64");
    let size = usize::try_from(range.size).expect("physical sizes fit in usize on RV64");
    base..base.saturating_add(size)
}

/// Returns the first page aligned address in `within` where `size` bytes miss every reserved
/// range.
fn find_unreserved(
    within: Range<usize>,
    size: usize,
    reserved: impl Iterator<Item = Range<usize>> + Clone,
) -> Option<usize> {
    let mut start = within.start;
    'retry: loop {
        let end = start.checked_add(size)?;
        if end > within.end {
            return None;
        }

        for range in reserved.clone() {
            if range.start < end && start < range.end {
                start = range.end.next_multiple_of(PAGE_SIZE);
                continue 'retry;
            }
        }
        return Some(start);
    }
}

// SAFETY: Frames are identity mapped as writable by `init` and will be mapped forever. The lock
// makes calls safe from any hart, and the frame allocator won't alloc from the heap.
unsafe impl PageSource for Spinlock<Option<FrameAlloc>> {
    unsafe fn alloc_pages(&self, count: usize, align: usize) -> Option<NonNull<u8>> {
        let addr = self.lock().as_mut()?.alloc(count, align)?;
        NonNull::new(addr as *mut u8)
    }

    unsafe fn dealloc_pages(&self, base: NonNull<u8>, count: usize) {
        self.lock()
            .as_mut()
            .expect("pages can only come from an initialized frame allocator")
            .free(base.addr().get(), count);
    }
}

/// Provides mapped kernel virtual memory to the heap.
///
/// # Safety
/// Implementations must:
/// - Return `count` contiguous, writable virtual pages.
/// - Align returned addresses to `align * PAGE_SIZE`.
/// - Keep all pages mapped until `dealloc_pages`.
/// - Support concurrent calls if used elsewhere.
/// - Not recursively allocate through the heap.
///
/// The physical frames do not need to be contiguous.
pub unsafe trait PageSource: Sync {
    /// Allocates `count` pages aligned to `align` pages.
    ///
    /// # Safety
    /// `count` must be nonzero. `align` must be a nonzero power of two.
    ///
    /// The caller must eventually return the allocation with the same base and page count.
    unsafe fn alloc_pages(&self, count: usize, align: usize) -> Option<NonNull<u8>>;

    /// Frees `count` pages starting at `base`.
    ///
    /// # Safety
    /// `base` must be a live allocation from `alloc_pages`, with `count` being the original count.
    ///
    /// No references into this allocation may remain in use.
    unsafe fn dealloc_pages(&self, base: NonNull<u8>, count: usize);
}

#[global_allocator]
static KERNEL_ALLOC: KernelAllocator = KernelAllocator {
    initialized: AtomicBool::new(false),
    heap: Spinlock::new(KernelHeap::empty()),
};

/// Gives the kernel heap its page source. Allocations fail until this is called.
///
/// # Panics
/// Panics if called more than once.
pub fn init_heap(page_source: &'static dyn PageSource) {
    KERNEL_ALLOC.heap.lock().init(page_source);
    KERNEL_ALLOC.initialized.store(true, Ordering::Release);
}

struct KernelAllocator {
    // Lets allocations before `init_heap` fail without touching the lock, which needs per-hart
    // state.
    initialized: AtomicBool,
    heap: Spinlock<KernelHeap>,
}

// SAFETY: Allocation and deallocation go through the heap lock.
unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return ptr::null_mut();
        }

        if !self.initialized.load(Ordering::Acquire) {
            return ptr::null_mut();
        }

        let mut heap = self.heap.lock();

        // SAFETY: The lock gives exclusive access to the heap.
        unsafe { heap.alloc(layout).map_or(ptr::null_mut(), |p| p.as_ptr()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let Some(ptr) = NonNull::new(ptr) else {
            return;
        };
        if layout.size() == 0 {
            return;
        }

        // Valid allocations must originate from an initialized heap.
        let mut heap = self.heap.lock();

        // SAFETY: The caller guarantees `ptr` was allocated with `layout` by this allocator.
        unsafe { heap.dealloc(ptr, layout) };
    }
}
