use crate::io::StdErr;
use crate::syscalls::exit;

#[panic_handler]
fn __panic_handler(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    // Discard the write result; We are already panicking...
    let _ = writeln!(StdErr, "{}", info);

    unsafe { exit(1) }
}

#[alloc_error_handler]
fn __alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Failed to allocate memory of layout {:?}", layout)
}

#[no_mangle]
extern "C" fn rust_eh_personality() {}
