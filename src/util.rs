use crate::io::StdErr;
use crate::syscalls::exit;

#[panic_handler]
fn __panic_handler(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    // Discard the write result; Are already panicking...
    let _ = writeln!(StdErr, "{}", info);

    unsafe { exit(1) }
}

#[alloc_error_handler]
fn __alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Failed to allocate memory of layout {:?}", layout)
}

// TODO: this is probably wrong :(
#[no_mangle]
fn rust_eh_personality() -> ! {
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) {
    for i in 0..n {
        *dest.add(i) = c as u8;
    }
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) {
    for i in 0..n {
        *dest.add(i) = *src.add(i);
    }
}
