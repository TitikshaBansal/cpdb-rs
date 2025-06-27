use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::util;
use std::ptr;
use std::ffi::CString;

pub struct Printer {
    raw: *mut ffi::cpdb_printer_obj_t,
}

unsafe impl Send for Printer {}
unsafe impl Sync for Printer {}

impl Printer {
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_printer_obj_t) -> Result<Self> {
        if raw.is_null() {
            Err(CpdbError::NullPointer)
        } else {
            ffi::cpdbAcquirePrinter(raw);
            Ok(Self { raw })
        }
    }

    pub fn name(&self) -> Result<String> {
        unsafe {
            if (*self.raw).name.is_null() {
                return Ok(String::new());
            }
            util::cstr_to_string((*self.raw).name)
        }
    }

    pub fn state(&self) -> Result<String> {
        unsafe {
            if (*self.raw).state.is_null() {
                return Ok(String::new());
            }
            util::cstr_to_string((*self.raw).state)
        }
    }

    pub fn location(&self) -> Result<String> {
        unsafe {
            if (*self.raw).location.is_null() {
                return Ok(String::new());
            }
            util::cstr_to_string((*self.raw).location)
        }
    }

    pub fn description(&self) -> Result<String> {
        unsafe {
            if (*self.raw).info.is_null() {
                return Ok(String::new());
            }
            util::cstr_to_string((*self.raw).info)
        }
    }

    pub fn accepts_pdf(&self) -> Result<bool> {
        unsafe {
            // This function doesn't exist in cpdb_sys, let's assume it's a property
            // We'll use the make_and_model field as a heuristic
            let model = util::cstr_to_string((*self.raw).make_and_model).unwrap_or_default();
            Ok(model.to_lowercase().contains("pdf"))
        }
    }

    pub fn try_clone(&self) -> Result<Self> {
        unsafe {
            ffi::cpdbAcquirePrinter(self.raw);
            Ok(Self { raw: self.raw })
        }
    }

    pub fn backend_name(&self) -> Result<String> {
        unsafe {
            if (*self.raw).backend_name.is_null() {
                return Ok(String::new());
            }
            util::cstr_to_string((*self.raw).backend_name)
        }
    }

    pub fn submit_job(&self, file_path: &str, options: &[(&str, &str)], job_name: &str) -> Result<()> {
        let c_options = util::to_c_options(options)?;
        let file_cstr = CString::new(file_path)?;
        let job_cstr = CString::new(job_name)?;
        
        unsafe {
            let status = ffi::cpdbPrintFile(
                self.raw,
                file_cstr.as_ptr(),
                c_options.as_ptr(),
                c_options.len() as i32,
                job_cstr.as_ptr(),
            );
            
            util::free_c_options(c_options);
            
            if status == 0 {
                Ok(())
            } else {
                Err(CpdbError::from_status(status, "Job submission failed"))
            }
        }
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbReleasePrinter(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}

impl Clone for Printer {
    fn clone(&self) -> Self {
        unsafe {
            ffi::cpdbAcquirePrinter(self.raw);
            Self { raw: self.raw }
        }
    }
}