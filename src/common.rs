use crate::error::{CpdbError, Result};
use crate::ffi as cpdb_sys; // Assuming ffi.rs makes symbols available under `cpdb_sys`
use std::ffi::CStr;

/// Returns the version string of the underlying cpdb-libs library.
pub fn version() -> Result<String> {
    unsafe {
        let c_ptr = cpdb_sys::cpdbGetVersion();
        if c_ptr.is_null() {
            // Although cpdbGetVersion is unlikely to return NULL, good practice to check.
            return Err(CpdbError::NullPointerFromC { context: "cpdbGetVersion returned null" });
        }
        let c_str = CStr::from_ptr(c_ptr);
        c_str.to_str()
            .map(|s| s.to_string())
            .map_err(CpdbError::from) // Converts std::str::Utf8Error to CpdbError::CStringUtf8Error
    }
}
/// Initializes the cpdb-libs library.
pub fn init() {
    unsafe {
        cpdb_sys::cpdbInit();
    }
}
