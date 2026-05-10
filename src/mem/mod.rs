pub mod alloc;

const PAGE_SIZE: usize = 4096;

pub trait PageAllocator: Sized {
    // unsafe fn new_zeroed() -> usize {
    //     let boxed_page = Box::<Self>::new_zeroed().assume_init();
    //     let ptr = Box::into_raw(boxed_page) as usize;
    //     ptr
    // }
}
