pub struct RuntimeOptions {
    pub(crate) alloc: bool,
    pub(crate) logging: bool,
    pub(crate) segv_handling: bool,
    pub(crate) stack_protection: bool,
    pub(crate) tls: bool,
    pub(crate) io: bool,
}

impl RuntimeOptions {
    pub const fn all() -> Self {
        Self {
            alloc: true,
            logging: true,
            segv_handling: true,
            stack_protection: true,
            tls: true,
            io: true,
        }
    }

    /// # Safety
    /// not initializing runtime features **will** cause
    /// undefined behaviour if these features are used.
    /// Note that some features may rely on others to be present
    pub const unsafe fn none() -> Self {
        Self {
            alloc: false,
            logging: false,
            segv_handling: false,
            stack_protection: false,
            tls: false,
            io: false,
        }
    }

    pub const unsafe fn add_only_alloc(mut self) -> Self {
        self.alloc = true;
        self
    }
    pub const unsafe fn add_only_logging(mut self) -> Self {
        self.logging = true;
        self
    }
    pub const unsafe fn add_only_segv_handling(mut self) -> Self {
        self.segv_handling = true;
        self
    }
    pub const unsafe fn add_only_stack_protection(mut self) -> Self {
        self.stack_protection = true;
        self
    }
    pub const unsafe fn add_only_tls(mut self) -> Self {
        self.tls = true;
        self
    }
    pub const unsafe fn add_only_io(mut self) -> Self {
        self.io = true;
        self
    }

    pub const unsafe fn remove_only_alloc(mut self) -> Self {
        self.alloc = false;
        self
    }
    pub const unsafe fn remove_only_logging(mut self) -> Self {
        self.logging = false;
        self
    }
    pub const unsafe fn remove_only_segv_handling(mut self) -> Self {
        self.segv_handling = false;
        self
    }
    pub const unsafe fn remove_only_stack_protection(mut self) -> Self {
        self.stack_protection = false;
        self
    }
    pub const unsafe fn remove_only_tls(mut self) -> Self {
        self.tls = false;
        self
    }
    pub const unsafe fn remove_only_io(mut self) -> Self {
        self.io = false;
        self
    }

    pub const fn add_alloc(self) -> Self {
        unsafe { self.add_only_alloc() }
    }
    pub const fn add_logging(self) -> Self {
        unsafe { self.add_only_logging().add_io() }
    }
    pub const fn add_segv_handling(self) -> Self {
        unsafe { self.add_only_segv_handling() }
    }
    pub const fn add_stack_protection(self) -> Self {
        unsafe {
            self.add_only_stack_protection()
                .add_tls()
                .add_segv_handling()
        }
    }
    pub const fn add_tls(self) -> Self {
        unsafe { self.add_only_tls().add_alloc() }
    }
    pub const fn add_io(self) -> Self {
        unsafe { self.add_only_io().add_alloc() }
    }
}

pub static mut RUNTIME_OPTIONS: RuntimeOptions = RuntimeOptions::all();

/// creates the program entry point which sets up the program runtime
/// optionally takes a `RuntimeOptions` to selectively en-/disable runtime
/// features
pub macro create_init {
    ($main: path) => {
        create_init!($main, $crate::start::RuntimeOptions::all());
    },
    ($main: path, $opts: expr) => {
        /// The program entry point
        /// # Safety
        /// should **never** be called manually
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn _start() -> ! {
            // C call: rdi, rsi, rdx, rcx, r8, r9
            ::core::arch::asm!(
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


        /// Sets up the program runtime
        /// # Safety
        /// should **only** ever be called by `_start`
        unsafe extern "C" fn _init(n_args: usize, args_start: *const *const u8) -> ! {
            fn set_runtime_options() {
                let opts = $opts;
                unsafe {
                    RUNTIME_OPTIONS = opts;
                }
            }

            set_runtime_options();


            let env = $crate::env::Environment::from_raw_parts(n_args, args_start);


            if RUNTIME_OPTIONS.alloc {
                $crate::allocator::init().expect("Failed to initialize global allocator");
            }

            if RUNTIME_OPTIONS.logging {
                $crate::logger::init::<false>(log::LevelFilter::Trace).expect("Failed to initialize logger");
            }

            let mut stack_base = core::ptr::null_mut();
            let mut stack_limit = 0;

            if RUNTIME_OPTIONS.stack_protection {
                stack_base = env.calculate_stack_base();
                stack_limit = $crate::syscalls::getrlimit($crate::syscalls::Resource::STACK)
                    .expect("Failed to determine stack limit")
                    .current as usize;

                $crate::stack_protection::create_guard_for_stack(stack_base, stack_limit)
                    .expect("Failed to alloate guard page/s");
            }

            if RUNTIME_OPTIONS.tls {
                $crate::tls::setup_tls($crate::tls::Tls {
                    stack_base,
                    stack_limit,
                })
                .expect("Failed to set tls");
            }

            if RUNTIME_OPTIONS.segv_handling {
                $crate::stack_protection::setup_alt_stack().expect("Failed to set up a signal handling stack");
                $crate::stack_protection::setup_segv_handler().expect("Failed to set up segv handler");
            }

            // assert that main has the correct type (and safety)
            let main: fn($crate::env::Environment) -> i8 = $main;

            let exit_code = main(env) as i32;

            if RUNTIME_OPTIONS.io {
                $crate::io::cleanup();
            }

            if RUNTIME_OPTIONS.segv_handling {
                $crate::stack_protection::teardown_segv_handler().expect("Failed to tear down segv handler");
                $crate::stack_protection::teardown_alt_stack()
                    .expect("Failed to tear down signal handling stack");
            }

            if RUNTIME_OPTIONS.tls {
                let _ = $crate::tls::teardown_tls().expect("Failed to teardown tls");
            }

            if RUNTIME_OPTIONS.stack_protection {
                $crate::stack_protection::free_guard_for_stack(stack_base, stack_limit)
                    .expect("Failed to alloate guard page/s");
            }

            if RUNTIME_OPTIONS.alloc {
                $crate::allocator::deinit().expect("Failed to de-initialize global allocator");
            }


            $crate::syscalls::exit(exit_code)
        }
    }
}
