use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::printer::Printer;
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

    pub fn get_printer(&self, printer_name: &str) -> Result<Printer> {
        let c_name = CString::new(printer_name)?;
        unsafe {
            let printer_ptr = ffi::cpdbGetPrinterFromBackend(self.raw, c_name.as_ptr());
            if printer_ptr.is_null() {
                Err(CpdbError::InvalidPrinter)
            } else {
                Printer::from_raw(printer_ptr)
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
        let mut c_options = Vec::with_capacity(options.len());
        let mut keep_alive = Vec::new();

        for (k, v) in options {
            let c_key = CString::new(*k)?;
            let c_val = CString::new(*v)?;
            keep_alive.push(c_key);
            keep_alive.push(c_val);
            
            c_options.push(ffi::cpdb_option_t {
                option_name: keep_alive[keep_alive.len()-2].as_ptr() as *mut i8,
                default_value: keep_alive[keep_alive.len()-1].as_ptr() as *mut i8,
                ..Default::default()
            });
        }

        unsafe {
            let c_printer = CString::new(printer_name)?;
            let c_file = CString::new(file_path)?;
            let c_job = CString::new(job_name)?;
            
            let status = ffi::cpdbSubmitJob(
                self.raw,
                c_printer.as_ptr(),
                c_file.as_ptr(),
                c_options.as_ptr(),
                c_options.len() as i32,
                c_job.as_ptr(),
            );
            
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