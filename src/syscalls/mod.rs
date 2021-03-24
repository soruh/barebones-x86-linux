#[macro_use]
pub mod helper;
pub mod raw;
use core::{ptr::null_mut, sync::atomic::AtomicU32};

pub use raw::*;

pub use helper::{SyscallError, SyscallResult};

// TODO: do these need to be `unsafe`?
pub unsafe fn read(fd: u32, buf: &mut [u8]) -> SyscallResult<usize> {
    raw::read(fd, buf.as_mut_ptr(), buf.len())
}

pub unsafe fn write(fd: u32, buf: &[u8]) -> SyscallResult<usize> {
    raw::write(fd, buf.as_ptr(), buf.len())
}

#[repr(i32)]
pub enum FutexOp {
    Wait = 0,
    Wake = 1,
    Fd = 2,
    Requeue = 3,
    CmpRequeue = 4,
    WakeOp = 5,
    LockPi = 6,
    UnlockPi = 7,
    TrylockPi = 8,
    WaitBitset = 9,
    WakeBitset = 10,
    WaitRequeuePi = 11,
    CmpRequeuePi = 12,
}

bitflags::bitflags! {
    pub struct FutexFlags: i32 {
        const PRIVATE_FLAG = 128;
        const CLOCK_REALTIME = 256;
    }
}

type FutexVar = *const AtomicU32;

pub unsafe fn futex_wait(
    uaddr: FutexVar,
    val: u32,
    time: Option<&mut Timespec>,
    flags: FutexFlags,
) -> SyscallResult<u64> {
    let op = FutexOp::Wait as i32 | flags.bits();

    let utime = time
        .map(|r| r as *mut Timespec)
        .unwrap_or(core::ptr::null_mut());

    raw::futex(uaddr, op, val, utime, core::ptr::null_mut(), 0)
}

pub unsafe fn futex_wake(uaddr: FutexVar, n: Option<u32>) -> SyscallResult<u64> {
    let op = FutexOp::Wake as i32;

    let val = n.unwrap_or(u32::MAX);

    raw::futex(
        uaddr,
        op,
        val,
        core::ptr::null_mut(),
        core::ptr::null_mut(),
        0,
    )
}

#[inline(never)]
pub unsafe fn clone3(f: unsafe fn() -> i32, args: CloneArgs) -> SyscallResult<u32> {
    // asm!("int3");

    asm!("mov r13, {}", in(reg) f, lateout("r13") _);

    let res = raw::clone3(&args as *const _, core::mem::size_of::<CloneArgs>());
    if res == 0 {
        let f: unsafe fn() -> i32;

        asm!("mov {}, r13", out(reg) f);

        let res = f();

        exit(res);
    } else if res < 0 {
        Err(SyscallError(-res as usize))
    } else {
        Ok(res as u32)
    }
}

pub unsafe fn clone(
    f: unsafe fn() -> i32,
    flags: CloneFlags,
    stack: *mut u8,
    parent_tid: *mut u32,
    child_tid: *mut u32,
    thread_local: *mut (),
) -> SyscallResult<u32> {
    asm!("mov r13, {}", in(reg) f, lateout("r13") _);

    let res = raw::clone(flags, stack, parent_tid, child_tid, thread_local);
    if res == 0 {
        let f: unsafe fn() -> i32;

        asm!("mov {}, r13", out(reg) f);

        exit(f());
    } else if res < 0 {
        Err(SyscallError(-res as usize))
    } else {
        Ok(res as u32)
    }
}

pub fn sleep(duration: core::time::Duration) -> SyscallResult<()> {
    unsafe {
        nanosleep(
            &Timespec::new(duration.as_secs() as i64, duration.subsec_nanos() as i64) as *const _,
            null_mut(),
        )?;
    }

    Ok(())
}
