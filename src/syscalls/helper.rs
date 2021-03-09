// Arch/ABI    Instruction           System  Ret  Ret  Error
//                                   call #  val  val2
// ---------------------------------------------------------
// x86-64      syscall               rax     rax  rdx  -
//
//  Arch/ABI      arg1  arg2  arg3  arg4  arg5  arg6  arg7
// ---------------------------------------------------------
//  x86-64        rdi   rsi   rdx   r10   r8    r9    -

#[inline(always)]
pub unsafe fn syscall0(syscall_no: usize) -> isize {
    let ret;

    asm!(
        "syscall",
        inlateout("rax") syscall_no => ret,
        lateout("rdx") _,
        lateout("rcx") _,
        lateout("r11") _,
    );

    ret
}

#[inline(always)]
pub unsafe fn syscall1(syscall_no: usize, arg1: usize) -> isize {
    let ret;

    asm!(
        "syscall",
        in("rdi") arg1,
        inlateout("rax") syscall_no => ret,
        lateout("rdx") _,
        lateout("rcx") _,
        lateout("r11") _,
    );

    ret
}

#[inline(always)]
pub unsafe fn syscall2(syscall_no: usize, arg1: usize, arg2: usize) -> isize {
    let ret;

    asm!(
        "syscall",
        in("rdi") arg1,
        in("rsi") arg2,
        inlateout("rax") syscall_no => ret,
        lateout("rdx") _,
        lateout("rcx") _,
        lateout("r11") _,
    );

    ret
}

#[inline(always)]
pub unsafe fn syscall3(syscall_no: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret;

    asm!(
        "syscall",
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        inlateout("rax") syscall_no => ret,
        lateout("rdx") _,
        lateout("rcx") _,
        lateout("r11") _,
    );

    ret
}

#[inline(always)]
pub unsafe fn syscall4(
    syscall_no: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
) -> isize {
    let ret;

    asm!(
        "syscall",
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        inlateout("rax") syscall_no => ret,
        lateout("rdx") _,
        lateout("rcx") _,
        lateout("r11") _,
    );

    ret
}

#[inline(always)]
pub unsafe fn syscall5(
    syscall_no: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> isize {
    let ret;

    asm!(
        "syscall",
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        inlateout("rax") syscall_no => ret,
        lateout("rdx") _,
        lateout("rcx") _,
        lateout("r11") _,
    );

    ret
}

#[inline(always)]
pub unsafe fn syscall6(
    syscall_no: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> isize {
    let ret;

    asm!(
        "syscall",
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        in("r9") arg6,
        inlateout("rax") syscall_no => ret,
        lateout("rdx") _,
        lateout("rcx") _,
        lateout("r11") _,
    );

    ret
}

macro_rules! syscall_inner {
    ($syscall_no: expr) => {
        $crate::syscalls::helper::syscall0($syscall_no)
    };

    ($syscall_no: expr, $arg1: expr) => {
        $crate::syscalls::helper::syscall1($syscall_no, $arg1)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr) => {
        $crate::syscalls::helper::syscall2($syscall_no, $arg1, $arg2)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr) => {
        $crate::syscalls::helper::syscall3($syscall_no, $arg1, $arg2, $arg3)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr) => {
        $crate::syscalls::helper::syscall4($syscall_no, $arg1, $arg2, $arg3, $arg4)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr) => {
        $crate::syscalls::helper::syscall5($syscall_no, $arg1, $arg2, $arg3, $arg4, $arg5)
    };

    ($syscall_no: expr, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr, $arg6: expr) => {
        $crate::syscalls::helper::syscall6($syscall_no, $arg1, $arg2, $arg3, $arg4, $arg5, $arg6)
    };
}

use alloc::format;

#[repr(transparent)]
pub struct SyscallError(pub usize);

pub type SyscallResult<T> = Result<T, SyscallError>;

impl core::fmt::Debug for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "SyscallError({})",
            match self.0 {
                22 => "EINVAL".into(),

                i => format!("{}", i),
            }
        )
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

}
