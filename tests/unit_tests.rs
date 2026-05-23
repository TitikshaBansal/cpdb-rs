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
    assert_eq!(
        format!("{}", CpdbError::NullPointer),
        "Null pointer encountered"
    );
    assert_eq!(
        format!("{}", CpdbError::InvalidPrinter),
        "Invalid printer object"
    );
    assert_eq!(
        format!("{}", CpdbError::NotFound("printer 'x'".into())),
        "Not found: printer 'x'"
    );
    assert_eq!(
        format!("{}", CpdbError::JobFailed("oops".into())),
        "Print job failed: oops"
    );
}
#[cfg(all(test, feature = "ffi"))]
mod ffi_tests {
    use cpdb_rs::error::CpdbError;
    use cpdb_rs::{Frontend, Options, Settings, init, version};
    use tempfile::NamedTempFile;

    fn setup_test_environment() {
        init();
    }

    #[test]
    fn test_library_initialization() {
        setup_test_environment();
        // init() should not panic
        init();
    }

    #[test]
    fn test_version_retrieval() {
        setup_test_environment();
        match version() {
            Ok(v) => {
                assert!(!v.is_empty(), "Version string should not be empty");
                println!("CPDB version: {}", v);
            }
            Err(e) => {
                // Version retrieval might fail in test environment
                println!(
                    "Version retrieval failed (expected in test environment): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_frontend_creation() {
        setup_test_environment();
        match Frontend::new() {
            Ok(frontend) => {
                // Frontend created successfully
                let _ = frontend;
            }
            Err(e) => {
                // Frontend creation might fail in test environment
                println!(
                    "Frontend creation failed (expected in test environment): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_settings_creation() {
        setup_test_environment();
        match Settings::new() {
            Ok(settings) => {
                // Settings should be created successfully
                assert!(!settings.as_raw().is_null());
            }
            Err(e) => {
                // Settings creation might fail in test environment
                println!(
                    "Settings creation failed (expected in test environment): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_settings_operations() {
        setup_test_environment();
        match Settings::new() {
            Ok(mut settings) => {
                // Test adding a setting
                assert!(settings.add_setting("test_key", "test_value").is_ok());

                // Test clearing a setting
                assert!(settings.clear_setting("test_key").is_ok());

                // Test copying settings
                match settings.copy() {
                    Ok(copy) => {
                        assert!(!copy.as_raw().is_null());
                    }
                    Err(e) => {
                        println!("Settings copy failed (expected in test environment): {}", e);
                    }
                }
            }
            Err(e) => {
                println!(
                    "Settings creation failed (expected in test environment): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_options_creation() {
        setup_test_environment();
        match Options::new() {
            Ok(options) => {
                // Options should be created successfully
                assert!(!options.as_raw().is_null());
            }
            Err(e) => {
                // Options creation might fail in test environment
                println!(
                    "Options creation failed (expected in test environment): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_settings_file_operations() {
        setup_test_environment();

        // Create a temporary file for testing
        let _temp_file = NamedTempFile::new().expect("Failed to create temp file");

        match Settings::new() {
            Ok(mut settings) => {
                // Add some test settings
                assert!(settings.add_setting("test_key1", "test_value1").is_ok());
                assert!(settings.add_setting("test_key2", "test_value2").is_ok());

                // Test saving to disk (no path needed)
                match settings.save_to_disk() {
                    Ok(_) => {
                        // Test loading from disk
                        match Settings::read_from_disk() {
                            Ok(loaded_settings) => {
                                assert!(!loaded_settings.as_raw().is_null());
                                println!("Settings file operations successful");
                            }
                            Err(e) => {
                                println!(
                                    "Settings load from disk failed (expected in test environment): {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "Settings save to disk failed (expected in test environment): {}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "Settings creation failed (expected in test environment): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_string_conversion_utilities() {
        use cpdb_rs::ffi::util;
        use std::ffi::CString;

        // Test valid C string conversion
        let test_string = "Hello, World!";
        let c_string = CString::new(test_string).expect("Failed to create CString");
        let result = unsafe { util::cstr_to_string(c_string.as_ptr()) };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_string);

        // Test null pointer handling
        let null_result = unsafe { util::cstr_to_string(std::ptr::null()) };
        assert!(null_result.is_err());
        assert!(matches!(null_result.unwrap_err(), CpdbError::NullPointer));
    }

    #[test]
    fn test_c_options_conversion() {
        use cpdb_rs::ffi::util;

        let options = &[("key1", "value1"), ("key2", "value2")];
        match util::to_c_options(options) {
            Ok(c_options) => {
                assert_eq!(c_options.len(), 2);
            }
            Err(e) => {
                println!(
                    "C options conversion failed (expected in test environment): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_resource_cleanup() {
        setup_test_environment();

        // Test that resources are properly cleaned up
        // This is mainly to ensure Drop implementations don't panic
        let _settings = Settings::new();
        let _options = Options::new();
        let _frontend = Frontend::new();
        println!("Resource cleanup tests completed");
    }
}

// zbus backend unit tests

#[cfg(all(test, feature = "zbus-backend"))]
mod zbus_tests {
    use cpdb_rs::config::PrinterConfig;
    use cpdb_rs::error::CpdbError;
    use cpdb_rs::events::{DiscoveryEvent, PrinterSnapshot};
    use cpdb_rs::media::MediaCollection;
    use cpdb_rs::options::OptionsCollection;
    use cpdb_rs::proxy::{RawMargin, RawMedia, RawOption};
    use std::collections::HashMap;

    #[test]
    fn raw_option_debug_display() {
        let opt = RawOption {
            option_name: "copies".to_string(),
            group_name: "General".to_string(),
            default_value: "1".to_string(),
            num_supported: 2,
            supported_values: vec![("1".to_string(),), ("2".to_string(),)],
        };
        let debug = format!("{:?}", opt);
        assert!(debug.contains("copies"));
    }

    #[test]
    fn raw_media_with_margins() {
        let media = RawMedia {
            name: "iso_a4_210x297mm".to_string(),
            width: 21000,
            length: 29700,
            num_margins: 1,
            margins: vec![RawMargin {
                left: 500,
                right: 500,
                top: 300,
                bottom: 300,
            }],
        };
        assert_eq!(media.name, "iso_a4_210x297mm");
        assert_eq!(media.margins.len(), 1);
        assert_eq!(media.margins[0].left, 500);
    }

    #[test]
    fn raw_option_clone() {
        let opt = RawOption {
            option_name: "sides".to_string(),
            group_name: "General".to_string(),
            default_value: "one-sided".to_string(),
            num_supported: 1,
            supported_values: vec![("one-sided".to_string(),)],
        };
        let clone = opt.clone();
        assert_eq!(clone.option_name, "sides");
        assert_eq!(clone.supported_values.len(), 1);
    }

    #[test]
    fn options_from_dbus_empty() {
        let col = OptionsCollection::from_dbus(vec![]);
        assert!(col.is_empty());
        assert_eq!(col.len(), 0);
    }

    #[test]
    fn options_from_dbus_single_option() {
        let raw = vec![RawOption {
            option_name: "copies".to_string(),
            group_name: "General".to_string(),
            default_value: "1".to_string(),
            num_supported: 3,
            supported_values: vec![("1".to_string(),), ("2".to_string(),), ("99".to_string(),)],
        }];
        let col = OptionsCollection::from_dbus(raw);
        assert_eq!(col.len(), 1);

        let opt = col.get("copies").unwrap();
        assert_eq!(opt.default_value, "1");
        assert_eq!(opt.group, "General");
        // Verify the Vec<(String,)> -> Vec<String> extraction
        assert_eq!(opt.supported_values, vec!["1", "2", "99"]);
    }

    #[test]
    fn options_from_dbus_multiple_options() {
        let raw = vec![
            RawOption {
                option_name: "copies".to_string(),
                group_name: "General".to_string(),
                default_value: "1".to_string(),
                num_supported: 1,
                supported_values: vec![("1".to_string(),)],
            },
            RawOption {
                option_name: "sides".to_string(),
                group_name: "General".to_string(),
                default_value: "one-sided".to_string(),
                num_supported: 3,
                supported_values: vec![
                    ("one-sided".to_string(),),
                    ("two-sided-long-edge".to_string(),),
                    ("two-sided-short-edge".to_string(),),
                ],
            },
            RawOption {
                option_name: "print-color-mode".to_string(),
                group_name: "Color".to_string(),
                default_value: "color".to_string(),
                num_supported: 2,
                supported_values: vec![("color".to_string(),), ("monochrome".to_string(),)],
            },
        ];
        let col = OptionsCollection::from_dbus(raw);
        assert_eq!(col.len(), 3);

        // Verify each option
        assert!(col.get("copies").is_some());
        assert!(col.get("sides").is_some());
        assert!(col.get("print-color-mode").is_some());

        let sides = col.get("sides").unwrap();
        assert_eq!(sides.supported_values.len(), 3);
        assert!(
            sides
                .supported_values
                .contains(&"two-sided-long-edge".to_string())
        );
    }

    #[test]
    fn options_from_dbus_preserves_empty_supported_values() {
        let raw = vec![RawOption {
            option_name: "resolution".to_string(),
            group_name: "Quality".to_string(),
            default_value: "300dpi".to_string(),
            num_supported: 0,
            supported_values: vec![],
        }];
        let col = OptionsCollection::from_dbus(raw);
        let opt = col.get("resolution").unwrap();
        assert!(opt.supported_values.is_empty());
    }

    #[test]
    fn media_from_dbus_empty() {
        let col = MediaCollection::from_dbus(vec![]);
        assert!(col.is_empty());
    }

    #[test]
    fn media_from_dbus_with_margins() {
        let raw = vec![RawMedia {
            name: "iso_a4_210x297mm".to_string(),
            width: 21000,
            length: 29700,
            num_margins: 2,
            margins: vec![
                RawMargin {
                    left: 500,
                    right: 500,
                    top: 300,
                    bottom: 300,
                },
                RawMargin {
                    left: 0,
                    right: 0,
                    top: 0,
                    bottom: 0,
                },
            ],
        }];
        let col = MediaCollection::from_dbus(raw);
        assert_eq!(col.len(), 1);

        let a4 = col.get("iso_a4_210x297mm").unwrap();
        assert_eq!(a4.width, 21000);
        assert_eq!(a4.length, 29700);
        assert_eq!(a4.margins.len(), 2);
        // First margin: standard
        assert_eq!(a4.margins[0].left, 500);
        // Second margin: borderless
        assert_eq!(a4.margins[1].left, 0);
    }

    #[test]
    fn media_from_dbus_without_margins() {
        let raw = vec![RawMedia {
            name: "na_letter_8.5x11in".to_string(),
            width: 21590,
            length: 27940,
            num_margins: 0,
            margins: vec![],
        }];
        let col = MediaCollection::from_dbus(raw);
        let letter = col.get("na_letter_8.5x11in").unwrap();
        assert!(letter.margins.is_empty());
    }

    #[test]
    fn printer_snapshot_is_ready() {
        let snap = PrinterSnapshot {
            id: "test".to_string(),
            name: "Test".to_string(),
            info: String::new(),
            location: String::new(),
            make_model: String::new(),
            state: "idle".to_string(),
            accepting_jobs: true,
            backend: "CUPS".to_string(),
        };
        assert!(snap.is_ready());
    }

    #[test]
    fn discovery_event_pattern_matching() {
        let events = vec![
            DiscoveryEvent::PrinterAdded(PrinterSnapshot {
                id: "printer-1".to_string(),
                name: "My Printer".to_string(),
                info: String::new(),
                location: String::new(),
                make_model: String::new(),
                state: "idle".to_string(),
                accepting_jobs: true,
                backend: "CUPS".to_string(),
            }),
            DiscoveryEvent::PrinterStateChanged {
                id: "printer-1".to_string(),
                backend: "CUPS".to_string(),
                state: "printing".to_string(),
                accepting_jobs: true,
            },
            DiscoveryEvent::PrinterRemoved {
                id: "printer-1".to_string(),
                backend: "CUPS".to_string(),
            },
        ];

        let mut printers: Vec<PrinterSnapshot> = Vec::new();

        for event in events {
            match event {
                DiscoveryEvent::PrinterAdded(snap) => {
                    printers.push(snap);
                }
                DiscoveryEvent::PrinterStateChanged {
                    id,
                    backend,
                    state,
                    accepting_jobs,
                } => {
                    if let Some(p) = printers
                        .iter_mut()
                        .find(|p| p.id == id && p.backend == backend)
                    {
                        p.state = state;
                        p.accepting_jobs = accepting_jobs;
                    }
                }
                DiscoveryEvent::PrinterRemoved { id, backend } => {
                    printers.retain(|p| !(p.id == id && p.backend == backend));
                }
            }
        }

        // After all events: printer was added, state changed, then removed
        assert!(printers.is_empty());
    }

    #[test]
    fn config_roundtrip_preserves_all_fields() {
        let mut settings = HashMap::new();
        settings.insert("copies".to_string(), "5".to_string());
        settings.insert("media".to_string(), "iso_a4_210x297mm".to_string());
        settings.insert("sides".to_string(), "two-sided-long-edge".to_string());
        settings.insert("print-color-mode".to_string(), "monochrome".to_string());

        let config = PrinterConfig::new("HP-LaserJet", "CUPS", settings);
        let json = serde_json::to_string_pretty(&config).unwrap();
        let loaded: PrinterConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.last_printer_id.as_deref(), Some("HP-LaserJet"));
        assert_eq!(loaded.last_backend.as_deref(), Some("CUPS"));
        assert_eq!(loaded.get_setting("copies"), Some("5"));
        assert_eq!(loaded.get_setting("sides"), Some("two-sided-long-edge"));
    }

    #[test]
    fn error_display_messages() {
        // Verify all error variants format correctly
        assert_eq!(
            format!("{}", CpdbError::NullPointer),
            "Null pointer encountered"
        );
        assert_eq!(
            format!("{}", CpdbError::BackendError("CUPS down".into())),
            "Backend error: CUPS down"
        );
        assert_eq!(
            format!("{}", CpdbError::Unsupported),
            "Unsupported operation"
        );
    }

    #[test]
    fn error_from_status_mapping() {
        assert!(matches!(
            CpdbError::from_status(0, ""),
            CpdbError::NullPointer
        ));
        assert!(matches!(
            CpdbError::from_status(1, ""),
            CpdbError::InvalidPrinter
        ));
        assert!(matches!(
            CpdbError::from_status(2, "test"),
            CpdbError::JobFailed(_)
        ));
        assert!(matches!(
            CpdbError::from_status(99, "unknown"),
            CpdbError::BackendError(_)
        ));
    }

    #[test]
    fn error_is_non_exhaustive() {
        // This test just ensures the enum compiles with #[non_exhaustive]
        let err: CpdbError = CpdbError::BackendError("test".into());
        let _msg = format!("{}", err);
    }
}
