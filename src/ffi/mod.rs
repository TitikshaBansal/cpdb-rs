//! Foreign Function Interface (FFI) bindings for `cpdb-libs`.
//!
//! This module contains raw C bindings to the official Common Print Dialog Backends
//! (`cpdb-libs`) C library. It is only compiled when the `ffi` feature is enabled.
//!
//! While `cpdb-rs` provides a pure native Rust async implementation over D-Bus via
//! the `zbus` crate (the recommended approach), these bindings are provided for
//! interoperability, legacy support, or direct C library integration if required.
//!
//! # Warning
//! These are unsafe C bindings. In most modern Rust applications, you should use
//! the native `CpdbClient` instead of these FFI bindings.

#![cfg(feature = "ffi")]

pub mod bindings;
pub mod callbacks;
pub mod common;
pub mod frontend;
pub mod printer;
pub mod settings;
pub mod util;

pub use bindings::*;
