#![no_main]
#![no_std]

mod io;
mod sbi;

use crate::sbi::{
    base::{get_impl_id, get_impl_ver, get_march_id, get_mimpid, get_spec_version, get_vendor_id},
    reset::{ResetReason, ResetType, system_reset},
};
use core::{arch::global_asm, panic::PanicInfo, ptr::addr_of_mut};

unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

global_asm!(include_str!("asm/boot.s"));

#[unsafe(no_mangle)]
pub extern "C" fn entry() -> ! {
    clear_bss();
    let color = 34;
    println!("\u{1B}[{}m[Boot Complete]\u{1B}[0m", color);
    let spec_ver = get_spec_version();
    println!(
        "Specification Version: {}.{}\nVendor ID: {}\nSBI Implementation ID: {}\nSBI Implementation Version: {}\nMachine Architecture ID: {}\nMachine Implementation ID: {}\n",
        spec_ver.0,
        spec_ver.1,
        get_vendor_id(),
        get_impl_id(),
        get_impl_ver(),
        get_march_id(),
        get_mimpid()
    );

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
