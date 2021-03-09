#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_ref)]
#![feature(link_args)]
#![feature(lang_items)]

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
use core::ptr::{null, null_mut};
use core::time::Duration;
use env::Environment;
use syscalls::sleep;

unsafe fn main(_env: Environment) -> i8 {
    println!("Hello, World!");

    eprintln!("spawning...");

    let handle = thread::spawn(
        || {
            eprintln!("child...");

            sleep(Duration::from_secs(2)).unwrap();

            eprintln!("child done");

            42
        },
        1024 * 1024,
    )
    .expect("Failed to spawn thread");

    sleep(Duration::from_secs(1)).unwrap();

    eprintln!("parent waiting...");

    let res = handle.join();

    eprintln!("parent done");

    res
}
