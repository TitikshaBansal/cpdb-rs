#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::Frontend;
    use crate::backend::Backend;
    use std::fs;
    use std::thread;
    use std::time::Duration;

    // Create test file
    fn create_test_file() -> String {
        let path = "/tmp/test-print.txt";
        fs::write(path, "Test print job").unwrap();
        path.to_string()
    }

    #[test]
    fn test_frontend_creation() {
        let frontend = Frontend::new().expect("Failed to create frontend");
        assert!(!frontend.raw.is_null());
    }

    #[test]
    fn test_get_printers() {
        let frontend = Frontend::new().expect("Failed to create frontend");
        let printers = frontend.get_printers().expect("Failed to get printers");
        assert!(!printers.is_empty(), "No printers found");
        
        for printer in printers {
            println!("Printer: {}", printer.name().unwrap());
            assert!(!printer.name().unwrap().is_empty());
        }
    }

    #[test]
    fn test_async_printer_discovery() {
        let frontend = Frontend::new().unwrap();
        let rx = frontend.get_printers_channel().unwrap();
        
        let mut count = 0;
        while let Ok(printer) = rx.recv_timeout(Duration::from_secs(5)) {
            let printer = printer.expect("Invalid printer received");
            println!("Async Printer: {}", printer.name().unwrap());
            count += 1;
        }
        
        assert!(count > 0, "No printers found via async discovery");
    }

    #[test]
    fn test_job_submission() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let file_path = create_test_file();
            let result = printer.submit_job(&file_path, &[("copies", "1")], "Test Job");
            assert!(result.is_ok(), "Job submission failed: {:?}", result);
        }
    }

    #[test]
    fn test_job_lifecycle() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let file_path = create_test_file();
            let mut job = PrintJob::new(
                &printer,
                &[("copies", "1")],
                &file_path,
                "Lifecycle Test"
            ).unwrap();
            
            assert!(job.submit().is_ok());
            assert!(job.id().is_some());
            assert!(job.cancel().is_ok());
        }
    }

    #[test]
    fn test_backend_operations() {
        let backend = Backend::new("CUPS").expect("Failed to create backend");
        let printers = backend.get_printers().expect("Failed to get printers");
        
        if !printers.is_empty() {
            let printer = printers.first().unwrap();
            let file_path = create_test_file();
            
            // Test direct backend submission
            let result = backend.submit_job(
                &printer.name().unwrap(),
                &file_path,
                &[("copies", "1"), ("media", "A4")],
                "Backend Test Job"
            );
            
            assert!(result.is_ok(), "Backend job submission failed");
            
            // Test job object submission
            let mut job = PrintJob::new(
                &printer,
                &[("sides", "two-sided-long-edge")],
                &file_path,
                "Job Object Test"
            ).unwrap();
            
            assert!(job.submit().is_ok());
            assert!(job.id().is_some());
        }
    }

    #[test]
    fn test_printer_cloning() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let cloned = printer.try_clone().unwrap();
            assert_eq!(printer.name().unwrap(), cloned.name().unwrap());
            assert_eq!(printer.backend_name().unwrap(), cloned.backend_name().unwrap());
        }
    }
}