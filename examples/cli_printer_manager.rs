//! Small CLI for inspecting cpdb-rs from the shell.
//!
//! Usage:
//!   cli_printer_manager list
//!   cli_printer_manager info <printer_name>
//!   cli_printer_manager print <printer_name> <file_path>
//!   cli_printer_manager options <printer_name>
//!   cli_printer_manager media <printer_name>
//!   cli_printer_manager save-config <printer_name> <config_file>
//!   cli_printer_manager load-config <config_file>

use cpdb_rs::{Frontend, Printer, init, version};
use std::env;
use std::fs;

type ExResult = Result<(), Box<dyn std::error::Error>>;

fn main() -> ExResult {
    println!("cpdb-rs CLI printer manager");
    init();

    match version() {
        Ok(v) => println!("cpdb-libs version: {v}"),
        Err(e) => eprintln!("could not read cpdb-libs version: {e}"),
    }

    let args: Vec<String> = env::args().collect();
    let prog = args.first().cloned().unwrap_or_else(|| "cli".into());
    let cmd = match args.get(1).map(String::as_str) {
        Some(c) => c,
        None => {
            print_usage(&prog);
            return Ok(());
        }
    };

    match cmd {
        "list" => list_printers(),
        "info" => with_arg(&args, 2, |name| show_printer_info(name)),
        "print" => with_two_args(&args, |p, f| print_file(p, f)),
        "options" => with_arg(&args, 2, |name| show_printer_options(name)),
        "media" => with_arg(&args, 2, |name| show_printer_media(name)),
        "save-config" => with_two_args(&args, |p, f| save_printer_config(p, f)),
        "load-config" => with_arg(&args, 2, |f| load_printer_config(f)),
        other => {
            eprintln!("unknown command: {other}");
            print_usage(&prog);
            Ok(())
        }
    }
}

fn with_arg(args: &[String], idx: usize, f: impl FnOnce(&str) -> ExResult) -> ExResult {
    match args.get(idx) {
        Some(arg) => f(arg),
        None => {
            eprintln!("missing argument");
            Ok(())
        }
    }
}

fn with_two_args(args: &[String], f: impl FnOnce(&str, &str) -> ExResult) -> ExResult {
    match (args.get(2), args.get(3)) {
        (Some(a), Some(b)) => f(a, b),
        _ => {
            eprintln!("missing arguments");
            Ok(())
        }
    }
}

fn print_usage(prog: &str) {
    println!("\nUsage:");
    println!("  {prog} list");
    println!("  {prog} info <printer_name>");
    println!("  {prog} print <printer_name> <file_path>");
    println!("  {prog} options <printer_name>");
    println!("  {prog} media <printer_name>");
    println!("  {prog} save-config <printer_name> <config_file>");
    println!("  {prog} load-config <config_file>");
}

fn connect() -> Result<Frontend, Box<dyn std::error::Error>> {
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    Ok(frontend)
}

fn list_printers() -> ExResult {
    let frontend = connect()?;
    let printers = frontend.get_printers()?;
    if printers.is_empty() {
        println!("no printers discovered");
        return Ok(());
    }
    println!("{:<24} {:<14} {:<18} {:<10}", "Name", "Backend", "State", "Accepts");
    println!("{}", "-".repeat(70));
    for p in printers {
        let name = p.name().unwrap_or_else(|_| "?".into());
        let backend = p.backend_name().unwrap_or_else(|_| "?".into());
        let state = p.get_updated_state().unwrap_or_else(|_| "?".into());
        let accepting = if p.is_accepting_jobs().unwrap_or(false) {
            "yes"
        } else {
            "no"
        };
        println!("{name:<24} {backend:<14} {state:<18} {accepting:<10}");
    }
    Ok(())
}

fn show_printer_info(name: &str) -> ExResult {
    let frontend = connect()?;
    let p = frontend.get_printer(name)?;
    println!("Name: {}", p.name().unwrap_or_default());
    println!("ID: {}", p.id().unwrap_or_default());
    println!("Location: {}", p.location().unwrap_or_default());
    println!("Description: {}", p.description().unwrap_or_default());
    println!("Make & Model: {}", p.make_and_model().unwrap_or_default());
    println!("Backend: {}", p.backend_name().unwrap_or_default());
    println!("State: {}", p.get_updated_state().unwrap_or_default());
    println!("Accepting jobs: {}", p.is_accepting_jobs().unwrap_or(false));
    Ok(())
}

fn print_file(name: &str, file_path: &str) -> ExResult {
    if fs::metadata(file_path).is_err() {
        eprintln!("file not found: {file_path}");
        return Ok(());
    }
    let frontend = connect()?;
    let printer = frontend.get_printer(name)?;
    if !printer.is_accepting_jobs().unwrap_or(false) {
        eprintln!("printer is not accepting jobs");
        return Ok(());
    }
    match printer.print_file(file_path) {
        Ok(job_id) => println!("submitted: {job_id}"),
        Err(e) => eprintln!("submit failed: {e}"),
    }
    Ok(())
}

fn show_printer_options(name: &str) -> ExResult {
    let frontend = connect()?;
    let printer = frontend.get_printer(name)?;
    let common = [
        "copies",
        "page-ranges",
        "orientation-requested",
        "print-quality",
        "sides",
        "media",
        "printer-resolution",
    ];
    for opt in common {
        match printer.get_option(opt)? {
            Some(v) => println!("  {opt}: {v}"),
            None => println!("  {opt}: (unset)"),
        }
    }
    Ok(())
}

fn show_printer_media(name: &str) -> ExResult {
    let frontend = connect()?;
    let printer = frontend.get_printer(name)?;
    let media = printer
        .get_current("media")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "iso_a4_210x297mm".into());
    println!("media: {media}");
    match printer.get_media(&media) {
        Ok(Some(name)) => println!("media name: {name}"),
        Ok(None) => println!("media name: (unset)"),
        Err(e) => println!("media name: error: {e}"),
    }
    match printer.get_media_size(&media) {
        Ok(size) => println!("size: {}x{} (1/100 mm)", size.width, size.length),
        Err(e) => println!("size: error: {e}"),
    }
    match printer.get_media_margins(&media) {
        Ok(margins) => {
            for (i, m) in margins.0.iter().enumerate() {
                println!(
                    "margin[{i}]: top={}, bottom={}, left={}, right={}",
                    m.top, m.bottom, m.left, m.right
                );
            }
        }
        Err(e) => println!("margins: error: {e}"),
    }
    Ok(())
}

fn save_printer_config(name: &str, config_file: &str) -> ExResult {
    let frontend = connect()?;
    let printer = frontend.get_printer(name)?;
    match printer.pickle_to_file(config_file, &frontend) {
        Ok(()) => println!("saved {name} -> {config_file}"),
        Err(e) => eprintln!("save failed: {e}"),
    }
    Ok(())
}

fn load_printer_config(config_file: &str) -> ExResult {
    match Printer::load_from_file(config_file) {
        Ok(p) => {
            println!("loaded printer from {config_file}");
            println!("  name: {}", p.name().unwrap_or_default());
            println!("  backend: {}", p.backend_name().unwrap_or_default());
        }
        Err(e) => eprintln!("load failed: {e}"),
    }
    Ok(())
}
