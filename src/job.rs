use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::printer::Printer;
use std::ffi::CString;
use std::ptr;

pub struct PrintJob {
    raw: *mut ffi::cpdb_print_job_t,
    printer_name: String,
}

impl PrintJob {
    /// Create a new print job
    pub fn new(
        printer: &Printer,
        options: &[(&str, &str)],
        file_path: &str,
    ) -> Result<Self> {
        let printer_name = printer.name()?;
        let file_cstr = CString::new(file_path)?;
        
        // Create C-compatible options array
        let mut c_options = Vec::with_capacity(options.len());
        for (k, v) in options {
            c_options.push(ffi::cpdb_option_t {
                name: k.as_ptr() as *const i8,
                value: v.as_ptr() as *const i8,
            });
        }

        unsafe {
            let job = ffi::cpdbNewPrintJob(
                printer_name.as_ptr() as *const i8,
                c_options.as_ptr(),
                c_options.len() as i32,
                file_cstr.as_ptr(),
            );

            if job.is_null() {
                Err(CpdbError::JobFailed("Creation failed".into()))
            } else {
                Ok(Self {
                    raw: job,
                    printer_name,
                })
            }
        }
    }

    /// Submit the print job
    pub fn submit(&self) -> Result<()> {
        unsafe {
            if ffi::cpdbSubmitPrintJob(self.raw) == 0 {
                Ok(())
            } else {
                Err(CpdbError::JobFailed("Submission failed".into()))
            }
        }
    }

    /// Get job ID
    pub fn id(&self) -> Result<i32> {
        unsafe {
            Ok(ffi::cpdbGetJobId(self.raw))
        }
    }

    /// Cancel the print job
    pub fn cancel(&mut self) -> Result<()> {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbCancelPrintJob(self.raw);
                self.raw = ptr::null_mut();
                Ok(())
            } else {
                Err(CpdbError::JobFailed("Job already completed".into()))
            }
        }
    }
}

impl Drop for PrintJob {
    fn drop(&mut self) {
        self.cancel().ok(); // Best-effort cancellation
    }
}