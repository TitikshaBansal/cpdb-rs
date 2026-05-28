//! Library-wide entry points: version query and one-shot initialisation.

use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::util;

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
