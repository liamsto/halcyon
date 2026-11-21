//! Trap handling.

use core::{
    arch::{asm, global_asm},
    fmt,
    mem::{offset_of, size_of},
};

const SCAUSE_INTERRUPT: usize = 1 << (usize::BITS - 1);

/// ABI names of `x0` through `x31`.
const REG_NAMES: [&str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
    "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
    "t5", "t6",
];

global_asm!(
    include_str!("asm/trap.s"),
    FRAME_SIZE = const size_of::<TrapFrame>(),
    FRAME_SSTATUS = const offset_of!(TrapFrame, sstatus),
    FRAME_SEPC = const offset_of!(TrapFrame, sepc),
    handler = sym handle,
);

unsafe extern "C" {
    fn trap_entry();
}

/// State of the interrupted code, saved on its stack by `trap_entry`.
#[repr(C)]
struct TrapFrame {
    /// `regs[n]` holds `xn`.
    regs: [usize; 32],
    sstatus: usize,
    sepc: usize,
}

// `trap_entry` stores `xn` at `8 * n` and must keep `sp` 16-byte aligned for the call to `handle`.
const _: () = assert!(
    offset_of!(TrapFrame, regs) == 0
        && size_of::<usize>() == 8
        && size_of::<TrapFrame>().is_multiple_of(16)
);

impl fmt::Display for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sepc={:#018x} sstatus={:#018x}", self.sepc, self.sstatus)?;

        // `x0` is always zero.
        let regs = REG_NAMES.iter().zip(&self.regs).skip(1);
        for (i, (name, value)) in regs.enumerate() {
            f.write_str(if i % 4 == 0 { "\n" } else { "  " })?;
            write!(f, "{name:>3}={value:#018x}")?;
        }
        Ok(())
    }
}

/// Installs the trap vector on the current hart.
///
/// `stvec` is per-hart, so every hart must call this. Until it does, a trap on that hart jumps to
/// whatever address the firmware left behind.
///
/// Traps are handled on the stack of the code they interrupt.
pub fn init() {
    let entry = (trap_entry as *const ()).addr();

    // SAFETY: `trap_entry` handles every trap taken from S-mode. It's 4-byte aligned, which
    // leaves the mode bits of `stvec` as zeros and picks direct mode.
    unsafe {
        asm!("csrw stvec, {}", in(reg) entry, options(nomem, nostack));
    }
}

/// Handles a trap. Returning resumes the interrupted code with the state in `frame`.
extern "C" fn handle(frame: &TrapFrame) {
    let scause: usize;
    let stval: usize;
    // SAFETY: Reading `scause` and `stval` has no side effects.
    unsafe {
        asm!("csrr {}, scause", out(reg) scause, options(nomem, nostack));
        asm!("csrr {}, stval", out(reg) stval, options(nomem, nostack));
    }

    let code = scause & !SCAUSE_INTERRUPT;
    if scause & SCAUSE_INTERRUPT != 0 {
        // Nothing enables an interrupt source yet.
        panic!("unexpected interrupt {code}\n{frame}");
    }

    panic!(
        "{} (exception {code}), stval={stval:#x}\n{frame}",
        exception_name(code)
    );
}

fn exception_name(code: usize) -> &'static str {
    match code {
        0 => "instruction address misaligned",
        1 => "instruction access fault",
        2 => "illegal instruction",
        3 => "breakpoint",
        4 => "load address misaligned",
        5 => "load access fault",
        6 => "store address misaligned",
        7 => "store access fault",
        8 => "environment call from U-mode",
        12 => "instruction page fault",
        13 => "load page fault",
        15 => "store page fault",
        _ => "unknown exception",
    }
}
