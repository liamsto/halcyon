#![no_main]
#![no_std]

mod io;
mod sbi;

use crate::sbi::reset::{ResetReason, ResetType, system_reset};
use core::{arch::global_asm, panic::PanicInfo, ptr::addr_of_mut};

unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

global_asm!(include_str!("asm/boot.s"));

#[unsafe(no_mangle)]
pub extern "C" fn entry() -> ! {
    clear_bss();

    println!("Hello from RISC-V and the SBI!");
    let _ = system_reset(ResetType::SHUTDOWN, Some(ResetReason::NO_REASON));
    unreachable!()
}

fn clear_bss() {
    unsafe {
        let mut bss = addr_of_mut!(__bss_start);
        let end = addr_of_mut!(__bss_end);

        while bss < end {
            bss.write_volatile(0);
            bss = bss.add(1);
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(loc) = info.location() {
        println!(
            "panic in file {}, line {}: {}",
            loc.file(),
            loc.line(),
            info.message()
        )
    }
    let _ = system_reset(ResetType::SHUTDOWN, Some(ResetReason::SYSTEM_FAILURE));
    unreachable!() // this pattern is annoying
}
