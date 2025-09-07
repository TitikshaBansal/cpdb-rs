use cpdb_rs::{init, version, Frontend, Printer};
use std::env;
use std::fs;
use std::io; // retained if future interactive features are added

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖨️  CPDB Rust CLI Printer Manager");
    println!("=====================================");

    // Initialize the library
    init();
    println!("✓ CPDB library initialized");

    // Show version
    match version() {
        Ok(v) => println!("✓ CPDB version: {}", v),
        Err(e) => eprintln!("✗ Failed to get version: {}", e),
    }

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "list" => list_printers(),
        "info" => {
            if args.len() < 3 {
                eprintln!("Usage: {} info <printer_name>", args[0]);
                return Ok(());
            }
            show_printer_info(&args[2])
        }
        "print" => {
            if args.len() < 4 {
                eprintln!("Usage: {} print <printer_name> <file_path>", args[0]);
                return Ok(());
            }
            print_file(&args[2], &args[3])
        }
        "options" => {
            if args.len() < 3 {
                eprintln!("Usage: {} options <printer_name>", args[0]);
                return Ok(());
            }
            show_printer_options(&args[2])
        }
        "media" => {
            if args.len() < 3 {
                eprintln!("Usage: {} media <printer_name>", args[0]);
                return Ok(());
            }
            show_printer_media(&args[2])
        }
        "save-config" => {
            if args.len() < 4 {
                eprintln!("Usage: {} save-config <printer_name> <config_file>", args[0]);
                return Ok(());
            }
            save_printer_config(&args[2], &args[3])
        }
        "load-config" => {
            if args.len() < 3 {
                eprintln!("Usage: {} load-config <config_file>", args[0]);
                return Ok(());
            }
            load_printer_config(&args[2])
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    println!("\nUsage:");
    println!("  {} list                           - List all available printers", env::args().next().unwrap());
    println!("  {} info <printer_name>            - Show detailed printer information", env::args().next().unwrap());
    println!("  {} print <printer_name> <file>    - Print a file to the specified printer", env::args().next().unwrap());
    println!("  {} options <printer_name>         - Show printer options", env::args().next().unwrap());
    println!("  {} media <printer_name>           - Show printer media information", env::args().next().unwrap());
    println!("  {} save-config <printer> <file>   - Save printer configuration", env::args().next().unwrap());
    println!("  {} load-config <file>             - Load printer configuration", env::args().next().unwrap());
}

fn list_printers() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Discovering printers...");
    
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    let printers = frontend.get_printers()?;
    
    if printers.is_empty() {
        println!("No printers found.");
        println!("Make sure:");
        println!("  - CUPS or other print services are running");
        println!("  - Printers are configured and accessible");
        println!("  - D-Bus session is active");
        return Ok(());
    }

    println!("Found {} printer(s):", printers.len());
    println!("{:<20} {:<15} {:<20} {:<10}", "Name", "Backend", "State", "Accepting Jobs");
    println!("{}", "-".repeat(70));

    for printer in printers {
        let name = printer.name().unwrap_or_else(|_| "Unknown".to_string());
        let backend = printer.backend_name().unwrap_or_else(|_| "Unknown".to_string());
        let state = printer.get_updated_state().unwrap_or_else(|_| "Unknown".to_string());
        let accepting = printer.is_accepting_jobs().unwrap_or(false);
        
        println!("{:<20} {:<15} {:<20} {:<10}", 
                 name, backend, state, if accepting { "Yes" } else { "No" });
    }

    Ok(())
}

fn show_printer_info(printer_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Getting printer information for: {}", printer_name);
    
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    let printer = frontend.get_printer(printer_name)?;
    
    println!("📄 Printer Details:");
    println!("  Name: {}", printer.name().unwrap_or_else(|_| "Unknown".to_string()));
    println!("  ID: {}", printer.id().unwrap_or_else(|_| "Unknown".to_string()));
    println!("  Location: {}", printer.location().unwrap_or_else(|_| "Not specified".to_string()));
    println!("  Description: {}", printer.description().unwrap_or_else(|_| "Not specified".to_string()));
    println!("  Make & Model: {}", printer.make_and_model().unwrap_or_else(|_| "Unknown".to_string()));
    println!("  Backend: {}", printer.backend_name().unwrap_or_else(|_| "Unknown".to_string()));
    println!("  Current State: {}", printer.get_updated_state().unwrap_or_else(|_| "Unknown".to_string()));
    println!("  Accepting Jobs: {}", printer.is_accepting_jobs().unwrap_or(false));
    println!("  Accepts PDF: {}", printer.accepts_pdf().unwrap_or(false));

    Ok(())
}

fn print_file(printer_name: &str, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🖨️  Printing file: {} to printer: {}", file_path, printer_name);
    
    // Check if file exists
    if !fs::metadata(file_path).is_ok() {
        eprintln!("✗ File not found: {}", file_path);
        return Ok(());
    }

    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    let printer = frontend.get_printer(printer_name)?;
    
    // Check if printer is accepting jobs
    if !printer.is_accepting_jobs().unwrap_or(false) {
        eprintln!("✗ Printer is not accepting jobs");
        return Ok(());
    }

    // Print the file
    match printer.print_single_file(file_path) {
        Ok(job_id) => {
            println!("✓ Print job submitted successfully!");
            println!("  Job ID: {}", job_id);
        }
        Err(e) => {
            eprintln!("✗ Print job failed: {}", e);
            eprintln!("  Make sure the printer is ready and the file format is supported");
        }
    }

    Ok(())
}

fn show_printer_options(printer_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚙️  Getting options for printer: {}", printer_name);
    
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    let printer = frontend.get_printer(printer_name)?;
    
    // Common printer options to check
    let common_options = [
        "copies", "page-ranges", "orientation-requested", 
        "print-quality", "sides", "media", "printer-resolution"
    ];

    println!("📋 Printer Options:");
    for option in &common_options {
        match printer.get_option(option) {
            Ok(value) => println!("  {}: {}", option, value),
            Err(_) => {
                // Try to get default value
                match printer.get_default(option) {
                    Ok(default) => println!("  {}: {} (default)", option, default),
                    Err(_) => println!("  {}: Not available", option),
                }
            }
        }
    }

    Ok(())
}

fn show_printer_media(printer_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📄 Getting media information for printer: {}", printer_name);
    
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    let printer = frontend.get_printer(printer_name)?;
    
    println!("📋 Media Information:");

    // Try to detect current media from printer options; fall back to a common name if unavailable
    let media_name = printer
        .get_current("media")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "iso_a4_210x297mm".to_string());
    println!("  Using media: {}", media_name);

    match printer.get_media(&media_name) {
        Ok(media) => println!("  Media: {}", media),
        Err(_) => println!("  Media: Not available"),
    }
    match printer.get_media_size(&media_name) {
        Ok(size) => println!("  Size: {:?}", size),
        Err(_) => println!("  Size: Not available"),
    }
    match printer.get_media_margins(&media_name) {
        Ok(margins) => println!("  Margins: {}", margins),
        Err(_) => println!("  Margins: Not available"),
    }

    Ok(())
}

fn save_printer_config(printer_name: &str, config_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💾 Saving printer configuration: {} -> {}", printer_name, config_file);
    
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    let printer = frontend.get_printer(printer_name)?;
    
    match printer.save_to_file(config_file, &frontend) {
        Ok(_) => println!("✓ Printer configuration saved successfully"),
        Err(e) => eprintln!("✗ Failed to save configuration: {}", e),
    }

    Ok(())
}

fn load_printer_config(config_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📂 Loading printer configuration from: {}", config_file);
    
    match Printer::load_from_file(config_file) {
        Ok(printer) => {
            println!("✓ Printer configuration loaded successfully");
            println!("  Name: {}", printer.name().unwrap_or_else(|_| "Unknown".to_string()));
            println!("  Backend: {}", printer.backend_name().unwrap_or_else(|_| "Unknown".to_string()));
        }
        Err(e) => eprintln!("✗ Failed to load configuration: {}", e),
    }

    Ok(())
}
