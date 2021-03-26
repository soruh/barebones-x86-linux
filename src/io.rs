use core::{fmt::Write, num::NonZeroUsize, str::Utf8Error};

use crate::{
    sync::{FutexMutexGuard, Mutex},
    syscalls::SyscallError,
};
use alloc::string::String;
use smallstr::SmallString;

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
#[repr(transparent)]

pub struct Fd(pub u32);
impl Fd {
    pub fn write(&self, bytes: &[u8]) -> Result<usize> {
        Ok(crate::syscalls::write(self.0, bytes)?)
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
        NonZeroUsize::new(crate::syscalls::read(self.0, dest)?).ok_or(Error::UnexpectedEOF)
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

impl Write for Fd {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_all(s.as_bytes())
            .map(|_| ())
            .map_err(|_| core::fmt::Error)
    }
}

const STD_OUT_BUFFER_SIZE: usize = 8192;
const STD_ERR_BUFFER_SIZE: usize = 8192;
const STD_IN_BUFFER_SIZE: usize = 8192;

pub struct StdOut(Mutex<BufferedWriter<STD_OUT_BUFFER_SIZE>>);

impl StdOut {
    pub const FD: Fd = Fd(1);

    /// # Safety: Self needs to be `Pin`ed in memory
    pub const unsafe fn new() -> Self {
        Self(Mutex::new(BufferedWriter::new(Self::FD)))
    }
}

pub struct StdErr(Mutex<BufferedWriter<STD_ERR_BUFFER_SIZE>>);

impl StdErr {
    pub const FD: Fd = Fd(2);

    /// # Safety: Self needs to be `Pin`ed in memory
    pub const unsafe fn new() -> Self {
        Self(Mutex::new(BufferedWriter::new(Self::FD)))
    }
}

pub struct StdIn(Mutex<BufferedReader<STD_IN_BUFFER_SIZE>>);

impl StdIn {
    pub const FD: Fd = Fd(0);

    /// # Safety: Self needs to be `Pin`ed in memory
    pub const unsafe fn new() -> Self {
        Self(Mutex::new(BufferedReader::new(Self::FD)))
    }
}

/// # Safety: `Pin`ed, since they are statics
pub static STD_OUT: StdOut = unsafe { StdOut::new() };
pub static STD_ERR: StdErr = unsafe { StdErr::new() };
pub static STD_IN: StdIn = unsafe { StdIn::new() };

pub fn stdout() -> FutexMutexGuard<'static, BufferedWriter<STD_OUT_BUFFER_SIZE>> {
    STD_OUT.0.lock()
}

pub fn stderr() -> FutexMutexGuard<'static, BufferedWriter<STD_ERR_BUFFER_SIZE>> {
    STD_ERR.0.lock()
}

pub fn stdin() -> FutexMutexGuard<'static, BufferedReader<STD_IN_BUFFER_SIZE>> {
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

// TODO: make these Ring buffers?
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

    pub fn read_line(&mut self, line: &mut String) -> Result<()> {
        loop {
            // TODO: checking the byte for \n might cause utf-8 problems
            if let Some((i, _)) = self.buffer[..self.cursor]
                .iter()
                .enumerate()
                .find(|(_, &x)| x == b'\n')
            {
                let i = i + 1;
                line.push_str(core::str::from_utf8(&self.buffer[..i])?);

                self.buffer.copy_within(i..self.cursor, 0);

                self.cursor -= i;

                break Ok(());
            } else if self.cursor < BUFFER_SIZE - 1 {
                let n_read = self.fill_buffer()?;

                self.cursor += n_read.get();
            } else {
                todo!("line is longer than buffer size");
            }
        }
    }
    pub fn read_line_inline<const N_INLINE: usize>(
        &mut self,
        line: &mut SmallString<[u8; N_INLINE]>,
    ) -> Result<()> {
        loop {
            // TODO: checking the byte for \n might cause utf-8 problems
            if let Some((i, _)) = self.buffer[..self.cursor]
                .iter()
                .enumerate()
                .find(|(_, &x)| x == b'\n')
            {
                let i = i + 1;
                line.push_str(core::str::from_utf8(&self.buffer[..i])?);

                self.buffer.copy_within(i..self.cursor, 0);

                self.cursor -= i;

                break Ok(());
            } else if self.cursor < BUFFER_SIZE - 1 {
                let n_read = self.fill_buffer()?;

                self.cursor += n_read.get();
            } else {
                todo!("line is longer than buffer size");
            }
        }
    }

    pub fn lines(&mut self) -> Lines<'_, BUFFER_SIZE> {
        Lines { reader: self }
    }

    pub fn inline_lines<const LINE_SIZE: usize>(
        &mut self,
    ) -> InlineLines<'_, BUFFER_SIZE, LINE_SIZE> {
        InlineLines { reader: self }
    }
}

pub struct Lines<'reader, const BUFFER_SIZE: usize> {
    reader: &'reader mut BufferedReader<BUFFER_SIZE>,
}

impl<const BUFFER_SIZE: usize> Iterator for Lines<'_, BUFFER_SIZE> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Err(Error::UnexpectedEOF) => None,
            res => Some(res.map(|_| line)),
        }
    }
}

pub struct InlineLines<'reader, const BUFFER_SIZE: usize, const LINE_SIZE: usize> {
    reader: &'reader mut BufferedReader<BUFFER_SIZE>,
}

impl<const BUFFER_SIZE: usize, const LINE_SIZE: usize> Iterator
    for InlineLines<'_, BUFFER_SIZE, LINE_SIZE>
{
    type Item = Result<SmallString<[u8; LINE_SIZE]>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = SmallString::new();
        match self.reader.read_line_inline(&mut line) {
            Err(Error::UnexpectedEOF) => None,
            res => Some(res.map(|_| line)),
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
