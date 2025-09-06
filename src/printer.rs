use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::util;
use std::ffi::{CString, CStr};
use std::ptr;
use libc::c_char;

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
            Ok(Self { raw })
        }
    }

    fn get_string_field<F>(&self, field_accessor: F, field_name_for_error: &'static str) -> Result<String>
    where
        F: FnOnce(*mut ffi::cpdb_printer_obj_t) -> *const c_char,
    {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(format!("Printer object pointer is null when accessing {}", field_name_for_error)));
        }
        unsafe {
            let c_ptr = field_accessor(self.raw);
            match util::cstr_to_string(c_ptr) {
                Ok(s) => Ok(s),
                Err(CpdbError::NullPointer) => Ok(String::new()),
                Err(e) => Err(e),
            }
        }
    }

    pub fn id(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).id }, "id")
    }

    pub fn name(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).name }, "name")
    }

    pub fn location(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).location }, "location")
    }

    pub fn description(&self) -> Result<String> { 
        self.get_string_field(|p| unsafe { (*p).info }, "info")
    }

    pub fn make_and_model(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).make_and_model }, "make_and_model")
    }

    pub fn current_state_field(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).state }, "state_field")
    }

    pub fn get_updated_state(&self) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for get_updated_state".to_string()));
        }
        unsafe {
            let c_state_ptr = ffi::cpdbGetState(self.raw);
            util::cstr_to_string_and_g_free(c_state_ptr)
        }
    }

    pub fn is_accepting_jobs(&self) -> Result<bool> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for is_accepting_jobs".to_string()));
        }
        unsafe {
            let accepting = ffi::cpdbIsAcceptingJobs(self.raw);
            Ok(accepting != 0) 
        }
    }

    pub fn print_single_file(&self, file_path: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for print_single_file".to_string()));
        }
        let c_file_path = CString::new(file_path)?;
        unsafe {
            let job_id_ptr = ffi::cpdbPrintFile(self.raw, c_file_path.as_ptr());
            util::cstr_to_string_and_g_free(job_id_ptr)
        }
    }

    pub fn backend_name(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).backend_name }, "backend_name")
    }

    pub fn accepts_pdf(&self) -> Result<bool> {
        let model = self.make_and_model().unwrap_or_default();
        Ok(model.to_lowercase().contains("pdf"))
    }

    pub fn submit_job(&self, file_path: &str, options: &[(&str, &str)], job_name: &str) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for submit_job".to_string()));
        }
        let c_options = util::to_c_options(options)?;
        let file_cstr = CString::new(file_path)?;
        let job_cstr = CString::new(job_name)?;
        
        unsafe {
            let status = ffi::cpdbPrintFileWithJobTitle(
                self.raw,
                file_cstr.as_ptr(),
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
            return Err(CpdbError::BackendError("Cannot clone null printer object".to_string()));
        }
        Ok(Self { raw: self.raw })
    }

    /// Gets all available options for this printer
    pub fn get_all_options(&self) -> Result<Vec<crate::settings::Options>> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for get_all_options".to_string()));
        }
        unsafe {
            let options_ptr = ffi::cpdbGetAllOptions(self.raw);
            if options_ptr.is_null() {
                Ok(Vec::new())
            } else {
                // Note: This is a simplified implementation
                // The actual cpdb-libs API might return a different structure
                Ok(vec![crate::settings::Options::new()?])
            }
        }
    }

    /// Gets a specific option value
    pub fn get_option(&self, option_name: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for get_option".to_string()));
        }
        let c_option_name = CString::new(option_name)?;
        unsafe {
            let value_ptr = ffi::cpdbGetOption(self.raw, c_option_name.as_ptr());
            util::cstr_to_string_and_g_free(value_ptr)
        }
    }

    /// Gets the default value for an option
    pub fn get_default(&self, option_name: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for get_default".to_string()));
        }
        let c_option_name = CString::new(option_name)?;
        unsafe {
            let value_ptr = ffi::cpdbGetDefault(self.raw, c_option_name.as_ptr());
            util::cstr_to_string_and_g_free(value_ptr)
        }
    }

    /// Gets the current value for an option
    pub fn get_current(&self, option_name: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for get_current".to_string()));
        }
        let c_option_name = CString::new(option_name)?;
        unsafe {
            let value_ptr = ffi::cpdbGetCurrent(self.raw, c_option_name.as_ptr());
            util::cstr_to_string_and_g_free(value_ptr)
        }
    }

    /// Gets media information for this printer
    pub fn get_media(&self) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for get_media".to_string()));
        }
        unsafe {
            let media_ptr = ffi::cpdbGetMedia(self.raw);
            util::cstr_to_string_and_g_free(media_ptr)
        }
    }

    /// Gets media size information
    pub fn get_media_size(&self) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for get_media_size".to_string()));
        }
        unsafe {
            let size_ptr = ffi::cpdbGetMediaSize(self.raw);
            util::cstr_to_string_and_g_free(size_ptr)
        }
    }

    /// Gets media margins information
    pub fn get_media_margins(&self) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for get_media_margins".to_string()));
        }
        unsafe {
            let margins_ptr = ffi::cpdbGetMediaMargins(self.raw);
            util::cstr_to_string_and_g_free(margins_ptr)
        }
    }

    /// Saves printer configuration to a file
    pub fn save_to_file(&self, filename: &str) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError("Printer object pointer is null for save_to_file".to_string()));
        }
        let c_filename = CString::new(filename)?;
        unsafe {
            let result = ffi::cpdbPicklePrinterToFile(self.raw, c_filename.as_ptr());
            if result == 0 {
                Ok(())
            } else {
                Err(CpdbError::BackendError(format!("Failed to save printer to file: {}", result)))
            }
        }
    }

    /// Loads printer configuration from a file
    pub fn load_from_file(filename: &str) -> Result<Self> {
        let c_filename = CString::new(filename)?;
        unsafe {
            let printer_ptr = ffi::cpdbResurrectPrinterFromFile(c_filename.as_ptr());
            if printer_ptr.is_null() {
                Err(CpdbError::BackendError("Failed to load printer from file".into()))
            } else {
                Self::from_raw(printer_ptr)
            }
        }
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            self.raw = ptr::null_mut();
        }
    }
}

impl Clone for Printer {
    fn clone(&self) -> Self {
        if self.raw.is_null() {
            panic!("Cannot clone a Printer with a null raw pointer");
        }
        Self { raw: self.raw }
    }
}