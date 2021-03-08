use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::Ordering,
};

// TODO: implement using syscalls
pub struct BadMutex<T> {
    is_locked: core::sync::atomic::AtomicBool,
    data: T,
}

impl<T: Send + Sync> BadMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            is_locked: false.into(),
            data,
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        // This can probably be `Aquire`
        // spin until we can take the lock
        while self
            .is_locked
            .compare_exchange_weak(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            core::hint::spin_loop();
        }

        MutexGuard {
            mutex: self as *const BadMutex<T> as *mut BadMutex<T>,
            _phantom: Default::default(),
        }
    }
}

pub struct MutexGuard<'d, T> {
    mutex: *mut BadMutex<T>,
    _phantom: PhantomData<&'d mut T>,
}

impl<'d, T> Deref for MutexGuard<'d, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.mutex).data }
    }
}

impl<'d, T> DerefMut for MutexGuard<'d, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.mutex).data }
    }
}

impl<'d, T> Drop for MutexGuard<'d, T> {
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
