//! Live D-Bus integration tests. All `#[ignore]`d by default — they
//! require a session D-Bus and at least one cpdb backend to be active.
//! Run with `cargo test -- --ignored`.

use cpdb_rs::Frontend;
use std::fs;
use std::io::Write;

fn write_temp_test_file(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(name);
    let mut f = fs::File::create(&path).expect("failed to create test print file");
    writeln!(f, "cpdb-rs integration test").unwrap();
    path
}

#[test]
#[ignore]
fn printer_discovery() {
    cpdb_rs::init();
    let frontend = Frontend::new().expect("frontend init failed");
    frontend.connect_to_dbus().expect("connect_to_dbus failed");
    let printers = frontend.get_printers().expect("get_printers failed");
    for p in &printers {
        let name = p.name().unwrap_or_default();
        let state = p.get_updated_state().unwrap_or_default();
        eprintln!("found {name}: {state}");
    }
}

#[test]
#[ignore]
fn job_submission_applies_options() {
    cpdb_rs::init();
    let frontend = Frontend::new().expect("frontend init failed");
    frontend.connect_to_dbus().expect("connect_to_dbus failed");
    let printers = frontend.get_printers().unwrap();
    let printer = match printers.first() {
        Some(p) => p,
        None => return, // no printer in CI is fine
    };
    let file = write_temp_test_file("cpdb-rs-test.txt");
    let job_id = printer
        .submit_job(file.to_str().unwrap(), &[("copies", "1")], "cpdb-rs test")
        .expect("submit_job failed");
    assert!(!job_id.is_empty(), "job id must not be empty");
    let _ = fs::remove_file(&file);
}
#[cfg(all(test, feature = "ffi"))]
mod tests {
    use cpdb_rs::{Frontend, PrintJob};
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
            let _ = printer;
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
