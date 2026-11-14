mod heap;
use crate::mem::heap::KernelHeap;
use crate::sync::Spinlock;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{self, NonNull},
    sync::atomic::{AtomicBool, Ordering},
};

pub const PAGE_SIZE_BYTES: usize = 4096;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(pub usize);

/// Provides mapped kernel virtual memory to the heap.
///
/// The following invariants need to be held by all implementations:
/// - Return `page_count` contiguous, writable virtual pages.
/// - Align returned virtaddr to `align_pages * PAGE_SIZE_BYTES`.
/// - Keep all pages mapped until `dealloc_pages`.
/// - Support concurrent calls if used elsewhere.
/// - Don't recursively allocate through the heap.
///
/// The actual physical frames don't need to be contiguous.
pub trait PageSource: Sync {
    /// # Safety
    ///
    /// `pages` must be nonzero.
    /// `align_pages` must be a nonzero and a power of two.
    ///
    /// The caller must eventually return the allocation with the same base and page count.
    unsafe fn alloc_pages(&self, page_count: usize, alignment: usize) -> Option<NonNull<u8>>;

    /// # Safety
    ///
    /// `base` needs to be a live allocation given by `alloc_pages`, with `pages` being the original count.
    ///
    /// No references using this allocation may remain in use.
    unsafe fn dealloc_pages(&self, base: NonNull<u8>, pages: usize);
}

#[global_allocator]
pub static KERNEL_ALLOC: KernelAllocator = KernelAllocator::new();

pub struct KernelAllocator {
    initialized: AtomicBool,
    heap: Spinlock<KernelHeap>,
}

impl KernelAllocator {
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            heap: Spinlock::new(KernelHeap::empty()),
        }
    }

    /// # Safety
    ///
    /// Called only once, after PageSource is fully initialized and before other harts can allocate.
    ///
    /// PageSource must remain valid for the kernel's lifetime.
    pub unsafe fn init(&self, page_source: &'static dyn PageSource) {
        let mut heap = self.heap.lock();

        heap.init(page_source);

        self.initialized.store(true, Ordering::Release);
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return ptr::null_mut();
        }

        if !self.initialized.load(Ordering::Acquire) {
            return ptr::null_mut();
        }

        let mut heap = self.heap.lock();

        unsafe { heap.alloc(layout).map_or(ptr::null_mut(), |p| p.as_ptr()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() || layout.size() == 0 {
            return;
        }

        // Valid allocations must originate from an initialized heap.
        let mut heap = self.heap.lock();

        unsafe {
            heap.dealloc(NonNull::new_unchecked(ptr), layout);
        }
    }
}

impl Default for KernelAllocator {
    fn default() -> Self {
        Self::new()
    }
}
