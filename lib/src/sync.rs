//! Synchronization primitives.

use crate::cpu::InterruptGuard;
use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

/// A mutual exclusion lock that spins while waiting.
///
/// Interrupts stay disabled on the locking hart while the lock is held, so an interrupt handler
/// can't deadlock on a lock its hart already holds.
pub struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: The lock gives one hart at a time access to `T`.
unsafe impl<T: Send> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    /// Acquires the lock, spinning until it is available.
    ///
    /// # Panics
    /// Panics if [`init_current_cpu`](crate::cpu::init_current_cpu) hasn't been called on this
    /// hart.
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        let irq = InterruptGuard::new();
        while self.locked.swap(true, Ordering::Acquire) {
            while self.locked.load(Ordering::Relaxed) {
                spin_loop();
            }
        }

        SpinlockGuard {
            lock: self,
            _irq: irq,
        }
    }
}

/// Access to the data of a locked [`Spinlock`]. Unlocks on drop.
pub struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>,
    // Dropped after `Drop::drop` unlocks, restoring interrupts once the lock is free.
    _irq: InterruptGuard,
}

impl<T> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<T> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: The guard holds the lock.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: The guard holds the lock.
        unsafe { &mut *self.lock.data.get() }
    }
}
