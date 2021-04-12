#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(link_args)]
#![allow(clippy::missing_safety_doc)]

use barebones_x86_linux::*;

#[macro_use] extern crate log;

extern crate alloc;

pub mod tests;

create_init!(main);

fn main(env: env::Environment) -> i8 {
    unsafe { tests::main(env, tests::TestFunction::ThreadingAndMutex) }
}
