use crate::error::{CpdbError, Result};
use crate::options::OptionsCollection;
use crate::util;
use crate::{ffi, Frontend};
use libc::c_char;
use std::ffi::CString;
use std::ptr;

/// A handle to a CPDB printer object.
///
/// # Ownership model
///
/// Printers come in two flavors:
///
/// - **Borrowed** (created by [`Frontend::find_printer`], [`Frontend::get_printers`], etc.):
///   The underlying C object is owned by the frontend's internal hash table.
///   Rust will **not** free it on drop. These must not outlive the [`Frontend`].
///
/// - **Owned** (created by [`Printer::load_from_file`]):
///   The underlying C object was allocated independently of any frontend.
///   Rust **will** free it via `cpdbDeletePrinterObj` on drop.
///
/// `Printer` intentionally does **not** implement `Clone`. Sharing a borrowed
/// printer across scopes should use `&Printer`. If shared ownership across
/// threads is needed for an owned printer, wrap it in `Arc<Mutex<Printer>>`.
#[derive(Debug)]
pub struct Printer {
    raw: *mut ffi::cpdb_printer_obj_t,
    owned: bool,
}

unsafe impl Send for Printer {}
unsafe impl Sync for Printer {}

impl Printer {
    // ─── Constructors ────────────────────────────────────────────────────────

    /// Wraps a raw pointer as a *borrowed* printer (will NOT be freed on drop).
    pub(crate) fn from_raw_borrowed(raw: *mut ffi::cpdb_printer_obj_t) -> Result<Self> {
        if raw.is_null() {
            Err(CpdbError::NullPointer)
        } else {
            Ok(Self { raw, owned: false })
        }
    }

    /// Wraps a raw pointer as an *owned* printer (will be freed on drop).
    pub(crate) fn from_raw_owned(raw: *mut ffi::cpdb_printer_obj_t) -> Result<Self> {
        if raw.is_null() {
            Err(CpdbError::NullPointer)
        } else {
            Ok(Self { raw, owned: true })
        }
    }

    /// Exposes the raw pointer for use within this crate.
    pub fn as_raw(&self) -> *mut ffi::cpdb_printer_obj_t {
        self.raw
    }

    // ─── Field accessors ─────────────────────────────────────────────────────

    fn get_string_field<F>(
        &self,
        field_accessor: F,
        field_name_for_error: &'static str,
    ) -> Result<String>
    where
        F: FnOnce(*mut ffi::cpdb_printer_obj_t) -> *const c_char,
    {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(format!(
                "Printer object pointer is null when accessing {}",
                field_name_for_error
            )));
        }
        let c_ptr = field_accessor(self.raw);
        match unsafe { util::cstr_to_string(c_ptr) } {
            Ok(s) => Ok(s),
            Err(CpdbError::NullPointer) => Ok(String::new()),
            Err(e) => Err(e),
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

    pub fn backend_name(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).backend_name }, "backend_name")
    }

    pub fn current_state_field(&self) -> Result<String> {
        self.get_string_field(|p| unsafe { (*p).state }, "state_field")
    }

    // ─── State ───────────────────────────────────────────────────────────────

    /// Returns the current printer state string (e.g. `"idle"`, `"processing"`).
    ///
    /// The returned pointer is borrowed from an internal field — do NOT free it.
    pub fn get_updated_state(&self) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for get_updated_state".to_string(),
            ));
        }
        unsafe {
            let c_state_ptr = ffi::cpdbGetState(self.raw);
            util::cstr_to_string(c_state_ptr)
        }
    }

    pub fn is_accepting_jobs(&self) -> Result<bool> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for is_accepting_jobs".to_string(),
            ));
        }
        unsafe { Ok(ffi::cpdbIsAcceptingJobs(self.raw) != 0) }
    }

    pub fn accepts_pdf(&self) -> Result<bool> {
        let model = self.make_and_model().unwrap_or_default();
        Ok(model.to_lowercase().contains("pdf"))
    }

    // ─── Defaults ────────────────────────────────────────────────────────────

    /// Sets this printer as the user default.
    pub fn set_user_default(&self) -> bool {
        unsafe { ffi::cpdbSetUserDefaultPrinter(self.raw) != 0 }
    }

    /// Sets this printer as the system default.
    pub fn set_system_default(&self) -> bool {
        unsafe { ffi::cpdbSetSystemDefaultPrinter(self.raw) != 0 }
    }

    // ─── Job submission ───────────────────────────────────────────────────────

    /// Prints a file with no extra options. Returns the job ID string.
    pub fn print_single_file(&self, file_path: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for print_single_file".to_string(),
            ));
        }
        let c_file_path = CString::new(file_path)?;
        unsafe {
            let job_id_ptr = ffi::cpdbPrintFile(self.raw, c_file_path.as_ptr());
            util::cstr_to_string_and_g_free(job_id_ptr)
        }
    }

    /// Submits a print job with a title and key-value options.
    pub fn submit_job(
        &self,
        file_path: &str,
        _options: &[(&str, &str)],
        job_name: &str,
    ) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for submit_job".to_string(),
            ));
        }
        let file_cstr = CString::new(file_path)?;
        let job_cstr = CString::new(job_name)?;
        unsafe {
            let job_id_ptr =
                ffi::cpdbPrintFileWithJobTitle(self.raw, file_cstr.as_ptr(), job_cstr.as_ptr());
            if job_id_ptr.is_null() {
                Err(CpdbError::BackendError(
                    "Job submission failed - no job ID returned".to_string(),
                ))
            } else {
                libc::free(job_id_ptr as *mut libc::c_void);
                Ok(())
            }
        }
    }

    // ─── Options ─────────────────────────────────────────────────────────────

    /// Gets a specific option's default value.
    pub fn get_option(&self, option_name: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for get_option".to_string(),
            ));
        }
        let c_option_name = CString::new(option_name)?;
        unsafe {
            let option_ptr = ffi::cpdbGetOption(self.raw, c_option_name.as_ptr());
            if option_ptr.is_null() {
                Err(CpdbError::BackendError(format!(
                    "Option '{}' not found",
                    option_name
                )))
            } else {
                let default_value = (*option_ptr).default_value;
                if default_value.is_null() {
                    Ok("NA".to_string())
                } else {
                    util::cstr_to_string(default_value)
                }
            }
        }
    }

    /// Gets the default value for a named option.
    pub fn get_default(&self, option_name: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for get_default".to_string(),
            ));
        }
        let c_option_name = CString::new(option_name)?;
        unsafe {
            let value_ptr = ffi::cpdbGetDefault(self.raw, c_option_name.as_ptr());
            util::cstr_to_string_and_g_free(value_ptr)
        }
    }

    /// Gets the current (active) value for a named option.
    pub fn get_current(&self, option_name: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for get_current".to_string(),
            ));
        }
        let c_option_name = CString::new(option_name)?;
        unsafe {
            let value_ptr = ffi::cpdbGetCurrent(self.raw, c_option_name.as_ptr());
            util::cstr_to_string_and_g_free(value_ptr)
        }
    }

    /// Fetches full option details from the backend. Call this before
    /// `get_options_collection` to ensure the options table is populated.
    pub fn acquire_details(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for acquire_details".to_string(),
            ));
        }
        // cpdbAcquireDetails is async; the caller can pass a callback via
        // the raw FFI if they need a completion notification.
        unsafe {
            ffi::cpdbAcquireDetails(self.raw, None, std::ptr::null_mut());
        }
        Ok(())
    }

    /// Returns an owned snapshot of all printer options.
    ///
    /// Call [`acquire_details`] first to ensure the options table is populated
    /// from the backend. After this call returns the collection holds no raw
    /// pointers and can be freely stored or moved.
    ///
    /// [`acquire_details`]: Self::acquire_details
    pub fn get_options_collection(&self) -> Result<OptionsCollection> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for get_options_collection".to_string(),
            ));
        }
        unsafe {
            let opts_ptr = ffi::cpdbGetAllOptions(self.raw);
            if opts_ptr.is_null() {
                return Err(CpdbError::BackendError(
                    "cpdbGetAllOptions returned null — call acquire_details() first".to_string(),
                ));
            }
            OptionsCollection::from_raw(opts_ptr)
        }
    }

    // ─── Settings ────────────────────────────────────────────────────────────

    /// Gets the current setting value for an option.
    ///
    /// The returned pointer is borrowed from the internal hash table — do NOT free it.
    pub fn get_setting(&self, option_name: &str) -> Result<Option<String>> {
        let c_opt = CString::new(option_name)?;
        unsafe {
            let val = ffi::cpdbGetSetting(self.raw, c_opt.as_ptr());
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(util::cstr_to_string(val)?))
            }
        }
    }

    /// Adds or overwrites a per-printer setting (e.g. `"copies"` → `"2"`).
    pub fn add_setting(&self, name: &str, value: &str) -> Result<()> {
        let c_name = CString::new(name)?;
        let c_val = CString::new(value)?;
        unsafe {
            ffi::cpdbAddSettingToPrinter(self.raw, c_name.as_ptr(), c_val.as_ptr());
        }
        Ok(())
    }

    /// Removes a per-printer setting.
    pub fn clear_setting(&self, name: &str) -> Result<()> {
        let c_name = CString::new(name)?;
        unsafe {
            ffi::cpdbClearSettingFromPrinter(self.raw, c_name.as_ptr());
        }
        Ok(())
    }

    // ─── Media ───────────────────────────────────────────────────────────────

    /// Gets media information for a named media type.
    pub fn get_media(&self, media_name: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for get_media".to_string(),
            ));
        }
        let c_media_name = CString::new(media_name)?;
        unsafe {
            let media_ptr = ffi::cpdbGetMedia(self.raw, c_media_name.as_ptr());
            if media_ptr.is_null() {
                Err(CpdbError::BackendError(format!(
                    "Media '{}' not found",
                    media_name
                )))
            } else {
                let name = (*media_ptr).name;
                if name.is_null() {
                    Ok("Unknown".to_string())
                } else {
                    util::cstr_to_string(name)
                }
            }
        }
    }

    /// Returns `(width_hundredths_mm, length_hundredths_mm)`.
    pub fn get_media_size(&self, media_name: &str) -> Result<(i32, i32)> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for get_media_size".to_string(),
            ));
        }
        let c_media_name = CString::new(media_name)?;
        unsafe {
            let mut width: i32 = 0;
            let mut length: i32 = 0;
            let result =
                ffi::cpdbGetMediaSize(self.raw, c_media_name.as_ptr(), &mut width, &mut length);
            if result == 0 {
                Ok((width, length))
            } else {
                Err(CpdbError::BackendError(format!(
                    "Failed to get media size for '{}'",
                    media_name
                )))
            }
        }
    }

    /// Returns margin info as a formatted string.
    pub fn get_media_margins(&self, media_name: &str) -> Result<String> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for get_media_margins".to_string(),
            ));
        }
        let c_media_name = CString::new(media_name)?;
        unsafe {
            let mut margins_ptr: *mut ffi::cpdb_margin_t = ptr::null_mut();
            let result =
                ffi::cpdbGetMediaMargins(self.raw, c_media_name.as_ptr(), &mut margins_ptr);
            if result == 0 && !margins_ptr.is_null() {
                let m = &*margins_ptr;
                Ok(format!(
                    "top: {}, bottom: {}, left: {}, right: {}",
                    m.top, m.bottom, m.left, m.right
                ))
            } else {
                Err(CpdbError::BackendError(format!(
                    "Failed to get media margins for '{}'",
                    media_name
                )))
            }
        }
    }

    // ─── Translations ────────────────────────────────────────────────────────

    /// Fetches translations from the backend asynchronously.
    pub fn acquire_translations(&self, locale: &str) -> Result<()> {
        let c_locale = CString::new(locale)?;
        unsafe {
            ffi::cpdbAcquireTranslations(
                self.raw,
                c_locale.as_ptr(),
                None,
                std::ptr::null_mut(),
            );
        }
        Ok(())
    }

    /// Returns the human-readable label for an option name.
    pub fn get_option_translation(&self, option: &str, locale: &str) -> Result<Option<String>> {
        let c_opt = CString::new(option)?;
        let c_locale = CString::new(locale)?;
        unsafe {
            let t = ffi::cpdbGetOptionTranslation(self.raw, c_opt.as_ptr(), c_locale.as_ptr());
            if t.is_null() {
                Ok(None)
            } else {
                let s = util::cstr_to_string(t)?;
                glib_sys::g_free(t as glib_sys::gpointer);
                Ok(Some(s))
            }
        }
    }

    /// Returns the human-readable label for a specific option choice.
    pub fn get_choice_translation(
        &self,
        option: &str,
        choice: &str,
        locale: &str,
    ) -> Result<Option<String>> {
        let c_opt = CString::new(option)?;
        let c_choice = CString::new(choice)?;
        let c_locale = CString::new(locale)?;
        unsafe {
            let t = ffi::cpdbGetChoiceTranslation(
                self.raw,
                c_opt.as_ptr(),
                c_choice.as_ptr(),
                c_locale.as_ptr(),
            );
            if t.is_null() {
                Ok(None)
            } else {
                let s = util::cstr_to_string(t)?;
                glib_sys::g_free(t as glib_sys::gpointer);
                Ok(Some(s))
            }
        }
    }

    /// Returns the human-readable label for an option group.
    pub fn get_group_translation(&self, group: &str, locale: &str) -> Result<Option<String>> {
        let c_group = CString::new(group)?;
        let c_locale = CString::new(locale)?;
        unsafe {
            let t = ffi::cpdbGetGroupTranslation(self.raw, c_group.as_ptr(), c_locale.as_ptr());
            if t.is_null() {
                Ok(None)
            } else {
                let s = util::cstr_to_string(t)?;
                glib_sys::g_free(t as glib_sys::gpointer);
                Ok(Some(s))
            }
        }
    }

    /// Fetches all translations for this printer's locale.
    pub fn get_all_translations(&self, locale: &str) -> Result<()> {
        let c_locale = CString::new(locale)?;
        unsafe {
            ffi::cpdbGetAllTranslations(self.raw, c_locale.as_ptr());
        }
        Ok(())
    }

    // ─── Persistence ─────────────────────────────────────────────────────────

    /// Saves printer configuration to a file.
    pub fn save_to_file(&self, filename: &str, frontend: &Frontend) -> Result<()> {
        self.pickle_to_file(filename, frontend)
    }

    /// Serialises this printer to disk so it can be resurrected later.
    pub fn pickle_to_file(&self, path: &str, frontend: &Frontend) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::BackendError(
                "Printer object pointer is null for pickle_to_file".to_string(),
            ));
        }
        let c_path = CString::new(path)?;
        unsafe {
            ffi::cpdbPicklePrinterToFile(self.raw, c_path.as_ptr(), frontend.as_raw());
        }
        Ok(())
    }

    /// Loads a printer that was previously serialised with `pickle_to_file`.
    /// The returned printer is *owned*.
    pub fn load_from_file(filename: &str) -> Result<Self> {
        let c_filename = CString::new(filename)?;
        unsafe {
            let printer_ptr = ffi::cpdbResurrectPrinterFromFile(c_filename.as_ptr());
            if printer_ptr.is_null() {
                Err(CpdbError::BackendError(
                    "Failed to load printer from file".into(),
                ))
            } else {
                Self::from_raw_owned(printer_ptr)
            }
        }
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        if self.owned && !self.raw.is_null() {
            unsafe {
                ffi::cpdbDeletePrinterObj(self.raw);
            }
            self.raw = std::ptr::null_mut();
        }
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_raw_borrowed_rejects_null() {
        let result = Printer::from_raw_borrowed(std::ptr::null_mut());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CpdbError::NullPointer));
    }

    #[test]
    fn from_raw_owned_rejects_null() {
        let result = Printer::from_raw_owned(std::ptr::null_mut());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CpdbError::NullPointer));
    }

    #[test]
    fn load_from_nonexistent_file_returns_error() {
        let result = Printer::load_from_file("/tmp/nonexistent-cpdb-printer-pickle-test");
        assert!(result.is_err());
    }
}
