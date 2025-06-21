// build.rs
extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    // Get home directory
    let home_dir = env::var("HOME").expect("Could not find home directory");
    let cpdb_include_path = format!("{}/cpdb-libs/cpdb", home_dir);
	
    // Add linker search path
    println!("cargo:rustc-link-search=native={}/cpdb-libs/cpdb/.libs", home_dir);
    
    // Link libraries
    println!("cargo:rustc-link-lib=cpdb");
    println!("cargo:rustc-link-lib=cpdb-frontend");
    println!("cargo:rustc-link-lib=cpdb-backend");
    println!("cargo:rustc-link-lib=glib-2.0");
    
    // Common include paths
    let include_paths = [
        "/usr/include",
        "/usr/include/glib-2.0",
        "/usr/lib/x86_64-linux-gnu/glib-2.0/include",
        &cpdb_include_path
    ];

    println!("Using include paths:");
    for path in &include_paths {
        println!("- {}", path);
    }

    let mut builder = bindgen::Builder::default()
        .header("include/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("cpdb_.*")
        .allowlist_type("cpdb_.*")
        .allowlist_function("cpdb-.*")
        .allowlist_type("cpdb-.*")
        .size_t_is_usize(true)
        .derive_default(true);

    // Add include paths
    for path in &include_paths {
        builder = builder.clang_arg(format!("-I{}", path));
    }

    let bindings = builder
        .generate()
        .expect("Unable to generate bindings");

    // Write output
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("cpdb_sys.rs"))
        .expect("Couldn't write bindings!");
}
