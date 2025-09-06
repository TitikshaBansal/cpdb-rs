use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::util;
use std::ffi::{CString, CStr};
use std::ptr;

/// Represents printer settings/options in a safe Rust wrapper
pub struct Settings {
    raw: *mut ffi::cpdb_settings_t,
}

unsafe impl Send for Settings {}
unsafe impl Sync for Settings {}

impl Settings {
    /// Creates a new settings object
    pub fn new() -> Result<Self> {
        unsafe {
            let raw = ffi::cpdbGetNewSettings();
            if raw.is_null() {
                Err(CpdbError::BackendError("Failed to create settings object".into()))
            } else {
                Ok(Self { raw })
            }
        }
    }

    /// Creates a copy of the settings
    pub fn copy(&self) -> Result<Self> {
        if self.raw.is_null() {
            return Err(CpdbError::NullPointer);
        }
        unsafe {
            let copy_raw = ffi::cpdbCopySettings(self.raw);
            if copy_raw.is_null() {
                Err(CpdbError::BackendError("Failed to copy settings".into()))
            } else {
                Ok(Self { raw: copy_raw })
            }
        }
    }

    /// Adds a setting to the settings object
    pub fn add_setting(&mut self, key: &str, value: &str) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::NullPointer);
        }
        let c_key = CString::new(key)?;
        let c_value = CString::new(value)?;
        
        unsafe {
            ffi::cpdbAddSetting(self.raw, c_key.as_ptr(), c_value.as_ptr());
        }
        Ok(())
    }

    /// Clears a setting from the settings object
    pub fn clear_setting(&mut self, key: &str) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::NullPointer);
        }
        let c_key = CString::new(key)?;
        
        unsafe {
            ffi::cpdbClearSetting(self.raw, c_key.as_ptr());
        }
        Ok(())
    }

    /// Serializes settings to GVariant format
    pub fn serialize_to_gvariant(&self) -> Result<*mut ffi::GVariant> {
        if self.raw.is_null() {
            return Err(CpdbError::NullPointer);
        }
        unsafe {
            let variant = ffi::cpdbSerializeToGVariant(self.raw);
            if variant.is_null() {
                Err(CpdbError::BackendError("Failed to serialize settings".into()))
            } else {
                Ok(variant)
            }
        }
    }

    /// Saves settings to disk
    pub fn save_to_disk(&self, filename: &str) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::NullPointer);
        }
        let c_filename = CString::new(filename)?;
        
        unsafe {
            let result = ffi::cpdbSaveSettingsToDisk(self.raw, c_filename.as_ptr());
            if result == 0 {
                Ok(())
            } else {
                Err(CpdbError::BackendError(format!("Failed to save settings to disk: {}", result)))
            }
        }
    }

    /// Reads settings from disk
    pub fn read_from_disk(filename: &str) -> Result<Self> {
        let c_filename = CString::new(filename)?;
        
        unsafe {
            let raw = ffi::cpdbReadSettingsFromDisk(c_filename.as_ptr());
            if raw.is_null() {
                Err(CpdbError::BackendError("Failed to read settings from disk".into()))
            } else {
                Ok(Self { raw })
            }
        }
    }

    /// Gets the raw pointer (for internal use)
    pub fn as_raw(&self) -> *mut ffi::cpdb_settings_t {
        self.raw
    }
}

impl Drop for Settings {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteSettings(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}

impl Clone for Settings {
    fn clone(&self) -> Self {
        self.copy().expect("Failed to clone settings")
    }
}

/// Represents printer options in a safe Rust wrapper
pub struct Options {
    raw: *mut ffi::cpdb_options_t,
}

unsafe impl Send for Options {}
unsafe impl Sync for Options {}

impl Options {
    /// Creates a new options object
    pub fn new() -> Result<Self> {
        unsafe {
            let raw = ffi::cpdbGetNewOptions();
            if raw.is_null() {
                Err(CpdbError::BackendError("Failed to create options object".into()))
            } else {
                Ok(Self { raw })
            }
        }
    }

    /// Gets the raw pointer (for internal use)
    pub fn as_raw(&self) -> *mut ffi::cpdb_options_t {
        self.raw
    }
}

impl Drop for Options {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteOptions(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}

/// Represents media information in a safe Rust wrapper
pub struct Media {
    raw: *mut ffi::cpdb_media_t,
}

unsafe impl Send for Media {}
unsafe impl Sync for Media {}

impl Media {
    /// Creates a new media object from raw pointer
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_media_t) -> Result<Self> {
        if raw.is_null() {
            Err(CpdbError::NullPointer)
        } else {
            Ok(Self { raw })
        }
    }

    /// Gets the raw pointer (for internal use)
    pub fn as_raw(&self) -> *mut ffi::cpdb_media_t {
        self.raw
    }
}

impl Drop for Media {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteMedia(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}
