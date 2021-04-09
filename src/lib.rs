#![no_std]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(alloc_error_handler)]
#![feature(maybe_uninit_ref)]
#![feature(naked_functions)]
#![feature(lang_items)]
#![feature(core_intrinsics)]
#![feature(panic_info_message)]
#![feature(array_methods)]
#![feature(const_mut_refs)]
#![allow(unused_macros, dead_code)]

#[macro_use] extern crate alloc;
#[macro_use] extern crate log;
extern crate compiler_builtins;

pub mod start;
pub use start::create_init;

pub mod allocator;
pub mod env;
pub mod executor;
pub mod ffi;
pub mod fs;
pub mod io;
pub mod lang_items;
pub mod logger;
pub mod stack_protection;
pub mod sync;
pub mod syscalls;
pub mod thread;
pub mod tls;
