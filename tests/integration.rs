#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::Frontend;
    use std::fs;

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
    fn test_job_submission() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let file_path = create_test_file();
            let result = printer.submit_job(&file_path, &[("copies", "1")]);
            assert!(result.is_ok(), "Job submission failed: {:?}", result);
        }
    }

    #[test]
    fn test_job_lifecycle() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let file_path = create_test_file();
            let job = PrintJob::new(&printer, &[("copies", "1")], &file_path).unwrap();
            assert!(job.submit().is_ok());
            assert!(job.id().unwrap() > 0);
        }
    }
}