// link this binary without any startfiles
#[allow(unused_attributes)]
#[link_args = "-nostartfiles"]
extern "C" {}

#[allow(clippy::clippy::missing_safety_doc)] // (haha)
#[no_mangle]
#[naked]
unsafe extern "C" fn _start() -> ! {
    // C call: rdi, rsi, rdx, rcx, r8, r9
    asm!(
        "endbr64",

        // clear base pointer
        "xor rbp, rbp",

        // save original stack pointer
        "mov rdx, rsp",

        // pop n_args into rdi (arg1)
        "pop rdi",

        // mov start pointer to start of args to rsi (arg2)
        "mov rsi, rsp",

        // restore original stack pointer
        "mov rsp, rdx",

        // align the stack pointer
        // this invalidates the last two words (16bits) on the stack once we use the stack
        // this is why we just read them into registers
        "and rsp, 0xfffffffffffffff0",

        // call _init
        "call {}",
        sym _init,

        options(noreturn)
    );
}
unsafe extern "C" fn _init(n_args: usize, args_start: *const *const u8) -> ! {
    let env = crate::env::Environment::from_raw_parts(n_args, args_start);

    crate::allocator::init().expect("Failed to initialize global allocator");

    let exit_code = crate::main(env) as i32;

    crate::syscalls::exit(exit_code)
}
