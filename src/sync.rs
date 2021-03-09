use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

use crate::syscalls::{futex_wait, futex_wake, FutexFlags};

pub struct SpinMutex<T> {
    is_locked: AtomicBool,
    data: T,
}

impl<T: Send + Sync> SpinMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            is_locked: false.into(),
            data,
        }
    }

    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        // This can probably be `Aquire`
        // spin until we can take the lock
        while self
            .is_locked
            .compare_exchange_weak(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            core::hint::spin_loop();
        }

        SpinMutexGuard {
            mutex: self as *const SpinMutex<T> as *mut SpinMutex<T>,
            _phantom: Default::default(),
        }
    }
}

pub struct SpinMutexGuard<'d, T> {
    mutex: *mut SpinMutex<T>,
    _phantom: PhantomData<&'d mut T>,
}

impl<'d, T> Deref for SpinMutexGuard<'d, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.mutex).data }
    }
}

impl<'d, T> DerefMut for SpinMutexGuard<'d, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.mutex).data }
    }
}

impl<'d, T> Drop for SpinMutexGuard<'d, T> {
    fn drop(&mut self) {
        unsafe { &(*self.mutex).is_locked }.store(false, Ordering::Release);
    }
}

pub struct Mutex<T> {
    is_locked: AtomicU32,
    data: UnsafeCell<T>,
}

impl<T: Send + Sync> Mutex<T> {
    pub fn new(data: T) -> Self {
        Mutex {
            is_locked: 0.into(),
            data: UnsafeCell::new(data),
        }
    }

    /// Lock the Mutex
    pub fn lock(&self) -> MutexGuard<'_, T> {
        // TODO: benchmark this
        const N_SPIN: usize = 100;

        let mutex_var = &self.is_locked;

        'outer: loop {
            for _ in 0..N_SPIN {
                // TODO: at least one of these Orderings can probably be `Aquire`
                if mutex_var
                    .compare_exchange_weak(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break 'outer;
                }

                core::hint::spin_loop();
            }

            let mutex_var = mutex_var as *const AtomicU32;

            // Try to wait on the futex
            let res = unsafe { futex_wait(mutex_var as *mut u32, 1, None, FutexFlags::empty()) };

            if let Err(err) = res {
                if err.0 != 11 {
                    panic!("Failed to wait on mutex: {}", err);
                } else {
                    // The Lock was unlocked while before we could wait on it.
                    // Try to aquire it.
                }
            } else {
                // We finished waiting on the Futex.
                // Try to aquire the lock.
            }
        }

        MutexGuard {
            mutex_var: mutex_var as *const AtomicU32,
            data: self.data.get(),
            _phantom: Default::default(),
        }
    }

    /// Wait until someone else locks the mutex at least once
    /// If the lock is already locked reutrn immediately
    /// returns if we actually waited
    pub fn wait(&self) -> bool {
        let mutex_var = &self.is_locked as *const AtomicU32 as *mut u32;

        // Wait until the futex is locked
        let res = unsafe { futex_wait(mutex_var, 0, None, FutexFlags::empty()) };

        if let Err(err) = res {
            if err.0 != 11 {
                panic!("Failed to wait on mutex: {}", err);
            } else {
                // The Lock was already locked

                false
            }
        } else {
            // the lock was locked at least once
            // Since `wake` is only called on guard drop and only wakes one
            // waiter an arbitrary amount (>=1) of locks may have accured

            // Since we didn't take the guard we need to wake a new waiter.
            unsafe {
                futex_wake(mutex_var, Some(1)).expect("Failed to wake futex");
            }

            true
        }
    }
}

pub struct MutexGuard<'d, T> {
    mutex_var: *const AtomicU32,
    data: *mut T,
    _phantom: PhantomData<&'d mut T>,
}

impl<'d, T> MutexGuard<'d, T> {
    /// consume the guard, returning the value and permanantly locking the mutex
    /// TODO: is this function safe?
    /// What about `Pin`? Do we need to require `Unpin`?
    pub fn consume(self) -> T {
        let res = unsafe { self.data.read() };

        core::mem::forget(self);

        res
    }
}

impl<'d, T> Deref for MutexGuard<'d, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'d, T> DerefMut for MutexGuard<'d, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<'d, T> Drop for MutexGuard<'d, T> {
    fn drop(&mut self) {
        unsafe {
            // Unlock lock
            (&*self.mutex_var).store(0, Ordering::Release);

            // Wake up one waiting thread
            futex_wake(self.mutex_var as *mut u32, Some(1)).expect("Failed to wake futex");
        }
    }
}
