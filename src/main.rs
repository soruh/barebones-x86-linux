#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_ref)]
#![feature(link_args)]
#![feature(lang_items)]
#![feature(core_intrinsics)]
#![allow(unused_macros, dead_code)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate log;

// extern crate compiler_builtins;

#[macro_use]
mod io;

mod allocator;
mod env;
mod lang_items;
mod logger;
mod start;
mod sync;
mod syscalls;
mod thread;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{time::Duration, usize};
use env::Environment;
use sync::Mutex;

unsafe fn main(env: Environment) -> i8 {
    if false {
        alloc_test_main(env)
    } else {
        thread_test_main(env)
    }
}

#[allow(clippy::many_single_char_names)]
unsafe fn alloc_test_main(_env: Environment) -> i8 {
    println!("Hello, World!");

    let x = Box::new([0u8; (2 << 11) - 1]);

    dbg!(x.len());

    let mut v = Vec::with_capacity(1024);

    // Test that allocating and dropping values in a loop does not repeatedly allocate new blocks
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

    let mut v: Vec<Box<[u8; 32]>> = Vec::with_capacity(1024 * 1024);

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
    const N_LOOPS: usize = 1_000_000;
    const N_THREADS: usize = 16;

    println!("Hello, World!");

    info!("spawning...");

    let data = Arc::new(Mutex::new(0));

    let handles: Vec<_> = (0..N_THREADS)
        .into_iter()
        .map(|i| {
            let data = data.clone();

            thread::spawn(
                move || {
                    info!("child {:X?}...", i);

                    for _ in 0..N_LOOPS {
                        *data.lock() += 1;
                    }

                    info!("child {:X?} done", i);

                    42
                },
                1024 * 1024,
            )
            .expect("Failed to spawn thread")
        })
        .collect();

    for _ in 0..(N_THREADS * N_LOOPS) {
        *data.lock() -= 1;
    }

    info!("parent waiting...");

    for handle in handles {
        assert_eq!(handle.join(), 42);
    }

    info!("parent done");

    assert_eq!(*data.lock(), 0);

    info!("sleeping...");

    syscalls::sleep(Duration::from_secs(1)).unwrap();

    info!("done");

    0
}
