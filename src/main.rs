#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]
#![feature(maybe_uninit_extra)]
#![feature(link_args)]

extern crate alloc;

// extern crate compiler_builtins;

#[macro_use]
mod io;

mod allocator;
mod env;
mod start;
mod sync;
mod syscalls;
mod util;
use env::Environment;

unsafe fn main(_env: Environment) -> i8 {
    println!("Test");

    let mut vec = alloc::vec::Vec::new();

    vec.extend(0..0x100);

    eprintln!("{:02X?}", vec);

    0
}
