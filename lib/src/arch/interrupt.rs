use core::arch::asm;

const SSTATUS_SIE: usize = 1 << 1;

/// `true` if supervisor interrupts are currently enabled.
#[inline]
pub fn interrupts_enabled() -> bool {
    let sstatus: usize;

    unsafe {
        asm!(
            "csrr {0}, sstatus",
            out(reg) sstatus,
            options(nomem, nostack),
        );
    }

    (sstatus & SSTATUS_SIE) != 0
}

/// Disable supervisor interrupts on the current hart.
#[inline]
pub unsafe fn disable_interrupts() {
    unsafe {
        asm!(
            "csrci sstatus, 2", // clear bit 1: SIE
            options(nomem, nostack),
        );
    }
}

/// Enable supervisor interrupts on the current hart.
#[inline]
pub unsafe fn enable_interrupts() {
    unsafe {
        asm!(
            "csrsi sstatus, 2", // set bit 1: SIE
            options(nomem, nostack),
        );
    }
}
