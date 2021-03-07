#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]

// extern crate compiler_builtins;

#[macro_use]
mod io;

mod env;
mod start;
mod syscalls;
mod util;
use env::Environment;

use io::*;

fn main(env: Environment) -> i32 {
    println!("Test");

    for (i, arg) in env.args().enumerate() {
        println!("arg[{:2?}]: {}", i, arg);
    }

    for (i, (key, value)) in env.env().enumerate() {
        println!("env[{:2?}]: {} = {}", i, key, value);
    }

    0
}
