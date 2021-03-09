use super::helper::SyscallResult;
use bitflags::bitflags;
use core::hint::unreachable_unchecked;

pub const SYS_NO_READ: usize = 0;
pub const SYS_NO_WRITE: usize = 1;
pub const SYS_NO_MMAP: usize = 9;
pub const SYS_NO_MUNMAP: usize = 11;
pub const SYS_NO_BRK: usize = 12;
pub const SYS_NO_NANOSLEEP: usize = 35;
pub const SYS_NO_CLONE: usize = 56;
pub const SYS_NO_FORK: usize = 57;
pub const SYS_NO_EXIT: usize = 60;
pub const SYS_NO_WAIT4: usize = 61;
pub const SYS_NO_FUTEX: usize = 202;
pub const SYS_NO_WAITID: usize = 247;

pub unsafe fn read(fd: u32, buf: *mut u8, count: usize) -> SyscallResult<usize> {
    syscall!(SYS_NO_READ, fd, buf, count)
}

pub unsafe fn write(fd: u32, buf: *const u8, count: usize) -> SyscallResult<usize> {
    syscall!(SYS_NO_WRITE, fd, buf, count)
}

bitflags! {
    pub struct MProt: u64 {
        const NONE = 0;
        const READ = 1;
        const WRITE = 2;
        const EXEC = 4;
        const GROWSDOWN = 0x01000000;
        const GROWSUP = 0x02000000;
    }
}

bitflags! {
    pub struct MMapFlags: u64 {
        const SHARED = 0x01;
        const PRIVATE = 0x02;
        const SHARED_VALIDATE = 0x03;
        const TYPE = 0x0f;
        const FIXED = 0x10;
        const ANONYMOUS = 0x20;
        const NORESERVE = 0x4000;
        const GROWSDOWN = 0x0100;
        const DENYWRITE = 0x0800;
        const EXECUTABLE = 0x1000;
        const LOCKED = 0x2000;
        const POPULATE = 0x8000;
        const NONBLOCK = 0x10000;
        const STACK = 0x20000;
        const HUGETLB = 0x40000;
        const SYNC = 0x80000;
        const FIXED_NOREPLACE = 0x100000;
    }
}

pub unsafe fn mmap(
    addr: *mut u8,
    len: usize,
    prot: MProt,
    flags: MMapFlags,
    fd: u64,
    offset: u64,
) -> SyscallResult<*mut u8> {
    syscall!(
        SYS_NO_MMAP,
        addr,
        len,
        prot.bits(),
        flags.bits(),
        fd,
        offset
    )
}

pub unsafe fn munmap(addr: *mut u8, len: usize) -> SyscallResult<usize> {
    syscall!(SYS_NO_MUNMAP, addr, len)
}

pub unsafe fn brk(brk: *const u8) -> SyscallResult<*const u8> {
    syscall!(SYS_NO_BRK, brk)
}

bitflags! {
    pub struct CloneFlags: u64 {
        /// set if VM shared between processes
        const VM = 0x00000100;
        /// set if fs info shared between processes
        const FS = 0x00000200;
        /// set if open files shared between processes
        const FILES = 0x00000400;
        /// set if signal handlers and blocked signals shared
        const SIGHAND = 0x00000800;
        /// set if a pidfd should be placed in parent
        const PIDFD = 0x00001000;
        /// set if we want to let tracing continue on the child too
        const PTRACE = 0x00002000;
        /// set if the parent wants the child to wake it up on mm_release
        const VFORK = 0x00004000;
        /// set if we want to have the same parent as the cloner
        const PARENT = 0x00008000;
        /// Same thread group?
        const THREAD = 0x00010000;
        /// New mount namespace group
        const NEWNS = 0x00020000;
        /// share system V SEM_UNDO semantics
        const SYSVSEM = 0x00040000;
        /// create a new TLS for the child
        const SETTLS = 0x00080000;
        /// set the TID in the parent
        const PARENT_SETTID = 0x00100000;
        /// clear the TID in the child
        const CHILD_CLEARTID = 0x00200000;
        /// Unused, ignored
        const DETACHED = 0x00400000;
        /// set if the tracing process can't force CLONE_PTRACE on this clone
        const UNTRACED = 0x00800000;
        /// set the TID in the child
        const CHILD_SETTID = 0x01000000;
        /// New cgroup namespace
        const NEWCGROUP = 0x02000000;
        /// New utsname namespace
        const NEWUTS = 0x04000000;
        /// New ipc namespace
        const NEWIPC = 0x08000000;
        /// New user namespace
        const NEWUSER = 0x10000000;
        /// New pid namespace
        const NEWPID = 0x20000000;
        /// New network namespace
        const NEWNET = 0x40000000;
        /// Clone io context
        const IO = 0x80000000;
        /// Clear any signal handler and reset to SIG_DFL.
        const CLEAR_SIGHAND = 0x100000000;
        /// Clone into a specific cgroup given the right permissions.
        const INTO_CGROUP = 0x200000000;
        /// New time namespace
        const NEWTIME = 0x00000080;
    }
}

#[inline(always)]
pub unsafe fn clone(
    clone_flags: CloneFlags,
    child_stack: *mut u8,
    parent_tid: *mut (),
    child_tid: *mut (),
    tls: u32,
) -> isize {
    syscall!(RAW
        SYS_NO_CLONE,
        clone_flags.bits(),
        child_stack,
        parent_tid,
        child_tid,
        tls
    )
}

pub unsafe fn nanosleep(rqtp: *const Timespec, rmtp: *mut Timespec) -> SyscallResult<usize> {
    syscall!(SYS_NO_NANOSLEEP, rqtp, rmtp)
}

pub unsafe fn fork() -> SyscallResult<u32> {
    syscall!(SYS_NO_FORK)
}

pub unsafe fn exit(code: i32) -> ! {
    let _: usize = syscall!(SYS_NO_EXIT, code).expect("Failed to call exit");
    unreachable_unchecked()
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Rusage {
    /// user time used
    ru_utime: Timespec,
    /// system time used
    ru_stime: Timespec,
    /// maximum resident set size
    ru_maxrss: i64,
    /// integral shared memory size
    ru_ixrss: i64,
    /// integral unshared data size
    ru_idrss: i64,
    /// integral unshared stack size
    ru_isrss: i64,
    /// page reclaims
    ru_minflt: i64,
    /// page faults
    ru_majflt: i64,
    /// swaps
    ru_nswap: i64,
    /// block input operations
    ru_inblock: i64,
    /// block output operations
    ru_oublock: i64,
    /// messages sent
    ru_msgsnd: i64,
    /// messages received
    ru_msgrcv: i64,
    /// signals received
    ru_nsignals: i64,
    /// voluntary context switches
    ru_nvcsw: i64,
    /// involuntary
    ru_nivcsw: i64,
}

pub unsafe fn wait4(
    upid: u32,
    stat_addr: *mut i32,
    options: i32,
    ru: *mut Rusage,
) -> SyscallResult<u32> {
    syscall!(SYS_NO_WAIT4, upid, stat_addr, options, ru)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Timespec {
    seconds: i64,
    nano_seconds: i64,
}
impl Timespec {
    pub fn new(seconds: i64, nano_seconds: i64) -> Self {
        Self {
            seconds,
            nano_seconds,
        }
    }
}

pub unsafe fn futex(
    uaddr: *mut u32,
    op: i32,
    val: u32,
    utime: *mut Timespec,
    uaddr2: *mut u32,
    val3: u32,
) -> SyscallResult<u64> {
    syscall!(SYS_NO_FUTEX, uaddr, op, val, utime, uaddr2, val3)
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum IdType {
    All = 0,
    Pid = 1,
    Gid = 2,
    PidFd = 3,
}

bitflags::bitflags! {
    pub struct WaitIdOption: u32 {
        const NOHANG = 1;
        const UNTRACED = 2;
        const STOPPED = 2;
        const EXITED = 4;
        const CONTINUED = 8;
        const NOWAIT = 0x1000000;
    }
}

pub unsafe fn waitid(
    which: IdType,
    upid: u32,
    infop: *mut (),
    options: WaitIdOption,
    ru: *mut Rusage,
) -> SyscallResult<u64> {
    syscall!(SYS_NO_WAITID, which, upid, infop, options.bits(), ru)
}
