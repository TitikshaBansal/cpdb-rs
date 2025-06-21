use crate::error::{CpdbError, Result};
use crate::ffi;
use std::ffi::CStr;
use std::ptr;

pub struct Printer {
    raw: *mut ffi::cpdb_printer_obj_t,
}

impl Printer {
    /// Create from raw pointer (unsafe)
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_printer_obj_t) -> Result<Self> {
        if raw.is_null() {
            Err(CpdbError::NullPointer)
        } else {
            Ok(Self { raw })
        }
    }

    /// Get printer name
    pub fn name(&self) -> Result<String> {
        unsafe {
            let c_name = ffi::cpdbGetPrinterName(self.raw);
            CpdbError::cstr_to_string(c_name)
        }
    }

    /// Get printer state
    pub fn state(&self) -> Result<String> {
        unsafe {
            let c_state = ffi::cpdbGetPrinterState(self.raw);
            CpdbError::cstr_to_string(c_state)
        }
    }

    /// Get printer location
    pub fn location(&self) -> Result<String> {
        unsafe {
            let c_location = ffi::cpdbGetPrinterLocation(self.raw);
            CpdbError::cstr_to_string(c_location)
        }
    }

    /// Get printer description
    pub fn description(&self) -> Result<String> {
        unsafe {
            let c_desc = ffi::cpdbGetPrinterDescription(self.raw);
            CpdbError::cstr_to_string(c_desc)
        }
    }

    /// Check if printer accepts PDF
    pub fn accepts_pdf(&self) -> Result<bool> {
        unsafe {
            Ok(ffi::cpdbPrinterAcceptsPDF(self.raw) != 0)
        }
    }

    /// Submit a print job
    pub fn submit_job(&self, file_path: &str, options: &[(&str, &str)]) -> Result<()> {
        // Create C-compatible options array
        let mut c_options = Vec::with_capacity(options.len());
        for (k, v) in options {
            c_options.push(ffi::cpdb_option_t {
                name: k.as_ptr() as *const i8,
                value: v.as_ptr() as *const i8,
            });
        }

        unsafe {
            let file_cstr = std::ffi::CString::new(file_path)?;
            let status = ffi::cpdbPrintFile(
                self.raw,
                file_cstr.as_ptr(),
                c_options.as_ptr(),
                c_options.len() as i32,
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
                ffi::cpdbDeletePrinterObject(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}