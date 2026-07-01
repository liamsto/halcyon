#![no_main]
#![no_std]

extern crate alloc;

use core::{
    arch::global_asm,
    hint::spin_loop,
    panic::PanicInfo,
    ptr::addr_of_mut,
    sync::atomic::{AtomicUsize, Ordering},
};
use lib::{
    cpu::{HART_NONE, init_current_cpu},
    dtb, println,
    sbi::{
        base::{
            get_impl_id, get_impl_ver, get_march_id, get_mimpid, get_spec_version, get_vendor_id,
        },
        reset::{ResetReason, ResetType, system_reset},
    },
};

const BOOT_NOT_READY: usize = 0;
const BOOT_READY: usize = 1;

#[unsafe(link_section = ".data.boot")]
static BOOT_HART: AtomicUsize = AtomicUsize::new(HART_NONE);
#[unsafe(link_section = ".data.boot")]
static BOOT_STATE: AtomicUsize = AtomicUsize::new(BOOT_NOT_READY);

unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

global_asm!(include_str!("asm/boot.s"));

#[unsafe(no_mangle)]
pub extern "C" fn entry(hart_id: usize, opaque: usize) -> ! {
    let boot = is_boot_hart(hart_id);

    if boot {
        clear_bss();

        unsafe { init_current_cpu(hart_id) };

        /*
         * TODO:
         * global initialization here before we release any secondaries
         * - initialize global alloc
         * - initialize trap vector
         * - initialize page tables
         * - initialize interrupt controller/timer
         * - discover harts
         */
        let boot_info = unsafe { dtb::parse(opaque) }.unwrap_or_else(|err| {
            panic!("failed to parse DTB at {opaque:#x}: {err:?}");
        });

        BOOT_STATE.store(BOOT_READY, Ordering::Release);

        kmain(hart_id, &boot_info)
    } else {
        while BOOT_STATE.load(Ordering::Acquire) != BOOT_READY {
            spin_loop();
        }

        unsafe { init_current_cpu(hart_id) };

        secondary_main(hart_id, opaque)
    }
}

fn kmain(hart_id: usize, boot_info: &dtb::Info<'_>) -> ! {
    // do kernel stuff
    println!("\u{1B}[{}m[Halcyon - Boot Complete]\u{1B}[0m", 32);
    println!("Boot hart: {}", hart_id);

    if let Some(model) = boot_info.model {
        println!("Device Tree Model: {}", model);
    }

    println!("DTB boot CPU ID: {}", boot_info.boot_cpuid);
    println!("Detected harts: {}", boot_info.harts().len());
    for hart in boot_info.harts() {
        println!("  hart {}", hart);
    }

    println!("Memory ranges: {}", boot_info.mem().len());
    for range in boot_info.mem() {
        println!(
            "  {:#x}..{:#x}",
            range.base,
            range.base.saturating_add(range.size)
        );
    }

    println!("Memreserve ranges: {}", boot_info.memreserve().len());
    for range in boot_info.memreserve() {
        println!(
            "  {:#x}..{:#x}",
            range.base,
            range.base.saturating_add(range.size)
        );
    }

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
    match BOOT_HART.compare_exchange(HART_NONE, hart_id, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => true,
        Err(existing) => existing == hart_id,
    }
}

fn secondary_main(_hart_id: usize, _opaque: usize) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
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
