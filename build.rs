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
//! - `DOCS_RS=1` (set automatically by docs.rs) — bypasses bindgen
//!   entirely and writes a hand-rolled stub `cpdb_sys.rs`. docs.rs
//!   builds in a sandbox without cpdb-libs installed, so the stub
//!   contains every symbol our crate references but no actual
//!   implementation. Linking is skipped because library crates never
//!   link during `cargo doc`.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=include/wrapper.h");
    println!("cargo:rerun-if-env-changed=CPDB_LIBS_PATH");
    println!("cargo:rerun-if-env-changed=CPDB_NO_LINK");
    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=DOCS_RS");

    // docs.rs builder doesn't have cpdb-libs available. Emit a stub
    // bindings file and skip all link directives.
    if env::var_os("DOCS_RS").is_some() {
        emit_docsrs_stub();
        return;
    }

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

/// Writes a hand-rolled `cpdb_sys.rs` for docs.rs builds.
///
/// The stub mirrors every symbol the crate references — types, struct
/// shapes, function signatures, callback typedefs — but contains no
/// implementations. `cargo doc` for a library crate compiles to an rlib
/// without invoking the linker, so the missing symbols never matter.
fn emit_docsrs_stub() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    std::fs::write(out.join("cpdb_sys.rs"), DOCSRS_STUB)
        .expect("failed to write docs.rs stub bindings");
}

const DOCSRS_STUB: &str = r#"
//! Stub bindings emitted for docs.rs builds.
//!
//! docs.rs builds in a sandbox without cpdb-libs installed, so the real
//! bindgen output is unavailable. This stub mirrors the public surface
//! of the bindgen output well enough to compile, but contains no
//! implementations — every function symbol is left unresolved. Library
//! crates do not invoke the linker during `cargo doc`, so this is safe.

use libc;

// ─── Primitive aliases ──────────────────────────────────────────────────────

pub type gboolean = libc::c_int;
pub type cpdb_printer_update_t = libc::c_uint;

// ─── Opaque pointer targets ─────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Default)]
pub struct cpdb_margin_t {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct cpdb_media_t {
    pub name: *mut libc::c_char,
    pub width: i32,
    pub length: i32,
    pub num_margins: i32,
    pub margins: *mut cpdb_margin_t,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct cpdb_option_t {
    pub option_name: *mut libc::c_char,
    pub group_name: *mut libc::c_char,
    pub num_supported: i32,
    pub supported_values: *mut *mut libc::c_char,
    pub default_value: *mut libc::c_char,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct cpdb_options_t {
    pub count: i32,
    pub media_count: i32,
    pub table: *mut libc::c_void,
    pub media: *mut libc::c_void,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct cpdb_settings_t {
    pub count: i32,
    pub table: *mut libc::c_void,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct cpdb_printer_obj_s {
    pub backend_proxy: *mut libc::c_void,
    pub backend_name: *mut libc::c_char,
    pub id: *mut libc::c_char,
    pub name: *mut libc::c_char,
    pub location: *mut libc::c_char,
    pub info: *mut libc::c_char,
    pub make_and_model: *mut libc::c_char,
    pub state: *mut libc::c_char,
    pub accepting_jobs: gboolean,
    pub options: *mut cpdb_options_t,
    pub settings: *mut cpdb_settings_t,
    pub locale: *mut libc::c_char,
    pub translations: *mut libc::c_void,
}
pub type cpdb_printer_obj_t = cpdb_printer_obj_s;

#[repr(C)]
#[derive(Debug, Default)]
pub struct cpdb_frontend_obj_s {
    pub connection: *mut libc::c_void,
    pub printer_cb: cpdb_printer_callback,
    pub num_backends: i32,
    pub backend: *mut libc::c_void,
    pub num_printers: i32,
    pub printer: *mut libc::c_void,
    pub hide_remote: gboolean,
    pub hide_temporary: gboolean,
    pub stop_flag: gboolean,
    pub last_saved_settings: *mut cpdb_settings_t,
    pub background_thread: *mut libc::c_void,
    pub dbus_subscriptions: *mut libc::c_void,
}
pub type cpdb_frontend_obj_t = cpdb_frontend_obj_s;

// ─── Callback typedefs ──────────────────────────────────────────────────────

pub type cpdb_printer_callback = ::core::option::Option<
    unsafe extern "C" fn(
        frontend: *mut cpdb_frontend_obj_t,
        printer: *mut cpdb_printer_obj_t,
        update: cpdb_printer_update_t,
    ),
>;

pub type cpdb_async_callback = ::core::option::Option<
    unsafe extern "C" fn(
        printer: *mut cpdb_printer_obj_t,
        status: libc::c_int,
        user_data: *mut libc::c_void,
    ),
>;

// ─── Function declarations (no implementations on docs.rs) ──────────────────

unsafe extern "C" {
    pub fn cpdbGetVersion() -> *const libc::c_char;
    pub fn cpdbInit();

    pub fn cpdbGetNewFrontendObj(cb: cpdb_printer_callback) -> *mut cpdb_frontend_obj_t;
    pub fn cpdbDeleteFrontendObj(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbConnectToDBus(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbDisconnectFromDBus(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbStartListingPrinters(cb: cpdb_printer_callback) -> *mut cpdb_frontend_obj_t;
    pub fn cpdbStopListingPrinters(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbActivateBackends(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbStartBackendListRefreshing(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbStopBackendListRefreshing(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbIgnoreLastSavedSettings(frontend: *mut cpdb_frontend_obj_t);

    pub fn cpdbGetAllPrinters(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbFindPrinterObj(
        frontend: *mut cpdb_frontend_obj_t,
        id: *const libc::c_char,
        backend: *const libc::c_char,
    ) -> *mut cpdb_printer_obj_t;
    pub fn cpdbGetDefaultPrinter(frontend: *mut cpdb_frontend_obj_t) -> *mut cpdb_printer_obj_t;
    pub fn cpdbGetDefaultPrinterForBackend(
        frontend: *mut cpdb_frontend_obj_t,
        backend: *const libc::c_char,
    ) -> *mut cpdb_printer_obj_t;
    pub fn cpdbAddPrinter(
        frontend: *mut cpdb_frontend_obj_t,
        printer: *mut cpdb_printer_obj_t,
    ) -> gboolean;
    pub fn cpdbRemovePrinter(
        frontend: *mut cpdb_frontend_obj_t,
        id: *const libc::c_char,
        backend: *const libc::c_char,
    ) -> *mut cpdb_printer_obj_t;
    pub fn cpdbRefreshPrinterList(
        frontend: *mut cpdb_frontend_obj_t,
        backend: *const libc::c_char,
    ) -> bool;
    pub fn cpdbHideRemotePrinters(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbUnhideRemotePrinters(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbHideTemporaryPrinters(frontend: *mut cpdb_frontend_obj_t);
    pub fn cpdbUnhideTemporaryPrinters(frontend: *mut cpdb_frontend_obj_t);

    pub fn cpdbGetNewPrinterObj() -> *mut cpdb_printer_obj_t;
    pub fn cpdbDeletePrinterObj(printer: *mut cpdb_printer_obj_t);
    pub fn cpdbGetState(printer: *mut cpdb_printer_obj_t) -> *mut libc::c_char;
    pub fn cpdbIsAcceptingJobs(printer: *mut cpdb_printer_obj_t) -> gboolean;
    pub fn cpdbSetUserDefaultPrinter(printer: *mut cpdb_printer_obj_t) -> gboolean;
    pub fn cpdbSetSystemDefaultPrinter(printer: *mut cpdb_printer_obj_t) -> gboolean;
    pub fn cpdbPrintFile(
        printer: *mut cpdb_printer_obj_t,
        file_path: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbPrintFileWithJobTitle(
        printer: *mut cpdb_printer_obj_t,
        file_path: *const libc::c_char,
        title: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbPrintFD(
        printer: *mut cpdb_printer_obj_t,
        jobid_out: *mut *mut libc::c_char,
        title: *const libc::c_char,
        socket_path_out: *mut *mut libc::c_char,
    ) -> libc::c_int;
    pub fn cpdbPrintSocket(
        printer: *mut cpdb_printer_obj_t,
        jobid_out: *mut *mut libc::c_char,
        title: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbGetAllOptions(printer: *mut cpdb_printer_obj_t) -> *mut cpdb_options_t;
    pub fn cpdbGetOption(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
    ) -> *mut cpdb_option_t;
    pub fn cpdbGetDefault(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbGetSetting(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbGetCurrent(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbAddSettingToPrinter(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
        value: *const libc::c_char,
    );
    pub fn cpdbClearSettingFromPrinter(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
    ) -> gboolean;
    pub fn cpdbAcquireDetails(
        printer: *mut cpdb_printer_obj_t,
        cb: cpdb_async_callback,
        user_data: *mut libc::c_void,
    );
    pub fn cpdbAcquireTranslations(
        printer: *mut cpdb_printer_obj_t,
        locale: *const libc::c_char,
        cb: cpdb_async_callback,
        user_data: *mut libc::c_void,
    );
    pub fn cpdbGetAllTranslations(
        printer: *mut cpdb_printer_obj_t,
        locale: *const libc::c_char,
    );
    pub fn cpdbGetOptionTranslation(
        printer: *mut cpdb_printer_obj_t,
        option: *const libc::c_char,
        locale: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbGetChoiceTranslation(
        printer: *mut cpdb_printer_obj_t,
        option: *const libc::c_char,
        choice: *const libc::c_char,
        locale: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbGetGroupTranslation(
        printer: *mut cpdb_printer_obj_t,
        group: *const libc::c_char,
        locale: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbGetOptionTranslationFromTable(
        printer: *mut cpdb_printer_obj_t,
        option: *const libc::c_char,
        locale: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbGetChoiceTranslationFromTable(
        printer: *mut cpdb_printer_obj_t,
        option: *const libc::c_char,
        choice: *const libc::c_char,
        locale: *const libc::c_char,
    ) -> *mut libc::c_char;
    pub fn cpdbGetMedia(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
    ) -> *mut cpdb_media_t;
    pub fn cpdbGetMediaSize(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
        width: *mut i32,
        length: *mut i32,
    ) -> i32;
    pub fn cpdbGetMediaMargins(
        printer: *mut cpdb_printer_obj_t,
        name: *const libc::c_char,
        margins: *mut *mut cpdb_margin_t,
    ) -> i32;
    pub fn cpdbPicklePrinterToFile(
        printer: *mut cpdb_printer_obj_t,
        path: *const libc::c_char,
        frontend: *const cpdb_frontend_obj_t,
    );
    pub fn cpdbResurrectPrinterFromFile(path: *const libc::c_char) -> *mut cpdb_printer_obj_t;
    pub fn cpdbDebugPrinter(printer: *const cpdb_printer_obj_t);
    pub fn cpdbPrintBasicOptions(printer: *const cpdb_printer_obj_t);

    pub fn cpdbGetNewSettings() -> *mut cpdb_settings_t;
    pub fn cpdbDeleteSettings(settings: *mut cpdb_settings_t);
    pub fn cpdbCopySettings(src: *const cpdb_settings_t, dst: *mut cpdb_settings_t);
    pub fn cpdbAddSetting(
        settings: *mut cpdb_settings_t,
        name: *const libc::c_char,
        value: *const libc::c_char,
    );
    pub fn cpdbClearSetting(
        settings: *mut cpdb_settings_t,
        name: *const libc::c_char,
    ) -> gboolean;
    pub fn cpdbSaveSettingsToDisk(settings: *mut cpdb_settings_t);
    pub fn cpdbReadSettingsFromDisk() -> *mut cpdb_settings_t;

    pub fn cpdbGetNewOptions() -> *mut cpdb_options_t;
    pub fn cpdbDeleteOptions(options: *mut cpdb_options_t);
    pub fn cpdbDeleteOption(option: *mut cpdb_option_t);
    pub fn cpdbDeleteMedia(media: *mut cpdb_media_t);

    // D-Bus connection probe.
    pub fn cpdbGetDbusConnection() -> *mut libc::c_void;

    // Path / config helpers.
    pub fn cpdbGetUserConfDir() -> *mut libc::c_char;
    pub fn cpdbGetSysConfDir() -> *mut libc::c_char;
    pub fn cpdbGetAbsolutePath(path: *const libc::c_char) -> *mut libc::c_char;
    pub fn cpdbConcatSep(a: *const libc::c_char, b: *const libc::c_char) -> *mut libc::c_char;
    pub fn cpdbConcatPath(a: *const libc::c_char, b: *const libc::c_char) -> *mut libc::c_char;
    pub fn cpdbGetGroup(option_name: *const libc::c_char) -> *mut libc::c_char;
}
"#;

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
    "cpdbGetOptionTranslationFromTable",
    "cpdbGetChoiceTranslationFromTable",
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
