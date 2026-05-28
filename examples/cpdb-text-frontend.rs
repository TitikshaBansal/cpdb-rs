//! `cpdb-text-frontend` port: an interactive driver for cpdb-rs.
//!
//! Single-threaded — cpdb-libs spawns its own background thread for
//! backend refreshing, but the Rust caller stays on one thread so we
//! never alias the `Frontend` from two Rust contexts at once.

use cpdb_rs::{Frontend, init, version};
use std::io::{self, BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();

    println!("cpdb-text-frontend (Rust)");
    if let Ok(v) = version() {
        println!("cpdb-libs version: {}", v);
    }

    let frontend = Frontend::new()?;
    frontend.ignore_last_saved_settings();
    frontend.connect_to_dbus()?;

    // `cpdbGetAllPrinters` prints via `cpdbPrintBasicOptions`.
    frontend.refresh_printers();

    frontend.start_backend_list_refreshing();
    run_command_loop(&frontend);
    frontend.stop_backend_list_refreshing();

    Ok(())
}

fn run_command_loop(frontend: &Frontend) {
    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "help" => print_help(),
            "stop" => {
                println!("Stopping...");
                break;
            }
            "version" => {
                if let Ok(v) = version() {
                    println!("CPDB v{}", v);
                }
            }
            "restart" => {
                frontend.stop_backend_list_refreshing();
                frontend.activate_backends();
                frontend.start_backend_list_refreshing();
                println!("Restarted.");
            }
            "hide-remote" => {
                frontend.hide_remote_printers();
                println!("Hiding remote printers.");
            }
            "unhide-remote" => {
                frontend.unhide_remote_printers();
                println!("Unhiding remote printers.");
            }
            "hide-temporary" => {
                frontend.hide_temporary_printers();
                println!("Hiding temporary printers.");
            }
            "unhide-temporary" => {
                frontend.unhide_temporary_printers();
                println!("Unhiding temporary printers.");
            }
            "get-all-printers" => {
                frontend.refresh_printers();
            }
            "get-default-printer" => match frontend.get_default_printer() {
                Ok(p) => println!(
                    "Default printer: {}#{}",
                    p.name().unwrap_or_default(),
                    p.backend_name().unwrap_or_default()
                ),
                Err(_) => println!("No default printer found"),
            },
            "get-default-printer-for-backend" => {
                if parts.len() < 2 {
                    eprintln!("Usage: get-default-printer-for-backend <backend name>");
                } else {
                    match frontend.get_default_printer_for_backend(parts[1]) {
                        Ok(p) => println!("{}", p.name().unwrap_or_default()),
                        Err(_) => println!("No default printer for backend found"),
                    }
                }
            }
            "set-user-default-printer" => {
                if parts.len() < 3 {
                    eprintln!("Usage: set-user-default-printer <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => match p.set_user_default() {
                            Ok(true) => println!("Set printer as user default"),
                            Ok(false) => println!("Couldn't set printer as user default"),
                            Err(e) => eprintln!("{}", e),
                        },
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "set-system-default-printer" => {
                if parts.len() < 3 {
                    eprintln!("Usage: set-system-default-printer <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => match p.set_system_default() {
                            Ok(true) => println!("Set printer as system default"),
                            Ok(false) => println!("Couldn't set printer as system default"),
                            Err(e) => eprintln!("{}", e),
                        },
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "print-file" => {
                if parts.len() < 4 {
                    eprintln!("Usage: print-file <file path> <printer_id> <backend_name>");
                } else {
                    match frontend.find_printer(parts[2], parts[3]) {
                        Ok(p) => {
                            p.add_setting("copies", "3").ok();
                            match p.print_file(parts[1]) {
                                Ok(id) => println!("Job submitted. ID: {}", id),
                                Err(e) => eprintln!("Print failed: {}", e),
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-state" => {
                if parts.len() < 3 {
                    eprintln!("Usage: get-state <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => match p.get_updated_state() {
                            Ok(s) => println!("{}", s),
                            Err(e) => eprintln!("{}", e),
                        },
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "is-accepting-jobs" => {
                if parts.len() < 3 {
                    eprintln!("Usage: is-accepting-jobs <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => match p.is_accepting_jobs() {
                            Ok(b) => println!("Accepting jobs ? : {}", b),
                            Err(e) => eprintln!("{}", e),
                        },
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-all-options" => {
                if parts.len() < 3 {
                    eprintln!("Usage: get-all-options <printer id> <backend name>");
                } else {
                    cmd_get_all_options(frontend, parts[1], parts[2]);
                }
            }
            "get-all-media" => {
                if parts.len() < 3 {
                    eprintln!("Usage: get-all-media <printer id> <backend name>");
                } else {
                    cmd_get_all_media(frontend, parts[1], parts[2]);
                }
            }
            "get-default" => {
                if parts.len() < 4 {
                    eprintln!("Usage: get-default <option name> <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[2], parts[3]) {
                        Ok(p) => match p.get_default(parts[1]) {
                            Ok(v) => println!("Default : {}", v),
                            Err(e) => eprintln!("{}", e),
                        },
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-setting" => {
                if parts.len() < 4 {
                    eprintln!("Usage: get-setting <option name> <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[2], parts[3]) {
                        Ok(p) => match p.get_setting(parts[1]) {
                            Ok(Some(v)) => println!("Setting value : {}", v),
                            Ok(None) => println!("Setting {} doesn't exist.", parts[1]),
                            Err(e) => eprintln!("{}", e),
                        },
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-current" => {
                if parts.len() < 4 {
                    eprintln!("Usage: get-current <option name> <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[2], parts[3]) {
                        Ok(p) => match p.get_current(parts[1]) {
                            Ok(v) => println!("Current value : {}", v),
                            Err(e) => eprintln!("{}", e),
                        },
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "add-setting" => {
                if parts.len() < 5 {
                    eprintln!(
                        "Usage: add-setting <option name> <option value> <printer id> <backend name>"
                    );
                } else {
                    match frontend.find_printer(parts[3], parts[4]) {
                        Ok(p) => {
                            println!("{} : {}", parts[1], parts[2]);
                            if let Err(e) = p.add_setting(parts[1], parts[2]) {
                                eprintln!("{}", e);
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "clear-setting" => {
                if parts.len() < 4 {
                    eprintln!("Usage: clear-setting <option name> <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[2], parts[3]) {
                        Ok(p) => {
                            if let Err(e) = p.clear_setting(parts[1]) {
                                eprintln!("{}", e);
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-media-size" => {
                if parts.len() < 4 {
                    eprintln!("Usage: get-media-size <media> <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[2], parts[3]) {
                        Ok(p) => match p.get_media_size(parts[1]) {
                            Ok(size) => println!("{}x{}", size.width, size.length),
                            Err(e) => eprintln!("{}", e),
                        },
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-media-margins" => {
                if parts.len() < 4 {
                    eprintln!("Usage: get-media-margins <media> <printer id> <backend name>");
                } else {
                    cmd_get_media_margins(frontend, parts[1], parts[2], parts[3]);
                }
            }
            "acquire-details" => {
                if parts.len() < 3 {
                    eprintln!("Usage: acquire-details <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => {
                            println!("Acquiring printer details asynchronously...");
                            p.acquire_details_with(|p, ok| {
                                let name = p.name().unwrap_or_default();
                                let backend = p.backend_name().unwrap_or_default();
                                if ok {
                                    println!("Details acquired for {} : {}", name, backend);
                                } else {
                                    println!(
                                        "Could not acquire printer details for {} : {}",
                                        name, backend
                                    );
                                }
                            });
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "acquire-translations" => {
                if parts.len() < 3 {
                    eprintln!("Usage: acquire-translations <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => {
                            let locale = get_locale();
                            println!("Acquiring printer translations asynchronously...");
                            if let Err(e) = p.acquire_translations_with(&locale, |p, ok| {
                                let name = p.name().unwrap_or_default();
                                let backend = p.backend_name().unwrap_or_default();
                                if ok {
                                    println!(
                                        "Translations acquired for {} : {}",
                                        name, backend
                                    );
                                    // SAFETY: borrowed printer is valid here.
                                    print_translations(p.as_raw());
                                } else {
                                    println!(
                                        "Could not acquire printer translations for {} : {}",
                                        name, backend
                                    );
                                }
                            }) {
                                eprintln!("{}", e);
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-option-translation" => {
                if parts.len() < 4 {
                    eprintln!("Usage: get-option-translation <option> <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[2], parts[3]) {
                        Ok(p) => {
                            let locale = get_locale();
                            match p.get_option_translation(parts[1], &locale) {
                                Ok(Some(t)) => println!("{}", t),
                                Ok(None) => println!("No translation found"),
                                Err(e) => eprintln!("{}", e),
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-choice-translation" => {
                if parts.len() < 5 {
                    eprintln!(
                        "Usage: get-choice-translation <option> <choice> <printer id> <backend name>"
                    );
                } else {
                    match frontend.find_printer(parts[3], parts[4]) {
                        Ok(p) => {
                            let locale = get_locale();
                            match p.get_choice_translation(parts[1], parts[2], &locale) {
                                Ok(Some(t)) => println!("{}", t),
                                Ok(None) => println!("No translation found"),
                                Err(e) => eprintln!("{}", e),
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-group-translation" => {
                if parts.len() < 4 {
                    eprintln!("Usage: get-group-translation <group> <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[2], parts[3]) {
                        Ok(p) => {
                            let locale = get_locale();
                            match p.get_group_translation(parts[1], &locale) {
                                Ok(Some(t)) => println!("{}", t),
                                Ok(None) => println!("No translation found"),
                                Err(e) => eprintln!("{}", e),
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "get-all-translations" => {
                if parts.len() < 3 {
                    eprintln!("Usage: get-all-translations <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => {
                            let locale = get_locale();
                            if let Err(e) = p.get_all_translations(&locale) {
                                eprintln!("{}", e);
                            } else {
                                print_translations(p.as_raw());
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "pickle-printer" => {
                if parts.len() < 3 {
                    eprintln!("Usage: pickle-printer <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => {
                            if let Err(e) = p.pickle_to_file("/tmp/.printer-pickle", frontend) {
                                eprintln!("{}", e);
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            _ => eprintln!(
                "Unknown command: '{}'. Type 'help' for available commands.",
                parts[0]
            ),
        }
    }
}

// ─── Safe command implementations ────────────────────────────────────────────

/// Lists all printer options using the safe `OptionsCollection` type.
///
/// This function contains zero `unsafe` blocks.
fn cmd_get_all_options(frontend: &Frontend, printer_id: &str, backend_name: &str) {
    let p = match frontend.find_printer(printer_id, backend_name) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };

    // Ensure the backend has populated the options table before reading it.
    if let Err(e) = p.acquire_details() {
        eprintln!("acquire_details failed: {}", e);
        return;
    }

    match p.get_options_collection() {
        Ok(collection) if collection.is_empty() => {
            println!("No options available.");
        }
        Ok(collection) => {
            println!("Retrieved {} options.", collection.len());
            for opt in collection.iter() {
                println!("[+] {}", opt.name);
                println!(" --> GROUP: {}", opt.group);
                for val in &opt.supported_values {
                    println!("   * {}", val);
                }
                println!(" --> DEFAULT: {}\n", opt.default_value);
            }
        }
        Err(e) => eprintln!("get_options_collection failed: {}", e),
    }
}

// ─── Unsafe command implementations ──────────────────────────────────────────
// TODO: cmd_get_all_media and print_translations still iterate GHashTable
// directly. These will be replaced with safe MediaCollection and
// TranslationMap types in a future PR.

fn cmd_get_all_media(frontend: &Frontend, printer_id: &str, backend_name: &str) {
    match frontend.find_printer(printer_id, backend_name) {
        Ok(p) => unsafe {
            let opts = cpdb_rs::ffi::cpdbGetAllOptions(p.as_raw());
            if opts.is_null() {
                println!("No options.");
                return;
            }
            println!("Retrieved {} medias.", (*opts).media_count);
            let media_table = (*opts).media as *mut glib_sys::GHashTable;
            if !media_table.is_null() {
                let mut iter: glib_sys::GHashTableIter = std::mem::zeroed();
                let mut _key: glib_sys::gpointer = std::ptr::null_mut();
                let mut value: glib_sys::gpointer = std::ptr::null_mut();
                glib_sys::g_hash_table_iter_init(&mut iter, media_table);
                while glib_sys::g_hash_table_iter_next(&mut iter, &mut _key, &mut value)
                    != glib_sys::GFALSE
                {
                    let media = value as *const cpdb_rs::ffi::cpdb_media_t;
                    if !media.is_null() {
                        print_media(media);
                    }
                }
            }
        },
        Err(e) => eprintln!("{}", e),
    }
}

fn cmd_get_media_margins(
    frontend: &Frontend,
    media_name: &str,
    printer_id: &str,
    backend_name: &str,
) {
    match frontend.find_printer(printer_id, backend_name) {
        Ok(p) => match p.get_media_margins(media_name) {
            Ok(margins) => {
                for m in &margins.0 {
                    println!("{} {} {} {}", m.left, m.right, m.top, m.bottom);
                }
            }
            Err(e) => eprintln!("{}", e),
        },
        Err(e) => eprintln!("{}", e),
    }
}

// ─── Display helpers ─────────────────────────────────────────────────────────

fn print_media(media: *const cpdb_rs::ffi::cpdb_media_t) {
    unsafe {
        let name = cstr_or((*media).name, "?");
        println!("[+] Media: {}", name);
        println!("   * width = {}", (*media).width);
        println!("   * length = {}", (*media).length);
        println!(" --> Supported margins: {}", (*media).num_margins);
        println!("     left, right, top, bottom");
        for i in 0..(*media).num_margins {
            let m = &*(*media).margins.offset(i as isize);
            println!("     * {}, {}, {}, {}", m.left, m.right, m.top, m.bottom);
        }
        println!();
    }
}

fn print_translations(p: *mut cpdb_rs::ffi::cpdb_printer_obj_t) {
    unsafe {
        if p.is_null() || (*p).locale.is_null() || (*p).translations.is_null() {
            println!("No translations found");
            return;
        }
        let table = (*p).translations as *mut glib_sys::GHashTable;
        let mut iter: glib_sys::GHashTableIter = std::mem::zeroed();
        let mut key: glib_sys::gpointer = std::ptr::null_mut();
        let mut value: glib_sys::gpointer = std::ptr::null_mut();
        glib_sys::g_hash_table_iter_init(&mut iter, table);
        while glib_sys::g_hash_table_iter_next(&mut iter, &mut key, &mut value)
            != glib_sys::GFALSE
        {
            let k = key as *const libc::c_char;
            let v = value as *const libc::c_char;
            if !k.is_null() && !v.is_null() {
                let ks = std::ffi::CStr::from_ptr(k).to_string_lossy();
                let vs = std::ffi::CStr::from_ptr(v).to_string_lossy();
                println!("'{}' : '{}'", ks, vs);
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn cstr_or(ptr: *const libc::c_char, fallback: &str) -> String {
    if ptr.is_null() {
        fallback.to_string()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
    }
}

fn get_locale() -> String {
    std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en_US".to_string())
}

fn print_help() {
    println!("Available commands:");
    println!("  stop");
    println!("  restart");
    println!("  version");
    println!("  hide-remote / unhide-remote");
    println!("  hide-temporary / unhide-temporary");
    println!("  get-all-printers");
    println!("  get-default-printer");
    println!("  get-default-printer-for-backend <backend name>");
    println!("  set-user-default-printer <printer id> <backend name>");
    println!("  set-system-default-printer <printer id> <backend name>");
    println!("  print-file <file path> <printer_id> <backend_name>");
    println!("  get-state <printer id> <backend name>");
    println!("  is-accepting-jobs <printer id> <backend name>");
    println!("  acquire-details <printer id> <backend name>");
    println!("  acquire-translations <printer id> <backend name>");
    println!("  get-all-options <printer id> <backend name>");
    println!("  get-all-media <printer id> <backend name>");
    println!("  get-default <option name> <printer id> <backend name>");
    println!("  get-setting <option name> <printer id> <backend name>");
    println!("  get-current <option name> <printer id> <backend name>");
    println!("  add-setting <option name> <option value> <printer id> <backend name>");
    println!("  clear-setting <option name> <printer id> <backend name>");
    println!("  get-media-size <media> <printer id> <backend name>");
    println!("  get-media-margins <media> <printer id> <backend name>");
    println!("  get-option-translation <option> <printer id> <backend name>");
    println!("  get-choice-translation <option> <choice> <printer id> <backend name>");
    println!("  get-group-translation <group> <printer id> <backend name>");
    println!("  get-all-translations <printer id> <backend name>");
    println!("  pickle-printer <printer id> <backend name>");
}
