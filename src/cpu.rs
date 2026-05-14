use crate::arch::interrupt::{disable_interrupts, enable_interrupts, interrupts_enabled};
use core::{
    arch::asm,
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering, fence},
};

pub const MAX_CPUS: usize = 32;
pub const HART_NONE: usize = usize::MAX;
pub const HART_INIT: usize = usize::MAX - 1;
pub static CPUS: [CpuSlot; MAX_CPUS] = [const { CpuSlot::new() }; MAX_CPUS];

pub struct CpuSlot {
    owner: AtomicUsize,
    local: CpuLocal,
}

/// Initializes a given hart's CPU-local pointer.
///
/// ## Safety
/// - `hart_id` must be a unique identifier for the current hardware hart.
/// - Must be called before `current_cpu()` can be used.
/// - Must not be called twice for the same hart.
pub unsafe fn init_current_cpu(hart_id: usize) {
    let local = slot_for_hart(hart_id);
    // We know that `slot_for_hart()` has either:
    // - observed existing owner with Acquire
    // - reset the slot and published for Release
    // The fencing here is conservative, and forces the publication before any other tp-related traffic.
    fence(Ordering::SeqCst);
    unsafe {
        write_tp(local as *const CpuLocal as usize);
    }
}

fn slot_for_hart(hart_id: usize) -> &'static CpuLocal {
    // Sentinel values can never be hart IDs
    debug_assert_ne!(hart_id, HART_NONE);
    debug_assert_ne!(hart_id, HART_INIT);

    // maybe this hart already owns a slot
    for cpu in CPUS.iter() {
        if cpu.owner.load(Ordering::Acquire) == hart_id {
            return &cpu.local;
        }
    }

    // If not, claim an unowned slot
    for cpu in CPUS.iter() {
        match cpu
            .owner
            .compare_exchange(HART_NONE, HART_INIT, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {
                // This is now the only hart that can initialize this slot
                cpu.local.reset();
                cpu.owner.store(hart_id, Ordering::Release);
                return &cpu.local;
            }

            Err(_) => {
                // Someone else claimed it
                continue;
            }
        }
    }

    panic!("no CPU slot available for hart_id={hart_id}; MAX_CPUS={MAX_CPUS}");
}

impl CpuSlot {
    pub const fn new() -> Self {
        Self {
            owner: AtomicUsize::new(HART_NONE),
            local: CpuLocal::new(),
        }
    }
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
#[inline(always)]
pub unsafe fn read_tp() -> usize {
    let value: usize;
    unsafe {
        asm!(
            "mv {out}, tp",
            out = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Sets the current thread pointer (`tp`) to a given value.
#[inline(always)]
pub unsafe fn write_tp(value: usize) {
    unsafe {
        asm!(
            "mv tp, {val}",
            val = in(reg) value,
            options(nostack, preserves_flags)
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

/// A RAII struct that automatically calls `pop_off()` when it goes out of scope.
pub struct InterruptGuard {
    local: &'static CpuLocal,
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        unsafe {
            self.local.pop_off();
        }
    }
}

impl CpuLocal {
    pub const fn new() -> Self {
        Self {
            interrupt_disable_depth: UnsafeCell::new(0),
            interrupts_were_enabled: UnsafeCell::new(false),
        }
    }

    fn reset(&self) {
        // Safe because caller has claimed the slot by setting owner from HART_NONE -> HART_INIT
        unsafe {
            *self.interrupt_disable_depth.get() = 0;
            *self.interrupts_were_enabled.get() = false;
        }
    }

    pub unsafe fn push_off_guard(&'static self) -> InterruptGuard {
        unsafe {
            self.push_off();
        }

        InterruptGuard { local: self }
    }

    unsafe fn push_off(&self) {
        let enabled = interrupts_enabled();

        unsafe {
            disable_interrupts();

            let depth = self.interrupt_disable_depth.get();
            let old = *depth;

            if old == 0 {
                *self.interrupts_were_enabled.get() = enabled;
            }

            *depth = old
                .checked_add(1)
                .expect("interrupt_disable_depth overflow");
        }
    }

    unsafe fn pop_off(&self) {
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
