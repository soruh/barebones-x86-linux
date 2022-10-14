#[macro_use]
pub mod helper;
pub mod raw;
use core::{ptr::null_mut, sync::atomic::AtomicU32, arch::asm};

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

/// # Safety:
//
// - r12 needs to contain a f: `unsafe fn(*mut ()) -> !`
// - r13 needs to contain a user_data: *mut () that is valid as an argument to f
// - realigns the stack
unsafe extern "C" fn clone_callback() -> ! {
    // NOTE: We are a thread that was just spawned

    // retore data the parent saved for us
    let f: unsafe fn(*mut ()) -> !;
    asm!("mov {}, r12", out(reg) f);

    let user_data: *mut ();
    asm!("mov {}, r13", out(reg) user_data);

    f(user_data)
}

#[naked]
/// # Safety: arguments to clone3 are expected to already be in rdi and rsi
// NOTE:
unsafe extern "C" fn clone_proxy() -> isize {
    asm!(
        "mov rax, {}",
        "syscall",
        // This is the point where exection splits.
        // We use the fact that the thread has a different stack to return
        // to its handler if we are the thread (the value at the top of its stack),
        // or to the clone function if we are the parent
        "ret",
        const crate::syscalls::raw::SYS_NO_CLONE3,
        options(noreturn)
    )
}

/// sets the supplied stack up so that the clone will call `f` with the supplied
/// `user_data` and then executes the clone3 syscall with the supplied
/// `clone_args`.
///
/// # Safety:
/// - CloneArgs::VM must be set
/// - arguments must be valid for a call to `clone`
#[inline(never)]
pub unsafe fn clone3_vm_safe(
    f: unsafe fn(*mut ()) -> !,
    user_data: *mut (),
    clone_args: CloneArgs,
) -> SyscallResult<u32> {
    let stack_top = clone_args.stack.add(clone_args.stack_size) as *mut usize;

    // Write the address the thread should jump to to it's stack
    *stack_top = clone_callback as *const () as usize;

    // save thread handler to r12 and user data to r13

    asm!(
        "mov r12, {}",
        "mov r13, {}",
        in(reg) f,
        in(reg) user_data,
    );

    // write `clone3` arguments to rdi and rsi as expected by `clone_proxy`
    asm!(
        "mov rdi, {}",
        "mov rsi, {}",
        in(reg) &clone_args as *const _,
        in(reg) core::mem::size_of::<CloneArgs>(),
    );

    // Clone and direct thread to registered handler
    let res: isize = clone_proxy();

    // NOTE: we are the parent

    if res < 0 {
        Err(SyscallError((-res) as u32))
    } else {
        debug_assert_ne!(res, 0);

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
        Err(SyscallError(-res as u32))
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
