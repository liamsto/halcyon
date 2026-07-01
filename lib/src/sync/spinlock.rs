use crate::cpu::InterruptGuard;
use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Spinlock<T: ?Sized> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct SpinlockGuard<'a, T: ?Sized> {
    lock: &'a Spinlock<T>,
    // Interrupts must be held in a disabled state for the lifetime of the lock guard
    _irq_guard: InterruptGuard,
    // Prevents sending the guard cross-thread
    _not_send: PhantomData<*mut ()>,
}

impl<T> Spinlock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
        // Rust drops the fields of SpinlockGuard, then that drops
        // `_irq_guard`, and in turn that Drop impl calls pop_off().
    }
}

impl<T: ?Sized> Spinlock<T> {
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        let irq_guard = unsafe { crate::cpu::current_cpu().push_off_guard() };

        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.locked.load(Ordering::Relaxed) {
                spin_loop();
            }
        }

        SpinlockGuard {
            lock: self,
            _irq_guard: irq_guard,
            _not_send: PhantomData,
        }
    }

    pub fn try_lock(&self) -> Option<SpinlockGuard<'_, T>> {
        let irq_guard = unsafe { crate::cpu::current_cpu().push_off_guard() };

        let acquired = self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok();

        if acquired {
            Some(SpinlockGuard {
                lock: self,
                _irq_guard: irq_guard,
                _not_send: PhantomData,
            })
        } else {
            // irq_guard drops here automatically, and interrupt state is restored
            None
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

impl<T: ?Sized> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

// Lock gives sync shared access to T
unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}
