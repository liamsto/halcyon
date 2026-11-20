//! Per-hart state.

use core::{
    arch::asm,
    cell::Cell,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

pub(crate) const MAX_CPUS: usize = 32;

const NO_HART: usize = usize::MAX;
const SSTATUS_SIE: usize = 1 << 1;

static CPUS: [CpuSlot; MAX_CPUS] = [const { CpuSlot::new() }; MAX_CPUS];

struct CpuSlot {
    owner: AtomicUsize,
    local: CpuLocal,
}

impl CpuSlot {
    const fn new() -> Self {
        Self {
            owner: AtomicUsize::new(NO_HART),
            local: CpuLocal {
                interrupt_disable_depth: Cell::new(0),
                interrupts_were_enabled: Cell::new(false),
            },
        }
    }
}

struct CpuLocal {
    interrupt_disable_depth: Cell<usize>,
    interrupts_were_enabled: Cell<bool>,
}

// SAFETY: A `CpuLocal` is only reached through `tp` on the hart that claimed its slot, and no two
// harts ever share one slot.
unsafe impl Sync for CpuLocal {}

/// Binds the current hart to its per-hart state.
///
/// Must run on each hart before it locks a [`Spinlock`](crate::sync::Spinlock) or allocates.
///
/// # Safety
/// - Boot code must zero `tp` before entering Rust.
/// - Must be called at most once per hart.
///
/// # Panics
/// Panics if more than `MAX_CPUS` harts call this.
pub unsafe fn init_current_cpu(hart_id: usize) {
    assert_ne!(
        hart_id, NO_HART,
        "hart ID collides with the free slot sentinel"
    );

    for slot in &CPUS {
        // Relaxed: only the uniqueness of the claim matters, and the slot's state is never written
        // before it is claimed.
        if slot
            .owner
            .compare_exchange(NO_HART, hart_id, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            // SAFETY: Only Rust code in this module reads `tp`, and it expects a `CpuLocal`
            // pointer. This hart owns `slot`.
            unsafe {
                asm!(
                    "mv tp, {}",
                    in(reg) &raw const slot.local,
                    options(nostack, preserves_flags),
                );
            }
            return;
        }
    }

    panic!("no CPU slot available for hart {hart_id}; MAX_CPUS={MAX_CPUS}");
}

/// Returns the current hart's state.
fn current_cpu() -> &'static CpuLocal {
    let ptr: *const CpuLocal;
    // SAFETY: Reading `tp` has no side effects.
    unsafe {
        asm!("mv {}, tp", out(reg) ptr, options(nomem, nostack, preserves_flags));
    }
    assert!(!ptr.is_null(), "init_current_cpu not called on this hart");

    // SAFETY: `tp` is zero until `init_current_cpu` points it at a `CpuLocal` in `CPUS`.
    unsafe { &*ptr }
}

/// Stops the current hart forever.
pub fn halt() -> ! {
    loop {
        // SAFETY: `wfi` only pauses the hart until an interrupt is pending.
        unsafe { asm!("wfi", options(nomem, nostack)) };
    }
}

fn interrupts_enabled() -> bool {
    let sstatus: usize;
    // SAFETY: Reading `sstatus` has no side effects.
    unsafe {
        asm!("csrr {}, sstatus", out(reg) sstatus, options(nomem, nostack));
    }
    sstatus & SSTATUS_SIE != 0
}

// No `nomem` on the next two because they need to act as compiler barriers so memory accesses in a critical
// section don't get moved outside it.

fn disable_interrupts() {
    // SAFETY: Clearing `sstatus.SIE` only masks interrupts.
    unsafe { asm!("csrci sstatus, 2", options(nostack)) };
}

/// # Safety
/// The trap vector must be ready to handle interrupts.
unsafe fn enable_interrupts() {
    // SAFETY: The caller guarantees interrupts can be handled.
    unsafe { asm!("csrsi sstatus, 2", options(nostack)) };
}

/// Keeps interrupts disabled on the current hart while alive.
///
/// Guards nest, dropping the outermost guard restores the interrupt state from before it was
/// created.
pub(crate) struct InterruptGuard {
    local: &'static CpuLocal,
    // Must drop on the hart that created it.
    _not_send: PhantomData<*mut ()>,
}

impl InterruptGuard {
    pub(crate) fn new() -> Self {
        let local = current_cpu();
        let enabled = interrupts_enabled();
        disable_interrupts();

        let depth = local.interrupt_disable_depth.get();
        if depth == 0 {
            local.interrupts_were_enabled.set(enabled);
        }
        local.interrupt_disable_depth.set(depth + 1);

        Self {
            local,
            _not_send: PhantomData,
        }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        let depth = self.local.interrupt_disable_depth.get();
        debug_assert!(
            depth > 0,
            "InterruptGuard dropped without matching creation"
        );
        debug_assert!(
            !interrupts_enabled(),
            "interrupts enabled while an InterruptGuard is alive"
        );

        self.local.interrupt_disable_depth.set(depth - 1);
        if depth == 1 && self.local.interrupts_were_enabled.get() {
            // SAFETY: Interrupts were enabled before the outermost guard, so they can be handled.
            unsafe { enable_interrupts() };
        }
    }
}
