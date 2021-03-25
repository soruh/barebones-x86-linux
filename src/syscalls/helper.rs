// Arch/ABI    Instruction           System  Ret  Ret  Error
//                                   call #  val  val2
// ---------------------------------------------------------
// x86-64      syscall               rax     rax  rdx  -
//
//  Arch/ABI      arg1  arg2  arg3  arg4  arg5  arg6  arg7
// ---------------------------------------------------------
//  x86-64        rdi   rsi   rdx   r10   r8    r9    -
macro_rules! syscall0 {
    ($syscall_no: expr) => {{
        let ret: isize;

        asm!(
            "syscall",
            inlateout("rax") $syscall_no => ret,
            lateout("rdx") _,
            lateout("rcx") _,
            lateout("r11") _,
        );

        ret
    }}
}

macro_rules! syscall1 {
    ($syscall_no: expr, $arg1: expr) => {{
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            inlateout("rax") $syscall_no => ret,
            lateout("rdx") _,
            lateout("rcx") _,
            lateout("r11") _,
        );

        ret
    }}
}

macro_rules! syscall2 {
    ($syscall_no: expr, $arg1: expr, $arg2: expr) => {{
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            inlateout("rax") $syscall_no => ret,
            lateout("rdx") _,
            lateout("rcx") _,
            lateout("r11") _,
        );

        ret
    }}
}

macro_rules! syscall3 {
    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr) => {{
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            in("rdx") $arg3,
            inlateout("rax") $syscall_no => ret,
            lateout("rdx") _,
            lateout("rcx") _,
            lateout("r11") _,
        );

        ret
    }}
}

macro_rules! syscall4 {
    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr) => {{
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            in("rdx") $arg3,
            in("r10") $arg4,
            inlateout("rax") $syscall_no => ret,
            lateout("rdx") _,
            lateout("rcx") _,
            lateout("r11") _,
        );

        ret
    }}
}

macro_rules! syscall5 {
    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr) => {{
        let ret: isize;

        asm!(
            "syscall",
            in("rdi") $arg1,
            in("rsi") $arg2,
            in("rdx") $arg3,
            in("r10") $arg4,
            in("r8") $arg5,
            inlateout("rax") $syscall_no => ret,
            lateout("rdx") _,
            lateout("rcx") _,
            lateout("r11") _,
        );

        ret
    }}
}

macro_rules! syscall6 {
    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr, $arg6: expr) => {{
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
            lateout("rdx") _,
            lateout("rcx") _,
            lateout("r11") _,
        );

        ret
    }}
}

macro_rules! syscall_inner {
    ($syscall_no: expr) => {
        syscall0!($syscall_no)
    };

    ($syscall_no: expr, $arg1: expr) => {
        syscall1!($syscall_no, $arg1)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr) => {
        syscall2!($syscall_no, $arg1, $arg2)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr) => {
        syscall3!($syscall_no, $arg1, $arg2, $arg3)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr) => {
        syscall4!($syscall_no, $arg1, $arg2, $arg3, $arg4)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr) => {
        syscall5!($syscall_no, $arg1, $arg2, $arg3, $arg4, $arg5)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr, $arg6: expr) => {
        syscall6!($syscall_no, $arg1, $arg2, $arg3, $arg4, $arg5, $arg6)
    };
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SyscallError(pub usize);

pub type SyscallResult<T> = Result<T, SyscallError>;

impl core::fmt::Debug for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self.0 {
            1 => "EPERM",
            2 => "ENOENT",
            3 => "ESRCH",
            4 => "EINTR",
            5 => "EIO",
            6 => "ENXIO",
            7 => "E2BIG",
            8 => "ENOEXEC",
            9 => "EBADF",
            10 => "ECHILD",
            11 => "EAGAIN",
            12 => "ENOMEM",
            13 => "EACCES",
            14 => "EFAULT",
            15 => "ENOTBLK",
            16 => "EBUSY",
            17 => "EEXIST",
            18 => "EXDEV",
            19 => "ENODEV",
            20 => "ENOTDIR",
            21 => "EISDIR",
            22 => "EINVAL",
            23 => "ENFILE",
            24 => "EMFILE",
            25 => "ENOTTY",
            26 => "ETXTBSY",
            27 => "EFBIG",
            28 => "ENOSPC",
            29 => "ESPIPE",
            30 => "EROFS",
            31 => "EMLINK",
            32 => "EPIPE",
            33 => "EDOM",
            34 => "ERANGE",
            35 => "EDEADLK",
            36 => "ENAMETOOLONG",
            37 => "ENOLCK",
            38 => "ENOSYS",
            39 => "ENOTEMPTY",
            40 => "ELOOP",
            42 => "ENOMSG",
            43 => "EIDRM",
            44 => "ECHRNG",
            45 => "EL2NSYNC",
            46 => "EL3HLT",
            47 => "EL3RST",
            48 => "ELNRNG",
            49 => "EUNATCH",
            50 => "ENOCSI",
            51 => "EL2HLT",
            52 => "EBADE",
            53 => "EBADR",
            54 => "EXFULL",
            55 => "ENOANO",
            56 => "EBADRQC",
            57 => "EBADSLT",
            59 => "EBFONT",
            60 => "ENOSTR",
            61 => "ENODATA",
            62 => "ETIME",
            63 => "ENOSR",
            64 => "ENONET",
            65 => "ENOPKG",
            66 => "EREMOTE",
            67 => "ENOLINK",
            68 => "EADV",
            69 => "ESRMNT",
            70 => "ECOMM",
            71 => "EPROTO",
            72 => "EMULTIHOP",
            73 => "EDOTDOT",
            74 => "EBADMSG",
            75 => "EOVERFLOW",
            76 => "ENOTUNIQ",
            77 => "EBADFD",
            78 => "EREMCHG",
            79 => "ELIBACC",
            80 => "ELIBBAD",
            81 => "ELIBSCN",
            82 => "ELIBMAX",
            83 => "ELIBEXEC",
            84 => "EILSEQ",
            85 => "ERESTART",
            86 => "ESTRPIPE",
            87 => "EUSERS",
            88 => "ENOTSOCK",
            89 => "EDESTADDRREQ",
            90 => "EMSGSIZE",
            91 => "EPROTOTYPE",
            92 => "ENOPROTOOPT",
            93 => "EPROTONOSUPPORT",
            94 => "ESOCKTNOSUPPORT",
            95 => "EOPNOTSUPP",
            96 => "EPFNOSUPPORT",
            97 => "EAFNOSUPPORT",
            98 => "EADDRINUSE",
            99 => "EADDRNOTAVAIL",
            100 => "ENETDOWN",
            101 => "ENETUNREACH",
            102 => "ENETRESET",
            103 => "ECONNABORTED",
            104 => "ECONNRESET",
            105 => "ENOBUFS",
            106 => "EISCONN",
            107 => "ENOTCONN",
            108 => "ESHUTDOWN",
            109 => "ETOOMANYREFS",
            110 => "ETIMEDOUT",
            111 => "ECONNREFUSED",
            112 => "EHOSTDOWN",
            113 => "EHOSTUNREACH",
            114 => "EALREADY",
            115 => "EINPROGRESS",
            116 => "ESTALE",
            117 => "EUCLEAN",
            118 => "ENOTNAM",
            119 => "ENAVAIL",
            120 => "EISNAM",
            121 => "EREMOTEIO",
            122 => "EDQUOT",
            123 => "ENOMEDIUM",
            124 => "EMEDIUMTYPE",
            125 => "ECANCELED",
            126 => "ENOKEY",
            127 => "EKEYEXPIRED",
            128 => "EKEYREVOKED",
            129 => "EKEYREJECTED",
            130 => "EOWNERDEAD",
            131 => "ENOTRECOVERABLE",
            132 => "ERFKILL",
            133 => "EHWPOISON",

            _ => "",
        };

        if name.is_empty() {
            write!(f, "SyscallError({})", self.0)
        } else {
            write!(f, "SyscallError({})", name)
        }
    }
}

impl core::fmt::Display for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self)
    }
}

macro_rules! syscall {
    ($syscall_no: expr $(, $arg: expr)*) => {
        {
            let res = syscall_inner!($syscall_no $(, $arg as usize)*);

            if res < 0 {
                Err($crate::syscalls::helper::SyscallError((-res) as usize))
            } else {
                Ok(res as _)
            }
        }
    };
    (RAW $syscall_no: expr $(, $arg: expr)*) => {
        {
             syscall_inner!($syscall_no $(, $arg as usize)*)
        }
    };

}
