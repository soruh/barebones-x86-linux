use core::ptr::null_mut;

use crate::{sync::Mutex, syscalls};
use alloc::{boxed::Box, sync::Arc};
use syscalls::{CloneFlags, SyscallResult};

struct JoinHandleInner<T> {
    data: Mutex<Option<T>>,
    child_stack_allocation: *mut u8,
    allocated_size: usize,
}

impl<T> Drop for JoinHandleInner<T> {
    fn drop(&mut self) {
        // Drop the child's stack
        unsafe {
            syscalls::munmap(self.child_stack_allocation, self.allocated_size)
                .expect("Failed to munmap child stack")
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
pub unsafe fn spawn<T: Send + Sync, F: FnOnce() -> T + 'static>(
    f: F,
    stack_size: usize,
) -> SyscallResult<JoinHandle<T>> {
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

    let child_stack = child_stack_allocation.add(stack_size);

    let child_stack = child_stack.add(child_stack.align_offset(ALIGN));

    let inner = Arc::new(JoinHandleInner {
        data: Mutex::new(None),
        child_stack_allocation,
        allocated_size,
    });

    // We create a payload on the Heap so that we don't rely on any data on the stack after the clone
    // If we didn't to this we would just read uninitialized memory from the `child_stack`
    // We pass the pointer to this heap allocation via `r12` since it's not used by syscalls
    // neither by parameters nor clobbers

    struct Payload<T, F> {
        closure: F,
        inner: Arc<JoinHandleInner<T>>,
    }

    let payload = Box::new(Payload {
        closure: f,
        inner: inner.clone(),
    });

    let payload_ptr = Box::into_raw(payload);

    // TODO: are we certain r12 are not overwritten during the syscall?
    asm!("mov r12, {}", in(reg) payload_ptr, out("r12") _);

    syscalls::clone(
        || {
            let payload_ptr: *mut Payload<T, F>;

            asm!("mov {}, r12", out(reg) payload_ptr);

            let payload: Box<Payload<T, F>> = Box::from_raw(payload_ptr);

            // Call the provided closure
            let res = (payload.closure)();

            // Write result to return value
            *payload.inner.data.lock() = Some(res);

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
