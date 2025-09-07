#[cfg(test)]
mod tests {
    use cpdb_rs::frontend::Frontend;
    use cpdb_rs::printer::Printer;
    use cpdb_rs::job::PrintJob;
    use std::fs;

    // Create test file
    fn create_test_file() -> String {
        let path = "/tmp/test-print.txt";
        fs::write(path, "Test print job").unwrap();
        path.to_string()
    }

    #[test]
    #[ignore]
    fn test_printer_discovery() {
        let frontend = Frontend::new().expect("Failed to create frontend");
        let printers = frontend.get_printers().expect("Failed to get printers");
        // In CI, printers might not be available; skip strict assertions
        
        if let Some(printer) = printers.first() {
            println!("Printer: {}", printer.name().unwrap_or_default());
            let _ = printer.clone();
        }
    }

    #[test]
    #[ignore]
    fn test_job_submission() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let file_path = create_test_file();
            let options = &[("copies", "1")];
            let result = printer.submit_job(&file_path, options, "Test Job");
            assert!(result.is_ok(), "Job submission failed: {:?}", result);
        }
    }

    #[test]
    #[ignore]
    fn test_job_lifecycle() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let printer_name = printer.name().unwrap();
            let options = &[("copies", "1")];
            let mut job = PrintJob::new(&printer_name, options, "Test Job").unwrap();
            
            let file_path = create_test_file();
            assert!(job.submit_with_file(&file_path).is_ok());
            assert!(job.id().is_some());
            
            assert!(job.cancel().is_ok());
            assert!(job.id().is_none());
        }
    }
}