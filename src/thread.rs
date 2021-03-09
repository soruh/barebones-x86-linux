use core::mem::MaybeUninit;
use core::{fmt::Debug, ptr::null_mut};

use crate::{sync::FutexMutex, syscalls};
use alloc::sync::Arc;
use syscalls::{CloneFlags, SyscallResult};

struct JoinHandleInner<T> {
    data: FutexMutex<Option<T>>,
    child_stack: *mut u8,
    stack_size: usize,
}

impl<T> Drop for JoinHandleInner<T> {
    fn drop(&mut self) {
        unsafe {
            let child_stack = self.child_stack as *mut usize;

            let child_stack = child_stack.sub(self.stack_size / core::mem::size_of::<usize>());

            alloc::vec::Vec::<usize>::from_raw_parts(
                child_stack,
                0,
                self.stack_size / core::mem::size_of::<usize>(),
            )
        };
    }
}

pub struct JoinHandle<T>(Arc<JoinHandleInner<T>>);

impl<T: Send + Sync> JoinHandle<T> {
    pub fn join(&self) -> T {
        loop {
            {
                let lock = self.0.data.lock();

                if lock.is_some() {
                    break lock.consume().unwrap();
                }
            }

            self.0.data.wait();
        }
    }
}

/// Safety: the provided stack size must be big enough
pub unsafe fn spawn<T: Send + Sync>(
    f: impl Fn() -> T + 'static,
    stack_size: usize,
) -> SyscallResult<JoinHandle<T>> {
    let child_stack =
        alloc::vec::Vec::<usize>::with_capacity(stack_size / core::mem::size_of::<usize>())
            .as_mut_ptr();

    let child_stack = child_stack.add(stack_size / core::mem::size_of::<usize>());

    let child_stack = child_stack as *mut u8;

    let inner = Arc::new(JoinHandleInner {
        data: FutexMutex::new(None),
        child_stack,
        stack_size,
    });

    let ptr = Arc::as_ptr(&inner);

    // TODO: are we certain r12 can not be overwritten during the syscall?
    asm!("mov r12, {}", in(reg) ptr, out("r12") _);

    syscalls::clone(
        || {
            let ptr: *const JoinHandleInner<T>;

            asm!("mov {}, r12", out(reg) ptr);

            let inner: Arc<JoinHandleInner<T>> = Arc::from_raw(ptr);

            let res = f();

            *inner.data.lock() = Some(res);

            0
        },
        CloneFlags::VM | CloneFlags::IO,
        child_stack,
        null_mut(),
        null_mut(),
        0,
    )?;

    Ok(JoinHandle(inner))
}
