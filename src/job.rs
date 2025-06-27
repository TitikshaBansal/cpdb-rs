use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::util;
use std::ffi::CString;
use std::ptr;

pub struct PrintJob {
    raw: *mut ffi::cpdb_print_job_t,
    id: i32,
}

impl PrintJob {
    pub fn new(
        printer_name: &str,
        options: &[(&str, &str)],
        job_name: &str,
    ) -> Result<Self> {
        let c_printer_name = CString::new(printer_name)?;
        let c_job_name = CString::new(job_name)?;
        let c_options = util::to_c_options(options)?;
        
        unsafe {
            let job = ffi::cpdbNewPrintJob(
                c_printer_name.as_ptr() as *mut i8,
                c_options.as_ptr(),
                c_options.len() as i32,
                c_job_name.as_ptr() as *mut i8,
            );

            util::free_c_options(c_options);

            if job.is_null() {
                Err(CpdbError::JobFailed("Creation failed".into()))
            } else {
                Ok(Self {
                    raw: job,
                    id: -1,
                })
            }
        }
    }

    pub fn submit_with_file(&mut self, file_path: &str) -> Result<()> {
        let file_cstr = CString::new(file_path)?;
        unsafe {
            let job_id = ffi::cpdbSubmitPrintJobWithFile(self.raw, file_cstr.as_ptr());
            if job_id < 0 {
                Err(CpdbError::JobFailed("Submission failed".into()))
            } else {
                self.id = job_id;
                Ok(())
            }
        }
    }

    pub fn id(&self) -> Option<i32> {
        if self.id > 0 { Some(self.id) } else { None }
    }

    pub fn cancel(&mut self) -> Result<()> {
        unsafe {
            if !self.raw.is_null() {
                if self.id > 0 {
                    ffi::cpdbCancelJobById(self.id);
                }
                ffi::cpdbDeletePrintJob(self.raw);
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