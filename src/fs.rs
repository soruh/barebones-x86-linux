use core::ops::{Deref, DerefMut};

use crate::io::{BufferedWriter, Fd};
use crate::syscalls::{self, OpenFlags, OpenMode, SyscallResult};
use crate::{ffi::CStr, io::BufferedReader};

pub struct File {
    fd: Fd,
}

pub struct BufferedFile<const BUFFER_SIZE: usize> {
    reader: BufferedReader<BUFFER_SIZE>,
    writer: BufferedWriter<BUFFER_SIZE>,
    file: File,
}

impl<const BUFFER_SIZE: usize> Deref for BufferedFile<BUFFER_SIZE> {
    type Target = BufferedReader<BUFFER_SIZE>;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl<const BUFFER_SIZE: usize> DerefMut for BufferedFile<BUFFER_SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

impl File {
    pub fn close_ref(&mut self) -> SyscallResult<()> {
        syscalls::close(self.fd)
    }

    pub fn close(mut self) -> SyscallResult<()> {
        let res = self.close_ref();
        core::mem::forget(self);
        res
    }

    /// Get a the file's fd.
    pub fn fd(&self) -> Fd {
        self.fd
    }

    pub fn buffer<const BUFFER_SIZE: usize>(self) -> BufferedFile<BUFFER_SIZE> {
        BufferedFile {
            reader: BufferedReader::new(self.fd),
            writer: BufferedWriter::new(self.fd),
            file: self,
        }
    }
}

impl Deref for File {
    type Target = Fd;

    fn deref(&self) -> &Self::Target {
        &self.fd
    }
}

impl Drop for File {
    fn drop(&mut self) {
        self.close_ref()
            .unwrap_or_else(|err| panic!("Failed to close file {}: {}", self.fd().0, err));
    }
}

impl File {
    pub fn open(path: impl AsRef<CStr>, flags: OpenFlags, mode: OpenMode) -> SyscallResult<Self> {
        syscalls::open(path, flags, mode).map(|fd| Self { fd })
    }
}
