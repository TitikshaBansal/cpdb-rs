use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::printer::Printer;
use crate::util;
use std::ffi::CString;
use std::ptr;

pub struct Backend {
    raw: *mut ffi::cpdb_backend_obj_t,
}

impl Backend {
    pub fn new(backend_name: &str) -> Result<Self> {
        let c_name = CString::new(backend_name)?;
        unsafe {
            let raw = ffi::cpdbGetNewBackendObj(c_name.as_ptr());
            if raw.is_null() {
                Err(CpdbError::BackendError("Failed to create backend".into()))
            } else {
                Ok(Self { raw })
            }
        }
    }

    pub fn submit_job(
        &self,
        printer_name: &str,
        file_path: &str,
        options: &[(&str, &str)],
        job_name: &str,
    ) -> Result<()> {
        let c_options = util::to_c_options(options)?;
        let c_printer = CString::new(printer_name)?;
        let c_file = CString::new(file_path)?;
        let c_job = CString::new(job_name)?;
        
        unsafe {
            let status = ffi::cpdbSubmitJob(
                self.raw,
                c_printer.as_ptr(),
                c_file.as_ptr(),
                c_options.as_ptr(),
                c_options.len() as i32,
                c_job.as_ptr(),
            );
            
            util::free_c_options(c_options);
            
            if status == 0 {
                Ok(())
            } else {
                Err(CpdbError::from_status(status, "Backend job submission failed"))
            }
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteBackendObj(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}