#![allow(clippy::upper_case_acronyms)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use crate::io::Fd;

use super::{helper::SyscallResult, SyscallError};
use bitflags::bitflags;
use core::{hint::unreachable_unchecked, sync::atomic::AtomicU32};

pub const SYS_NO_READ: usize = 0;
pub const SYS_NO_WRITE: usize = 1;
pub const SYS_NO_OPEN: usize = 2;
pub const SYS_NO_CLOSE: usize = 3;
pub const SYS_NO_MMAP: usize = 9;
pub const SYS_NO_MUNMAP: usize = 11;
pub const SYS_NO_BRK: usize = 12;
pub const SYS_NO_RT_SIGACTION: usize = 13;
pub const SYS_NO_NANOSLEEP: usize = 35;
pub const SYS_NO_CLONE: usize = 56;
pub const SYS_NO_FORK: usize = 57;
pub const SYS_NO_EXIT: usize = 60;
pub const SYS_NO_WAIT4: usize = 61;
pub const SYS_NO_GETRLIMIT: usize = 97;
pub const SYS_NO_SIGALTSTACK: usize = 131;
pub const SYS_NO_ARCH_PTRCTL: usize = 158;
pub const SYS_NO_SETRLIMIT: usize = 160;
pub const SYS_NO_GETTID: usize = 186;
pub const SYS_NO_FUTEX: usize = 202;
pub const SYS_NO_WAITID: usize = 247;
pub const SYS_NO_CLONE3: usize = 435;

pub unsafe fn read(fd: u32, buf: *mut u8, count: usize) -> SyscallResult<usize> {
    syscall!(SYS_NO_READ, fd, buf, count)
}

pub unsafe fn write(fd: u32, buf: *const u8, count: usize) -> SyscallResult<usize> {
    syscall!(SYS_NO_WRITE, fd, buf, count)
}

pub unsafe fn open(filename: *const u8, flags: i32, mode: i32) -> SyscallResult<usize> {
    syscall!(SYS_NO_OPEN, filename, flags, mode)
}
pub unsafe fn close(fd: u32) -> SyscallResult<usize> {
    syscall!(SYS_NO_CLOSE, fd)
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
    fd: i32,
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
    pub struct CloneFlags: usize {
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
    parent_tid: *mut u32,
    child_tid: *mut u32,
    tls: *mut (),
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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CloneArgs {
    /// Flags bit mask
    pub flags: CloneFlags,
    /// Where to store PID file descriptor (pid_t *)
    pub pidfd: usize,
    /// Where to store child TID, in child's memory (pid_t *)
    pub child_tid: *mut u32,
    /// Where to store child TID, in parent's memory (int *)
    pub parent_tid: *mut u32,
    /// Signal to deliver to parent on child termination
    pub exit_signal: usize,
    /// Pointer to lowest byte of stack
    pub stack: *mut u8,
    /// Size of stack
    pub stack_size: usize,
    /// Location of new TLS
    pub tls: *mut (),
    /// Pointer to a pid_t array (since Linux 5.5)
    pub set_tid: *mut u32,
    /// Number of elements in set_tid (since Linux 5.5)
    pub set_tid_size: usize,
    /// File descriptor for target cgroup of child (since Linux 5.7)
    pub cgroup: usize,
}

#[inline(always)]
pub unsafe fn clone3(cl_args: *const CloneArgs, size: usize) -> isize {
    syscall!(RAW
        SYS_NO_CLONE3,
        cl_args,
        size
    )
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Signal(i32);

impl core::fmt::Debug for Signal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Signal({:?})", self.kind())
    }
}

impl Signal {
    pub fn kind(self) -> SignalKind {
        let x = self.0;

        unsafe {
            if (1..=SignalKind::SYS as i32).contains(&x) {
                core::mem::transmute(x)
            } else {
                SignalKind::Unknown
            }
        }
    }
}

impl From<SignalKind> for Signal {
    fn from(kind: SignalKind) -> Self {
        Signal(kind as i32)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum SignalKind {
    HUP = 1,
    INT = 2,
    QUIT = 3,
    ILL = 4,
    TRAP = 5,
    ABRT = 6,
    BUS = 7,
    FPE = 8,
    KILL = 9,
    USR1 = 10,
    SEGV = 11,
    USR2 = 12,
    PIPE = 13,
    ALRM = 14,
    TERM = 15,
    STKFLT = 16,
    CHLD = 17,
    CONT = 18,
    STOP = 19,
    TSTP = 20,
    TTIN = 21,
    TTOU = 22,
    URG = 23,
    XCPU = 24,
    XFSZ = 25,
    VTALRM = 26,
    PROF = 27,
    WINCH = 28,
    POLL = 29,
    PWR = 30,
    SYS = 31,
    Unknown,
}

type UContext = ();
/*
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Sigset([u64; 128 / core::mem::size_of::<u64>()]);
pub struct MContext {
    gregs: gregset_t,
    fpregs: fpregset_t,
    __reserved1: [64; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UContext {
    flags: u64,
    link: *mut UContext,
    stack: SignalStack,
    mcontext: MContext,
    sigmask: Sigset,
    __fpregs_mem: [u64; 64],
}
*/

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct SignalHandler(*const ());

impl Default for SignalHandler {
    fn default() -> Self {
        Self::default_handler()
    }
}

impl SignalHandler {
    pub fn default_handler() -> Self {
        Self(0 as *const ())
    }
    pub fn ignore() -> Self {
        Self(1 as *const ())
    }

    pub fn handler(handler: unsafe extern "C" fn(Signal, *mut SignalInfo, *mut UContext)) -> Self {
        Self(handler as *const ())
    }

    // TODO: write methods for different use cases
}

bitflags! {
    #[derive(Default)]
    pub struct SigactionFlags: u64 {
        const NOCLDSTOP  = 1;
        const NOCLDWAIT  = 2;
        const SIGINFO    = 4;
        const ONSTACK    = 0x08000000;
        const RESTART    = 0x10000000;
        const NODEFER    = 0x40000000;
        const RESETHAND  = 0x80000000;
        const RESTORER   = 0x04000000;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Sigaction {
    pub handler: SignalHandler,
    pub flags: SigactionFlags,
    pub restorer: SignalHandler,
    pub mask: u64,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SignalInfoCode(i32);

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegvCode {
    MAPERR = 1,
    ACCERR = 2,
    Unknown,
}

impl SignalInfoCode {
    pub fn segv(self) -> SegvCode {
        match self.0 {
            1 => SegvCode::MAPERR,
            2 => SegvCode::ACCERR,
            _ => SegvCode::Unknown,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SignalInfo {
    pub signo: Signal,
    pub errno: SyscallError,
    pub code: SignalInfoCode,
    pub inner: SignalInfoInner,
}

impl core::fmt::Debug for SignalInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let code = match self.signo.kind() {
            SignalKind::SEGV => format!("{:?}", self.code.segv()),
            _ => format!("{:?}", self.code),
        };
        f.debug_struct("SignalInfo")
            .field("signo", &self.signo)
            .field("errno", &self.errno)
            .field("code", &code)
            .field("inner", &"[union]")
            .finish()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PidUid {
    pub pid: i32,
    pub uid: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SignalInfoInnerCommonTimer {
    pub timerid: i32,
    pub overrun: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union SignalInfoInnerCommonFirst {
    pub piduid: PidUid,
    pub timer: SignalInfoInnerCommonTimer,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SigChld {
    pub status: i32,
    pub utime: i64,
    pub stime: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union SignalInfoInnerCommonSecond {
    pub value: usize,
    pub sigchld: SigChld,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SignalInfoInnerCommon {
    pub first: SignalInfoInnerCommonFirst,
    pub second: SignalInfoInnerCommonSecond,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AddrBnd {
    pub lower: *const (),
    pub upper: *const (),
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union SignalInfoInnerSigFaultFirst {
    pub addr_bnd: AddrBnd,
    pub pkey: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SignalInfoInnerSigFault {
    pub addr: *mut (),
    pub addr_lsb: u16,
    pub first: SignalInfoInnerSigFaultFirst,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SignalInfoInnerSigPoll {
    pub band: i64,
    pub fd: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SignalInfoInnerSigSys {
    pub call_addr: *mut (),
    pub syscall: i32,
    pub arch: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union SignalInfoInner {
    pub __pad: [u8; 128 - 2 * core::mem::size_of::<u32>() - core::mem::size_of::<u64>()],
    pub common: SignalInfoInnerCommon,
    pub sig_fault: SignalInfoInnerSigFault,
    pub sig_poll: SignalInfoInnerSigPoll,
    pub sig_sys: SignalInfoInnerSigSys,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct _SignalInfo {
    /// Signal number
    pub signo: Signal,

    /// An errno value
    pub errno: i32,

    /// Signal code
    pub code: i32,

    /// Trap number that caused hardware-generated signal (unused on most architectures)
    pub trapno: i32,

    /// Sending process ID
    pub pid: i32,

    /// Real user ID of sending process
    pub uid: u32,

    /// Exit value or signal
    pub status: i32,

    /// User time consumed
    pub utime: i64,

    /// System time consumed
    pub stime: i64,

    /// Signal value
    pub value: *const (),

    /// POSIX.1b signal
    pub i32: i32,

    /// POSIX.1b signal
    pub ptr: *const (),

    /// Timer overrun count; POSIX.1b timers
    pub overrun: i32,

    /// Timer ID; POSIX.1b timers
    pub timerid: i32,

    /// Memory location which caused fault
    pub addr: *const (),

    /// Band event (was i32 in glibc 2.3.2 and earlier)
    pub band: i64,

    /// File descriptor
    pub fd: Fd,

    /// Least significant bit of address (since kernel 2.6.32)
    pub addr_lsb: i32,
}

#[inline(always)]
pub unsafe fn rt_sigaction(
    sig: Signal,
    action: *const Sigaction,
    old_action: *mut Sigaction,
    sigsetsize: usize,
) -> SyscallResult<()> {
    syscall!(SYS_NO_RT_SIGACTION, sig.0, action, old_action, sigsetsize).map(|_: usize| ())
}

#[inline(always)]
pub unsafe fn nanosleep(rqtp: *const Timespec, rmtp: *mut Timespec) -> SyscallResult<usize> {
    syscall!(SYS_NO_NANOSLEEP, rqtp, rmtp)
}

#[inline(always)]
pub unsafe fn fork() -> SyscallResult<u32> {
    syscall!(SYS_NO_FORK)
}

#[inline(always)]
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

bitflags! {
    pub struct SignalStackFlags: i32 {
        const ONSTACK = 1;
        const DISABLE = 2;
        const AUTODISARM = 1 << 31;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SignalStack {
    /// Base address of stack
    pub stack_pointer: *mut u8,
    /// Flags
    pub flags: SignalStackFlags,
    /// Number of bytes in stack
    pub size: usize,
}

#[inline(always)]
pub unsafe fn sigaltstack(
    signal_stack: *const SignalStack,
    old_signal_stack: *const SignalStack,
) -> SyscallResult<()> {
    syscall!(SYS_NO_SIGALTSTACK, signal_stack, old_signal_stack).map(|_: usize| ())
}

#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum Resource {
    /// CPU time in sec
    CPU = 0,
    /// Maximum filesize
    FSIZE = 1,
    /// max data size
    DATA = 2,
    /// max stack size
    STACK = 3,
    /// max core file size
    CORE = 4,
    /// max resident set size
    RSS = 5,
    /// max number of processes
    NPROC = 6,
    /// max number of open files
    NOFILE = 7,
    /// max locked-in-memory address space
    MEMLOCK = 8,
    /// address space limit
    AS = 9,
    /// maximum file locks held
    LOCKS = 10,
    /// max number of pending signals
    SIGPENDING = 11,
    /// maximum bytes in POSIX mqueues
    MSGQUEUE = 12,
    /// max nice prio allowed to raise
    NICE = 13,
    /// maximum realtime priority
    RTPRIO = 14,
    /// timeout for RT tasks in us
    RTTIME = 15,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RLimit {
    /// Soft limit
    pub current: u64,
    /// Hard limit (ceiling for current)
    pub max: u64,
}

pub unsafe fn getrlimit(resource: Resource) -> SyscallResult<RLimit> {
    let mut limit = RLimit::default();

    let _: usize = syscall!(SYS_NO_GETRLIMIT, resource, &mut limit as *mut _)?;

    Ok(limit)
}

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum PrctlCode {
    SET_GS = 0x1001,
    SET_FS = 0x1002,
    GET_FS = 0x1003,
    GET_GS = 0x1004,
    GET_CPUID = 0x1011,
    SET_CPUID = 0x1012,
    // MAP_VDSO_X32 = 0x2001,
    // MAP_VDSO_32 = 0x2002,
    // MAP_VDSO_64 = 0x2003,
}

#[inline(always)]
pub unsafe fn arch_prctl(code: PrctlCode, addr: *mut u64) -> SyscallResult<u32> {
    syscall!(SYS_NO_ARCH_PTRCTL, code, addr)
}

#[inline(always)]
pub unsafe fn wait4(
    upid: u32,
    stat_addr: *mut i32,
    options: i32,
    ru: *mut Rusage,
) -> SyscallResult<u32> {
    syscall!(SYS_NO_WAIT4, upid, stat_addr, options, ru)
}

pub unsafe fn setrlimit(resource: Resource, limit: RLimit) -> SyscallResult<()> {
    syscall!(
        SYS_NO_SETRLIMIT,
        resource,
        &limit as *const RLimit as *mut RLimit
    )
    .map(|_: usize| ())
}

#[inline(always)]
pub fn gettid() -> u32 {
    unsafe { syscall!(RAW SYS_NO_GETTID) as u32 }
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

#[inline(always)]
pub unsafe fn futex(
    uaddr: *const AtomicU32,
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

#[inline(always)]
pub unsafe fn waitid(
    which: IdType,
    upid: u32,
    infop: *mut (),
    options: WaitIdOption,
    ru: *mut Rusage,
) -> SyscallResult<u64> {
    syscall!(SYS_NO_WAITID, which, upid, infop, options.bits(), ru)
}
