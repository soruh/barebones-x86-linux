use core::hint::unreachable_unchecked;

const SYS_NO_READ: usize = 0;
const SYS_NO_WRITE: usize = 1;
const SYS_BRK: usize = 12;
const SYS_NO_EXIT: usize = 60;
const SYS_NO_FUTEX: usize = 202;

pub unsafe fn read(fd: u32, buf: *mut u8, count: usize) -> Result<usize, isize> {
    syscall!(SYS_NO_READ, fd, buf, count)
}

pub unsafe fn write(fd: u32, buf: *const u8, count: usize) -> Result<usize, isize> {
    syscall!(SYS_NO_WRITE, fd, buf, count)
}

pub unsafe fn brk(brk: *const u8) -> Result<*const u8, isize> {
    syscall!(SYS_BRK, brk)
}

pub unsafe fn exit(code: i32) -> ! {
    let _: usize = syscall!(SYS_NO_EXIT, code).expect("Failed to call exit");
    unreachable_unchecked()
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Timespec {
    seconds: i64,
    nano_seconds: i64,
}

pub unsafe fn futex(
    uaddr: *mut u32,
    op: i32,
    val: u32,
    utime: *mut Timespec,
    uaddr2: *mut u32,
    val3: u32,
) -> Result<u64, isize> {
    syscall!(SYS_NO_FUTEX, uaddr, op, val, utime, uaddr2, val3)
}
