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

        // pop n_args into rdi (arg1)
        "pop rdi",

        // mov start pointer to start of args to rsi (arg2)
        "mov rsi, rsp",

        // align the stack pointer
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

    crate::logger::init::<false>(log::LevelFilter::Trace).expect("Failed to initialize logger");

    let stack_limit = crate::syscalls::getrlimit(crate::syscalls::Resource::STACK)
        .expect("Failed to determine stack limit")
        .current as usize;

    let stack_base = env.calculate_stack_base();

    crate::stack_protection::create_guard_for_stack(stack_base, stack_limit)
        .expect("Failed to alloate guard page/s");

    crate::tls::setup_tls(crate::tls::Tls {
        stack_base,
        stack_limit,
    })
    .expect("Failed to set tls");

    crate::stack_protection::setup_alt_stack().expect("Failed to set up a signal handling stack");

    crate::stack_protection::setup_segv_handler().expect("Failed to set up segv handler");

    let _: fn(crate::env::Environment) -> i8 = crate::main;

    let exit_code = crate::main(env) as i32;

    crate::io::cleanup();

    crate::stack_protection::teardown_segv_handler().expect("Failed to tear down segv handler");

    crate::stack_protection::teardown_alt_stack()
        .expect("Failed to tear down signal handling stack");

    let _ = crate::tls::teardown_tls().expect("Failed to teardown tls");

    crate::stack_protection::free_guard_for_stack(stack_base, stack_limit)
        .expect("Failed to alloate guard page/s");

    crate::allocator::deinit().expect("Failed to de-initialize global allocator");

    crate::syscalls::exit(exit_code)
}
