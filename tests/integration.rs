#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::Frontend;
    use crate::printer::Printer;
    use crate::job::PrintJob;
    use crate::util;
    use std::fs;

    // Create test file
    fn create_test_file() -> String {
        let path = "/tmp/test-print.txt";
        fs::write(path, "Test print job").unwrap();
        path.to_string()
    }

    #[test]
    fn test_printer_discovery() {
        let frontend = Frontend::new().expect("Failed to create frontend");
        let printers = frontend.get_printers().expect("Failed to get printers");
        assert!(!printers.is_empty(), "No printers found");
        
        let printer = &printers[0];
        println!("Printer: {}", printer.name().unwrap());
        assert!(!printer.name().unwrap().is_empty());
        
        // Test clone
        let cloned = printer.clone();
        assert_eq!(printer.name().unwrap(), cloned.name().unwrap());
    }

    #[test]
    fn test_job_submission() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let file_path = create_test_file();
            let options = util::to_c_options(&[("copies", "1")]).unwrap();
            let result = printer.submit_job(&file_path, &options, "Test Job");
            unsafe { util::free_c_options(options) };
            assert!(result.is_ok(), "Job submission failed: {:?}", result);
        }
    }

    #[test]
    fn test_job_lifecycle() {
        let frontend = Frontend::new().unwrap();
        let printers = frontend.get_printers().unwrap();
        
        if let Some(printer) = printers.first() {
            let printer_name = printer.name().unwrap();
            let options = util::to_c_options(&[("copies", "1")]).unwrap();
            let mut job = PrintJob::new(&printer_name, &options, "Test Job").unwrap();
            unsafe { util::free_c_options(options) };
            
            let file_path = create_test_file();
            assert!(job.submit_with_file(&file_path).is_ok());
            assert!(job.id().is_some());
            
            assert!(job.cancel().is_ok());
            assert!(job.id().is_none());
        }
    }
}