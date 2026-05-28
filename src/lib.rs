//! # cpdb-rs
//!
//! Safe Rust bindings for OpenPrinting's
//! [`cpdb-libs`](https://github.com/OpenPrinting/cpdb-libs) — the Common
//! Print Dialog Backends library.
//!
//! See the [`Frontend`] entry point for printer discovery and the [`Printer`]
//! type for job submission, options, and translations.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub mod callbacks;
pub mod common;
pub mod error;
pub mod ffi;
pub mod frontend;
pub mod options;
pub mod printer;
pub mod settings;
pub mod util;

pub use callbacks::PrinterUpdate;
pub use common::{
    absolute_path, concat_path, concat_sep, init, option_group, system_config_dir,
    user_config_dir, version,
};
pub use error::{CpdbError, Result};
pub use frontend::Frontend;
pub use options::{OptionInfo, OptionsCollection};
pub use printer::{
    Margin, Margins, MediaSize, PrintFdHandle, PrintSocketHandle, Printer, TranslationMap,
};
pub use settings::{Media, Options, Settings};
