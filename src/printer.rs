use crate::error::{CpdbError, Result};
use crate::ffi;
use std::ptr;

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
            let c_name = ffi::cpdbGetPrinterName(self.raw);
            CpdbError::cstr_to_string(c_name)
        }
    }

    pub fn state(&self) -> Result<String> {
        unsafe {
            let c_state = ffi::cpdbGetPrinterState(self.raw);
            CpdbError::cstr_to_string(c_state)
        }
    }

    pub fn location(&self) -> Result<String> {
        unsafe {
            let c_location = ffi::cpdbGetPrinterLocation(self.raw);
            CpdbError::cstr_to_string(c_location)
        }
    }

    pub fn description(&self) -> Result<String> {
        unsafe {
            let c_desc = ffi::cpdbGetPrinterDescription(self.raw);
            CpdbError::cstr_to_string(c_desc)
        }
    }

    pub fn accepts_pdf(&self) -> Result<bool> {
        unsafe {
            Ok(ffi::cpdbPrinterAcceptsPDF(self.raw) != 0)
        }
    }

    pub fn try_clone(&self) -> Result<Self> {
        unsafe {
            Self::from_raw(self.raw)
        }
    }

    pub fn backend_name(&self) -> Result<String> {
        unsafe {
            let c_name = ffi::cpdbGetPrinterBackendName(self.raw);
            CpdbError::cstr_to_string(c_name)
        }
    }

    pub fn submit_job(&self, file_path: &str, options: &[(&str, &str)], job_name: &str) -> Result<()> {
        let mut c_options = Vec::with_capacity(options.len());
        let mut keep_alive = Vec::new();

        for (k, v) in options {
            let c_key = std::ffi::CString::new(*k)?;
            let c_val = std::ffi::CString::new(*v)?;
            keep_alive.push(c_key);
            keep_alive.push(c_val);
            
            c_options.push(ffi::cpdb_option_t {
                option_name: keep_alive[keep_alive.len()-2].as_ptr() as *mut i8,
                default_value: keep_alive[keep_alive.len()-1].as_ptr() as *mut i8,
                ..Default::default()
            });
        }

        unsafe {
            let file_cstr = std::ffi::CString::new(file_path)?;
            let job_cstr = std::ffi::CString::new(job_name)?;
            let status = ffi::cpdbPrintFile(
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
                ffi::cpdbReleasePrinter(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}