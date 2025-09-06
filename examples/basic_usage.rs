use cpdb_rs::{init, version, Frontend, Printer};
use cpdb_rs::error::CpdbError;
use std::ptr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- CPDB Rust Bindings Manual Test ---");

    // 1. Initialize the library
    init();
    println!("[OK] cpdb_rs::init() called.");

    // 2. Get and print the cpdb-libs version
    match version() {
        Ok(v_str) => println!("[OK] cpdb-libs version: {}", v_str),
        Err(e) => {
            eprintln!("[FAIL] Failed to get cpdb-libs version: {}", e);
            return Err(Box::new(e));
        }
    }

    // 3. Create a Frontend object
    let frontend = match Frontend::new() {
        Ok(f) => {
            println!("[OK] Frontend::new() succeeded.");
            f
        }
        Err(e) => {
            eprintln!("[FAIL] Frontend::new() failed: {}", e);
            eprintln!("       Ensure D-Bus session is active and relevant print services (like CUPS) are running.");
            return Err(Box::new(e));
        }
    };

    // 4. Connect to D-Bus
    if let Err(e) = frontend.connect_to_dbus() {
        eprintln!("[FAIL] frontend.connect_to_dbus() failed: {}", e);
        return Err(Box::new(e));
    }
    println!("[OK] frontend.connect_to_dbus() called.");

    println!("[INFO] Attempting to list printers using frontend.get_printers()...");
    match frontend.get_printers() {
        Ok(printers) => {
            println!("[OK] frontend.get_printers() succeeded. Found {} printers.", printers.len());
            if printers.is_empty() {
                println!("[INFO] No printers found. This might be normal if no printers are configured or discoverable.");
            }
            for (i, printer) in printers.iter().enumerate() {
                print!("[INFO] Printer {}: ", i + 1);
                let name = printer.name().unwrap_or_else(|e| format!("(Error: {})", e));
                let state = printer.get_updated_state().unwrap_or_else(|e| format!("(Error: {})", e));
                println!("Name: '{}', State: '{}'", name, state);
            }
            
            if let Some(first_printer) = printers.first() {
                println!("[INFO] Attempting to print_single_file with the first printer (Name: {})...", first_printer.name().unwrap_or_default());
                let test_file_path = "cpdb_rust_test_file.txt";
                if std::fs::write(test_file_path, "Hello from cpdb-rs test!").is_ok() {
                    match first_printer.print_single_file(test_file_path) {
                        Ok(job_id) => println!("[OK] print_single_file succeeded for '{}'. Job ID: {}", first_printer.name().unwrap_or_default(), job_id),
                        Err(e) => eprintln!("[FAIL] print_single_file for '{}' failed: {}. (This can be expected if printer is not real/ready)", first_printer.name().unwrap_or_default(), e),
                    }
                    let _ = std::fs::remove_file(test_file_path); // Clean up
                } else {
                    eprintln!("[WARN] Could not create dummy test file for printing.");
                }
            }

        }
        Err(e) => {
            eprintln!("[FAIL] frontend.get_printers() failed: {}", e);
            eprintln!("       This could be due to D-Bus issues, no print backends running (like CUPS),");
            eprintln!("       or the underlying C function `cpdbGetPrinters` not behaving as expected.");
        }
    }

    println!("--- Test Complete ---");
    Ok(())
}