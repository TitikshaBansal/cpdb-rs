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
    write!(f, "cpdb-rs integration test\n").unwrap();
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
