//! Safe wrappers around `cpdb_settings_t`, `cpdb_options_t`, and `cpdb_media_t`.

use super::bindings as ffi;
use crate::error::{CpdbError, Result};
use std::ffi::CString;
use std::ptr::NonNull;

// ─── Settings ────────────────────────────────────────────────────────────────

/// A free-standing cpdb settings collection.
///
/// Settings are key/value pairs that can be serialised, persisted, and
/// later applied. For per-printer settings see [`crate::Printer`].
pub struct Settings {
    raw: NonNull<ffi::cpdb_settings_t>,
}

// SAFETY: `Settings` owns its `cpdb_settings_t *`. Moving it across threads
// is fine because no other Rust handle aliases the same C object. Mutators
// take `&mut self`, so shared `&Settings` access from multiple threads
// only ever reads the C state, which is sound.
unsafe impl Send for Settings {}
unsafe impl Sync for Settings {}

impl Settings {
    /// Creates a new empty settings collection.
    pub fn new() -> Result<Self> {
        // SAFETY: `cpdbGetNewSettings` is a constructor with no preconditions.
        let raw = unsafe { ffi::cpdbGetNewSettings() };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(|| CpdbError::BackendError("cpdbGetNewSettings returned null".into()))
    }

    /// Returns an independent deep copy.
    pub fn try_clone(&self) -> Result<Self> {
        let dst = Self::new()?;
        // SAFETY: both pointers are valid, distinct, and live for the duration
        // of this call.
        unsafe { ffi::cpdbCopySettings(self.raw.as_ptr(), dst.raw.as_ptr()) };
        Ok(dst)
    }

    /// Inserts or overwrites a setting.
    pub fn add_setting(&mut self, key: &str, value: &str) -> Result<()> {
        let key = CString::new(key)?;
        let value = CString::new(value)?;
        // SAFETY: pointer is non-null; the two `CString`s outlive the call.
        unsafe { ffi::cpdbAddSetting(self.raw.as_ptr(), key.as_ptr(), value.as_ptr()) };
        Ok(())
    }

    /// Removes a setting.
    ///
    /// Returns `Ok(true)` when the key existed before this call.
    pub fn clear_setting(&mut self, key: &str) -> Result<bool> {
        let key = CString::new(key)?;
        // SAFETY: pointer is non-null; `CString` outlives the call.
        let existed = unsafe { ffi::cpdbClearSetting(self.raw.as_ptr(), key.as_ptr()) };
        Ok(existed != 0)
    }

    /// Persists this settings collection to the cpdb-managed config dir.
    ///
    /// The path is chosen by cpdb-libs (see `cpdbGetUserConfDir`) — callers
    /// cannot override it.
    pub fn save_to_disk(&self) -> Result<()> {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbSaveSettingsToDisk(self.raw.as_ptr()) };
        Ok(())
    }

    /// Loads settings previously written via [`Settings::save_to_disk`].
    pub fn read_from_disk() -> Result<Self> {
        // SAFETY: `cpdbReadSettingsFromDisk` has no preconditions.
        let raw = unsafe { ffi::cpdbReadSettingsFromDisk() };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(|| CpdbError::BackendError("cpdbReadSettingsFromDisk returned null".into()))
    }

    /// Returns the underlying raw pointer for use within this crate.
    #[doc(hidden)]
    pub fn as_raw(&self) -> *mut ffi::cpdb_settings_t {
        self.raw.as_ptr()
    }
}

impl Drop for Settings {
    fn drop(&mut self) {
        // SAFETY: we own the pointer; cpdb-libs is responsible for freeing
        // the contained hash table.
        unsafe { ffi::cpdbDeleteSettings(self.raw.as_ptr()) };
    }
}

// ─── Options ─────────────────────────────────────────────────────────────────

/// An empty cpdb options container.
///
/// Most callers do not need this directly — printer options should be read
/// via [`crate::Printer::get_options_collection`]. This type exists for
/// the rare case where you need a stand-alone `cpdb_options_t`.
pub struct Options {
    raw: NonNull<ffi::cpdb_options_t>,
}

// SAFETY: same reasoning as `Settings` — owned pointer, no shared aliasing.
unsafe impl Send for Options {}
unsafe impl Sync for Options {}

impl Options {
    /// Creates a new empty options object.
    pub fn new() -> Result<Self> {
        // SAFETY: `cpdbGetNewOptions` has no preconditions.
        let raw = unsafe { ffi::cpdbGetNewOptions() };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(|| CpdbError::BackendError("cpdbGetNewOptions returned null".into()))
    }

    /// Returns the underlying raw pointer for use within this crate.
    #[doc(hidden)]
    pub fn as_raw(&self) -> *mut ffi::cpdb_options_t {
        self.raw.as_ptr()
    }
}

impl Drop for Options {
    fn drop(&mut self) {
        // SAFETY: we own the pointer.
        unsafe { ffi::cpdbDeleteOptions(self.raw.as_ptr()) };
    }
}

// ─── Media ───────────────────────────────────────────────────────────────────

/// A stand-alone cpdb media descriptor.
///
/// Note that [`crate::Printer::get_media`] returns a *borrowed* media
/// pointer that must not be wrapped in this type — only construct `Media`
/// from a pointer you intend to free with `cpdbDeleteMedia`.
pub struct Media {
    raw: NonNull<ffi::cpdb_media_t>,
}

// SAFETY: same reasoning as `Settings`.
unsafe impl Send for Media {}
unsafe impl Sync for Media {}

impl Media {
    /// Wraps an owned `cpdb_media_t *`.
    ///
    /// # Safety
    /// `raw` must be non-null, well-formed, and not aliased by any other
    /// Rust or C handle; this `Media` will free it with `cpdbDeleteMedia`
    /// on drop.
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_media_t) -> Result<Self> {
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or(CpdbError::NullPointer)
    }

    /// Returns the underlying raw pointer for use within this crate.
    #[doc(hidden)]
    pub fn as_raw(&self) -> *mut ffi::cpdb_media_t {
        self.raw.as_ptr()
    }
}

impl Drop for Media {
    fn drop(&mut self) {
        // SAFETY: we own the pointer.
        unsafe { ffi::cpdbDeleteMedia(self.raw.as_ptr()) };
    }
}
