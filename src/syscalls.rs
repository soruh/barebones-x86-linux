use core::hint::unreachable_unchecked;

// Arch/ABI    Instruction           System  Ret  Ret  Error
//                                   call #  val  val2
// ---------------------------------------------------------
// x86-64      syscall               rax     rax  rdx  -
//
//  Arch/ABI      arg1  arg2  arg3  arg4  arg5  arg6  arg7
// ---------------------------------------------------------
//  x86-64        rdi   rsi   rdx   r10   r8    r9    -

#[inline(always)]
pub unsafe fn syscall1(syscall_no: usize, arg1: usize) -> (usize, usize) {
    let ret1;
    let ret2;
    asm!(
        "syscall",
        in("rdi") arg1,
        inlateout("rax") syscall_no => ret1,
        lateout("rdx") ret2,
        lateout("rcx") _,
        lateout("r11") _,
    );

    (ret1, ret2)
}

#[inline(always)]
pub unsafe fn syscall3(syscall_no: usize, arg1: usize, arg2: usize, arg3: usize) -> (usize, usize) {
    let ret1;
    let ret2;
    asm!(
        "syscall",
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        inlateout("rax") syscall_no => ret1,
        lateout("rdx") ret2,
        lateout("rcx") _,
        lateout("r11") _,
    );

    (ret1, ret2)
}

const SYS_NO_READ: usize = 0;
const SYS_NO_WRITE: usize = 1;
const SYS_NO_EXIT: usize = 60;

pub unsafe fn write(fd: u32, buf: *const u8, count: usize) -> (usize, usize) {
    syscall3(SYS_NO_WRITE, fd as usize, buf as usize, count as usize)
}

pub unsafe fn exit(code: i32) -> ! {
    syscall1(SYS_NO_EXIT, code as usize);
    unreachable_unchecked()
}
