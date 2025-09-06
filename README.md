# cpdb-rs

[![Crates.io](https://img.shields.io/crates/v/cpdb-rs.svg)](https://crates.io/crates/cpdb-rs)
[![Documentation](https://docs.rs/cpdb-rs/badge.svg)](https://docs.rs/cpdb-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Safe and idiomatic Rust bindings for the Common Print Dialog Backends (cpdb-libs) library from OpenPrinting.

## Overview

cpdb-rs provides Rust bindings for the Common Print Dialog Backends library, enabling Rust applications to interact with printing systems across different platforms. The library offers safe abstractions over the C API, with proper memory management and error handling.

## Features

- **Printer Discovery**: Find and list available printers
- **Job Submission**: Submit print jobs with various options
- **Queue Management**: Monitor and manage print queues
- **Settings Management**: Handle printer settings and options
- **Media Information**: Get media size, margins, and capabilities
- **Cross-platform Support**: Works on Linux and macOS
- **Memory Safe**: Proper resource management with Rust's ownership system
- **Error Handling**: Comprehensive error types with detailed information

## Prerequisites

### System Dependencies

Before using cpdb-rs, you need to install the cpdb-libs C library:

#### Ubuntu/Debian
```bash
sudo apt-get install libcpdb-dev
```

#### Fedora/RHEL/CentOS
```bash
sudo dnf install cpdb-libs-devel
```

#### macOS (with Homebrew)
```bash
brew install cpdb-libs
```

#### Building from Source
If cpdb-libs is not available in your package manager, you can build it from source:

```bash
git clone https://github.com/OpenPrinting/cpdb-libs.git
cd cpdb-libs
./configure --prefix=/usr/local
make
sudo make install
```

### Rust Dependencies

- Rust 1.70+ (2021 edition)
- bindgen for FFI bindings
- pkg-config for library discovery

## Installation

Add cpdb-rs to your `Cargo.toml`:

```toml
[dependencies]
cpdb-rs = "0.1.0"
```

## Quick Start

```rust
use cpdb_rs::{init, Frontend, Printer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the library
    init();
    
    // Create a frontend and connect to D-Bus
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    // Get available printers
    let printers = frontend.get_printers()?;
    
    for printer in printers {
        println!("Printer: {}", printer.name().unwrap_or_default());
        println!("  Backend: {}", printer.backend_name().unwrap_or_default());
        println!("  State: {}", printer.get_updated_state().unwrap_or_default());
        println!("  Accepting Jobs: {}", printer.is_accepting_jobs().unwrap_or(false));
    }
    
    // Print a file (if printers are available)
    if let Some(printer) = printers.first() {
        printer.print_single_file("document.pdf")?;
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
use cpdb_rs::{init, Frontend, PrintJob};

fn submit_print_job(printer_name: &str, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    init();
    
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    let printer = frontend.get_printer(printer_name)?;
    
    // Simple print job
    printer.print_single_file(file_path)?;
    
    // Or with options
    let options = &[("copies", "2"), ("orientation-requested", "landscape")];
    printer.submit_job(file_path, options, "My Print Job")?;
    
    Ok(())
}
```

### Settings Management

```rust
use cpdb_rs::{init, Settings};

fn manage_settings() -> Result<(), Box<dyn std::error::Error>> {
    init();
    
    let mut settings = Settings::new()?;
    
    // Add settings
    settings.add_setting("copies", "1")?;
    settings.add_setting("orientation-requested", "portrait")?;
    settings.add_setting("media", "A4")?;
    
    // Save to file
    settings.save_to_disk("printer_settings.conf")?;
    
    // Load from file
    let loaded_settings = Settings::read_from_disk("printer_settings.conf")?;
    
    Ok(())
}
```

### Printer Options and Media

```rust
use cpdb_rs::{init, Frontend};

fn get_printer_details(printer_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    init();
    
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    
    let printer = frontend.get_printer(printer_name)?;
    
    // Get printer options
    println!("Copies: {}", printer.get_option("copies").unwrap_or_default());
    println!("Default Media: {}", printer.get_default("media").unwrap_or_default());
    println!("Current Quality: {}", printer.get_current("print-quality").unwrap_or_default());
    
    // Get media information
    println!("Media: {}", printer.get_media().unwrap_or_default());
    println!("Media Size: {}", printer.get_media_size().unwrap_or_default());
    println!("Media Margins: {}", printer.get_media_margins().unwrap_or_default());
    
    Ok(())
}
```

## CLI Example

The repository includes a comprehensive CLI example that demonstrates all features:

```bash
# List all printers
cargo run --example cli_printer_manager -- list

# Get detailed printer information
cargo run --example cli_printer_manager -- info "HP LaserJet"

# Print a file
cargo run --example cli_printer_manager -- print "HP LaserJet" document.pdf

# Show printer options
cargo run --example cli_printer_manager -- options "HP LaserJet"

# Show media information
cargo run --example cli_printer_manager -- media "HP LaserJet"

# Save printer configuration
cargo run --example cli_printer_manager -- save-config "HP LaserJet" config.conf

# Load printer configuration
cargo run --example cli_printer_manager -- load-config config.conf
```

## Error Handling

cpdb-rs provides comprehensive error handling with detailed error types:

```rust
use cpdb_rs::error::{CpdbError, Result};

fn handle_errors() -> Result<(), CpdbError> {
    match some_cpdb_operation() {
        Ok(result) => {
            // Handle success
            Ok(result)
        }
        Err(CpdbError::NullPointer) => {
            eprintln!("Null pointer encountered");
            Err(CpdbError::NullPointer)
        }
        Err(CpdbError::JobFailed(msg)) => {
            eprintln!("Print job failed: {}", msg);
            Err(CpdbError::JobFailed(msg))
        }
        Err(CpdbError::BackendError(msg)) => {
            eprintln!("Backend error: {}", msg);
            Err(CpdbError::BackendError(msg))
        }
        Err(e) => {
            eprintln!("Other error: {}", e);
            Err(e)
        }
    }
}
```

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test unit_tests

# Run only integration tests
cargo test integration

# Run tests with output
cargo test -- --nocapture
```

## Building

### Development Build

```bash
cargo build
```

### Release Build

```bash
cargo build --release
```

### Cross-compilation

For cross-compilation, you'll need to set up the appropriate toolchain and ensure cpdb-libs is available for the target platform.

## Troubleshooting

### Common Issues

1. **"cpdb-libs not found"**
   - Ensure cpdb-libs is installed on your system
   - Set `CPDB_LIBS_PATH` environment variable if installed in non-standard location

2. **"D-Bus connection failed"**
   - Ensure D-Bus session is running
   - Check that print services (CUPS, etc.) are active

3. **"No printers found"**
   - Verify printers are configured in your system
   - Check that print backends are running
   - Ensure printers are accessible

4. **"Linker errors"**
   - Install development packages for cpdb-libs
   - Ensure pkg-config can find the library

### Debug Mode

Enable debug logging:

```rust
use log::LevelFilter;
use env_logger;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .init();
    
    // Your cpdb-rs code here
}
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

1. Fork the repository
2. Clone your fork
3. Install dependencies: `cargo build`
4. Run tests: `cargo test`
5. Make your changes
6. Add tests for new functionality
7. Ensure all tests pass
8. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [OpenPrinting](https://openprinting.org/) for the cpdb-libs library
- The Rust community for excellent FFI tooling
- Contributors and users who help improve this project

## Related Projects

- [cpdb-libs](https://github.com/OpenPrinting/cpdb-libs) - The C library this project binds to
- [OpenPrinting](https://openprinting.org/) - OpenPrinting organization
- [CUPS](https://www.cups.org/) - Common Unix Printing System

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed list of changes.

## Support

- [GitHub Issues](https://github.com/your-username/cpdb-rs/issues) - Bug reports and feature requests
- [GitHub Discussions](https://github.com/your-username/cpdb-rs/discussions) - Questions and general discussion
- [Documentation](https://docs.rs/cpdb-rs) - API documentation
