use core::{fmt::Write, num::NonZeroUsize, str::Utf8Error};

use crate::{
    sync::{FutexMutexGuard, Mutex},
    syscalls::SyscallError,
};
use alloc::string::String;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    Syscall(SyscallError),
    UnexpectedEOF,
    InvalidUtf8(Utf8Error),
}

impl From<SyscallError> for Error {
    fn from(err: SyscallError) -> Self {
        Self::Syscall(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::InvalidUtf8(err)
    }
}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
pub struct Fd(u32);
impl Fd {
    pub fn write(&self, bytes: &[u8]) -> Result<usize> {
        unsafe {
            let res = crate::syscalls::write(self.0, bytes.as_ptr(), bytes.len())?;
            Ok(res)
        }
    }

    pub fn write_all(&self, bytes: &[u8]) -> Result<usize> {
        let mut n_written_total = 0;
        loop {
            let n_written = self.write(&bytes[n_written_total..])?;

            if n_written == 0 {
                return Ok(n_written_total);
            }

            n_written_total += n_written;

            if n_written_total == bytes.len() {
                return Ok(n_written_total);
            }
        }
    }

    pub fn read(&self, dest: &mut [u8]) -> Result<NonZeroUsize> {
        unsafe {
            let res = crate::syscalls::read(self.0, dest.as_mut_ptr(), dest.len())?;

            NonZeroUsize::new(res).ok_or(Error::UnexpectedEOF)
        }
    }

    pub fn read_exact(&self, dest: &mut [u8]) -> Result<usize> {
        let mut n_read_total = 0;
        loop {
            let n_read = self.read(&mut dest[n_read_total..]);

            if let Err(Error::UnexpectedEOF) = n_read {
                return Ok(n_read_total);
            }

            n_read_total += n_read?.get();

            if n_read_total == dest.len() {
                return Ok(n_read_total);
            }
        }
    }
}

pub struct StdOut(Mutex<BufferedWriter<1024>>);

impl StdOut {
    pub const FD: Fd = Fd(1);

    // Safety: Self needs to be `Pin`ed in memory
    pub const unsafe fn new() -> Self {
        Self(Mutex::new(BufferedWriter::new(Self::FD)))
    }
}

pub struct StdErr(Mutex<BufferedWriter<1024>>);

impl StdErr {
    pub const FD: Fd = Fd(2);

    // Safety: Self needs to be `Pin`ed in memory
    pub const unsafe fn new() -> Self {
        Self(Mutex::new(BufferedWriter::new(Self::FD)))
    }
}

pub struct StdIn(Mutex<BufferedReader<1024>>);

impl StdIn {
    pub const FD: Fd = Fd(0);

    // Safety: Self needs to be `Pin`ed in memory
    pub const unsafe fn new() -> Self {
        Self(Mutex::new(BufferedReader::new(Self::FD)))
    }
}

// Safety: `Pin`ed, since they are statics
pub static STD_OUT: StdOut = unsafe { StdOut::new() };
pub static STD_ERR: StdErr = unsafe { StdErr::new() };
pub static STD_IN: StdIn = unsafe { StdIn::new() };

pub fn stdout() -> FutexMutexGuard<'static, BufferedWriter<1024>> {
    STD_OUT.0.lock()
}

pub fn stderr() -> FutexMutexGuard<'static, BufferedWriter<1024>> {
    STD_ERR.0.lock()
}

pub fn stdin() -> FutexMutexGuard<'static, BufferedReader<1024>> {
    STD_IN.0.lock()
}

macro_rules! print {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        use ::core::fmt::Write;
        let mut stdout = $crate::io::stdout();
        write!(stdout, $format, $($arg)*).expect("Failed to write to stdout");
        stdout.flush().expect("Failed to flush stdout");
    }};
}

macro_rules! println {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        use ::core::fmt::Write;
        write!($crate::io::stdout(), concat!($format, "\n"), $($arg),*).expect("Failed to write to stdout");
    }};
    () => {
        println!("");
    };
}

macro_rules! eprint {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        use ::core::fmt::Write;
        let mut stderr = $crate::io::stderr();
        write!(stderr, $format, $($arg)*).expect("Failed to write to stderr");
        stderr.flush().expect("Failed to flush stderr");
    }};
}

macro_rules! eprintln {
    ($format: literal $(, $arg: expr)* $(,)?) => {{
        use ::core::fmt::Write;
        write!($crate::io::stderr(), concat!($format, "\n"), $($arg),*).expect("Failed to write to stderr");
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

pub struct BufferedReader<const BUFFER_SIZE: usize> {
    fd: Fd,
    buffer: [u8; BUFFER_SIZE],
    cursor: usize,
}

impl<const BUFFER_SIZE: usize> BufferedReader<BUFFER_SIZE> {
    pub const fn new(fd: Fd) -> Self {
        Self {
            fd,
            buffer: [0; BUFFER_SIZE],
            cursor: 0,
        }
    }

    fn fill_buffer(&mut self) -> Result<NonZeroUsize> {
        self.fd.read(&mut self.buffer[self.cursor..])
    }

    pub fn read_line(&mut self) -> Result<String> {
        loop {
            // TODO: checking the byte for \n might cause utf-8 problems
            if let Some((i, _)) = self.buffer[..self.cursor]
                .iter()
                .enumerate()
                .find(|(_, &x)| x == b'\n')
            {
                let line: String = core::str::from_utf8(&self.buffer[..=i])?.into();

                self.buffer.copy_within(i..self.cursor, 0);

                self.cursor = self.cursor - i - 1;

                break Ok(line);
            } else if self.cursor < BUFFER_SIZE - 1 {
                let n_read = self.fill_buffer()?;

                self.cursor += n_read.get();
            }
        }
    }

    pub fn lines(&mut self) -> Lines<'_, BUFFER_SIZE> {
        Lines { reader: self }
    }
}

pub struct Lines<'reader, const BUFFER_SIZE: usize> {
    reader: &'reader mut BufferedReader<BUFFER_SIZE>,
}

impl<const BUFFER_SIZE: usize> Iterator for Lines<'_, BUFFER_SIZE> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_line() {
            Ok(res) => Some(res),
            Err(Error::UnexpectedEOF) => None,
            Err(err) => panic!("failed to read line: {:?}", err),
        }
    }
}

pub struct BufferedWriter<const BUFFER_SIZE: usize> {
    fd: Fd,
    buffer: [u8; BUFFER_SIZE],
    cursor: usize,
}

impl<const BUFFER_SIZE: usize> Drop for BufferedWriter<BUFFER_SIZE> {
    fn drop(&mut self) {
        self.flush().expect("Failed to flush BufferedWriter");
    }
}

impl<const BUFFER_SIZE: usize> BufferedWriter<BUFFER_SIZE> {
    pub const fn new(fd: Fd) -> Self {
        Self {
            fd,
            buffer: [0; BUFFER_SIZE],
            cursor: 0,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        let new_cursor = self.cursor + data.len();

        if new_cursor <= BUFFER_SIZE {
            self.buffer[self.cursor..new_cursor].copy_from_slice(data);

            self.cursor = new_cursor;

            if new_cursor == BUFFER_SIZE {
                self.flush()?;
            }
        } else {
            self.flush()?;
            self.fd.write_all(data)?;
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<usize> {
        if self.cursor > 0 {
            let n = self.fd.write_all(&self.buffer[0..self.cursor])?;

            self.cursor = 0;

            Ok(n)
        } else {
            Ok(0)
        }
    }
}

impl<const BUFFER_SIZE: usize> Write for BufferedWriter<BUFFER_SIZE> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write(s.as_bytes()).map_err(|_| core::fmt::Error)?;

        if s.contains('\n') {
            self.flush().map_err(|_| core::fmt::Error)?;
        }

        Ok(())
    }
}

pub fn cleanup() {
    stdout().flush().expect("Failed to flush stdout");
    stderr().flush().expect("Failed to flush stderr");
}
