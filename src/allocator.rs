use crate::syscalls;
use crate::{sync::*, syscalls::*};
use core::{alloc::GlobalAlloc, mem::MaybeUninit};

struct AllocatorInner {
    base: *const u8,
    brk: *const u8,
    head: *const u8,
}

unsafe impl Send for AllocatorInner {}
unsafe impl Sync for AllocatorInner {}

struct Allocator(MaybeUninit<Mutex<AllocatorInner>>);

unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.dealloc(ptr, layout)
    }
}

#[global_allocator]
static mut GLOBAL_ALLOCATOR: Allocator = Allocator(MaybeUninit::uninit());

/// *must* be called before *any* allocations are made (probably in _start)
/// *must* be called exactly once
pub unsafe fn init() -> SyscallResult<()> {
    let base = syscalls::brk(core::ptr::null())?;
    let brk = base;

    GLOBAL_ALLOCATOR = Allocator(MaybeUninit::new(FutexMutex::new(AllocatorInner {
        base,
        brk,
        head: brk,
    })));

    Ok(())
}

const BLOCK_SHIFT: usize = 12;

const BLOCK_SIZE: usize = 1 << BLOCK_SHIFT;
const BLOCK_LOWER_MASK: usize = usize::MAX >> (core::mem::size_of::<usize>() * 8 - BLOCK_SHIFT);

impl AllocatorInner {
    unsafe fn resize_brk(&mut self, offset: isize) -> SyscallResult<*const u8> {
        let old_brk = self.brk;

        self.brk = syscalls::brk(self.brk.offset(offset))?;

        Ok(old_brk)
    }

    unsafe fn alloc_blocks(&mut self, n: usize) -> SyscallResult<*const u8> {
        eprintln!("allocating {} blocks", n);

        self.resize_brk((n * BLOCK_SIZE) as isize)
    }

    fn free_capacity(&self) -> usize {
        self.brk as usize - self.head as usize
    }
}

impl Allocator {
    unsafe fn lock(&self) -> FutexMutexGuard<'_, AllocatorInner> {
        self.0.assume_init_ref().lock()
    }

    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // eprintln!("alloc: {:?}", layout);

        let size = layout.size() + layout.align();

        let old_brk = {
            let mut inner = self.lock();

            let needed_space = size.saturating_sub(inner.free_capacity());

            if needed_space > 0 {
                let mut n_block = needed_space >> BLOCK_SHIFT;

                if needed_space & BLOCK_LOWER_MASK > 0 {
                    n_block += 1;
                }

                if inner.alloc_blocks(n_block).is_err() {
                    return core::ptr::null_mut();
                }
            }

            // adjust head
            let old_head = inner.head;

            inner.head = inner.head.add(size);

            old_head
        };

        let align_offset = old_brk.align_offset(layout.align());

        old_brk.add(align_offset) as *mut u8
    }

    // TODO: do not leak all allocated memory..
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        // eprintln!("dealloc: {:?} with {:?}", ptr, layout);
    }
}
