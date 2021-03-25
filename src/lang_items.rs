use crate::io::stderr;
use crate::syscalls::exit;

#[panic_handler]
fn __panic_handler(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let thread = crate::syscalls::gettid();

    // Discard the write result; We are already panicking...
    let _ = match (info.message(), info.location()) {
        (Some(message), Some(location)) => writeln!(
            stderr(),
            "thread {} \x1b[31mpanicked\x1b[m at '{:?}', {}",
            thread,
            message,
            location
        ),
        (Some(message), None) => writeln!(
            stderr(),
            "thread {} \x1b[31mpanicked\x1b[m at '{}'",
            thread,
            message
        ),
        (None, Some(location)) => {
            writeln!(
                stderr(),
                "thread {} \x1b[31mpanicked\x1b[m at {}",
                thread,
                location
            )
        }

        _ => writeln!(stderr(), "thread {} \x1b[31mpanicked\x1b[m", thread),
    };

    unsafe { exit(1) }
}

#[alloc_error_handler]
fn __alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Failed to allocate memory of layout {:?}", layout)
}

#[lang = "eh_personality"]
unsafe fn eh_personality() {
    asm!("ud2");
}

#[no_mangle]
unsafe extern "C" fn _Unwind_Resume() {
    asm!("ud2");
}
