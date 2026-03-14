# cpdb-rs

[![Crates.io](https://img.shields.io/crates/v/cpdb-rs.svg)](https://crates.io/crates/cpdb-rs)
[![Documentation](https://docs.rs/cpdb-rs/badge.svg)](https://docs.rs/cpdb-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Safe and idiomatic Rust bindings for the Common Print Dialog Backends (cpdb-libs) library from OpenPrinting.

## Overview

cpdb-rs provides Rust bindings for the Common Print Dialog Backends library, enabling Rust applications to interact with printing systems across different platforms. The library offers safe abstractions over the C API, with proper memory management and error handling.

## Features

- **Printer Discovery**: Find and list available printers via D-Bus
- **Job Submission**: Submit print jobs with options and titles
- **Queue Management**: Monitor and manage print queues
- **Settings Management**: Handle printer settings and options
- **Media Information**: Get media size, margins, and capabilities
- **Translations**: Option, choice, and group translation support
- **Cross-platform Support**: Full support on Linux; header-only compile verification on macOS
- **Memory Safe**: Owned/borrowed printer distinction with proper `Drop` and `Clone` semantics
- **Error Handling**: Comprehensive `#[non_exhaustive]` error type with detailed variants

## Prerequisites

### System Dependencies

Before using cpdb-rs, you need to install the cpdb-libs C library:

#### Ubuntu/Debian
```bash
sudo apt-get install libcpdb-dev libglib2.0-dev
```

#### Fedora/RHEL/CentOS
```bash
sudo dnf install cpdb-libs-devel glib2-devel
```

#### Building from Source
```bash
git clone https://github.com/OpenPrinting/cpdb-libs.git
cd cpdb-libs
./autogen.sh
./configure --prefix=/usr
make -j$(nproc)
sudo make install
sudo ldconfig
```

### Rust

Rust 1.85+ (2024 edition) is required.

## Installation

Add cpdb-rs to your `Cargo.toml`:

```toml
[dependencies]
cpdb-rs = "0.1.0"
```

## Quick Start

```rust
use cpdb_rs::{init, Frontend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the library
    init();

    // Create a frontend and connect to D-Bus
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;

    // Get available printers
    let printers = frontend.get_printers()?;

    for printer in &printers {
        println!("Printer: {}", printer.name().unwrap_or_default());
        println!("  Backend: {}", printer.backend_name().unwrap_or_default());
        println!("  State: {}", printer.get_updated_state().unwrap_or_default());
        println!("  Accepting Jobs: {}", printer.is_accepting_jobs().unwrap_or(false));
    }

    // Print a file using the first available printer
    if let Some(printer) = printers.first() {
        let job_id = printer.print_file("document.pdf")?;
        println!("Job submitted: {}", job_id);
    }

    Ok(())
}
```

## Examples

### Basic Printer Discovery

```rust
use cpdb_rs::{init, Frontend};

fn list_printers() -> Result<(), Box<dyn std::error::Error>> {
    init();

    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;

    let printers = frontend.get_printers()?;

    for printer in printers {
        println!("Name: {}", printer.name().unwrap_or_default());
        println!("Location: {}", printer.location().unwrap_or_default());
        println!("Description: {}", printer.description().unwrap_or_default());
        println!("Make & Model: {}", printer.make_and_model().unwrap_or_default());
    }

    Ok(())
}
```

### Print Job Submission

```rust
use cpdb_rs::{init, Frontend};

fn submit_print_job(printer_name: &str, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    init();

    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;

    let printer = frontend.get_printer(printer_name)?;

    // Simple print
    let job_id = printer.print_file(file_path)?;
    println!("Job ID: {}", job_id);

    // Or with options and a title
    printer.submit_job(file_path, &[("copies", "2"), ("sides", "two-sided-long-edge")], "My Job")?;

    Ok(())
}
```

### Settings Management

```rust
use cpdb_rs::{init, Settings};

fn manage_settings() -> Result<(), Box<dyn std::error::Error>> {
    init();

    let mut settings = Settings::new()?;
    settings.add_setting("copies", "1")?;
    settings.add_setting("orientation-requested", "portrait")?;
    settings.add_setting("media", "A4")?;

    // Save to the user's config directory (managed by cpdb-libs)
    settings.save_to_disk()?;

    // Load back
    let _loaded = Settings::read_from_disk()?;

    Ok(())
}
```

### Printer Options and Translations

```rust
use cpdb_rs::{init, Frontend};

fn get_printer_details(printer_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    init();

    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;

    let printer = frontend.get_printer(printer_name)?;

    println!("Default copies: {}", printer.get_default("copies").unwrap_or_default());
    println!("Current quality: {}", printer.get_current("print-quality").unwrap_or_default());
    println!("Media size: {:?}", printer.get_media_size("iso_a4_210x297mm").unwrap_or_default());

    // Translations
    println!("Option label: {}", printer.get_option_translation("copies").unwrap_or_default());
    println!("Choice label: {}", printer.get_choice_translation("sides", "two-sided-long-edge").unwrap_or_default());

    Ok(())
}
```

## CLI Examples

The repository includes several examples:

```bash
# Basic usage — list printers, check version, print a file
cargo run --example basic_usage

# Interactive CLI — list, inspect, configure printers
cargo run --example cli_printer_manager

# Full cpdb-text-frontend port — complete API demonstration
cargo run --example cpdb_text_frontend
```

## Ownership Model

Printers returned by `get_printers()`, `get_printer()`, and `find_printer()` are **borrowed** from the `Frontend` — they are valid as long as the `Frontend` is alive. Do not keep them longer than the `Frontend`.

Printers loaded via `Printer::load_from_file()` are **owned** and freed when dropped.

Cloning a `Printer` always produces a borrowing alias (`owned = false`). Prefer `&Printer` references over cloning where possible.

## Error Handling

`CpdbError` is `#[non_exhaustive]` — always include a wildcard arm when matching:

```rust
use cpdb_rs::error::CpdbError;

match some_cpdb_operation() {
    Ok(result) => { /* ... */ }
    Err(CpdbError::NullPointer) => eprintln!("Null pointer"),
    Err(CpdbError::JobFailed(msg)) => eprintln!("Job failed: {}", msg),
    Err(CpdbError::NotFound(msg)) => eprintln!("Not found: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Building on macOS

macOS is supported for header parsing and compilation verification only. Linking requires a Linux environment with D-Bus. Use `CPDB_NO_LINK=1` to skip link directives:

```bash
CPDB_NO_LINK=1 cargo build --lib
```

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Integration tests require a live D-Bus session and are #[ignore] by default
cargo test -- --ignored
```

## Troubleshooting

**"cpdb-libs not found"** — Set `CPDB_LIBS_PATH` to the installation prefix if installed in a non-standard location.

**"D-Bus connection failed"** — Ensure a D-Bus session is running and print services (CUPS, etc.) are active.

**"No printers found"** — Verify printers are configured and backends are running.

**Linker errors** — Ensure the `-dev` / `-devel` package for cpdb-libs is installed and `pkg-config` can locate it.

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

1. Fork the repository
2. Clone your fork and install dependencies: `cargo build`
3. Make your changes and add tests
4. Ensure `cargo test` and `cargo clippy -- -D warnings` both pass
5. Submit a pull request

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

- [OpenPrinting](https://openprinting.org/) for cpdb-libs
- The Rust community for excellent FFI tooling
- Contributors who help improve this project

## Related Projects

- [cpdb-libs](https://github.com/OpenPrinting/cpdb-libs) — the C library this project binds to
- [OpenPrinting](https://openprinting.org/) — OpenPrinting organisation
- [CUPS](https://www.cups.org/) — Common Unix Printing System

## Support

- [GitHub Issues](https://github.com/OpenPrinting/cpdb-rs/issues) — bug reports and feature requests
- [GitHub Discussions](https://github.com/OpenPrinting/cpdb-rs/discussions) — questions and general discussion
- [Documentation](https://docs.rs/cpdb-rs) — API documentation

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed list of changes.
