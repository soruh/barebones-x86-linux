use core::{
    cell::UnsafeCell,
    marker::PhantomPinned,
    mem::ManuallyDrop,
    pin::Pin,
    ptr::null_mut,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::syscalls::{self, futex_wait};
use alloc::{boxed::Box, sync::Arc};
use syscalls::{CloneArgs, CloneFlags, SyscallResult};

struct JoinHandleInner<T> {
    data: ManuallyDrop<UnsafeCell<Option<T>>>,
    child_stack_allocation: *mut u8,
    allocated_size: usize,
    child_tid_futex: AtomicU32,
    _pinned: PhantomPinned,
}

unsafe impl<T> Send for JoinHandle<T> where T: Send {}
unsafe impl<T> Sync for JoinHandle<T> where T: Sync {} // Do we need T: Sync here?

impl<T> Drop for JoinHandleInner<T> {
    fn drop(&mut self) {
        // debug!("dropping JoinHandleInner for stack at {:?}", self.child_stack_allocation);

        // Drop the child's stack
        unsafe {
            syscalls::munmap(self.child_stack_allocation, self.allocated_size)
                .expect("Failed to munmap child stack")
        };
    }
}

pub struct JoinHandle<T> {
    child_tid: u32,
    inner: Pin<Arc<JoinHandleInner<T>>>,
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        // TODO: It's pretty ugly that we have to `clone` here,
        // is there a better way?

        // Safety: we are not moving inner out of the Arc,
        // but are only reading the Arc's strong count.
        let strong_count = unsafe {
            let arc = Pin::into_inner_unchecked(self.inner.clone());
            Arc::strong_count(&arc)
        };

        // dbg!(strong_count);

        if strong_count > 2 {
            // NOTE: we need to do this, since we need to free the threads stack
            // TODO: is there a way for the thread to free it's own thread?
            panic!("dropped a JoinHandle while the thread was still running. Either `join` or `leak` it. (forgeting it leaks its stack.)");
        }
    }
}

impl<T: Send + Sync> JoinHandle<T> {
    /// Wait for the thread to finish, deallocate it's stack
    /// and return it's result
    pub fn join(self) -> SyscallResult<T> {
        loop {
            if self.inner.child_tid_futex.load(Ordering::Relaxed) == 0 {
                // Safety:
                // The child has exited
                // return the result
                // Safety: we can take ownership of the data here, since:
                // - the thread has exited (=> we have exclusive access)
                // - the data is `ManuallyDrop` so it will not be droppped twice.
                unsafe {
                    break Ok(self
                        .inner
                        .data
                        .get()
                        .read()
                        .expect("Child thread did not return the expected data"));
                }
            }

            let futex_var = &self.inner.child_tid_futex as *const AtomicU32;

            // Try to wait on the futex
            let res = unsafe {
                futex_wait(
                    futex_var,
                    self.child_tid,
                    None,
                    crate::syscalls::FutexFlags::empty(),
                )
            };

            if let Err(err) = res {
                if err.0 != 11 {
                    panic!("Failed to wait on mutex: {}", err);
                } else {
                    // The value at the futex was not the child_tid
                    // It was probably 0, but it could also be u32::MAX,
                    // because the child thread did not yet write it's tid there
                    // either way, we need to try again.
                }
            } else {
                // The futex was woken.
                // This most likely means the thread is done, but it could also
                // be spurious. We check if the thread is done either way.
            }
        }
    }

    /// drops the JoinHandle to the thread.
    /// NOTE: leaks the memory allocated for the thread stack!
    pub fn leak(self) {
        core::mem::forget(self);
    }
}

/// Safety: the provided stack size must be big enough
pub unsafe fn spawn<T, F>(f: F, stack_size: usize) -> SyscallResult<JoinHandle<T>>
where
    T: Send + Sync + 'static,
    F: FnOnce() -> T + 'static + Unpin,
{
    // This should be the same as we use with the main stack  %rsp & 0xfffffffffffffff0
    // TODO: if we randomly SegFault increase this :))
    const ALIGN: usize = 16;

    let allocated_size = stack_size + ALIGN;

    use syscalls::{MMapFlags, MProt};

    let child_stack_allocation = syscalls::mmap(
        null_mut(),
        allocated_size,
        MProt::WRITE | MProt::READ | MProt::GROWSDOWN,
        MMapFlags::ANONYMOUS | MMapFlags::PRIVATE | MMapFlags::GROWSDOWN | MMapFlags::STACK,
        0,
        0,
    )?;

    // This should never actually do anything because mmaped memeory *should* be page aligned
    // TODO: remove once completly certain, that this is the case.
    let child_stack = child_stack_allocation.add(child_stack_allocation.align_offset(ALIGN));

    let inner = Arc::pin(JoinHandleInner {
        // Safety: the Mutex is always pinned inside of `JoinHandleInner`s containing Arc
        data: ManuallyDrop::new(UnsafeCell::new(None)),
        child_stack_allocation,
        allocated_size,

        // used to check if the child has exited
        child_tid_futex: u32::MAX.into(),
        _pinned: PhantomPinned,
    });

    // Safety: this is okay, since `inner.child_tid_futex` which we are creating a reference to is
    // A: atomic and B: is Pined in memory and will live long enought due to it being inside of a `Arc::pin`
    let child_tid_futex = &inner.child_tid_futex as *const AtomicU32 as *mut u32;

    // We create a payload on the Heap so that we don't rely on any data on the stack after the clone
    // If we didn't to this we would just read uninitialized memory from the `child_stack`
    // We pass the pointer to this heap allocation via `r12` since it's not used by syscalls
    // neither by parameters nor clobbers

    struct Payload<T, F> {
        closure: F,
        inner: Pin<Arc<JoinHandleInner<T>>>,
    }

    let payload = Box::new(Payload {
        closure: f,
        inner: inner.clone(),
    });

    // uncomment and add pointer to CloneArgs to provide child tid in child memory
    // let child_tid_ptr = (&mut payload.child_tid) as *mut u32;

    let payload_ptr = Box::into_raw(payload);

    // Store thread owned data in `r12`
    asm!("mov r12, {}", in(reg) payload_ptr, out("r12") _);

    let child_tid = syscalls::clone3(
        || {
            let payload_ptr: *mut Payload<T, F>;

            // Restore thread owned data from `r12`
            asm!("mov {}, r12", out(reg) payload_ptr);

            let payload: Box<Payload<T, F>> = Box::from_raw(payload_ptr);

            // Call the provided closure
            let res = (payload.closure)();

            // Write result to return value
            *payload.inner.data.get() = Some(res);

            0
        },
        CloneArgs {
            flags: CloneFlags::VM
                | CloneFlags::IO
                | CloneFlags::THREAD
                | CloneFlags::SIGHAND
                | CloneFlags::CHILD_SETTID
                | CloneFlags::CHILD_CLEARTID,
            pidfd: 0,
            child_tid: child_tid_futex,
            parent_tid: null_mut(),
            exit_signal: 0,
            stack: child_stack,
            stack_size,
            tls: null_mut(),
            set_tid: null_mut(),
            set_tid_size: 0,
            cgroup: 0,
        },
    )?;

    // TODO: this line previously caused pointers in the child stack to be overwritten
    //       figure out if it still does and why => fix

    // dbg!(child_tid);

    Ok(JoinHandle { child_tid, inner })
}
