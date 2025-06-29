#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod ffi;

pub mod error;
pub mod frontend;
pub mod backend;
pub mod printer;
pub mod job;
pub mod util;  // Add this line

// Re-export main types
pub use frontend::Frontend;
pub use printer::Printer;
pub use job::PrintJob;
pub use backend::Backend;