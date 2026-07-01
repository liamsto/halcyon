use crate::mem::{PAGE_SIZE_BYTES, PageSource, VirtAddr};
use core::{alloc::Layout, ptr::NonNull};

const SIZE_CLASSES: &[usize] = &[
    8, 16, 32, 48, 64, 80, 96, 128, 160, 192, 256, 320, 384, 512, 768, 1024, 1536, 2048,
];
const SIZE_CLASS_COUNT: usize = SIZE_CLASSES.len();
const SMALL_MAX: usize = 2048;

pub struct KernelHeap {
    page_source: Option<&'static dyn PageSource>,
    bins: [Bin; SIZE_CLASS_COUNT],
    large: LargeAlloc,
}

impl KernelHeap {
    pub const fn empty() -> Self {
        Self {
            page_source: None,
            bins: [Bin::empty(); SIZE_CLASS_COUNT],
            large: LargeAlloc::empty(),
        }
    }

    pub fn init(&mut self, page_source: &'static dyn PageSource) {
        self.page_source = Some(page_source);
    }

    pub unsafe fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let needed = layout.size().max(layout.align());

        if needed <= SMALL_MAX {
            let class_idx = class_index(needed)?;
            unsafe { self.bins[class_idx].alloc(self.page_source?) }
        } else {
            unsafe { self.large.alloc(self.page_source?, layout) }
        }
    }

    pub unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let needed = layout.size().max(layout.align());

        if needed <= SMALL_MAX {
            if let Some(class_idx) = class_index(needed) {
                unsafe { self.bins[class_idx].dealloc(ptr) };
            }
        } else {
            unsafe {
                self.large.dealloc(self.page_source.unwrap(), ptr, layout);
            }
        }
    }
}

const fn class_index(size: usize) -> Option<usize> {
    let mut i = 0;
    while i < SIZE_CLASS_COUNT {
        if SIZE_CLASSES[i] >= size {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[derive(Copy, Clone)]
struct Bin {
    partial: *mut RunHeader,
    full: *mut RunHeader,
}

impl Bin {
    pub const fn empty() -> Self {
        Self {
            partial: core::ptr::null_mut(),
            full: core::ptr::null_mut(),
        }
    }

    unsafe fn alloc(&mut self, page_source: &'static dyn PageSource) -> Option<NonNull<u8>> {
        unsafe {
            if self.partial.is_null() {
                self.add_run(page_source)?;
            }

            let run = &mut *self.partial;
            let ptr = run.alloc_block()?;

            if run.free_count == 0 {
                self.move_partial_to_full();
            }
            Some(ptr)
        }
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>) {
        // find run via page metadata
        //
        // page_meta[virt_to_page(ptr)].run_head
        unsafe {
            let run = find_run_for_ptr(ptr);
            (*run).free_block(ptr);

            if (*run).free_count == (*run).total_count {
                self.release_empty_run(run);
            }
        }
    }

    unsafe fn add_run(&mut self, page_source: &'static dyn PageSource) -> Option<()> {
        let run_pages = 1;
        unsafe {
            let base = page_source.alloc_pages(run_pages, 1)?;

            let run = base.cast::<RunHeader>().as_ptr();
            (*run).init(/* class size, run pages, bitmap*/);

            self.push_partial(run);
        }
        Some(())
    }

    unsafe fn move_partial_to_full(&mut self) {
        // list manipulation
    }

    unsafe fn release_empty_run(&mut self, run: *mut RunHeader) {
        // remove from bin list and return pages to PageSource
    }

    unsafe fn push_partial(&mut self, run: *mut RunHeader) {
        unsafe { (*run).next = self.partial };
        self.partial = run;
    }
}

enum PageKind {
    Free,
    SmallRun {
        run_head: *mut RunHeader,
        class_idx: u16,
    },
    LargeHead {
        pages: usize,
    },
    LargeTail {
        head: VirtAddr,
    },
}

#[repr(C)]
struct RunHeader {
    class_size: usize,
    total_count: u16,
    free_count: u16,
    next: *mut Self,
    // TODO: bitmap or metadata
}

unsafe impl Sync for RunHeader {}
unsafe impl Send for RunHeader {}

impl RunHeader {
    unsafe fn init(&mut self) {
        todo!()
        // initialize bitmap and counters
    }

    unsafe fn alloc_block(&mut self) -> Option<NonNull<u8>> {
        // find first zero bit, set, return block pointer
        todo!()
    }

    unsafe fn free_block(&mut self, ptr: NonNull<u8>) {
        // compute block index, clear bit
        todo!()
    }
}

unsafe fn find_run_for_ptr(_ptr: NonNull<u8>) -> *mut RunHeader {
    // Use per-page metadata, not heap allocation.
    todo!()
}

struct LargeAlloc;

impl LargeAlloc {
    pub const fn empty() -> Self {
        Self
    }

    unsafe fn alloc(
        &mut self,
        page_source: &'static dyn PageSource,
        layout: Layout,
    ) -> Option<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();

        let pages = size.div_ceil(PAGE_SIZE_BYTES);

        let align_pages = if align <= PAGE_SIZE_BYTES {
            1
        } else {
            align.div_ceil(PAGE_SIZE_BYTES).next_power_of_two()
        };

        unsafe { page_source.alloc_pages(pages, align_pages) }
    }

    unsafe fn dealloc(
        &mut self,
        page_source: &'static dyn PageSource,
        ptr: NonNull<u8>,
        layout: Layout,
    ) {
        let pages = layout.size().div_ceil(PAGE_SIZE_BYTES);
        unsafe {
            page_source.dealloc_pages(ptr, pages);
        }
    }
}
