use crate::{
    env::Environment,
    ffi::const_cstr,
    io::*,
    sync::Mutex,
    syscalls::{OpenFlags, OpenMode},
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{time::Duration, arch::asm};

pub enum TestFunction {
    Alloc,
    ThreadingAndMutex,
    Async,
    UserInput,
    FsTest,
    StackOverflow,
}

pub unsafe fn main(env: Environment, test_function: TestFunction) -> i8 {
    match test_function {
        TestFunction::Alloc => alloc_test_main(env),
        TestFunction::ThreadingAndMutex => thread_test_main(env),
        TestFunction::Async => async_test_main(env),
        TestFunction::UserInput => user_input_main(env),
        TestFunction::FsTest => fs_test_main(env),
        TestFunction::StackOverflow => stack_overflow_test(env),
    }
}

// TODO: use cpuid?
fn ncpu() -> crate::io::IoResult<usize> {
    let mut file = crate::fs::File::open(
        const_cstr!("/proc/cpuinfo"),
        OpenFlags::empty(),
        OpenMode::RDONLY,
    )?
    .buffer::<512>();

    for line in file.inline_lines::<128>() {
        if let Some(siblings) = line?.strip_prefix("siblings\t:") {
            return Ok(siblings.trim().parse().unwrap());
        }
    }

    unreachable!("/proc/cpuinfo did not contain siblings")
}

unsafe fn fs_test_main(_env: Environment) -> i8 {
    let ncpu = ncpu().unwrap();

    dbg!(ncpu);

    0
}

unsafe fn stack_overflow_test(_env: Environment) -> i8 {
    unsafe fn overflow_stack_inner(initial: bool) {
        static mut START: usize = 0;

        let rsp: usize;
        asm!("mov {}, rsp", out(reg) rsp);

        if initial {
            START = rsp;
        } else {
            let size = START - rsp;

            if size % (128 * 1024) == 0 {
                dbg!(size);
            }
        }

        overflow_stack_inner(false);
    }

    // let _: u8 = core::ptr::read_volatile(core::ptr::null());

    // Safety: must only be called by one thread at once
    unsafe fn overflow_stack() {
        overflow_stack_inner(true);
    }

    let mut handle = crate::thread::spawn(
        || {
            let _b = Box::new("test");

            // syscalls::sleep(Duration::from_secs(60)).unwrap();

            debug!("I am the thread!");

            overflow_stack();

            // panic!("end of thread");
        },
        None,
    )
    .unwrap();

    let res = handle.join().unwrap();

    if res.is_some() {
        info!("thread {} succeeded", handle.tid());
    } else {
        info!("thread {} failed", handle.tid());
    }

    0
}

unsafe fn async_test_main(env: Environment) -> i8 {
    let executor = crate::executor::init(1);

    executor.block_on(async_test_main_inner(env))
}

async fn async_test_main_inner(_env: Environment) -> i8 {
    0
}

unsafe fn user_input_main(_env: Environment) -> i8 {
    println!("Hello, World!");

    print!("> ");
    for line in stdin().lines() {
        let line = line.unwrap();

        dbg!(line);

        print!("> ");
    }

    0
}

#[allow(clippy::many_single_char_names)]
unsafe fn alloc_test_main(_env: Environment) -> i8 {
    println!("Hello, World!");

    let x = Box::new([0u8; (2 << 11) - 1]);

    dbg!(x.len());

    let mut v = Vec::with_capacity(1024);

    // Test that allocating and dropping values in a loop does not repeatedly
    // allocate new blocks
    for i in 0..1024 {
        let a: Box<u32> = Box::new(42);
        let mut b: Box<u32> = Box::new(37);

        #[inline(never)]
        fn f(b: &mut u32) {
            *b += 5;
        }

        f(&mut b);

        assert_eq!(a, b);

        v.push(i);
    }

    {
        let a: Box<u8> = Box::new(1);
        let b: Box<u16> = Box::new(1);

        dbg!(a);

        Box::leak(b);
    }

    let mut v: Vec<Box<[u8; 32]>> = Vec::with_capacity(1024);

    for i in 0..10 * 1024 {
        let mut a = [0; 32];
        for (j, x) in a.iter_mut().enumerate() {
            *x = (i + j) as u8;
        }
        v.push(Box::new(a));
    }

    let a: Box<u8> = Box::new(42);
    let b: Box<u8> = Box::new(120);
    let c: Box<u8> = Box::new(36);
    let d: Box<u8> = Box::new(69);

    let e = core::mem::transmute::<[i64; 4], core::arch::x86_64::__m256i>([1, 69, 420, 9]);
    let e = Box::new(e);

    dbg!(a, b, c, d, e);

    dbg!(v.len(), v.capacity());

    for (i, a) in v.iter().enumerate() {
        for (j, x) in a.iter().enumerate() {
            assert_eq!(*x, (i + j) as u8);
        }
    }

    0
}

unsafe fn thread_test_main(_env: Environment) -> i8 {
    const N_LOOPS: usize = 2_000_000;
    const N_THREADS: usize = 16;

    println!("Hello, World!");

    info!("spawning...");

    // Safety: The Mutex is `Pin`ned by the `Arc::pin`.
    let data = Arc::pin(Mutex::new(0));

    // dbg!(data.deref() as *const _);

    let handles: Vec<_> = (0..N_THREADS)
        .into_iter()
        .map(|i| {
            let data = data.clone();

            crate::thread::spawn(
                move || {
                    info!("child {:X?}...", i);

                    // dbg!(data.deref() as *const _);

                    for _ in 0..N_LOOPS {
                        *data.lock() += 1;
                    }

                    info!("child {:X?} done", i);

                    42
                },
                None,
            )
            .expect("Failed to spawn thread")
        })
        .collect();

    for _ in 0..(N_THREADS * N_LOOPS) {
        *data.lock() -= 1;
    }

    info!("parent waiting...");

    for mut handle in handles {
        assert_eq!(handle.join().unwrap(), Some(42));
    }

    info!("parent done");

    assert_eq!(*data.lock(), 0);

    info!("sleeping...");

    crate::syscalls::sleep(Duration::from_secs(1)).unwrap();

    info!("done");

    0
}
