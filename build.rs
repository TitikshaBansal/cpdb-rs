extern crate bindgen;
extern crate pkg_config;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=include/wrapper.h");
    
    // Try to find cpdb-libs installation
    let cpdb_libs_path = find_cpdb_libs();
    
    // --- Linker Configuration ---
    if let Some(ref cpdb_path) = cpdb_libs_path {
        println!("cargo:rustc-link-search=native={}/cpdb/.libs", cpdb_path);
        println!("cargo:rustc-link-search=native={}/.libs", cpdb_path);
        // Also add cpdb subdir to include search for transitive headers
        println!("cargo:include={}", cpdb_path);
        println!("cargo:include={}/cpdb", cpdb_path);
    }
    
    // Add common system library paths
    add_system_library_paths();
    
    // Link required libraries
    println!("cargo:rustc-link-lib=cpdb");
    println!("cargo:rustc-link-lib=cpdb-frontend");
    if matches!(env::var("CPDB_LINK_BACKEND").ok().as_deref(), Some("1") | Some("true") | Some("yes")) {
        println!("cargo:rustc-link-lib=cpdb-backend");
    }
    println!("cargo:rustc-link-lib=glib-2.0");
    println!("cargo:rustc-link-lib=gobject-2.0");

    // --- Bindgen Builder Setup ---
    let mut builder = bindgen::Builder::default()
        .header("include/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .size_t_is_usize(true)
        .derive_default(true)
        .generate_comments(true)
        .ctypes_prefix("libc")
        .layout_tests(false)
        .raw_line("use libc;")
        .raw_line("#[allow(non_upper_case_globals)]")
        .raw_line("#[allow(non_camel_case_types)]")
        .raw_line("#[allow(non_snake_case)]")
        .raw_line("#[allow(dead_code)]");

    // Add include paths
    if let Some(ref cpdb_path) = cpdb_libs_path {
        builder = builder.clang_arg(format!("-I{}", cpdb_path));
        builder = builder.clang_arg(format!("-I{}/cpdb", cpdb_path));
        println!("Using cpdb-libs include path for bindgen: {}", cpdb_path);
    } else {
        // Fallback to common paths
        let home_dir = env::var("HOME").unwrap_or_default();
        let cpdb_libs_project_root_for_includes = format!("{}/cpdb-libs", home_dir);
        builder = builder.clang_arg(format!("-I{}", cpdb_libs_project_root_for_includes));
        builder = builder.clang_arg(format!("-I{}/cpdb", cpdb_libs_project_root_for_includes));
        println!("Using fallback cpdb-libs include path for bindgen: {}", cpdb_libs_project_root_for_includes);
    }

    if let Ok(lib_glib) = pkg_config::Config::new().probe("glib-2.0") {
        for path in lib_glib.include_paths {
            builder = builder.clang_arg(format!("-I{}", path.display()));
        }
    } else {
        println!("Warning: glib-2.0 not found via pkg-config. Adding default system GLib include paths for bindgen.");
        builder = builder.clang_arg("-I/usr/include/glib-2.0");
        builder = builder.clang_arg("-I/usr/lib/x86_64-linux-gnu/glib-2.0/include");
    }
    builder = builder.clang_arg("-I/usr/include");

    let functions_to_allow = [
        // Core functions
        "cpdbGetVersion", "cpdbInit",
        
        // Frontend functions
        "cpdbGetNewFrontendObj", "cpdbConnectToDBus", "cpdbDisconnectFromDBus",
        "cpdbStartListingPrinters", "cpdbStopListingPrinters", "cpdbDeleteFrontendObj",
        "cpdbGetPrinters", /* "cpdbGetPrinter", */ "cpdbGetAllPrinters",
        "cpdbFindPrinterObj", "cpdbGetDefaultPrinter", "cpdbGetDefaultPrinterForBackend",
        "cpdbSetUserDefaultPrinter", "cpdbSetSystemDefaultPrinter",
        "cpdbAddPrinter", "cpdbRemovePrinter",
        "cpdbHideRemotePrinters", "cpdbUnhideRemotePrinters",
        "cpdbHideTemporaryPrinters", "cpdbUnhideTemporaryPrinters",
        "cpdbRefreshPrinterList", "cpdbActivateBackends",
        "cpdbStartBackendListRefreshing", "cpdbStopBackendListRefreshing",
        
        // Printer functions
        "cpdbGetNewPrinterObj", "cpdbDeletePrinterObj",
        "cpdbGetState", "cpdbIsAcceptingJobs", "cpdbPrintFile",
        "cpdbPrintFileWithJobTitle", "cpdbPrintFD", "cpdbPrintSocket",
        "cpdbGetAllOptions", "cpdbGetOption", "cpdbGetDefault", "cpdbGetSetting", "cpdbGetCurrent",
        "cpdbAddSettingToPrinter", "cpdbClearSettingFromPrinter",
        "cpdbAcquireDetails", "cpdbAcquireTranslations",
        "cpdbGetAllTranslations", "cpdbGetOptionTranslation", "cpdbGetChoiceTranslation", "cpdbGetGroupTranslation",
        "cpdbGetMedia", "cpdbGetMediaSize", "cpdbGetMediaMargins",
        "cpdbPicklePrinterToFile", "cpdbResurrectPrinterFromFile",
        
        // Backend functions
        "cpdbGetNewBackendObj", "cpdbSubmitJob", "cpdbDeleteBackendObj",
        
        // Job functions
        "cpdbNewPrintJob", "cpdbSubmitPrintJobWithFile", "cpdbCancelJobById", "cpdbDeletePrintJob",
        
        // Settings functions
        "cpdbGetNewSettings", "cpdbDeleteSettings", "cpdbCopySettings",
        "cpdbAddSetting", "cpdbClearSetting", "cpdbSerializeToGVariant",
        "cpdbSaveSettingsToDisk", "cpdbReadSettingsFromDisk",
        
        // Options functions
        "cpdbGetNewOptions", "cpdbDeleteOptions", "cpdbDeleteOption",
        
        // Media functions
        "cpdbDeleteMedia",
        
        // Utility functions
        "cpdbNewCStringArray", "cpdbGetBoolean", "cpdbConcatSep", "cpdbConcatPath",
        "cpdbGetUserConfDir", "cpdbGetSysConfDir", "cpdbGetAbsolutePath",
        "cpdbGetGroup", "cpdbGetGroupTranslation2",
        "cpdbFDebugPrintf", "cpdbBDebugPrintf",
        "cpdbUnpackStringArray", "cpdbPackStringArray", "cpdbPackMediaArray",
        
        // Callback functions
        "cpdbPrinterCallback", "cpdbOnPrinterAdded", "cpdbOnPrinterRemoved", "cpdbOnPrinterStateChanged",
        "cpdbFillBasicOptions", "cpdbDebugPrinter", "cpdbPrintBasicOptions",
        
        // Lookup functions
        "hideRemoteLookup", "showRemoteLookup", "hideTemporaryLookup", "showTemporaryLookup",
        "stopListingLookup", "getAllPrintersLookup",
        
        // Backend creation
        "cpdbCreateBackend", "cpdbGetDbusConnection", "cpdbIgnoreLastSavedSettings",
    ];

    let types_to_allow = [
        "cpdb_frontend_obj_s", "cpdb_frontend_obj_t",
        "cpdb_printer_obj_s", "cpdb_printer_obj_t",
        "cpdb_option_t", 
        "cpdb_printer_callback", 
        "cpdb_backend_obj_s", "cpdb_backend_obj_t", 
        "cpdb_print_job_s", "cpdb_print_job_t",
        "CpdbDebugLevel",
        "gboolean",
    ];

    for func_name in functions_to_allow.iter() {
        builder = builder.allowlist_function(func_name);
    }
    for type_name in types_to_allow.iter() {
        builder = builder.allowlist_type(type_name);
    }

    let bindings = builder
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("cpdb_sys.rs"))
        .expect("Couldn't write bindings!");
}

fn find_cpdb_libs() -> Option<String> {
    // Try environment variable first
    if let Ok(path) = env::var("CPDB_LIBS_PATH") {
        return Some(path);
    }
    
    // Try common installation paths
    let home_dir = env::var("HOME").unwrap_or_default();
    let cpdb_home_path = format!("{}/cpdb-libs", home_dir);
    let cpdb_local_path = format!("{}/.local/lib/cpdb-libs", home_dir);
    let common_paths = [
        "/usr/local/lib/cpdb-libs",
        "/usr/lib/cpdb-libs", 
        "/opt/cpdb-libs",
        cpdb_home_path.as_str(),
        cpdb_local_path.as_str(),
    ];
    
    for path in &common_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    
    // Try pkg-config
    if let Ok(lib) = pkg_config::Config::new().probe("cpdb") {
        if let Some(path) = lib.link_paths.first() {
            return Some(path.to_string_lossy().to_string());
        }
    }
    
    None
}

fn add_system_library_paths() {
    let target = env::var("TARGET").unwrap_or_default();
    
    if target.contains("linux") {
        println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
        println!("cargo:rustc-link-search=native=/usr/lib");
        println!("cargo:rustc-link-search=native=/lib/x86_64-linux-gnu");
    } else if target.contains("darwin") {
        println!("cargo:rustc-link-search=native=/usr/local/lib");
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    }
}