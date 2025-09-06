#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod ffi;

pub mod error;
pub mod common;
pub mod frontend;
pub mod backend;
pub mod printer;
pub mod job;
pub mod settings;
pub mod util;

// Re-export main types
pub use common::{init, version};
pub use frontend::Frontend;
pub use printer::Printer;
pub use job::PrintJob;
pub use backend::Backend;
pub use settings::{Settings, Options, Media};