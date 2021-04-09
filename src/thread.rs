use core::{
    cell::UnsafeCell,
    hint::unreachable_unchecked,
    marker::PhantomPinned,
    mem::ManuallyDrop,
    pin::Pin,
    ptr::null_mut,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    stack_protection::setup_alt_stack,
    start::RUNTIME_OPTIONS,
    syscalls::{self, futex_wait, munmap},
    tls::{setup_tls, teardown_tls, Tls},
};
use alloc::{boxed::Box, sync::Arc};
use syscalls::{helper::SyscallErrorKind, CloneArgs, CloneFlags, SyscallResult};

/// default stack size (4Mib)
pub const DEFAULT_STACK_SIZE: usize = 4 * 1024 * 1024;

struct JoinHandleInner<T> {
    data: ManuallyDrop<UnsafeCell<Option<T>>>,
    child_stack_allocation: *mut u8,
    allocated_size: usize,
    child_tid_futex: *const AtomicU32,
    _pinned: PhantomPinned,
}

impl<T> Drop for JoinHandleInner<T> {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.child_tid_futex as *mut AtomicU32);
        }
    }
}

/*
impl<T> core::fmt::Debug for JoinHandleInner<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("JoinHandleInner")
            .field("child_stack_allocation", &self.child_stack_allocation)
            .field("allocated_size", &self.allocated_size)
            .field("child_tid_futex", &self.child_tid_futex)
            .finish()
    }
}

impl<T> Drop for JoinHandleInner<T> {
    fn drop(&mut self) {
        debug!("dropping join handle inner: {:?}", self);
    }
}
*/

unsafe impl<T> Send for JoinHandle<T> where T: Send {}
unsafe impl<T> Sync for JoinHandle<T> where T: Sync {} // Do we need T: Sync here?

pub struct JoinHandle<T> {
    child_tid: u32,
    inner: Option<Pin<Arc<JoinHandleInner<T>>>>,
}

impl<T: Send + Sync> JoinHandle<T> {
    /// Get a reference to the join handle's child tid.
    pub fn tid(&self) -> u32 {
        self.child_tid
    }

    /// Returns false if the handle has already been joined
    pub fn can_join(&self) -> bool {
        self.inner.is_some()
    }

    /// Wait for the thread to finish, deallocate it's stack
    /// and return it's result
    pub fn join(&mut self) -> SyscallResult<Option<T>> {
        let inner = self
            .inner
            .take()
            .expect("Tried to join thread that was already joined");

        // dbg!(&inner.child_tid_futex as *const _);

        loop {
            if unsafe { &*inner.child_tid_futex }.load(Ordering::SeqCst) == 0 {
                // The child has exited -> return the result

                // Safety: we can take ownership of the data here since:
                // - the thread has exited (=> we have exclusive access)
                // - the data is `ManuallyDrop` so it will not be droppped twice.
                let res = unsafe { inner.data.get().read() };

                if res.is_none() {
                    // Safety: we need to free the child stack if and only if the thread did not.
                    // proposition: The thread did not free its stack
                    //
                    // We know:
                    // - the thread has exited
                    // - the thread did not return any data
                    // - freeing its stack is necessarily the last thing the thread does
                    // => for the thread to free its stack it would have had to return data
                    // => it did not return data and thus did not free its stack
                    unsafe {
                        // Free the thread's stack guard
                        crate::stack_protection::free_guard_for_stack(
                            inner.child_stack_allocation.add(inner.allocated_size),
                            inner.allocated_size,
                        )
                        .expect("Failed to free stack guard");

                        // Free the thread's stack
                        munmap(inner.child_stack_allocation, inner.allocated_size)
                            .expect("Failed to free thread stack");
                    }
                }

                break Ok(res);
            }

            let futex_var = inner.child_tid_futex as *const AtomicU32;

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
                if err.kind() == SyscallErrorKind::EAGAIN {
                    let child_tid = unsafe { &*inner.child_tid_futex }.load(Ordering::SeqCst);
                    if !(child_tid == self.child_tid || child_tid == 0 || child_tid as i32 == -1) {
                        panic!(
                            "child_tid was neither self.child_tid({}) (child is stil running),
                             0 (child has exited) nor -1 (child has not started yet), but {}.
                             THIS IS PROBABLY A MEMORY INCONSITENCY.",
                            self.child_tid, child_tid as i32
                        );
                    }
                } else {
                    panic!("Failed to wait on mutex: {}", err);
                }
            } else {
                // The futex was woken.
                // This most likely means the thread is done, but it could also
                // be spurious. We check if the thread is done either way.
            }
        }
    }
}

/// NOTE: if the thread panics after the `JoinHandle` is dropped it's stack will
/// be leaked
#[inline(never)]
pub fn spawn<T, F>(f: F, stack_size: Option<usize>) -> SyscallResult<JoinHandle<T>>
where
    T: Send + Sync + 'static,
    F: FnOnce() -> T + 'static + Unpin,
{
    unsafe {
        let stack_size = stack_size.unwrap_or(DEFAULT_STACK_SIZE);

        // This should be the same as we use with the main stack  %rsp &
        // 0xfffffffffffffff0 TODO: if we randomly SegFault increase this :))
        const ALIGN: usize = 16;

        // make sure the top of the stack is aligned
        // we need to allocate at most 2*ALIGN more, because we need to adjust both top
        // top and the bottom of the stack to ensure there are at least
        // `stack_size` of aligned stack available
        let allocated_size = stack_size + ALIGN;

        // make sure the stack is an aligned number of bytes so it's top is aligne
        // we are basically calculating by how much the current size is misaligned
        // (allocated_size % ALIGN) and then adjust the size up by ALIGN- that
        // ammount to make it aligned, but if allocated_size % ALIGN == 0 we
        // don't add anything ((ALIGN - 0) % ALIGN == 0)
        let stack_size = allocated_size + (ALIGN - allocated_size % ALIGN) % ALIGN;

        use syscalls::{MMapFlags, MProt};

        let child_stack_allocation = syscalls::mmap(
            null_mut(),
            allocated_size,
            MProt::WRITE | MProt::READ | MProt::GROWSDOWN,
            MMapFlags::ANONYMOUS | MMapFlags::PRIVATE | MMapFlags::GROWSDOWN | MMapFlags::STACK,
            -1,
            0,
        )?;

        // dbg!(child_stack_allocation);

        if RUNTIME_OPTIONS.stack_protection {
            crate::stack_protection::create_guard_for_stack(
                child_stack_allocation.add(allocated_size),
                allocated_size,
            )?;
        }

        // This should never actually do anything because mmaped memeory *should* be
        // page aligned TODO: remove once completly certain, that this is the
        // case.
        let child_stack = child_stack_allocation.add(child_stack_allocation.align_offset(ALIGN));

        let inner = Arc::pin(JoinHandleInner {
            /// # Safety: the Mutex is always pinned inside of
            /// `JoinHandleInner`s containing Arc
            data: ManuallyDrop::new(UnsafeCell::new(None)),
            child_stack_allocation,
            allocated_size,

            // used to check if the child has exited
            child_tid_futex: Box::into_raw(Box::new(AtomicU32::new(-1_i32 as u32))),
            _pinned: PhantomPinned,
        });

        // TODO: find out why if we don't do this the memory of the `JoinHandleInner`
        // sometimes stays uninitialized
        core::mem::forget(core::ptr::read_volatile(&*inner));

        // Safety: this is okay, since `inner.child_tid_futex` which we are creating a
        // reference to is
        // - atomic
        // - Pinned in memory and will live long enought due to it being inside of an
        //   `Arc::pin`
        let child_tid_futex = inner.child_tid_futex as *mut u32;

        // dbg!(child_tid_futex);

        // dbg!(&inner);

        // dbg!(&inner.child_stack_allocation as *const _);
        // dbg!(child_tid_futex);
        // asm!("int3");

        // We create a payload on the Heap so that we don't rely on any data on the
        // stack after the clone If we didn't to this we would just read
        // uninitialized memory from the `child_stack` We pass the pointer to
        // this heap allocation via `r12` since it's not used by syscalls
        // neither by parameters nor clobbers

        struct Payload<T, F> {
            closure: F,
            inner: Pin<Arc<JoinHandleInner<T>>>,
        }

        let payload = Box::new(Payload {
            closure: f,
            inner: inner.clone(),
        });

        let payload_ptr = Box::into_raw(payload);

        let child_tid = syscalls::clone3_vm_safe(
            |payload_ptr: *mut ()| -> ! {
                let (child_stack_allocation, allocated_size) = {
                    let payload: Box<Payload<T, F>> =
                        Box::from_raw(payload_ptr as *mut Payload<T, F>);

                    let child_stack_allocation = payload.inner.child_stack_allocation;
                    let allocated_size = payload.inner.allocated_size;

                    if RUNTIME_OPTIONS.segv_handling {
                        setup_alt_stack().expect("Failed to set up a signal handling stack");
                    }

                    if RUNTIME_OPTIONS.tls {
                        setup_tls(Tls {
                            stack_base: child_stack_allocation.add(allocated_size),
                            stack_limit: allocated_size,
                        })
                        .expect("Failed to setup tls");
                    }

                    // Call the provided closure
                    let res = (payload.closure)();

                    // Write result to return value
                    *payload.inner.data.get() = Some(res);

                    if RUNTIME_OPTIONS.tls {
                        teardown_tls().expect("Failed to tear down tls");
                    }

                    if RUNTIME_OPTIONS.stack_protection {
                        // Free the stack guard
                        crate::stack_protection::free_guard_for_stack(
                            child_stack_allocation.add(allocated_size),
                            allocated_size,
                        )
                        .expect("Failed to free stack guard");
                    }

                    if RUNTIME_OPTIONS.segv_handling {
                        // free the signal stack
                        crate::stack_protection::teardown_alt_stack()
                            .expect("Failed to tear down signal handling stack");
                    }

                    drop(payload.inner);

                    (child_stack_allocation, allocated_size)
                    // Drop everything on the stack before unmaping it
                };

                // **ATTENTION**: We are going to unmap our own stack!
                // after this we **must not** touch the stack (or we **will** SegFault)
                // because of this we're going to do all the syscalls by hand.

                // !!!DANGER PAST THIS POINT!!!

                // munmap our stack
                // NOTE: we currently ignore if this fails because for it to fail
                // we would need to be running in unmapped memory and would already
                // have SegFaulted...
                asm!(
                    "syscall",
                    in("rdi") child_stack_allocation,
                    in("rsi") allocated_size,
                    inlateout("rax") crate::syscalls::raw::SYS_NO_MUNMAP => _,
                    lateout("rdx") _,
                    lateout("rcx") _,
                    lateout("r11") _,
                );

                // exit(0)
                // NOTE: if we wanted to return with a code specified by the user
                // we would need to temporarily save it in a register just before munmap-ing
                // and restore it into rdi here.
                // (because otherwise we would read from the stack we just unmapped)
                asm!("syscall", in("rax") crate::syscalls::raw::SYS_NO_EXIT, in("rdi") 0);

                // exit does not return so we can't get here
                // and if we did we would SegFault on the `ret`
                unreachable_unchecked()
            },
            payload_ptr as *mut (),
            CloneArgs {
                flags: CloneFlags::IO
                    | CloneFlags::FS
                    | CloneFlags::FILES
                    | CloneFlags::PARENT
                    | CloneFlags::VM
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

        // TODO: this line previously caused pointers in the child stack to be
        // overwritten       figure out if it still does and why => fix

        // dbg!(child_tid);

        Ok(JoinHandle {
            child_tid,
            inner: Some(inner),
        })
    }
}
