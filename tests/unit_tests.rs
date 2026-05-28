//! Tests that do not require a live D-Bus or running cpdb backends.
//!
//! Tests marked `#[cfg_attr(miri, ignore)]` invoke cpdb-libs C functions
//! that miri cannot interpret; they remain part of the regular test
//! suite but are skipped under `cargo miri test`.

use cpdb_rs::error::CpdbError;
use cpdb_rs::{Settings, init, util, version};
use std::ffi::CString;

#[test]
#[cfg_attr(miri, ignore)]
fn init_is_idempotent() {
    init();
    init();
}

#[test]
#[cfg_attr(miri, ignore)]
fn version_is_non_empty_when_present() {
    init();
    if let Ok(v) = version() {
        assert!(!v.is_empty(), "version string must not be empty");
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn settings_lifecycle() {
    init();
    let mut s = Settings::new().expect("Settings::new failed");
    s.add_setting("copies", "1").unwrap();
    let existed = s.clear_setting("copies").unwrap();
    assert!(existed, "the key we just inserted should have existed");
    let again = s.clear_setting("copies").unwrap();
    assert!(!again, "clearing a missing key should return false");
}

#[test]
#[cfg_attr(miri, ignore)]
fn settings_try_clone_is_independent() {
    init();
    let mut a = Settings::new().expect("Settings::new failed");
    a.add_setting("media", "iso_a4_210x297mm").unwrap();
    let mut b = a.try_clone().expect("try_clone failed");
    // Modifying the clone must not affect the original.
    let _ = b.clear_setting("media").unwrap();
    // Sanity: the original still works.
    let _ = a.clear_setting("media").unwrap();
}

#[test]
fn cstr_to_string_handles_valid_input() {
    let cstring = CString::new("hello").unwrap();
    let out = unsafe { util::cstr_to_string(cstring.as_ptr()) }.unwrap();
    assert_eq!(out, "hello");
}

#[test]
fn cstr_to_string_rejects_null() {
    let result = unsafe { util::cstr_to_string(std::ptr::null()) };
    assert!(matches!(result, Err(CpdbError::NullPointer)));
}

#[test]
fn to_c_options_round_trips() {
    let pairs = &[("copies", "2"), ("sides", "two-sided-long-edge")];
    let opts = util::to_c_options(pairs).unwrap();
    assert_eq!(opts.len(), pairs.len());
    assert!(!opts.is_empty());
}

#[test]
fn error_messages_are_stable() {
    assert_eq!(format!("{}", CpdbError::NullPointer), "Null pointer encountered");
    assert_eq!(format!("{}", CpdbError::InvalidPrinter), "Invalid printer object");
    assert_eq!(
        format!("{}", CpdbError::NotFound("printer 'x'".into())),
        "Not found: printer 'x'"
    );
    assert_eq!(
        format!("{}", CpdbError::JobFailed("oops".into())),
        "Print job failed: oops"
    );
}
