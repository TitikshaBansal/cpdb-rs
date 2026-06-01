//! Crate-level documentation for `cpdb-rs`.
//!
//! This library provides a native async Rust client for the
//! [Common Print Dialog Backends (CPDB)](https://github.com/OpenPrinting/cpdb-libs)
//! system. It communicates with CPDB backends (e.g. `cpdb-backend-cups`)
//! over D-Bus using [`zbus`], requiring no C dependencies.
//!
//! # Features
//!
//! - `zbus-backend` *(default)* - Async client via [`CpdbClient`].
//! - `ffi` - C FFI bindings via `cpdb-libs`.
//!
//! # Quick start
//!
//! ```no_run
//! # #[cfg(feature = "zbus-backend")]
//! # async fn example() -> cpdb_rs::Result<()> {
//! use cpdb_rs::CpdbClient;
//!
//! let client = CpdbClient::new().await?;
//! let printers = client.get_all_printers().await?;
//!
//! for p in &printers {
//!     println!("{} [{}] - {}", p.name, p.id, p.make_model);
//! }
//!
//! if let Some(p) = printers.first() {
//!     let (options, media) = client
//!         .get_printer_details(&p.id, &p.backend)
//!         .await?;
//!     println!("{} options, {} media sizes", options.len(), media.len());
//! }
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub mod error;
pub mod options;

#[cfg(feature = "zbus-backend")]
pub mod client;
#[cfg(feature = "zbus-backend")]
pub mod config;
#[cfg(feature = "zbus-backend")]
pub mod events;
#[cfg(feature = "zbus-backend")]
pub mod media;
#[cfg(feature = "zbus-backend")]
pub mod proxy;

// Re-export core types for convenience.
pub use config::PrinterConfig;
pub use error::{CpdbError, Result};
pub use events::{DiscoveryEvent, PrinterSnapshot};
pub use media::{MarginInfo, MediaCollection, MediaInfo};
pub use options::{OptionInfo, OptionsCollection};

#[cfg(feature = "zbus-backend")]
pub use client::CpdbClient;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "ffi")]
pub use ffi::callbacks::PrinterUpdate;
#[cfg(feature = "ffi")]
pub use ffi::{
    common::{
        absolute_path, concat_path, concat_sep, init, option_group, system_config_dir,
        user_config_dir, version,
    },
    frontend::Frontend,
    printer::{
        Margin, Margins, MediaSize, PrintFdHandle, PrintSocketHandle, Printer, TranslationMap,
    },
    settings::{Media, Options, Settings},
};
