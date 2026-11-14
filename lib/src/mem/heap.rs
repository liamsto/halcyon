use crate::mem::{PAGE_SIZE_BYTES, PageSource};
use core::{alloc::Layout, mem::size_of, ptr::NonNull};

const SIZE_CLASSES: &[usize] = &[
    8, 16, 32, 48, 64, 80, 96, 128, 160, 192, 256, 320, 384, 512, 768, 1024, 1536, 2048,
];
const SIZE_CLASS_COUNT: usize = SIZE_CLASSES.len();
const SMALL_MAX: usize = 2048;
const BITS_PER_WORD: usize = usize::BITS as usize;
const BITMAP_WORDS: usize = (PAGE_SIZE_BYTES / 8).div_ceil(BITS_PER_WORD);

// Require page-aligned, one-page small runs
const _: () = assert!(PAGE_SIZE_BYTES.is_power_of_two() && PAGE_SIZE_BYTES >= 4096);

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

    // Call once, before any allocation or deallocation.
    pub fn init(&mut self, page_source: &'static dyn PageSource) {
        assert!(self.page_source.is_none(), "heap already initialized");
        self.page_source = Some(page_source);
    }

    /// # Safety
    /// Caller must uphold exclusive-access and layout.
    pub unsafe fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        if layout.size() == 0 {
            return None;
        }
        let pages = self.page_source?;
        match class_index(layout) {
            Some(i) => unsafe { self.bins[i].alloc(pages, SIZE_CLASSES[i]) },
            None => unsafe { self.large.alloc(pages, layout) },
        }
    }

    /// # Safety
    /// `ptr` must be live, owned by this heap, and allocated with `layout`.
    /// Don't double free.
    pub unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        if layout.size() == 0 {
            return;
        }
        let pages = self.page_source.expect("heap not initialized");
        match class_index(layout) {
            Some(i) => unsafe { self.bins[i].dealloc(pages, ptr, SIZE_CLASSES[i]) },
            None => unsafe { self.large.dealloc(pages, ptr, layout) },
        }
    }
}

// *All* access to the heap has to be externally synchronized,
// and PageSource needs to be under that synchronization.
unsafe impl Send for KernelHeap {}

fn class_index(layout: Layout) -> Option<usize> {
    let size = layout.size();
    let align = layout.align();
    SIZE_CLASSES
        .iter()
        .position(|&class| class >= size && class % align == 0)
}

#[derive(Copy, Clone)]
struct Bin {
    partial: *mut RunHeader,
    full: *mut RunHeader,
}

impl Bin {
    const fn empty() -> Self {
        Self {
            partial: core::ptr::null_mut(),
            full: core::ptr::null_mut(),
        }
    }

    unsafe fn alloc(
        &mut self,
        page_source: &'static dyn PageSource,
        class_size: usize,
    ) -> Option<NonNull<u8>> {
        unsafe {
            if self.partial.is_null() {
                self.add_run(page_source, class_size)?;
            }

            let run = self.partial;
            let ptr = (*run).alloc()?;
            if (*run).free_count == 0 {
                Self::remove(&mut self.partial, run);
                Self::push(&mut self.full, run);
            }
            Some(ptr)
        }
    }

    unsafe fn dealloc(
        &mut self,
        page_source: &'static dyn PageSource,
        ptr: NonNull<u8>,
        class_size: usize,
    ) {
        unsafe {
            let run = run_for(ptr);
            assert_eq!((*run).class_size, class_size, "wrong allocation layout");

            let was_full = (*run).free_count == 0;
            (*run).free(ptr);

            if (*run).free_count == (*run).total_count {
                // Run was empty, unlink then return page
                if was_full {
                    Self::remove(&mut self.full, run);
                } else {
                    Self::remove(&mut self.partial, run);
                }
                page_source.dealloc_pages(NonNull::new_unchecked(run.cast::<u8>()), 1);
            } else if was_full {
                // Previously full run, now partial
                Self::remove(&mut self.full, run);
                Self::push(&mut self.partial, run);
            }
        }
    }

    unsafe fn add_run(
        &mut self,
        page_source: &'static dyn PageSource,
        class_size: usize,
    ) -> Option<()> {
        let header = RunHeader::new(class_size)?;
        unsafe {
            let page = page_source.alloc_pages(1, 1)?;
            debug_assert_eq!((page.as_ptr() as usize) % PAGE_SIZE_BYTES, 0);
            let run = page.as_ptr().cast::<RunHeader>();
            core::ptr::write(run, header);
            Self::push(&mut self.partial, run);
        }
        Some(())
    }

    unsafe fn push(head: &mut *mut RunHeader, run: *mut RunHeader) {
        unsafe {
            (*run).prev = core::ptr::null_mut();
            (*run).next = *head;
            if !(*head).is_null() {
                (**head).prev = run;
            }
            *head = run;
        }
    }

    unsafe fn remove(head: &mut *mut RunHeader, run: *mut RunHeader) {
        unsafe {
            let prev = (*run).prev;
            let next = (*run).next;
            if prev.is_null() {
                assert_eq!(*head, run, "run not at expected list head");
                *head = next;
            } else {
                (*prev).next = next;
            }
            if !next.is_null() {
                (*next).prev = prev;
            }
            (*run).prev = core::ptr::null_mut();
            (*run).next = core::ptr::null_mut();
        }
    }
}

#[repr(C)]
struct RunHeader {
    next: *mut Self,
    prev: *mut Self,
    class_size: usize,
    data_offset: usize,
    total_count: usize,
    free_count: usize,
    bitmap: [usize; BITMAP_WORDS],
}

impl RunHeader {
    fn new(class_size: usize) -> Option<Self> {
        if class_size == 0 || class_size > SMALL_MAX {
            return None;
        }
        let block_align = 1usize << class_size.trailing_zeros();
        let data_offset = size_of::<Self>().checked_add(block_align - 1)? & !(block_align - 1);
        if data_offset >= PAGE_SIZE_BYTES {
            return None;
        }
        let total_count = (PAGE_SIZE_BYTES - data_offset) / class_size;
        if total_count == 0 || total_count > BITMAP_WORDS * BITS_PER_WORD {
            return None;
        }

        Some(Self {
            next: core::ptr::null_mut(),
            prev: core::ptr::null_mut(),
            class_size,
            data_offset,
            total_count,
            free_count: total_count,
            bitmap: [0; BITMAP_WORDS],
        })
    }

    unsafe fn alloc(&mut self) -> Option<NonNull<u8>> {
        if self.free_count == 0 {
            return None;
        }

        let words = self.total_count.div_ceil(BITS_PER_WORD);
        for word_index in 0..words {
            let remaining = self.total_count - word_index * BITS_PER_WORD;
            let valid_bits = remaining.min(BITS_PER_WORD);
            let valid_mask = if valid_bits == BITS_PER_WORD {
                usize::MAX
            } else {
                (1usize << valid_bits) - 1
            };
            let free_bits = !self.bitmap[word_index] & valid_mask;
            if free_bits == 0 {
                continue;
            }

            let bit_index = free_bits.trailing_zeros() as usize;
            let index = word_index * BITS_PER_WORD + bit_index;
            self.bitmap[word_index] |= 1usize << bit_index;
            self.free_count -= 1;

            unsafe {
                let data = (self as *mut Self)
                    .cast::<u8>()
                    .add(self.data_offset + index * self.class_size);
                return Some(NonNull::new_unchecked(data));
            }
        }
        // Broken invariant, free_count claimed space not present in bitmap.
        panic!("run bitmap and free_count out of sync");
    }

    unsafe fn free(&mut self, ptr: NonNull<u8>) {
        let data_start = self as *mut Self as usize + self.data_offset;
        let offset = (ptr.as_ptr() as usize)
            .checked_sub(data_start)
            .expect("pointer precedes run slots");
        assert_eq!(offset % self.class_size, 0, "unaligned slot pointer");
        let index = offset / self.class_size;
        assert!(index < self.total_count, "pointer is outside run slots");

        let word_index = index / BITS_PER_WORD;
        let mask = 1usize << (index % BITS_PER_WORD);
        assert_ne!(self.bitmap[word_index] & mask, 0, "double free");
        self.bitmap[word_index] &= !mask;
        self.free_count += 1;
    }
}

fn run_for(ptr: NonNull<u8>) -> *mut RunHeader {
    let offset = (ptr.as_ptr() as usize) & (PAGE_SIZE_BYTES - 1);
    ptr.as_ptr().wrapping_sub(offset).cast::<RunHeader>()
}

struct LargeAlloc;

impl LargeAlloc {
    const fn empty() -> Self {
        Self
    }

    unsafe fn alloc(
        &mut self,
        page_source: &'static dyn PageSource,
        layout: Layout,
    ) -> Option<NonNull<u8>> {
        let pages = layout.size().div_ceil(PAGE_SIZE_BYTES);
        let align_pages = if layout.align() > PAGE_SIZE_BYTES {
            layout.align() / PAGE_SIZE_BYTES
        } else {
            1
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
        unsafe { page_source.dealloc_pages(ptr, pages) };
    }
}
