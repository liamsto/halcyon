use core::{
    alloc::{GlobalAlloc, Layout},
    mem::MaybeUninit,
};

pub static KERNEL_ALLOC: MaybeUninit<KernelAllocator> = MaybeUninit::uninit(); // this may change

pub struct KernelAllocator;

impl KernelAllocator {
    pub fn init() -> Self {
        todo!()
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}
