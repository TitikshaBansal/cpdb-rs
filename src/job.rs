use crate::error::{CpdbError, Result};
use crate::ffi;
use std::ffi::CString;
use std::ptr;

pub struct PrintJob {
    raw: *mut ffi::cpdb_print_job_t,
    id: i32,
}

impl PrintJob {
    /// Create a new print job
    pub fn new(
        printer: &crate::printer::Printer,
        options: &[(&str, &str)],
        file_path: &str,
        job_name: &str,
    ) -> Result<Self> {
        let printer_name = printer.name()?;
        let file_cstr = CString::new(file_path)?;
        let job_cstr = CString::new(job_name)?;
        
        // Create C-compatible options array
        let mut c_options = Vec::with_capacity(options.len());
        let mut keep_alive = Vec::new();

        for (k, v) in options {
            let c_key = CString::new(*k)?;
            let c_val = CString::new(*v)?;
            keep_alive.push(c_key);
            keep_alive.push(c_val);
            
            c_options.push(ffi::cpdb_option_t {
                option_name: keep_alive[keep_alive.len()-2].as_ptr(),
                default_value: keep_alive[keep_alive.len()-1].as_ptr(),
                ..Default::default()
            });
        }

        unsafe {
            let job = ffi::cpdb_new_print_job(
                printer_name.as_ptr(),
                c_options.as_ptr(),
                c_options.len() as i32,
                file_cstr.as_ptr(),
                job_cstr.as_ptr(),
            );

            if job.is_null() {
                Err(CpdbError::JobFailed("Creation failed".into()))
            } else {
                Ok(Self {
                    raw: job,
                    id: -1, // Will be set after submission
                })
            }
        }
    }

    /// Submit the print job
    pub fn submit(&mut self) -> Result<()> {
        unsafe {
            let job_id = ffi::cpdb_submit_print_job(self.raw);
            if job_id < 0 {
                Err(CpdbError::JobFailed("Submission failed".into()))
            } else {
                self.id = job_id;
                Ok(())
            }
        }
    }

    /// Get job ID (only valid after submission)
    pub fn id(&self) -> Option<i32> {
        if self.id > 0 { Some(self.id) } else { None }
    }

    /// Cancel the print job
    pub fn cancel(&mut self) -> Result<()> {
        unsafe {
            if !self.raw.is_null() {
                if self.id > 0 {
                    ffi::cpdb_cancel_job_by_id(self.id);
                }
                ffi::cpdb_delete_print_job(self.raw);
                self.raw = ptr::null_mut();
                self.id = -1;
                Ok(())
            } else {
                Err(CpdbError::JobFailed("Job already completed".into()))
            }
        }
    }
}

impl Drop for PrintJob {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            let _ = self.cancel();
        }
    }
}