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

extern crate alloc;

// extern crate compiler_builtins;

#[macro_use]
mod io;

mod allocator;
mod env;
mod lang_items;
mod start;
mod sync;
mod syscalls;
mod thread;
use alloc::{format, sync::Arc, vec::Vec};
use env::Environment;
use sync::FutexMutex;

const N: usize = 1_000_000;

unsafe fn main(_env: Environment) -> i8 {
    println!("Hello, World!");

    eprintln!("spawning...");

    let data = Arc::new(FutexMutex::new(0));

    let handles: Vec<_> = (0..10)
        .into_iter()
        .map(|i| {
            let data = data.clone();

            thread::spawn(
                move || {
                    worker(i, data);

                    42
                },
                1024 * 1024,
            )
            .expect("Failed to spawn thread")
        })
        .collect();

    for _ in 0..10 * N {
        *data.lock() -= 1;
    }

    // sleep(Duration::from_secs(1)).unwrap();

    eprintln!("parent waiting...");

    for handle in handles {
        handle.join();
    }

    eprintln!("parent done");

    assert_eq!(*data.lock(), 0);

    0
}

use alloc::boxed::Box;

#[inline(never)]
fn worker(i: i32, data: Arc<FutexMutex<i32>>) {
    eprint!("{}", &format!("child {}...\n", i));

    for _ in 0..N {
        *data.lock() += 1;
    }

    eprint!("{}", &format!("child {} done\n", i));
}
