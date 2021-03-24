#[macro_use]
pub mod helper;
pub mod raw;
use core::{ptr::null_mut, sync::atomic::AtomicU32};

pub use raw::*;

pub use helper::{SyscallError, SyscallResult};

use crate::{ffi::CStr, io::Fd};

// TODO: do these need to be `unsafe`?
pub fn read(fd: u32, buf: &mut [u8]) -> SyscallResult<usize> {
    unsafe { raw::read(fd, buf.as_mut_ptr(), buf.len()) }
}

pub fn write(fd: u32, buf: &[u8]) -> SyscallResult<usize> {
    unsafe { raw::write(fd, buf.as_ptr(), buf.len()) }
}

bitflags::bitflags! {
    pub struct OpenFlags: i32 {
        const CREAT = 0o100;
        const EXCL = 0o200;
        const NOCTTY = 0o400;
        const TRUNC = 0o1000;
        const APPEND = 0o2000;
        const NONBLOCK = 0o4000;
        const DSYNC = 0o10000;
        const SYNC = 0o4010000;
        const RSYNC = 0o4010000;
        const DIRECTORY = 0o200000;
        const NOFOLLOW = 0o400000;
        const CLOEXEC = 0o2000000;
        const ASYNC = 0o20000;
        const DIRECT = 0o40000;
        const LARGEFILE = 0o100000;
        const NOATIME = 0o1000000;
        const PATH = 0o10000000;
        const TMPFILE = 0o20200000;
    }
}

bitflags::bitflags! {
    pub struct OpenMode: i32 {
        const RDONLY = 0o0;
        const WRONLY = 0o1;
        const RDWR = 0o2;
    }
}

pub fn open(filename: impl AsRef<CStr>, flags: OpenFlags, mode: OpenMode) -> SyscallResult<Fd> {
    let filename = filename.as_ref();
    unsafe { raw::open(filename.as_ptr(), flags.bits(), mode.bits()).map(|x| Fd(x as u32)) }
}

pub fn close(fd: Fd) -> SyscallResult<()> {
    unsafe { raw::close(fd.0).map(|_| ()) }
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

#[allow(clippy::comparison_chain)]
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

#[allow(clippy::comparison_chain)]
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
