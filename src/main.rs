#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]
#![feature(maybe_uninit_ref)]
#![feature(link_args)]
#![feature(lang_items)]
#![feature(core_intrinsics)]
#![feature(panic_info_message)]
#![feature(array_methods)]
#![feature(const_mut_refs)]
#![allow(unused_macros, dead_code)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
extern crate compiler_builtins;

#[macro_use]
mod io;
#[macro_use]
mod ffi;
#[macro_use]
mod syscalls;
mod allocator;
mod env;
mod executor;
mod fs;
mod lang_items;
mod logger;
mod stack_protection;
mod start;
mod sync;
mod tests;
mod thread;
mod tls;

fn main(env: env::Environment) -> i8 {
    unsafe { tests::main(env, tests::TestFunction::StackOverflow) }
}
