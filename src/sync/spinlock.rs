use crate::cpu::current_cpu;
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

impl<T: ?Sized> Spinlock<T> {
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        unsafe {
            current_cpu().push_off();
        }

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
            _not_send: PhantomData,
        }
    }

    pub fn try_lock(&self) -> Option<SpinlockGuard<'_, T>> {
        unsafe {
            crate::cpu::current_cpu().push_off();
        }

        let acquired = self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok();

        if acquired {
            Some(SpinlockGuard {
                lock: self,
                _not_send: PhantomData,
            })
        } else {
            unsafe {
                crate::cpu::current_cpu().pop_off();
            }

            None
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);

        unsafe {
            current_cpu().pop_off();
        }
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

impl<T: ?Sized> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

// Lock gives sync shared access to T
unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}

// Moving lock between owners is ok if T is Send
unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}
