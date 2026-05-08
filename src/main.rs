#![no_main]
#![no_std]

mod io;
mod sbi;

use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
    ptr::addr_of_mut,
};

use crate::sbi::{console::write, error::SbiError};

unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

global_asm!(include_str!("asm/boot.s"));

#[unsafe(no_mangle)]
pub extern "C" fn entry() -> ! {
    clear_bss();

    match write("Hello from RISC-V and the SBI!\n") {
        Ok(_) => {}
        Err(e) => match e {
            SbiError::Denied => puts("Access to debug console denied."),
            SbiError::Failed => puts("An I/O error occured while accessing the debug console."),
            SbiError::InvalidParam => puts("A bad parameter was passed to sbi_call."),
            _ => puts("?"),
        },
    }

    loop {
        unsafe {
            asm!("wfi");
        }
    }
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

fn putchar(ch: u8) {
    const UART0: *mut u8 = 0x1000_0000 as *mut u8;

    unsafe {
        UART0.write_volatile(ch);
    }
}

fn puts(s: &str) {
    for b in s.bytes() {
        putchar(b);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    puts("kernel panic\n");

    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
