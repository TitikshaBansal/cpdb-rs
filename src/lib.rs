#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/cpdb_sys.rs"));
}

pub mod error;
pub mod frontend;
pub mod backend;
pub mod printer;
pub mod job;

// Re-export main functionality
pub use frontend::*;
pub use backend::*;