use crate::arch::interrupt::{disable_interrupts, enable_interrupts, interrupts_enabled};
use core::{arch::asm, cell::UnsafeCell, sync::atomic::AtomicUsize};

pub const MAX_CPUS: usize = 32;
pub const HART_NONE: usize = usize::MAX;
pub const HART_INIT: usize = usize::MAX - 1;
pub static CPUS: [CpuSlot; MAX_CPUS] = [CpuSlot::new(); MAX_CPUS];

pub struct CpuSlot {
    owner: AtomicUsize,
    local: CpuLocal,
}

impl CpuSlot {
    pub const fn new() -> Self {
        Self {
            owner: AtomicUsize::new(HART_NONE),
            local: CpuLocal::new(),
        }
    }

    fn slot_for_hart(hart_id: usize) -> &'static CpuLocal {}
}

pub struct CpuLocal {
    interrupt_disable_depth: UnsafeCell<usize>,
    interrupts_were_enabled: UnsafeCell<bool>,
}

// Safety invariants:
// one hart is the only mutator of its local
// no two harts ever share one slot.
unsafe impl Sync for CpuLocal {}

/// Initializes a hart.
pub fn init_cpu(ptr: *const CpuLocal) {
    unsafe {
        asm!(
            "mv tp, {src}",
            src = in(reg) ptr,
            options(nostack, nomem)
        );
    }
}

/// Read the current thread pointer (`tp`).
pub unsafe fn read_tp() -> usize {
    let tp: usize;
    unsafe {
        asm!(
            "mv {out}, tp",
            out = out(reg) tp,
            options(nomem, nostack)
        );
    }

    tp
}

/// Sets the current thread pointer (`tp`) to a given value.
pub unsafe fn set_tp(value: usize) {
    unsafe {
        asm!(
            "mv tp, {val}",
            val = in(reg) value,
            options(nomem, nostack)
        )
    }
}

// Returns a  `CpuLocal` for the current hart.
#[inline(always)]
pub unsafe fn current_cpu() -> &'static CpuLocal {
    unsafe {
        let ptr = read_tp() as *const CpuLocal;
        debug_assert!(!ptr.is_null(), "thread pointer was null");
        debug_assert_eq!((ptr as usize) & (core::mem::align_of::<CpuLocal>() - 1), 0);
        &*ptr
    }
}

impl CpuLocal {
    pub const fn new() -> Self {
        Self {
            interrupt_disable_depth: UnsafeCell::new(0),
            interrupts_were_enabled: UnsafeCell::new(false),
        }
    }

    pub unsafe fn push_off(&self) {
        let enabled = interrupts_enabled();
        unsafe {
            let depth = self.interrupt_disable_depth.get();
            disable_interrupts();
            let old = *depth;
            if old == 0 {
                *self.interrupts_were_enabled.get() = enabled;
            }

            *depth + old + 1;
        }
    }

    pub unsafe fn pop_off(&self) {
        unsafe {
            let depth = self.interrupt_disable_depth.get();
            debug_assert!(*depth > 0, "pop_off called without matching push_off");
            debug_assert!(
                !interrupts_enabled(),
                "interrupts enabled in push_off section"
            );

            *depth -= 1;
            if *depth == 0 && *self.interrupts_were_enabled.get() {
                enable_interrupts();
            }
        }
    }
}
