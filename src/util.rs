use crate::error::{CpdbError, Result};
use crate::ffi;
use libc::c_char;
use std::ffi::{CStr, CString};

/// Converts a C string pointer to an owned Rust String.
/// Returns CpdbError::NullPointer if the pointer is null.
pub unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String> {
    if ptr.is_null() {
        Err(CpdbError::NullPointer)
    } else {
        unsafe { Ok(CStr::from_ptr(ptr).to_string_lossy().into_owned()) }
    }
}

/// Converts a C string pointer to an owned Rust String and frees it with g_free.
/// Returns CpdbError::NullPointer if the pointer is null.
pub unsafe fn cstr_to_string_and_g_free(ptr: *mut c_char) -> Result<String> {
    if ptr.is_null() {
        Err(CpdbError::NullPointer)
    } else {
        unsafe {
            let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            glib_sys::g_free(ptr as glib_sys::gpointer);
            Ok(s)
        }
    }
}

/// Owns both the CString backing memory and the cpdb_option_t array
/// so that raw pointers in the options remain valid for the struct's lifetime.
pub struct COptions {
    _strings: Vec<CString>,
    options: Vec<ffi::cpdb_option_t>,
}

impl COptions {
    pub fn as_mut_ptr(&mut self) -> *mut ffi::cpdb_option_t {
        self.options.as_mut_ptr()
    }

    pub fn len(&self) -> usize {
        self.options.len()
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }
}

/// Converts a slice of (key, value) pairs into a COptions struct
/// suitable for passing to cpdb-libs C functions.
///
/// The returned COptions owns all backing memory - the raw pointers
/// inside `cpdb_option_t` remain valid as long as COptions is alive.
pub fn to_c_options(options: &[(&str, &str)]) -> Result<COptions> {
    let mut strings: Vec<CString> = Vec::with_capacity(options.len() * 2);
    let mut c_options: Vec<ffi::cpdb_option_t> = Vec::with_capacity(options.len());

    for (key, value) in options {
        strings.push(CString::new(*key)?);
        strings.push(CString::new(*value)?);
        let key_ptr = strings[strings.len() - 2].as_ptr() as *mut c_char;
        let val_ptr = strings[strings.len() - 1].as_ptr() as *mut c_char;
        c_options.push(ffi::cpdb_option_t {
            option_name: key_ptr,
            default_value: val_ptr,
            group_name: std::ptr::null_mut(),
            num_supported: 0,
            supported_values: std::ptr::null_mut(),
        });
    }

    Ok(COptions {
        _strings: strings,
        options: c_options,
    })
}
