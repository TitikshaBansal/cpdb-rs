use crate::error::CpdbError;
use crate::ffi;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

pub unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String, CpdbError> {
    if ptr.is_null() {
        return Err(CpdbError::NullPointer);
    }
    Ok(CStr::from_ptr(ptr)
        .to_str()?
        .to_string())
}

pub fn to_c_options(
    options: &[(&str, &str)]
) -> Result<Vec<ffi::cpdb_option_t>, CpdbError> {
    let mut c_options = Vec::with_capacity(options.len());
    let mut keep_alive = Vec::new();
    
    for (key, value) in options {
        let c_key = CString::new(*key)?;
        let c_val = CString::new(*value)?;
        keep_alive.push(c_key);
        keep_alive.push(c_val);
        
        c_options.push(ffi::cpdb_option_t {
            option_name: keep_alive[keep_alive.len()-2].as_ptr() as *mut i8,
            default_value: keep_alive[keep_alive.len()-1].as_ptr() as *mut i8,
            ..Default::default()
        });
    }
    
    Ok(c_options)
}

// Cleanup function for options
pub unsafe fn free_c_options(options: Vec<ffi::cpdb_option_t>) {
    for opt in options {
        if !opt.option_name.is_null() {
            let _ = CString::from_raw(opt.option_name);
        }
        if !opt.default_value.is_null() {
            let _ = CString::from_raw(opt.default_value);
        }
    }
}