use core::fmt::Write;

use crate::{
    sync::{FutexMutex, Mutex},
    syscalls::write_str,
};

const FD_STD_OUT: u32 = 0;
const FD_STD_ERR: u32 = 1;
const FD_STD_IN: u32 = 2;

pub struct StdOut;
impl Write for StdOut {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if write_str(FD_STD_OUT, s).is_ok() {
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}

pub struct StdErr;
impl Write for StdErr {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if write_str(FD_STD_ERR, s).is_ok() {
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}

pub static STD_OUT: Mutex<()> = Mutex::new(());
pub static STD_ERR: Mutex<()> = Mutex::new(());

macro_rules! print {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        let lock = $crate::io::STD_OUT.lock();

        use ::core::fmt::Write;
        write!($crate::io::StdOut, $format, $($arg)*).expect("Failed to write to stdout");

        drop(lock);
    }};
}

macro_rules! println {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        let lock = $crate::io::STD_OUT.lock();

        use ::core::fmt::Write;
        write!($crate::io::StdOut, concat!($format, "\n"), $($arg),*).expect("Failed to write to stdout");

        drop(lock);
    }};
    () => {
        println!("");
    };
}

macro_rules! eprint {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        let lock = $crate::io::STD_ERR.lock();

        use ::core::fmt::Write;
        write!($crate::io::StdErr, $format, $($arg)*).expect("Failed to write to stderr");

        drop(lock);
    }};
}

macro_rules! eprintln {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        let lock = $crate::io::STD_ERR.lock();

        use ::core::fmt::Write;
        write!($crate::io::StdErr, concat!($format, "\n"), $($arg),*).expect("Failed to write to stderr");

        drop(lock);
    }};
    () => {
        eprintln!("");
    };
}

macro_rules! dbg {
    () => {
        debug!("[{}:{}]", file!(), line!());
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                debug!("[{}:{}] {} = {:?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($(dbg!($val)),+,)
    };
}

macro_rules! dbg_p {
    () => {
        debug!("[{}:{}]", file!(), line!());
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                debug!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($(dbg!($val)),+,)
    };
}
