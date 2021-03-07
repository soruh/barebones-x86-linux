use core::fmt::Debug;
use core::fmt::Write;

use crate::syscalls::write;

fn write_str(fd: u32, s: &str) {
    let (res1, res2) = unsafe { write(fd, s.as_ptr(), s.len()) };

    // TODO: handle results
}

pub struct StdOut;
impl Write for StdOut {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(0, s);
        Ok(())
    }
}

pub struct StdErr;
impl Write for StdErr {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(1, s);
        Ok(())
    }
}

macro_rules! print {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        use ::core::fmt::Write;
        write!($crate::io::StdOut, $format, $($arg)*).expect("Failed to write to stdout");
    }};
}

macro_rules! println {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        use ::core::fmt::Write;
        write!($crate::io::StdOut, concat!($format, "\n"), $($arg),*).expect("Failed to write to stdout");
    }};
    () => {
        println!("\n");
    };
}

macro_rules! eprint {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        use ::core::fmt::Write;
        write!($crate::io::StdErr, $format, $($arg)*).expect("Failed to write to stderr");
    }};
}

macro_rules! eprintln {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        use ::core::fmt::Write;
        write!($crate::io::StdErr, concat!($format, "\n"), $($arg),*).expect("Failed to write to stderr");
    }};
    () => {
        eprintln!("\n");
    };
}

macro_rules! dbg {
    () => {
        eprintln!("[{}:{}]", file!(), line!());
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                eprintln!("[{}:{}] {} = {:?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($(dbg!($val)),+,)
    };
}
