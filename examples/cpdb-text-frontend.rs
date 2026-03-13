use cpdb_rs::{Frontend, init, version};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();

    println!("cpdb-text-frontend (Rust)");
    if let Ok(v) = version() {
        println!("cpdb-libs version: {}", v);
    }

    let frontend = Arc::new(Frontend::new()?);
    frontend.ignore_last_saved_settings();

    let control_frontend = Arc::clone(&frontend);
    let control_thread = thread::spawn(move || {
        if let Err(e) = control_frontend.connect_to_dbus() {
            eprintln!("Failed to connect to D-Bus: {}", e);
            return;
        }

        // get_all_printers calls `cpdbGetAllPrinters` which prints via `cpdbPrintBasicOptions`
        control_frontend.get_all_printers();
        run_command_loop(&control_frontend);
    });

    frontend.start_backend_list_refreshing();
    control_thread.join().unwrap();
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
                frontend.get_all_printers();
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
                        Ok(p) => {
                            if p.set_user_default() {
                                println!("Set printer as user default");
                            } else {
                                println!("Couldn't set printer as user default");
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            "set-system-default-printer" => {
                if parts.len() < 3 {
                    eprintln!("Usage: set-system-default-printer <printer id> <backend name>");
                } else {
                    match frontend.find_printer(parts[1], parts[2]) {
                        Ok(p) => {
                            if p.set_system_default() {
                                println!("Set printer as system default");
                            } else {
                                println!("Couldn't set printer as system default");
                            }
                        }
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
                            match p.print_single_file(parts[1]) {
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
                            Ok((w, l)) => println!("{}x{}", w, l),
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
                            unsafe {
                                cpdb_rs::ffi::cpdbAcquireDetails(
                                    p.as_raw(),
                                    Some(acquire_details_callback),
                                    std::ptr::null_mut(),
                                );
                            }
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
                            let c_locale = std::ffi::CString::new(locale.as_str()).unwrap();
                            println!("Acquiring printer translations asynchronously...");
                            unsafe {
                                cpdb_rs::ffi::cpdbAcquireTranslations(
                                    p.as_raw(),
                                    c_locale.as_ptr(),
                                    Some(acquire_translations_callback),
                                    std::ptr::null_mut(),
                                );
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
                            if let Err(e) = p.pickle_to_file("/tmp/.printer-pickle", &frontend) {
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

// TODO: these functions still need raw FFI for GHashTable iteration,
// should expose safe types in the future.

fn cmd_get_all_options(frontend: &Frontend, printer_id: &str, backend_name: &str) {
    match frontend.find_printer(printer_id, backend_name) {
        Ok(p) => unsafe {
            let opts = cpdb_rs::ffi::cpdbGetAllOptions(p.as_raw());
            if opts.is_null() {
                println!("No options.");
            } else {
                println!("Retrieved {} options.", (*opts).count);
                let table = (*opts).table as *mut glib_sys::GHashTable;
                if !table.is_null() {
                    let mut iter: glib_sys::GHashTableIter = std::mem::zeroed();
                    let mut _key: glib_sys::gpointer = std::ptr::null_mut();
                    let mut value: glib_sys::gpointer = std::ptr::null_mut();
                    glib_sys::g_hash_table_iter_init(&mut iter, table);
                    while glib_sys::g_hash_table_iter_next(&mut iter, &mut _key, &mut value)
                        != glib_sys::GFALSE
                    {
                        let opt = value as *const cpdb_rs::ffi::cpdb_option_t;
                        if !opt.is_null() {
                            print_option(opt);
                        }
                    }
                }
            }
        },
        Err(e) => eprintln!("{}", e),
    }
}

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
        Ok(p) => unsafe {
            let c_media = std::ffi::CString::new(media_name).unwrap();
            let mut margins: *mut cpdb_rs::ffi::cpdb_margin_t = std::ptr::null_mut();
            let num = cpdb_rs::ffi::cpdbGetMediaMargins(p.as_raw(), c_media.as_ptr(), &mut margins);
            for i in 0..num {
                let m = &*margins.offset(i as isize);
                println!("{} {} {} {}", m.left, m.right, m.top, m.bottom);
            }
        },
        Err(e) => eprintln!("{}", e),
    }
}

// Display helpers

fn print_option(opt: *const cpdb_rs::ffi::cpdb_option_t) {
    unsafe {
        let name = cstr_or((*opt).option_name, "?");
        let group = cstr_or((*opt).group_name, "?");
        let default_val = cstr_or((*opt).default_value, "?");

        println!("[+] {}", name);
        println!(" --> GROUP: {}", group);
        for i in 0..(*opt).num_supported {
            let val_ptr = *(*opt).supported_values.offset(i as isize);
            if !val_ptr.is_null() {
                let val = std::ffi::CStr::from_ptr(val_ptr).to_string_lossy();
                println!("   * {}", val);
            }
        }
        println!(" --> DEFAULT: {}\n", default_val);
    }
}

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
        while glib_sys::g_hash_table_iter_next(&mut iter, &mut key, &mut value) != glib_sys::GFALSE
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

// C-ABI callbacks

unsafe extern "C" fn acquire_details_callback(
    p: *mut cpdb_rs::ffi::cpdb_printer_obj_t,
    success: libc::c_int,
    _user_data: *mut libc::c_void,
) {
    unsafe {
        if p.is_null() {
            return;
        }
        let name = cstr_or((*p).name, "?");
        let backend = cstr_or((*p).backend_name, "?");
        if success != 0 {
            println!("Details acquired for {} : {}", name, backend);
        } else {
            println!(
                "Could not acquire printer details for {} : {}",
                name, backend
            );
        }
    }
}

unsafe extern "C" fn acquire_translations_callback(
    p: *mut cpdb_rs::ffi::cpdb_printer_obj_t,
    success: libc::c_int,
    _user_data: *mut libc::c_void,
) {
    unsafe {
        if p.is_null() {
            return;
        }
        let name = cstr_or((*p).name, "?");
        let backend = cstr_or((*p).backend_name, "?");
        if success != 0 {
            println!("Translations acquired for {} : {}", name, backend);
            print_translations(p);
        } else {
            println!(
                "Could not acquire printer translations for {} : {}",
                name, backend
            );
        }
    }
}

// Helpers

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
