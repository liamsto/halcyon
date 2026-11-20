//! Memory management.

mod heap;

use crate::sync::Spinlock;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{self, NonNull},
    sync::atomic::{AtomicBool, Ordering},
};
use heap::KernelHeap;

/// Size of a page in bytes.
pub const PAGE_SIZE: usize = 4096;

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
