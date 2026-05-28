//! Minimal example: initialise cpdb-rs, list printers, and submit a tiny
//! print job to the first available printer.

use cpdb_rs::{Frontend, init, version};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- cpdb-rs basic usage ---");

    init();
    println!("[OK] cpdb_rs::init()");

    match version() {
        Ok(v) => println!("[OK] cpdb-libs version: {v}"),
        Err(e) => {
            eprintln!("[FAIL] cpdb-libs version: {e}");
            return Err(Box::new(e));
        }
    }

    let frontend = Frontend::new()?;
    println!("[OK] Frontend::new()");
    frontend.connect_to_dbus()?;
    println!("[OK] connect_to_dbus()");

    let printers = frontend.get_printers()?;
    println!("[OK] {} printer(s) discovered", printers.len());
    for (i, p) in printers.iter().enumerate() {
        let name = p.name().unwrap_or_default();
        let state = p.get_updated_state().unwrap_or_default();
        println!("  [{i}] {name} ({state})");
    }

    if let Some(printer) = printers.first() {
        let path = "cpdb_rs_basic_usage_test.txt";
        std::fs::write(path, b"Hello from cpdb-rs basic_usage example.\n")?;
        match printer.print_file(path) {
            Ok(job_id) => println!("[OK] job submitted: {job_id}"),
            Err(e) => eprintln!("[FAIL] print_file: {e}"),
        }
        let _ = std::fs::remove_file(path);
    }

    Ok(())
}
