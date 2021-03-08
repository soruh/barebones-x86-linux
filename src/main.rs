#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_ref)]
#![feature(link_args)]

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
use core::ptr::{null, null_mut};

use env::Environment;
use syscalls::CloneFlags;

unsafe fn main(_env: Environment) -> i8 {
    println!("Hello, World!");

    // let pid = syscalls::fork().expect("Failed to fork");

    let STACK_SIZE = 1024 * 1024;

    // allocate u64's here to have an aligned stack
    let mut child_stack =
        alloc::vec::Vec::<u64>::with_capacity(STACK_SIZE / core::mem::size_of::<u64>());

    let child_stack = (child_stack.as_mut_ptr() as *mut u8).add(STACK_SIZE);

    eprintln!("cloning...");

    let pid = syscalls::clone(
        CloneFlags::empty(), //CloneFlags::VM, // TODO: | CloneFlags::IO
        null_mut(),
        null_mut(),
        null_mut(),
        0,
    )
    .expect("Failed to clone");

    dbg!(pid);

    let stack_pointer: *const u8;

    asm!(
        "mov {0}, rsp",
        out(reg) stack_pointer,
    );

    dbg!(stack_pointer);

    loop {}

    return 0;

    if pid == 0 {
        // We are the child

        let mut vec = alloc::vec::Vec::new();

        vec.extend(0..0x80);

        eprintln!("{:02X?}", vec);
    } else {
        // We are the parent

        let mut vec = alloc::vec::Vec::new();

        vec.extend(0x80..0x100);

        let mut status = 0;
        let mut r_usage = syscalls::Rusage::default();

        syscalls::wait4(pid, &mut status as *mut _, 0, &mut r_usage as *mut _)
            .expect("Failed to wait for child");

        dbg_p!(r_usage);
        dbg!(status);

        eprintln!("{:02X?}", vec);
    }

    0
}
