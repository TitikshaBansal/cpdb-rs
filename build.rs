//! Build script: locates cpdb-libs via pkg-config (preferred) or an
//! explicit `CPDB_LIBS_PATH` override, then generates bindings with
//! bindgen.
//!
//! Environment knobs:
//!
//! - `CPDB_LIBS_PATH` — point at an installed-or-built cpdb-libs prefix
//!   when pkg-config is unavailable.
//! - `CPDB_NO_LINK=1` — emit no `rustc-link-*` directives (used by the
//!   macOS CI job which only checks that bindgen + compile succeed).
//! - `BINDGEN_EXTRA_CLANG_ARGS` — forwarded to bindgen as extra `clang`
//!   args (standard bindgen knob, repeated here for visibility).

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=include/wrapper.h");
    println!("cargo:rerun-if-env-changed=CPDB_LIBS_PATH");
    println!("cargo:rerun-if-env-changed=CPDB_NO_LINK");
    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    let skip_link = env_truthy("CPDB_NO_LINK");

    let (cpdb_includes, glib_includes) = locate_includes(skip_link);

    let mut builder = bindgen::Builder::default()
        .header("include/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .size_t_is_usize(true)
        .derive_default(true)
        .generate_comments(false)
        .ctypes_prefix("libc")
        .layout_tests(false)
        .rust_edition(bindgen::RustEdition::Edition2024)
        .raw_line("use libc;");

    for path in &cpdb_includes {
        builder = builder.clang_arg(format!("-I{}", path.display()));
    }
    for path in &glib_includes {
        builder = builder.clang_arg(format!("-I{}", path.display()));
    }

    if let Ok(extra) = env::var("BINDGEN_EXTRA_CLANG_ARGS") {
        for arg in extra.split_whitespace() {
            builder = builder.clang_arg(arg);
        }
    }

    for func in ALLOWED_FUNCTIONS {
        builder = builder.allowlist_function(func);
    }
    for ty in ALLOWED_TYPES {
        builder = builder.allowlist_type(ty);
    }

    let bindings = builder
        .generate()
        .expect("unable to generate cpdb-libs bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    bindings
        .write_to_file(out_path.join("cpdb_sys.rs"))
        .expect("failed to write bindings file");
}

fn env_truthy(name: &str) -> bool {
    matches!(env::var(name).ok().as_deref(), Some("1" | "true" | "yes"))
}

/// Locates the cpdb-libs and glib-2.0 include paths and emits linker directives.
///
/// The preferred path is pkg-config (the upstream library ships `cpdb.pc`).
/// An explicit `CPDB_LIBS_PATH` override is consulted only when pkg-config
/// cannot find cpdb.
fn locate_includes(skip_link: bool) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut cpdb_includes = Vec::new();

    // 1. pkg-config (primary). probe() emits cargo:rustc-link-* directives
    //    automatically when not skipping link.
    let cpdb_via_pkg = if skip_link {
        pkg_config::Config::new()
            .cargo_metadata(false)
            .probe("cpdb")
            .ok()
    } else {
        pkg_config::Config::new().probe("cpdb").ok()
    };
    let cpdb_frontend_via_pkg = if skip_link {
        pkg_config::Config::new()
            .cargo_metadata(false)
            .probe("cpdb-frontend")
            .ok()
    } else {
        pkg_config::Config::new().probe("cpdb-frontend").ok()
    };

    if let Some(lib) = &cpdb_via_pkg {
        cpdb_includes.extend(lib.include_paths.iter().cloned());
    }
    if let Some(lib) = &cpdb_frontend_via_pkg {
        cpdb_includes.extend(lib.include_paths.iter().cloned());
    }

    // 2. Explicit env-var override. Useful for development builds against
    //    an uninstalled cpdb-libs checkout.
    if let Ok(path) = env::var("CPDB_LIBS_PATH") {
        let root = PathBuf::from(&path);
        cpdb_includes.push(root.clone());
        cpdb_includes.push(root.join("cpdb"));
        if !skip_link {
            println!("cargo:rustc-link-search=native={path}/cpdb/.libs");
            println!("cargo:rustc-link-search=native={path}/.libs");
        }
    }

    if cpdb_via_pkg.is_none() && env::var("CPDB_LIBS_PATH").is_err() && !skip_link {
        println!(
            "cargo:warning=cpdb-libs not found via pkg-config and CPDB_LIBS_PATH is unset; \
             linking may fail. Install libcpdb-dev (Debian/Ubuntu) or cpdb-libs-devel \
             (Fedora) and ensure pkg-config can find cpdb.pc."
        );
    }

    // 3. glib include paths come from pkg-config too.
    let mut glib_includes = Vec::new();
    let glib_via_pkg = if skip_link {
        pkg_config::Config::new()
            .cargo_metadata(false)
            .probe("glib-2.0")
            .ok()
    } else {
        pkg_config::Config::new().probe("glib-2.0").ok()
    };
    if let Some(lib) = &glib_via_pkg {
        glib_includes.extend(lib.include_paths.iter().cloned());
    } else {
        println!(
            "cargo:warning=glib-2.0 not found via pkg-config; bindgen may fail to parse <glib.h>."
        );
    }

    // 4. Linker libraries. With pkg-config we've already emitted link
    //    directives; only fall back to explicit names when pkg-config
    //    failed entirely.
    if !skip_link && cpdb_via_pkg.is_none() {
        println!("cargo:rustc-link-lib=cpdb");
        println!("cargo:rustc-link-lib=cpdb-frontend");
        if !skip_link && glib_via_pkg.is_none() {
            println!("cargo:rustc-link-lib=glib-2.0");
            println!("cargo:rustc-link-lib=gobject-2.0");
        }
    }

    (cpdb_includes, glib_includes)
}

// ─── Allowlists ──────────────────────────────────────────────────────────────

/// C functions exposed via the generated bindings. Anything outside this
/// list is filtered out by bindgen.
const ALLOWED_FUNCTIONS: &[&str] = &[
    // Core
    "cpdbGetVersion",
    "cpdbInit",
    // Frontend lifecycle
    "cpdbGetNewFrontendObj",
    "cpdbDeleteFrontendObj",
    "cpdbConnectToDBus",
    "cpdbDisconnectFromDBus",
    "cpdbStartListingPrinters",
    "cpdbStopListingPrinters",
    "cpdbActivateBackends",
    "cpdbStartBackendListRefreshing",
    "cpdbStopBackendListRefreshing",
    "cpdbIgnoreLastSavedSettings",
    // Printer discovery and defaults
    "cpdbGetAllPrinters",
    "cpdbFindPrinterObj",
    "cpdbGetDefaultPrinter",
    "cpdbGetDefaultPrinterForBackend",
    "cpdbSetUserDefaultPrinter",
    "cpdbSetSystemDefaultPrinter",
    "cpdbAddPrinter",
    "cpdbRemovePrinter",
    "cpdbRefreshPrinterList",
    "cpdbHideRemotePrinters",
    "cpdbUnhideRemotePrinters",
    "cpdbHideTemporaryPrinters",
    "cpdbUnhideTemporaryPrinters",
    // Printer object
    "cpdbGetNewPrinterObj",
    "cpdbDeletePrinterObj",
    "cpdbGetState",
    "cpdbIsAcceptingJobs",
    "cpdbPrintFile",
    "cpdbPrintFileWithJobTitle",
    "cpdbPrintFD",
    "cpdbPrintSocket",
    "cpdbGetAllOptions",
    "cpdbGetOption",
    "cpdbGetDefault",
    "cpdbGetSetting",
    "cpdbGetCurrent",
    "cpdbAddSettingToPrinter",
    "cpdbClearSettingFromPrinter",
    "cpdbAcquireDetails",
    "cpdbAcquireTranslations",
    "cpdbGetAllTranslations",
    "cpdbGetOptionTranslation",
    "cpdbGetChoiceTranslation",
    "cpdbGetGroupTranslation",
    "cpdbGetMedia",
    "cpdbGetMediaSize",
    "cpdbGetMediaMargins",
    "cpdbPicklePrinterToFile",
    "cpdbResurrectPrinterFromFile",
    // Settings
    "cpdbGetNewSettings",
    "cpdbDeleteSettings",
    "cpdbCopySettings",
    "cpdbAddSetting",
    "cpdbClearSetting",
    "cpdbSerializeToGVariant",
    "cpdbSaveSettingsToDisk",
    "cpdbReadSettingsFromDisk",
    // Options / media
    "cpdbGetNewOptions",
    "cpdbDeleteOptions",
    "cpdbDeleteOption",
    "cpdbDeleteMedia",
    // Misc utilities exposed by the C API
    "cpdbGetUserConfDir",
    "cpdbGetSysConfDir",
    "cpdbGetAbsolutePath",
    "cpdbGetGroup",
    "cpdbConcatSep",
    "cpdbConcatPath",
    "cpdbPackStringArray",
    "cpdbUnpackStringArray",
    "cpdbPackMediaArray",
    "cpdbDebugPrinter",
    "cpdbPrintBasicOptions",
    "cpdbFillBasicOptions",
];

/// C types exposed via the generated bindings.
const ALLOWED_TYPES: &[&str] = &[
    "cpdb_frontend_obj_s",
    "cpdb_frontend_obj_t",
    "cpdb_printer_obj_s",
    "cpdb_printer_obj_t",
    "cpdb_option_t",
    "cpdb_options_t",
    "cpdb_settings_t",
    "cpdb_media_t",
    "cpdb_margin_t",
    "cpdb_printer_callback",
    "cpdb_async_callback",
    "cpdb_printer_update_t",
    "CpdbDebugLevel",
    "gboolean",
];
