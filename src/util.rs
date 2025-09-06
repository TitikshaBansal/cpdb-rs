use crate::error::{CpdbError, Result};
use crate::ffi; // This will be used if ffi module contains cpdb_option_t etc.
use libc::{c_char, c_void}; // Use libc's c_char
use std::ffi::{CString, CStr};
use glib_sys; // For g_free

pub unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String, CpdbError> {
    if ptr.is_null() {
        return Err(CpdbError::NullPointer);
    }
    unsafe { // Explicit unsafe block
        Ok(CStr::from_ptr(ptr)
            .to_str()?
            .to_string())
    }
}

pub unsafe fn cstr_to_string_and_g_free(c_ptr: *mut c_char) -> Result<String, CpdbError> {
    if c_ptr.is_null() {
        return Err(CpdbError::NullPointer);
    }
    let conversion_result = unsafe { // Explicit unsafe block
        CStr::from_ptr(c_ptr)
            .to_str()
            .map(|s| s.to_string())
    }.map_err(CpdbError::from);
    
    glib_sys::g_free(c_ptr as *mut c_void);
    
    conversion_result
}

pub fn to_c_options(
    options: &[(&str, &str)]
) -> Result<Vec<ffi::cpdb_option_t>, CpdbError> {
    let mut c_options = Vec::with_capacity(options.len());
    let mut cstring_holder: Vec<CString> = Vec::new(); 

    for (key, value) in options {
        let c_key = CString::new(*key)?;
        let c_val = CString::new(*value)?;
        
        let key_ptr = c_key.as_ptr();
        let val_ptr = c_val.as_ptr();
        cstring_holder.push(c_key);
        cstring_holder.push(c_val);
        
        c_options.push(ffi::cpdb_option_t {
            option_name: key_ptr as *mut c_char,
            default_value: val_ptr as *mut c_char,
            group_name: std::ptr::null_mut(),
            num_supported: 0,
            supported_values: std::ptr::null_mut(),
        });
    }
    Ok(c_options)
}
pub unsafe fn free_c_options(_options: Vec<ffi::cpdb_option_t>) {
}