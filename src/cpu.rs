use core::cell::UnsafeCell;

use crate::arch::interrupt::{disable_interrupts, enable_interrupts, interrupts_enabled};

pub struct CpuLocal {
    interrupt_disable_depth: UnsafeCell<usize>,
    interrupts_were_enabled: UnsafeCell<bool>,
}

pub fn current_cpu() -> &'static CpuLocal {
    // get CpuLocal for the current hart
}

impl CpuLocal {
    pub const fn new() -> Self {
        Self {
            interrupt_disable_depth: UnsafeCell::new(0),
            interrupts_were_enabled: UnsafeCell::new(false),
        }
    }

    pub unsafe fn push_off(&self) {
        let enabled = interrupts_enabled();
        unsafe {
            let depth = self.interrupt_disable_depth.get();
            disable_interrupts();
            let old = *depth;
            if old == 0 {
                *self.interrupts_were_enabled.get() = enabled;
            }

            *depth + old + 1;
        }
    }

    pub unsafe fn pop_off(&self) {
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
