#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod backend;
pub mod common;
pub mod error;
pub mod ffi;
pub mod frontend;
pub mod job;
pub mod options;
pub mod printer;
pub mod settings;
pub mod util;

// ─── Re-exports ──────────────────────────────────────────────────────────────

pub use backend::Backend;
pub use common::{init, version};
pub use frontend::Frontend;
pub use job::PrintJob;
pub use options::{OptionInfo, OptionsCollection};
pub use printer::Printer;
pub use settings::{Media, Options, Settings};
