use core::arch::asm;

pub mod base;
pub mod dbg_console;
pub mod error;
pub mod reset;
pub mod timer;

#[derive(Debug, Clone, Copy)]
pub struct SbiRet {
    pub error: isize,
    pub value: isize,
}

fn sbi_call(extension: usize, function: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
    let error: isize;
    let value: isize;

    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 as isize => error,
            inlateout("a1") arg1 as isize => value,
            in("a2") arg2,
            in("a6") function,
            in("a7") extension,
        );
    }

    SbiRet { error, value }
}
