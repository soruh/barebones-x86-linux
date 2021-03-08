use crate::syscalls;
use crate::{sync::*, syscalls::Timespec};
use core::{alloc::GlobalAlloc, mem::MaybeUninit};

struct AllocatorInner {
    base: *const u8,
    brk: *const u8,
}

unsafe impl Send for AllocatorInner {}
unsafe impl Sync for AllocatorInner {}

struct Allocator(MaybeUninit<FutexMutex<AllocatorInner>>);

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
pub unsafe fn init() -> Result<(), isize> {
    let base = syscalls::brk(core::ptr::null())?;
    let brk = base;

    GLOBAL_ALLOCATOR = Allocator(MaybeUninit::new(FutexMutex::new(AllocatorInner {
        base,
        brk,
    })));

    Ok(())
}

impl Allocator {
    unsafe fn lock(&self) -> FutexMutexGuard<'_, AllocatorInner> {
        self.0.assume_init_ref().lock()
    }

    unsafe fn resize_brk(&self, offset: isize) -> Result<*const u8, isize> {
        let mut inner = self.lock();

        let old_brk = inner.brk;

        inner.brk = syscalls::brk(inner.brk.offset(offset))?;

        Ok(old_brk)
    }

    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        eprintln!("alloc: {:?}", layout);

        let size = layout.size() + layout.align();

        let old_brk = if let Ok(old_brk) = self.resize_brk(size as isize) {
            old_brk
        } else {
            return core::ptr::null_mut();
        };

        let align_offset = old_brk.align_offset(layout.align());

        old_brk.add(align_offset) as *mut u8
    }

    // TODO: do not leak all allocated memory..
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        eprintln!("dealloc: {:?} with {:?}", ptr, layout);
    }
}
