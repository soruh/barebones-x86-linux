// TODO: write safe wrappers here?
#[macro_use]
mod helper;
mod raw;
use bitflags::bitflags;
pub use raw::*;

pub fn write_str(fd: u32, s: &str) -> Result<usize, isize> {
    unsafe { write(fd, s.as_ptr(), s.len()) }
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

pub unsafe fn futex_wait(
    uaddr: *mut u32,
    val: u32,
    time: Option<&mut Timespec>,
    flags: FutexFlags,
) -> Result<u64, isize> {
    let op = FutexOp::Wait as i32 | flags.bits();

    let utime = time
        .map(|r| r as *mut Timespec)
        .unwrap_or(core::ptr::null_mut());

    raw::futex(uaddr, op, val, utime, core::ptr::null_mut(), 0)
}

pub unsafe fn futex_wake(uaddr: *mut u32, n: Option<u32>) -> Result<u64, isize> {
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
