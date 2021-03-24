use core::ops::{Deref, DerefMut};

use alloc::{string::String, vec::Vec};

#[derive(Clone)]
pub struct CString(Vec<u8>);

impl From<String> for CString {
    fn from(string: String) -> Self {
        let mut vec: Vec<u8> = string.into();
        vec.push(0);
        Self(vec)
    }
}

impl From<&str> for CString {
    fn from(string: &str) -> Self {
        let string: String = string.into();
        string.into()
    }
}

impl Into<String> for CString {
    fn into(mut self) -> String {
        unsafe {
            // remove 0 byte
            self.0.pop();
            // self.0 must also be a valid String, since it was created from one
            core::mem::transmute(self.0)
        }
    }
}

impl CString {
    pub fn as_cstr(&self) -> &CStr {
        unsafe { &*(self.0.as_slice() as *const [u8] as *const CStr) }
    }

    pub fn as_cstr_mut(&mut self) -> &mut CStr {
        unsafe { &mut *(self.0.as_mut_slice() as *mut [u8] as *mut CStr) }
    }
}

impl Deref for CString {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        self.as_cstr()
    }
}

impl DerefMut for CString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_cstr_mut()
    }
}

impl AsRef<CStr> for CString {
    fn as_ref(&self) -> &CStr {
        self.as_cstr()
    }
}

#[repr(transparent)]
pub struct CStr([u8]);

impl CStr {
    pub fn as_ptr(&self) -> *const u8 {
        self as *const CStr as *const u8
    }

    pub fn as_ptr_mut(&mut self) -> *mut u8 {
        self as *mut CStr as *mut u8
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            let len = self.0.len();
            core::str::from_utf8_unchecked(&self.0[0..len - 1])
        }
    }

    pub fn as_str_mut(&mut self) -> &mut str {
        unsafe {
            let len = self.0.len();
            core::str::from_utf8_unchecked_mut(&mut self.0[0..len - 1])
        }
    }
}

impl Into<CString> for &CStr {
    fn into(self) -> CString {
        CString(self.0.into())
    }
}

#[repr(transparent)]
pub struct ConstCString<const SIZE: usize>([u8; SIZE]);

impl<const SIZE: usize> ConstCString<SIZE> {
    pub const unsafe fn new_unchecked(string: &'static str) -> Self {
        let mut array = [0; SIZE];
        let bytes = string.as_bytes();

        // NOTE: `copy_from_slice` is not `const`
        let mut i = 0;
        while i < SIZE {
            array[i] = bytes[i];
            i += 1;
        }

        Self(array)
    }
}

macro_rules! const_cstr {
    ($string: literal) => {
        unsafe {
            const S: &'static str = concat!($string, "\0");
            $crate::ffi::ConstCString::<{ S.len() }>::new_unchecked(S)
        }
    };
}

impl<const SIZE: usize> ConstCString<SIZE> {
    pub fn as_cstr(&self) -> &CStr {
        unsafe { &*(self.0.as_slice() as *const [u8] as *const CStr) }
    }

    pub fn as_cstr_mut(&mut self) -> &mut CStr {
        unsafe { &mut *(self.0.as_mut_slice() as *mut [u8] as *mut CStr) }
    }
}

impl<const SIZE: usize> Deref for ConstCString<SIZE> {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        self.as_cstr()
    }
}

impl<const SIZE: usize> DerefMut for ConstCString<SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_cstr_mut()
    }
}

impl<const SIZE: usize> AsRef<CStr> for ConstCString<SIZE> {
    fn as_ref(&self) -> &CStr {
        self.as_cstr()
    }
}
