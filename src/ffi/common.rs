//! Library-wide entry points: version query, one-shot initialisation,
//! and the small set of path/config helpers cpdb-libs ships.

use super::bindings as ffi;
use super::util;
use crate::error::{CpdbError, Result};
use std::ffi::{CStr, CString};

/// Returns the version of the linked cpdb-libs C library.
pub fn version() -> Result<String> {
    // SAFETY: `cpdbGetVersion` returns a borrowed static `const char *`.
    let raw = unsafe { ffi::cpdbGetVersion() };
    if raw.is_null() {
        return Err(CpdbError::NullPointer);
    }
    unsafe { util::cstr_to_string(raw) }
}

/// Initialises cpdb-libs.
///
/// Idempotent — safe to call multiple times. Call once at process startup
/// before any other cpdb-rs API.
pub fn init() {
    // SAFETY: `cpdbInit` takes no arguments and is documented as
    // idempotent.
    unsafe { ffi::cpdbInit() };
}

// ─── Path / config helpers ───────────────────────────────────────────────────

/// Returns the user-scope configuration directory cpdb-libs uses for
/// persisted settings.
pub fn user_config_dir() -> Result<String> {
    // SAFETY: returns a `g_strdup`'d string we own.
    unsafe { util::cstr_to_string_and_g_free(ffi::cpdbGetUserConfDir()) }
}

/// Returns the system-scope configuration directory cpdb-libs uses for
/// shared settings.
pub fn system_config_dir() -> Result<String> {
    // SAFETY: returns a `g_strdup`'d string we own.
    unsafe { util::cstr_to_string_and_g_free(ffi::cpdbGetSysConfDir()) }
}

/// Resolves `path` to an absolute path, expanding `~` and relative
/// segments according to cpdb-libs' rules.
pub fn absolute_path(path: &str) -> Result<String> {
    let c_path = CString::new(path)?;
    // SAFETY: returns a `g_strdup`'d string we own.
    unsafe { util::cstr_to_string_and_g_free(ffi::cpdbGetAbsolutePath(c_path.as_ptr())) }
}

/// Concatenates two strings with a separator (cpdb-libs convention).
pub fn concat_sep(left: &str, right: &str) -> Result<String> {
    let l = CString::new(left)?;
    let r = CString::new(right)?;
    // SAFETY: returns a `g_strdup`'d string we own.
    unsafe { util::cstr_to_string_and_g_free(ffi::cpdbConcatSep(l.as_ptr(), r.as_ptr())) }
}

/// Concatenates two path segments (cpdb-libs convention).
pub fn concat_path(parent: &str, child: &str) -> Result<String> {
    let p = CString::new(parent)?;
    let c = CString::new(child)?;
    // SAFETY: returns a `g_strdup`'d string we own.
    unsafe { util::cstr_to_string_and_g_free(ffi::cpdbConcatPath(p.as_ptr(), c.as_ptr())) }
}

/// Returns the standard IPP group name for `option_name`, or `None` when
/// the option is not in cpdb-libs' built-in mapping.
pub fn option_group(option_name: &str) -> Result<Option<String>> {
    let c_opt = CString::new(option_name)?;
    // SAFETY: returns a `g_strdup`'d string we own; null indicates "not mapped".
    let raw = unsafe { ffi::cpdbGetGroup(c_opt.as_ptr()) };
    if raw.is_null() {
        Ok(None)
    } else {
        unsafe { util::cstr_to_string_and_g_free(raw) }.map(Some)
    }
}
