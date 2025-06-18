extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Ensure headers are found
    println!("cargo:rustc-link-search=/usr/local/lib");
    println!("cargo:rustc-link-lib=cpdb");
    println!("cargo:rustc-link-lib=cpdb-frontend");
    println!("cargo:rustc-link-lib=cpdb-backend");
    println!("cargo:rerun-if-changed=include/wrapper.h");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("include/wrapper.h")
        .clang_arg("-I/usr/local/include")
        .clang_arg("-I/usr/include/glib-2.0")
        .clang_arg("-I/usr/lib/x86_64-linux-gnu/glib-2.0/include")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("cpdb_.*")
        .allowlist_type("cpdb_.*")
        .size_t_is_usize(true)
        .derive_default(true)
        .generate()
        .expect("Unable to generate bindings");

    // Write to output
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("cpdb_sys.rs"))
        .expect("Couldn't write bindings!");
}