use crate::mem::{PageSource, heap::KernelHeap};
use crate::sync::spinlock::Spinlock;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{self, NonNull},
    sync::atomic::{AtomicBool, Ordering},
};

#[global_allocator]
pub static KERNEL_ALLOC: KernelAllocator = KernelAllocator::new();

pub struct KernelAllocator {
    initialized: AtomicBool,
    heap: Spinlock<KernelHeap>,
}

unsafe impl Sync for KernelAllocator {}

impl KernelAllocator {
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            heap: Spinlock::new(KernelHeap::empty()),
        }
    }

    /// call after :
    /// - physical memory discovery,
    /// - frame allocator init,
    /// - kernel PTs up,
    /// - PageSource can map and free
    ///
    /// Must be called before using any structures that alloc
    pub unsafe fn init(&self, page_source: &'static dyn PageSource) {
        let mut heap = self.heap.lock();
        heap.init(page_source);
        self.initialized.store(true, Ordering::Release);
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !self.initialized.load(Ordering::Acquire) {
            return ptr::null_mut();
        }

        if layout.size() == 0 {
            return ptr::null_mut();
        }

        let mut heap = self.heap.lock();

        unsafe {
            heap.alloc(layout)
                .map_or(ptr::null_mut(), |ptr| ptr.as_ptr())
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() || layout.size() == 0 {
            return;
        }

        let mut heap = self.heap.lock();
        unsafe { heap.dealloc(NonNull::new_unchecked(ptr), layout) };
    }
}

impl Default for KernelAllocator {
    fn default() -> Self {
        Self::new()
    }
}
