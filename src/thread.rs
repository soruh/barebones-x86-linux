use core::{
    cell::UnsafeCell,
    hint::unreachable_unchecked,
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

pub struct JoinHandle<T> {
    child_tid: u32,
    inner: Pin<Arc<JoinHandleInner<T>>>,
}

impl<T: Send + Sync> JoinHandle<T> {
    /// Wait for the thread to finish, deallocate it's stack
    /// and return it's result
    pub fn join(self) -> SyscallResult<T> {
        loop {
            if self.inner.child_tid_futex.load(Ordering::Relaxed) == 0 {
                // The child has exited -> return the result

                // Safety: we can take ownership of the data here since:
                // - the thread has exited (=> we have exclusive access)
                // - the data is `ManuallyDrop` so it will not be droppped twice.
                unsafe {
                    break Ok(self
                        .inner
                        .data
                        .get()
                        .read()
                        .expect("Child thread did not return data (it probably panicked)"));
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
}

//// # Safety: the provided stack size must be big enough
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
        /// # Safety: the Mutex is always pinned inside of `JoinHandleInner`s containing Arc
        data: ManuallyDrop::new(UnsafeCell::new(None)),
        child_stack_allocation,
        allocated_size,

        // used to check if the child has exited
        child_tid_futex: u32::MAX.into(),
        _pinned: PhantomPinned,
    });

    // Safety: this is okay, since `inner.child_tid_futex` which we are creating a reference to is
    // - atomic
    // - is Pinned in memory and will live long enought due to it being inside of a `Arc::pin`
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

    let payload_ptr = Box::into_raw(payload);

    let child_tid = syscalls::clone3_vm_safe(
        |payload_ptr: *mut ()| -> ! {
            let (child_stack_allocation, allocated_size) = {
                let payload: Box<Payload<T, F>> = Box::from_raw(payload_ptr as *mut Payload<T, F>);

                // Call the provided closure
                let res = (payload.closure)();

                // Write result to return value
                *payload.inner.data.get() = Some(res);

                (
                    payload.inner.child_stack_allocation,
                    payload.inner.allocated_size,
                )
            };

            // ATTENTION: We're going to unmap our own stack !
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

    // TODO: this line previously caused pointers in the child stack to be overwritten
    //       figure out if it still does and why => fix

    // dbg!(child_tid);

    Ok(JoinHandle { child_tid, inner })
}
