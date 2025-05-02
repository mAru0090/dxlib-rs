use crate::dxlib_constants::*;
use crate::dxlib_error::*;
use crate::dxlib_types::*;
use std::ffi::CString;

pub struct CStringHolder {
    _c_string: CString,
    ptr: *const CChar,
}

impl CStringHolder {
    pub fn new(s: impl ToString) -> Self {
        let c_string = CString::new(s.to_string()).unwrap();
        let ptr = c_string.as_ptr();
        Self {
            _c_string: c_string,
            ptr,
        }
    }

    pub fn ptr(&self) -> *const i8 {
        self.ptr
    }
}

pub struct VecHolder<T> {
    _vec: Vec<T>,
    ptr: *mut T,
}

impl<T> VecHolder<T> {
    pub fn new(vec: impl Into<Vec<T>>) -> Self {
        let mut vec = vec.into();
        let ptr = vec.as_mut_ptr();
        Self { _vec: vec, ptr }
    }

    pub fn ptr(&self) -> *mut T {
        self.ptr
    }
}
