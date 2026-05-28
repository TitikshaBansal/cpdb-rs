//! Small FFI utilities shared by the higher-level modules.

use crate::error::{CpdbError, Result};
use crate::ffi;
use libc::c_char;
use std::ffi::{CStr, CString};

/// Converts a borrowed C string into an owned `String`.
///
/// Returns [`CpdbError::NullPointer`] when `ptr` is null. Invalid UTF-8 is
/// replaced with U+FFFD (lossy) — cpdb-libs strings should always be valid
/// UTF-8, but we do not trust that strictly.
///
/// # Safety
/// `ptr` must either be null or point at a NUL-terminated C string that
/// stays valid for the duration of this call.
pub unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String> {
    if ptr.is_null() {
        return Err(CpdbError::NullPointer);
    }
    // SAFETY: caller guarantees `ptr` references a live NUL-terminated string.
    Ok(unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned())
}

/// Same as [`cstr_to_string`] but frees the underlying buffer with `g_free`.
///
/// Use this for return values from cpdb-libs functions that `g_strdup` their
/// result (`cpdbGetDefault`, `cpdbGetCurrent`, `cpdbPrintFile`, ...).
///
/// # Safety
/// `ptr` must be null, or a pointer returned by a GLib allocator that the
/// caller has ownership of (so freeing it is correct).
pub unsafe fn cstr_to_string_and_g_free(ptr: *mut c_char) -> Result<String> {
    if ptr.is_null() {
        return Err(CpdbError::NullPointer);
    }
    // SAFETY: caller guarantees ownership of a GLib-allocated string.
    let owned = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { glib_sys::g_free(ptr as glib_sys::gpointer) };
    Ok(owned)
}

/// A pinned, owned array of `cpdb_option_t` with backing `CString` storage.
///
/// The strings cannot be reallocated after construction, so raw pointers
/// embedded in the `cpdb_option_t` entries stay valid for the lifetime of
/// the `COptions`. Allocation in `to_c_options` therefore cannot invalidate
/// any pointer captured here.
pub struct COptions {
    // Boxed slice: the storage is fixed-length and never grows, so pointer
    // capture is sound for the whole lifetime of the struct.
    _strings: Box<[CString]>,
    options: Box<[ffi::cpdb_option_t]>,
}

impl COptions {
    /// Returns a raw mutable pointer to the underlying option array,
    /// suitable for passing to cpdb-libs functions that expect a
    /// `cpdb_option_t *` plus a length.
    pub fn as_mut_ptr(&mut self) -> *mut ffi::cpdb_option_t {
        self.options.as_mut_ptr()
    }

    /// Number of options held.
    pub fn len(&self) -> usize {
        self.options.len()
    }

    /// `true` when no options are stored.
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }
}

/// Converts a slice of `(name, value)` pairs into a [`COptions`] suitable
/// for passing to cpdb-libs.
///
/// The returned [`COptions`] owns its backing `CString` storage, so the
/// raw pointers embedded in each `cpdb_option_t` remain valid for the
/// `COptions` lifetime.
pub fn to_c_options(pairs: &[(&str, &str)]) -> Result<COptions> {
    let mut strings: Vec<CString> = Vec::with_capacity(pairs.len() * 2);
    for (key, value) in pairs {
        strings.push(CString::new(*key)?);
        strings.push(CString::new(*value)?);
    }
    let strings: Box<[CString]> = strings.into_boxed_slice();

    let mut options: Vec<ffi::cpdb_option_t> = Vec::with_capacity(pairs.len());
    for i in 0..pairs.len() {
        let key_ptr = strings[i * 2].as_ptr() as *mut c_char;
        let val_ptr = strings[i * 2 + 1].as_ptr() as *mut c_char;
        options.push(ffi::cpdb_option_t {
            option_name: key_ptr,
            default_value: val_ptr,
            group_name: std::ptr::null_mut(),
            num_supported: 0,
            supported_values: std::ptr::null_mut(),
        });
    }

    Ok(COptions {
        _strings: strings,
        options: options.into_boxed_slice(),
    })
}

#[cfg(test)]
mod tests {
    //! Pure-Rust unit tests. These do not touch cpdb-libs and are safe
    //! to run under `cargo miri test`.

    use super::*;

    /// Reads back the strings embedded in a `COptions` via the same raw
    /// pointer machinery cpdb-libs uses on the C side. Validates that
    /// `to_c_options` produces a layout consistent with the C ABI.
    fn read_back(opts: &COptions) -> Vec<(String, String)> {
        let mut out = Vec::with_capacity(opts.len());
        for i in 0..opts.len() {
            // SAFETY: pointer arithmetic inside the boxed slice; pointers
            // captured at construction remain valid for the slice's
            // lifetime.
            let entry = unsafe { &*opts.options.as_ptr().add(i) };
            let name = unsafe { CStr::from_ptr(entry.option_name) }
                .to_string_lossy()
                .into_owned();
            let value = unsafe { CStr::from_ptr(entry.default_value) }
                .to_string_lossy()
                .into_owned();
            out.push((name, value));
        }
        out
    }

    #[test]
    fn empty_input_yields_empty_options() {
        let opts = to_c_options(&[]).unwrap();
        assert!(opts.is_empty());
        assert_eq!(opts.len(), 0);
        assert!(read_back(&opts).is_empty());
    }

    #[test]
    fn single_pair_round_trips() {
        let opts = to_c_options(&[("copies", "2")]).unwrap();
        assert_eq!(opts.len(), 1);
        let echoed = read_back(&opts);
        assert_eq!(echoed, vec![("copies".to_string(), "2".to_string())]);
    }

    #[test]
    fn multiple_pairs_round_trip_in_order() {
        let input = &[
            ("copies", "3"),
            ("sides", "two-sided-long-edge"),
            ("orientation-requested", "portrait"),
            ("media", "iso_a4_210x297mm"),
        ];
        let opts = to_c_options(input).unwrap();
        assert_eq!(opts.len(), input.len());

        let echoed = read_back(&opts);
        let expected: Vec<(String, String)> = input
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert_eq!(echoed, expected);
    }

    #[test]
    fn interior_nul_in_key_is_rejected() {
        let result = to_c_options(&[("co\0pies", "1")]);
        assert!(matches!(result, Err(CpdbError::NulError(_))));
    }

    #[test]
    fn interior_nul_in_value_is_rejected() {
        let result = to_c_options(&[("copies", "1\0")]);
        assert!(matches!(result, Err(CpdbError::NulError(_))));
    }

    #[test]
    fn group_and_supported_fields_are_null_initialised() {
        let opts = to_c_options(&[("a", "b")]).unwrap();
        // SAFETY: opts.len() == 1, so index 0 is a valid entry.
        let entry = unsafe { &*opts.options.as_ptr() };
        assert!(entry.group_name.is_null());
        assert!(entry.supported_values.is_null());
        assert_eq!(entry.num_supported, 0);
    }

    #[test]
    fn pointers_stay_valid_across_move() {
        // Constructing then moving the COptions must not invalidate the
        // raw pointers embedded in its `options` slice. Boxed slices have
        // stable addresses; this test pins that invariant.
        let opts = to_c_options(&[("k", "v")]).unwrap();
        let moved = opts;
        let echoed = read_back(&moved);
        assert_eq!(echoed, vec![("k".to_string(), "v".to_string())]);
    }
}
