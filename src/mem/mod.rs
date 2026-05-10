pub mod kalloc;
use alloc::boxed::Box;

pub const PAGE_SIZE_BYTES: usize = 4096;

pub trait PageAllocator: Sized {
    unsafe fn new_zeroed() -> usize {
        let boxed_page = unsafe { Box::<Self>::new_zeroed().assume_init() };
        let ptr = Box::into_raw(boxed_page) as usize;
        ptr
    }
}
