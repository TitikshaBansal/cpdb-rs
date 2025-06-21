extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Tell cargo where to find libraries
    println!("cargo:rustc-link-lib=cpdb");
    println!("cargo:rustc-link-lib=cpdb-frontend");
    println!("cargo:rustc-link-lib=cpdb-backend");
    
    // Use pkg-config to find correct include paths
    let glib = pkg_config::probe_library("glib-2.0").unwrap();
    let cpdb = pkg_config::probe_library("cpdb").unwrap();

    let mut builder = bindgen::Builder::default()
        .header("include/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("cpdb_.*")
        .allowlist_type("cpdb_.*");

    // Add include paths from pkg-config
    for path in glib.include_paths.iter().chain(cpdb.include_paths.iter()) {
        builder = builder.clang_arg(format!("-I{}", path.display()));
    }

    // Generate bindings
    let bindings = builder
        .generate()
        .expect("Unable to generate bindings");

    // Write output
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out_path.join("cpdb_sys.rs")).unwrap();
}