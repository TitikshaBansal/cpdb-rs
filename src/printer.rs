use crate::error::{CpdbError, Result};
use crate::ffi;
use std::ptr;

pub struct Printer {
    raw: *mut ffi::cpdb_printer_obj_t,
}

// Mark as Send/Sync since CPDB handles thread safety
unsafe impl Send for Printer {}
unsafe impl Sync for Printer {}

impl Printer {
    /// Create from raw pointer (unsafe)
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_printer_obj_t) -> Result<Self> {
        if raw.is_null() {
            Err(CpdbError::NullPointer)
        } else {
            // SAFETY: raw is valid pointer
            unsafe { ffi::cpdb_acquire_printer(raw) };
            Ok(Self { raw })
        }
    }

    /// Get printer name
    pub fn name(&self) -> Result<String> {
        unsafe {
            let c_name = ffi::cpdb_get_printer_name(self.raw);
            CpdbError::cstr_to_string(c_name)
        }
    }

    /// Get printer state
    pub fn state(&self) -> Result<String> {
        unsafe {
            let c_state = ffi::cpdb_get_printer_state(self.raw);
            CpdbError::cstr_to_string(c_state)
        }
    }

    /// Get printer location
    pub fn location(&self) -> Result<String> {
        unsafe {
            let c_location = ffi::cpdb_get_printer_location(self.raw);
            CpdbError::cstr_to_string(c_location)
        }
    }

    /// Get printer description
    pub fn description(&self) -> Result<String> {
        unsafe {
            let c_desc = ffi::cpdb_get_printer_description(self.raw);
            CpdbError::cstr_to_string(c_desc)
        }
    }

    /// Check if printer accepts PDF
    pub fn accepts_pdf(&self) -> Result<bool> {
        unsafe {
            Ok(ffi::cpdb_printer_accepts_pdf(self.raw) != 0)
        }
    }

    /// Clone printer object with proper reference counting
    pub fn try_clone(&self) -> Result<Self> {
        unsafe {
            Self::from_raw(self.raw)
        }
    }

    /// Get backend name for this printer
    pub fn backend_name(&self) -> Result<String> {
        unsafe {
            let c_name = ffi::cpdb_get_printer_backend_name(self.raw);
            CpdbError::cstr_to_string(c_name)
        }
    }

    /// Submit a print job
    pub fn submit_job(&self, file_path: &str, options: &[(&str, &str)], job_name: &str) -> Result<()> {
        // Create C-compatible options array
        let mut c_options = Vec::with_capacity(options.len());
        let mut keep_alive = Vec::new();

        for (k, v) in options {
            let c_key = std::ffi::CString::new(*k)?;
            let c_val = std::ffi::CString::new(*v)?;
            keep_alive.push(c_key);
            keep_alive.push(c_val);
            
            c_options.push(ffi::cpdb_option_t {
                option_name: keep_alive[keep_alive.len()-2].as_ptr(),
                default_value: keep_alive[keep_alive.len()-1].as_ptr(),
                ..Default::default()
            });
        }

        unsafe {
            let file_cstr = std::ffi::CString::new(file_path)?;
            let job_cstr = std::ffi::CString::new(job_name)?;
            let status = ffi::cpdb_print_file(
                self.raw,
                file_cstr.as_ptr(),
                c_options.as_ptr(),
                c_options.len() as i32,
                job_cstr.as_ptr(),
            );
            
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
                ffi::cpdb_release_printer(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}