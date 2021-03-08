// TODO: write safe wrappers here?
#[macro_use]
mod helper;
mod raw;
pub use raw::*;

pub fn write_str(fd: u32, s: &str) -> Result<usize, isize> {
    unsafe { write(fd, s.as_ptr(), s.len()) }
}
