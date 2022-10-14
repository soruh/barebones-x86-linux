#![allow(clippy::upper_case_acronyms)]

use core::arch::asm;

// Arch/ABI    Instruction           System  Ret  Ret  Error
//                                   call #  val  val2
// ---------------------------------------------------------
// x86-64      syscall               rax     rax  rdx  -
//
//  Arch/ABI      arg1  arg2  arg3  arg4  arg5  arg6  arg7
// ---------------------------------------------------------
//  x86-64        rdi   rsi   rdx   r10   r8    r9    -
pub macro syscall0($syscall_no: expr) {
    {
        let ret: isize;

        asm!(
            "syscall",
            inlateout("rax") $syscall_no => ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );

        ret
    }
}

pub macro syscall1($syscall_no: expr, $arg1: expr) {
    {
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            inlateout("rax") $syscall_no => ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );

        ret
    }
}

pub macro syscall2($syscall_no: expr, $arg1: expr, $arg2: expr) {
    {
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            inlateout("rax") $syscall_no => ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );

        ret
    }
}

pub macro syscall3($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr) {
    {
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            in("rdx") $arg3,
            inlateout("rax") $syscall_no => ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );

        ret
    }
}

pub macro syscall4($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr) {
    {
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            in("rdx") $arg3,
            in("r10") $arg4,
            inlateout("rax") $syscall_no => ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );

        ret
    }
}

pub macro syscall5($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr) {
    {
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            in("rdx") $arg3,
            in("r10") $arg4,
            in("r8") $arg5,
            inlateout("rax") $syscall_no => ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );

        ret
    }
}

pub macro syscall6($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr, $arg6: expr) {
    {
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            in("rdx") $arg3,
            in("r10") $arg4,
            in("r8") $arg5,
            in("r9") $arg6,
            inlateout("rax") $syscall_no => ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );

        ret
    }
}

pub macro syscall_inner {
    ($syscall_no: expr) => {
        syscall0!($syscall_no)
    },
    ($syscall_no: expr, $arg1: expr) => {
        syscall1!($syscall_no, $arg1)
    },
    ($syscall_no: expr, $arg1: expr, $arg2: expr) => {
        syscall2!($syscall_no, $arg1, $arg2)
    },
    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr) => {
        syscall3!($syscall_no, $arg1, $arg2, $arg3)
    },
    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr) => {
        syscall4!($syscall_no, $arg1, $arg2, $arg3, $arg4)
    },
    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr) => {
        syscall5!($syscall_no, $arg1, $arg2, $arg3, $arg4, $arg5)
    },
    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr, $arg6: expr) => {
        syscall6!($syscall_no, $arg1, $arg2, $arg3, $arg4, $arg5, $arg6)
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct SyscallError(pub u32);

pub type SyscallResult<T> = Result<T, SyscallError>;

impl SyscallError {
    pub fn kind(self) -> SyscallErrorKind {
        unsafe {
            if self.0 <= 133 {
                core::mem::transmute(self.0)
            } else {
                SyscallErrorKind::Unknown
            }
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyscallErrorKind {
    NONE = 0,
    EPERM = 1,
    ENOENT = 2,
    ESRCH = 3,
    EINTR = 4,
    EIO = 5,
    ENXIO = 6,
    E2BIG = 7,
    ENOEXEC = 8,
    EBADF = 9,
    ECHILD = 10,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EFAULT = 14,
    ENOTBLK = 15,
    EBUSY = 16,
    EEXIST = 17,
    EXDEV = 18,
    ENODEV = 19,
    ENOTDIR = 20,
    EISDIR = 21,
    EINVAL = 22,
    ENFILE = 23,
    EMFILE = 24,
    ENOTTY = 25,
    ETXTBSY = 26,
    EFBIG = 27,
    ENOSPC = 28,
    ESPIPE = 29,
    EROFS = 30,
    EMLINK = 31,
    EPIPE = 32,
    EDOM = 33,
    ERANGE = 34,
    EDEADLK = 35,
    ENAMETOOLONG = 36,
    ENOLCK = 37,
    ENOSYS = 38,
    ENOTEMPTY = 39,
    ELOOP = 40,
    ENOMSG = 42,
    EIDRM = 43,
    ECHRNG = 44,
    EL2NSYNC = 45,
    EL3HLT = 46,
    EL3RST = 47,
    ELNRNG = 48,
    EUNATCH = 49,
    ENOCSI = 50,
    EL2HLT = 51,
    EBADE = 52,
    EBADR = 53,
    EXFULL = 54,
    ENOANO = 55,
    EBADRQC = 56,
    EBADSLT = 57,
    EBFONT = 59,
    ENOSTR = 60,
    ENODATA = 61,
    ETIME = 62,
    ENOSR = 63,
    ENONET = 64,
    ENOPKG = 65,
    EREMOTE = 66,
    ENOLINK = 67,
    EADV = 68,
    ESRMNT = 69,
    ECOMM = 70,
    EPROTO = 71,
    EMULTIHOP = 72,
    EDOTDOT = 73,
    EBADMSG = 74,
    EOVERFLOW = 75,
    ENOTUNIQ = 76,
    EBADFD = 77,
    EREMCHG = 78,
    ELIBACC = 79,
    ELIBBAD = 80,
    ELIBSCN = 81,
    ELIBMAX = 82,
    ELIBEXEC = 83,
    EILSEQ = 84,
    ERESTART = 85,
    ESTRPIPE = 86,
    EUSERS = 87,
    ENOTSOCK = 88,
    EDESTADDRREQ = 89,
    EMSGSIZE = 90,
    EPROTOTYPE = 91,
    ENOPROTOOPT = 92,
    EPROTONOSUPPORT = 93,
    ESOCKTNOSUPPORT = 94,
    EOPNOTSUPP = 95,
    EPFNOSUPPORT = 96,
    EAFNOSUPPORT = 97,
    EADDRINUSE = 98,
    EADDRNOTAVAIL = 99,
    ENETDOWN = 100,
    ENETUNREACH = 101,
    ENETRESET = 102,
    ECONNABORTED = 103,
    ECONNRESET = 104,
    ENOBUFS = 105,
    EISCONN = 106,
    ENOTCONN = 107,
    ESHUTDOWN = 108,
    ETOOMANYREFS = 109,
    ETIMEDOUT = 110,
    ECONNREFUSED = 111,
    EHOSTDOWN = 112,
    EHOSTUNREACH = 113,
    EALREADY = 114,
    EINPROGRESS = 115,
    ESTALE = 116,
    EUCLEAN = 117,
    ENOTNAM = 118,
    ENAVAIL = 119,
    EISNAM = 120,
    EREMOTEIO = 121,
    EDQUOT = 122,
    ENOMEDIUM = 123,
    EMEDIUMTYPE = 124,
    ECANCELED = 125,
    ENOKEY = 126,
    EKEYEXPIRED = 127,
    EKEYREVOKED = 128,
    EKEYREJECTED = 129,
    EOWNERDEAD = 130,
    ENOTRECOVERABLE = 131,
    ERFKILL = 132,
    EHWPOISON = 133,
    Unknown,
}

impl core::fmt::Display for SyscallErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            SyscallErrorKind::NONE => "None",
            SyscallErrorKind::EPERM => "EPERM",
            SyscallErrorKind::ENOENT => "ENOENT",
            SyscallErrorKind::ESRCH => "ESRCH",
            SyscallErrorKind::EINTR => "EINTR",
            SyscallErrorKind::EIO => "EIO",
            SyscallErrorKind::ENXIO => "ENXIO",
            SyscallErrorKind::E2BIG => "E2BIG",
            SyscallErrorKind::ENOEXEC => "ENOEXEC",
            SyscallErrorKind::EBADF => "EBADF",
            SyscallErrorKind::ECHILD => "ECHILD",
            SyscallErrorKind::EAGAIN => "EAGAIN",
            SyscallErrorKind::ENOMEM => "ENOMEM",
            SyscallErrorKind::EACCES => "EACCES",
            SyscallErrorKind::EFAULT => "EFAULT",
            SyscallErrorKind::ENOTBLK => "ENOTBLK",
            SyscallErrorKind::EBUSY => "EBUSY",
            SyscallErrorKind::EEXIST => "EEXIST",
            SyscallErrorKind::EXDEV => "EXDEV",
            SyscallErrorKind::ENODEV => "ENODEV",
            SyscallErrorKind::ENOTDIR => "ENOTDIR",
            SyscallErrorKind::EISDIR => "EISDIR",
            SyscallErrorKind::EINVAL => "EINVAL",
            SyscallErrorKind::ENFILE => "ENFILE",
            SyscallErrorKind::EMFILE => "EMFILE",
            SyscallErrorKind::ENOTTY => "ENOTTY",
            SyscallErrorKind::ETXTBSY => "ETXTBSY",
            SyscallErrorKind::EFBIG => "EFBIG",
            SyscallErrorKind::ENOSPC => "ENOSPC",
            SyscallErrorKind::ESPIPE => "ESPIPE",
            SyscallErrorKind::EROFS => "EROFS",
            SyscallErrorKind::EMLINK => "EMLINK",
            SyscallErrorKind::EPIPE => "EPIPE",
            SyscallErrorKind::EDOM => "EDOM",
            SyscallErrorKind::ERANGE => "ERANGE",
            SyscallErrorKind::EDEADLK => "EDEADLK",
            SyscallErrorKind::ENAMETOOLONG => "ENAMETOOLONG",
            SyscallErrorKind::ENOLCK => "ENOLCK",
            SyscallErrorKind::ENOSYS => "ENOSYS",
            SyscallErrorKind::ENOTEMPTY => "ENOTEMPTY",
            SyscallErrorKind::ELOOP => "ELOOP",
            SyscallErrorKind::ENOMSG => "ENOMSG",
            SyscallErrorKind::EIDRM => "EIDRM",
            SyscallErrorKind::ECHRNG => "ECHRNG",
            SyscallErrorKind::EL2NSYNC => "EL2NSYNC",
            SyscallErrorKind::EL3HLT => "EL3HLT",
            SyscallErrorKind::EL3RST => "EL3RST",
            SyscallErrorKind::ELNRNG => "ELNRNG",
            SyscallErrorKind::EUNATCH => "EUNATCH",
            SyscallErrorKind::ENOCSI => "ENOCSI",
            SyscallErrorKind::EL2HLT => "EL2HLT",
            SyscallErrorKind::EBADE => "EBADE",
            SyscallErrorKind::EBADR => "EBADR",
            SyscallErrorKind::EXFULL => "EXFULL",
            SyscallErrorKind::ENOANO => "ENOANO",
            SyscallErrorKind::EBADRQC => "EBADRQC",
            SyscallErrorKind::EBADSLT => "EBADSLT",
            SyscallErrorKind::EBFONT => "EBFONT",
            SyscallErrorKind::ENOSTR => "ENOSTR",
            SyscallErrorKind::ENODATA => "ENODATA",
            SyscallErrorKind::ETIME => "ETIME",
            SyscallErrorKind::ENOSR => "ENOSR",
            SyscallErrorKind::ENONET => "ENONET",
            SyscallErrorKind::ENOPKG => "ENOPKG",
            SyscallErrorKind::EREMOTE => "EREMOTE",
            SyscallErrorKind::ENOLINK => "ENOLINK",
            SyscallErrorKind::EADV => "EADV",
            SyscallErrorKind::ESRMNT => "ESRMNT",
            SyscallErrorKind::ECOMM => "ECOMM",
            SyscallErrorKind::EPROTO => "EPROTO",
            SyscallErrorKind::EMULTIHOP => "EMULTIHOP",
            SyscallErrorKind::EDOTDOT => "EDOTDOT",
            SyscallErrorKind::EBADMSG => "EBADMSG",
            SyscallErrorKind::EOVERFLOW => "EOVERFLOW",
            SyscallErrorKind::ENOTUNIQ => "ENOTUNIQ",
            SyscallErrorKind::EBADFD => "EBADFD",
            SyscallErrorKind::EREMCHG => "EREMCHG",
            SyscallErrorKind::ELIBACC => "ELIBACC",
            SyscallErrorKind::ELIBBAD => "ELIBBAD",
            SyscallErrorKind::ELIBSCN => "ELIBSCN",
            SyscallErrorKind::ELIBMAX => "ELIBMAX",
            SyscallErrorKind::ELIBEXEC => "ELIBEXEC",
            SyscallErrorKind::EILSEQ => "EILSEQ",
            SyscallErrorKind::ERESTART => "ERESTART",
            SyscallErrorKind::ESTRPIPE => "ESTRPIPE",
            SyscallErrorKind::EUSERS => "EUSERS",
            SyscallErrorKind::ENOTSOCK => "ENOTSOCK",
            SyscallErrorKind::EDESTADDRREQ => "EDESTADDRREQ",
            SyscallErrorKind::EMSGSIZE => "EMSGSIZE",
            SyscallErrorKind::EPROTOTYPE => "EPROTOTYPE",
            SyscallErrorKind::ENOPROTOOPT => "ENOPROTOOPT",
            SyscallErrorKind::EPROTONOSUPPORT => "EPROTONOSUPPORT",
            SyscallErrorKind::ESOCKTNOSUPPORT => "ESOCKTNOSUPPORT",
            SyscallErrorKind::EOPNOTSUPP => "EOPNOTSUPP",
            SyscallErrorKind::EPFNOSUPPORT => "EPFNOSUPPORT",
            SyscallErrorKind::EAFNOSUPPORT => "EAFNOSUPPORT",
            SyscallErrorKind::EADDRINUSE => "EADDRINUSE",
            SyscallErrorKind::EADDRNOTAVAIL => "EADDRNOTAVAIL",
            SyscallErrorKind::ENETDOWN => "ENETDOWN",
            SyscallErrorKind::ENETUNREACH => "ENETUNREACH",
            SyscallErrorKind::ENETRESET => "ENETRESET",
            SyscallErrorKind::ECONNABORTED => "ECONNABORTED",
            SyscallErrorKind::ECONNRESET => "ECONNRESET",
            SyscallErrorKind::ENOBUFS => "ENOBUFS",
            SyscallErrorKind::EISCONN => "EISCONN",
            SyscallErrorKind::ENOTCONN => "ENOTCONN",
            SyscallErrorKind::ESHUTDOWN => "ESHUTDOWN",
            SyscallErrorKind::ETOOMANYREFS => "ETOOMANYREFS",
            SyscallErrorKind::ETIMEDOUT => "ETIMEDOUT",
            SyscallErrorKind::ECONNREFUSED => "ECONNREFUSED",
            SyscallErrorKind::EHOSTDOWN => "EHOSTDOWN",
            SyscallErrorKind::EHOSTUNREACH => "EHOSTUNREACH",
            SyscallErrorKind::EALREADY => "EALREADY",
            SyscallErrorKind::EINPROGRESS => "EINPROGRESS",
            SyscallErrorKind::ESTALE => "ESTALE",
            SyscallErrorKind::EUCLEAN => "EUCLEAN",
            SyscallErrorKind::ENOTNAM => "ENOTNAM",
            SyscallErrorKind::ENAVAIL => "ENAVAIL",
            SyscallErrorKind::EISNAM => "EISNAM",
            SyscallErrorKind::EREMOTEIO => "EREMOTEIO",
            SyscallErrorKind::EDQUOT => "EDQUOT",
            SyscallErrorKind::ENOMEDIUM => "ENOMEDIUM",
            SyscallErrorKind::EMEDIUMTYPE => "EMEDIUMTYPE",
            SyscallErrorKind::ECANCELED => "ECANCELED",
            SyscallErrorKind::ENOKEY => "ENOKEY",
            SyscallErrorKind::EKEYEXPIRED => "EKEYEXPIRED",
            SyscallErrorKind::EKEYREVOKED => "EKEYREVOKED",
            SyscallErrorKind::EKEYREJECTED => "EKEYREJECTED",
            SyscallErrorKind::EOWNERDEAD => "EOWNERDEAD",
            SyscallErrorKind::ENOTRECOVERABLE => "ENOTRECOVERABLE",
            SyscallErrorKind::ERFKILL => "ERFKILL",
            SyscallErrorKind::EHWPOISON => "EHWPOISON",
            SyscallErrorKind::Unknown => "Unknown",
        };

        write!(f, "{}", name)
    }
}

impl core::fmt::Display for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SyscallError({})", self.kind())
    }
}

impl core::fmt::Debug for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SyscallError({:?})", self.kind())
    }
}

pub macro syscall {
    ($syscall_no: expr $(, $arg: expr)*) => {
        {
            let res = syscall_inner!($syscall_no $(, $arg as usize)*);

            // A value in the range between `-4095` and `-1` indicates an error,it is `-errno`.
            // from https://refspecs.linuxfoundation.org/elf/x86_64-abi-0.99.pdf (page 124)
            if (-4095..=-1).contains(&res) {
                Err($crate::syscalls::helper::SyscallError((-res) as u32))
            } else {
                Ok(res as _)
            }
        }
    },
    (RAW $syscall_no: expr $(, $arg: expr)*) => {
        {
             syscall_inner!($syscall_no $(, $arg as usize)*)
        }
    }
}
