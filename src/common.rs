use crate::error::{CpdbError, Result};
use crate::ffi;
use std::ffi::CStr;

pub fn version() -> Result<String> {
    unsafe {
        let c_ptr = ffi::cpdbGetVersion();
        if c_ptr.is_null() {
            return Err(CpdbError::NullPointer);
        }
        let version_str = CStr::from_ptr(c_ptr).to_str()?.to_string();
        Ok(version_str)
    }
}

pub fn init() {
    unsafe {
        ffi::cpdbInit();
    }
}