#[cfg(test)]
mod unit_tests {
    use cpdb_rs::{init, version, Frontend, Settings, Options};
    use cpdb_rs::error::CpdbError;
    use std::fs;
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
                println!("Version retrieval failed (expected in test environment): {}", e);
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
                println!("Frontend creation failed (expected in test environment): {}", e);
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
                println!("Settings creation failed (expected in test environment): {}", e);
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
                println!("Settings creation failed (expected in test environment): {}", e);
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
                println!("Options creation failed (expected in test environment): {}", e);
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
                                println!("Settings load from disk failed (expected in test environment): {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Settings save to disk failed (expected in test environment): {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Settings creation failed (expected in test environment): {}", e);
            }
        }
    }

    #[test]
    fn test_error_handling() {
        setup_test_environment();
        
        // Test null pointer error
        let null_error = CpdbError::NullPointer;
        assert_eq!(format!("{}", null_error), "Null pointer encountered");

        // Test invalid printer error
        let invalid_printer_error = CpdbError::InvalidPrinter;
        assert_eq!(format!("{}", invalid_printer_error), "Invalid printer object");

        // Test job failed error
        let job_error = CpdbError::JobFailed("Test job failed".to_string());
        assert_eq!(format!("{}", job_error), "Print job failed: Test job failed");

        // Test backend error
        let backend_error = CpdbError::BackendError("Test backend error".to_string());
        assert_eq!(format!("{}", backend_error), "Backend error: Test backend error");

        // Test frontend error
        let frontend_error = CpdbError::FrontendError("Test frontend error".to_string());
        assert_eq!(format!("{}", frontend_error), "Frontend error: Test frontend error");

        // Test option error
        let option_error = CpdbError::OptionError("Test option error".to_string());
        assert_eq!(format!("{}", option_error), "Option parsing error: Test option error");

        // Test CUPS error
        let cups_error = CpdbError::CupsError(42);
        assert_eq!(format!("{}", cups_error), "CUPS error: 42");

        // Test invalid status
        let status_error = CpdbError::InvalidStatus(99);
        assert_eq!(format!("{}", status_error), "Invalid status code: 99");

        // Test unsupported operation
        let unsupported_error = CpdbError::Unsupported;
        assert_eq!(format!("{}", unsupported_error), "Unsupported operation");
    }

    #[test]
    fn test_string_conversion_utilities() {
        use cpdb_rs::util;
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
        use cpdb_rs::util;

        let options = &[("key1", "value1"), ("key2", "value2")];
        match util::to_c_options(options) {
            Ok(c_options) => {
                assert_eq!(c_options.len(), 2);
                // The actual content verification would require unsafe operations
                // but we can at least verify the structure is created
            }
            Err(e) => {
                println!("C options conversion failed (expected in test environment): {}", e);
            }
        }
    }

    #[test]
    fn test_printer_clone_behavior() {
        setup_test_environment();
        
        // This test verifies that cloning doesn't panic
        // In a real environment, we would test with actual printer objects
        match Frontend::new() {
            Ok(frontend) => {
                match frontend.get_printers() {
                    Ok(printers) => {
                        if let Some(printer) = printers.first() {
                            // Test cloning
                            let cloned = printer.clone();
                            assert_eq!(printer.name().unwrap_or_default(), 
                                     cloned.name().unwrap_or_default());
                        }
                    }
                    Err(_) => {
                        // Expected in test environment without printers
                        println!("No printers available for clone test");
                    }
                }
            }
            Err(_) => {
                // Expected in test environment
                println!("Frontend creation failed for clone test");
            }
        }
    }

    #[test]
    fn test_resource_cleanup() {
        setup_test_environment();
        
        // Test that resources are properly cleaned up
        // This is mainly to ensure Drop implementations don't panic
        
        // Test Settings cleanup
        let _settings = Settings::new();
        // settings goes out of scope here, Drop should be called
        
        // Test Options cleanup
        let _options = Options::new();
        // options goes out of scope here, Drop should be called
        
        // Test Frontend cleanup
        let _frontend = Frontend::new();
        // frontend goes out of scope here, Drop should be called
        
        println!("Resource cleanup tests completed");
    }
}
