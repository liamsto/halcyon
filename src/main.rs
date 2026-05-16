#![no_main]
#![no_std]

extern crate alloc;

mod arch;
mod cpu;
mod io;
mod mem;
mod sbi;
mod sync;

use crate::{
    cpu::init_current_cpu,
    sbi::{
        base::{
            get_impl_id, get_impl_ver, get_march_id, get_mimpid, get_spec_version, get_vendor_id,
        },
        reset::{ResetReason, ResetType, system_reset},
    },
};
use core::{arch::global_asm, panic::PanicInfo, ptr::addr_of_mut};

unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

global_asm!(include_str!("asm/boot.s"));

#[unsafe(no_mangle)]
pub extern "C" fn entry(hart_id: usize, opaque: usize) -> ! {
    if is_boot_hart(hart_id) {
        clear_bss();
    }
    // TODO
    // we might need to wait on the boot hart to finish, depending on how asm goes...
    // if all harts get dropped into __start at the same time, will need to check an atomic
    // to avoid entering kmain with the system in an uninitialized state
    unsafe { init_current_cpu(hart_id) };
    kmain(hart_id, opaque)
}

fn kmain(hart_id: usize, opaque: usize) -> ! {
    // do kernel stuff
    println!("\u{1B}[{}m[Halcyon - Boot Complete]\u{1B}[0m", 32);
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

// is this the boot hart? If not, we shouldn't clear BSS.
fn is_boot_hart(hart_id: usize) -> bool {
    todo!()
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
