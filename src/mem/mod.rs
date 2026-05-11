mod heap;
pub mod kalloc;

use core::ptr::NonNull;

pub const PAGE_SIZE_BYTES: usize = 4096;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(pub usize);

pub trait PageAllocator: Sync {
    /// Allocate 2^order contiguous physical 4 KiB frames
    unsafe fn alloc_order(&self, order: usize) -> Option<PhysAddr>;

    /// Free 2^order contiguous physical 4 KiB frames
    unsafe fn dealloc_order(&self, base: PhysAddr, order: usize);
}

/// Kernel heap page provider.
///
/// Returns mapped kernel virtual memory.
pub trait PageSource: Sync {
    /// Allocate `pages` contiguous mapped kernel pages.
    /// `align_pages` is a power of 2 and page aligned
    unsafe fn alloc_pages(&self, pages: usize, align_pages: usize) -> Option<NonNull<u8>>;

    unsafe fn dealloc_pages(&self, base: NonNull<u8>, pages: usize);
}
