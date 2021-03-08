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
        let res = unsafe { &(*self.mutex).is_locked }.compare_exchange(
            true,
            false,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );

        assert_eq!(res, Ok(true))
    }
}

pub struct FutexMutex<T> {
    is_locked: AtomicU32,
    data: UnsafeCell<T>,
}

impl<T: Send + Sync> FutexMutex<T> {
    pub fn new(data: T) -> Self {
        FutexMutex {
            is_locked: 0.into(),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> FutexMutexGuard<'_, T> {
        // TODO: benchmark this
        const N_SPIN: usize = 100;

        let mutex_var = &self.is_locked;

        'outer: loop {
            for _ in 0..N_SPIN {
                if mutex_var
                    .compare_exchange_weak(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break 'outer;
                }

                core::hint::spin_loop();
            }

            let mutex_var = mutex_var as *const AtomicU32;

            eprintln!("waiting on futex");

            // Try to wait on the futex
            let res = unsafe { futex_wait(mutex_var as *mut u32, 1, None, FutexFlags::empty()) };

            if let Err(err) = res {
                if err != -11 {
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

        eprintln!("aquired lock");

        FutexMutexGuard {
            mutex_var: mutex_var as *const AtomicU32,
            data: self.data.get(),
            _phantom: Default::default(),
        }
    }
}

pub struct FutexMutexGuard<'d, T> {
    mutex_var: *const AtomicU32,
    data: *mut T,
    _phantom: PhantomData<&'d mut T>,
}

impl<'d, T> Deref for FutexMutexGuard<'d, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'d, T> DerefMut for FutexMutexGuard<'d, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<'d, T> Drop for FutexMutexGuard<'d, T> {
    fn drop(&mut self) {
        unsafe {
            eprintln!("releasing lock");

            // Unlock lock
            (&*self.mutex_var).store(0, Ordering::SeqCst);

            // Wake up one waiting thread
            futex_wake(self.mutex_var as *mut u32, Some(1)).expect("Failed to wake futex");
        }
    }
}
