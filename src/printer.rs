use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::util;
use std::ffi::{CString, CStr};
use std::ptr;

pub struct Printer {
    raw: *mut ffi::cpdb_printer_obj_t,
}

unsafe impl Send for Printer {}
unsafe impl Sync for Printer {}

impl Printer {
    /// Creates a Printer wrapper from a raw pointer.
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_printer_obj_t) -> Result<Self> {
        if raw.is_null() {
            Err(CpdbError::NullPointer)
        } else {
            if cfg!(feature = "refcounted_printer") {
                ffi::cpdbAcquirePrinter(raw); 
            }
            Ok(Self { raw })
        }
    }

    /// Accesses the printer's ID (often a unique name or path).
    pub fn id(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).id }, "id")
    }

    /// Accesses the printer's human-readable name.
    pub fn name(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).name }, "name")
    }

    /// Accesses the printer's physical location description.
    pub fn location(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).location }, "location")
    }

    /// Accesses the printer's additional information or comments (often called 'info' or 'description').
    pub fn description(&self) -> Result<String> { // Maps to 'info' field in C struct
        self.get_string_field(|p| unsafe { (*p).info }, "info")
    }

    /// Accesses the printer's make and model string.
    pub fn make_and_model(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).make_and_model }, "make_and_model")
    }

    /// Accesses the printer's current state directly from its struct field.
    pub fn current_state_field(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).state }, "state_field")
    }

    /// Fetches and returns the printer's current state by calling `cpdbGetState`.
    pub fn get_updated_state(&self) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::InvalidPrinter{ reason: "Printer object pointer is null".to_string() });
        }
        unsafe {
            let c_state_ptr = ffi::cpdbGetState(self.raw);
            if c_state_ptr.is_null() {
                return Err(CpdbError::OperationFailed("cpdbGetState returned null".to_string()));
            }
            let result = CStr::from_ptr(c_state_ptr).to_str().map(|s| s.to_string()).map_err(CpdbError::from);
            ffi::g_free(c_state_ptr as *mut libc::c_void);
            result
        }
    }

    /// Checks if the printer is currently accepting print jobs by calling `cpdbIsAcceptingJobs`.
    pub fn is_accepting_jobs(&self) -> Result<bool> {
        if self.raw.is_null() {
            return Err(CpdbError::InvalidPrinter{ reason: "Printer object pointer is null".to_string() });
        }
        unsafe {
            let accepting = ffi::cpdbIsAcceptingJobs(self.raw);
            Ok(accepting != 0)
        }
    }

    /// Submits a single file for printing using the simple `cpdbPrintFile` C API.
    /// Returns a Job ID string if successful.
    pub fn print_single_file(&self, file_path: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::InvalidPrinter{ reason: "Printer object pointer is null".to_string() });
        }
        let c_file_path = CString::new(file_path)?;
        unsafe {
            let job_id_ptr = ffi::cpdbPrintFile(self.raw, c_file_path.as_ptr());
            if job_id_ptr.is_null() {
                return Err(CpdbError::JobFailed { message: "cpdbPrintFile returned null, failed to submit job".to_string() });
            }
            let result = CStr::from_ptr(job_id_ptr).to_str().map(|s| s.to_string()).map_err(CpdbError::from);
            ffi::g_free(job_id_ptr as *mut libc::c_void); 
            result
        }
    }

    /// Helper function to get string fields from the raw printer object.
    fn get_string_field<F>(&self, field_accessor: F, field_name_for_error: &'static str) -> Result<String>
    where
        F: FnOnce(*mut ffi::cpdb_printer_obj_t) -> *const libc::c_char,
    {
        if self.raw.is_null() {
            return Err(CpdbError::InvalidPrinter{ reason: format!("Printer object pointer is null when accessing {}", field_name_for_error) });
        }
        unsafe {
            let c_ptr = field_accessor(self.raw);
            if c_ptr.is_null() {
                return Ok(String::new()); 
            }
            CStr::from_ptr(c_ptr).to_str().map(|s| s.to_string()).map_err(CpdbError::from)
        }
    }
    /// Returns the name of the backend that this printer uses.
    pub fn backend_name(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).backend_name }, "backend_name")
    }
    /// Checks if the printer accepts PDF files by checking its make and model string.
    pub fn accepts_pdf(&self) -> Result<bool> {
        let model = self.make_and_model().unwrap_or_default();
        Ok(model.to_lowercase().contains("pdf"))
    }

    pub fn submit_job(&self, file_path: &str, options: &[(&str, &str)], job_name: &str) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::InvalidPrinter{ reason: "Printer object pointer is null".to_string() });
        }
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

    pub fn try_clone(&self) -> Result<Self> {
        if self.raw.is_null() {
            return Err(CpdbError::InvalidPrinter{ reason: "Cannot clone null printer object".to_string()});
        }
        if cfg!(feature = "refcounted_printer") {
             unsafe { ffi::cpdbAcquirePrinter(self.raw); }
        }
        Ok(Self { raw: self.raw })
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            if cfg!(feature = "refcounted_printer") { 
                unsafe { ffi::cpdbReleasePrinter(self.raw); }
            }
            self.raw = ptr::null_mut();
        }
    }
}

impl Clone for Printer {
    fn clone(&self) -> Self {
        if self.raw.is_null() {
            panic!("Cannot clone a Printer with a null raw pointer");
        }
        // If cpdb-libs uses refcounting for printers
        if cfg!(feature = "refcounted_printer") {
            unsafe { ffi::cpdbAcquirePrinter(self.raw); }
        }
        Self { raw: self.raw }
    }
}

mod ffi_assumed {
    #[allow(unused_imports)]
    use libc::c_void;
}