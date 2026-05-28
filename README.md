# cpdb-rs

[![Crates.io](https://img.shields.io/crates/v/cpdb-rs.svg)](https://crates.io/crates/cpdb-rs)
[![Documentation](https://docs.rs/cpdb-rs/badge.svg)](https://docs.rs/cpdb-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Safe Rust bindings for the Common Print Dialog Backends
([`cpdb-libs`](https://github.com/OpenPrinting/cpdb-libs)) library from
OpenPrinting.

## Overview

cpdb-rs lets Rust applications drive cpdb-libs over D-Bus: discover
printers, inspect their options and translations, and submit print jobs.
The crate is built around safe owning/borrowing types and `Result`-based
error handling on top of bindgen-generated FFI.

## Features

- **Printer discovery** over D-Bus
- **Job submission** with per-job options and titles
- **Settings management** — global (`Settings`) and per-printer
- **Option & translation lookup**, including localised labels
- **Media information** — sizes and per-media margin tables
- **Memory-safe** — owned/borrowed split enforced by lifetimes
- **Linux-first**; macOS supports a headers-only verification build

## Prerequisites

### System dependencies

Install the cpdb-libs C library and GLib headers.

```bash
# Debian / Ubuntu
sudo apt-get install libcpdb-dev libglib2.0-dev

# Fedora / RHEL / CentOS
sudo dnf install cpdb-libs-devel glib2-devel
```

Building from source:

```bash
git clone https://github.com/OpenPrinting/cpdb-libs.git
cd cpdb-libs
./autogen.sh
./configure --prefix=/usr
make -j"$(nproc)"
sudo make install
sudo ldconfig
```

### Rust

Rust 1.85+ (2024 edition) is required.

## Installation

```toml
[dependencies]
cpdb-rs = "0.1.0"
```

## Quick start

```rust
use cpdb_rs::{Frontend, init};

fn main() -> cpdb_rs::Result<()> {
    init();

    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;

    for printer in frontend.get_printers()? {
        println!("Printer: {}", printer.name()?);
        println!("  Backend: {}", printer.backend_name()?);
        println!("  State:   {}", printer.get_updated_state()?);
        println!("  Accepts: {}", printer.is_accepting_jobs()?);
    }

    Ok(())
}
```

## Examples

### Printer discovery

```rust
use cpdb_rs::{Frontend, init};

fn list_printers() -> cpdb_rs::Result<()> {
    init();
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    for printer in frontend.get_printers()? {
        println!("Name: {}", printer.name()?);
        println!("Location: {}", printer.location()?);
        println!("Description: {}", printer.description()?);
        println!("Make & Model: {}", printer.make_and_model()?);
    }
    Ok(())
}
```

### Looking up a specific printer

```rust
use cpdb_rs::{Frontend, init};

fn find_one() -> cpdb_rs::Result<()> {
    init();
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;

    // By (id, backend) — the canonical lookup; O(1) inside cpdb-libs.
    let p = frontend.find_printer("HP_LaserJet_4", "CUPS")?;
    println!("found {} on {}", p.name()?, p.backend_name()?);
    Ok(())
}
```

### Print job submission

```rust
use cpdb_rs::{Frontend, init};

fn submit(printer_name: &str, file_path: &str) -> cpdb_rs::Result<()> {
    init();
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;

    let printer = frontend.get_printer(printer_name)?;

    // No-options print.
    let job_id = printer.print_file(file_path)?;
    println!("job: {job_id}");

    // With options and a title — options are applied to the printer's
    // setting table before submission.
    let job_id = printer.submit_job(
        file_path,
        &[("copies", "2"), ("sides", "two-sided-long-edge")],
        "My Job",
    )?;
    println!("job: {job_id}");
    Ok(())
}
```

### Settings persistence

```rust
use cpdb_rs::{Settings, init};

fn manage() -> cpdb_rs::Result<()> {
    init();
    let mut s = Settings::new()?;
    s.add_setting("copies", "1")?;
    s.add_setting("orientation-requested", "portrait")?;
    s.add_setting("media", "A4")?;

    // Persists to the cpdb-managed user config directory.
    s.save_to_disk()?;
    let _loaded = Settings::read_from_disk()?;
    Ok(())
}
```

### Options and translations

```rust
use cpdb_rs::{Frontend, init};

fn details(printer_name: &str) -> cpdb_rs::Result<()> {
    init();
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;

    let p = frontend.get_printer(printer_name)?;

    println!("default copies:  {:?}", p.get_default("copies")?);
    println!("current quality: {:?}", p.get_current("print-quality")?);

    let size = p.get_media_size("iso_a4_210x297mm")?;
    println!("A4: {} x {} (1/100 mm)", size.width, size.length);

    if let Some(label) = p.get_option_translation("copies", "en_US")? {
        println!("option label: {label}");
    }
    if let Some(label) = p.get_choice_translation("sides", "two-sided-long-edge", "en_US")? {
        println!("choice label: {label}");
    }
    Ok(())
}
```

## CLI examples

```bash
# Basic usage — list printers, check version, submit a tiny file
cargo run --example basic_usage

# Interactive CLI — list, inspect, configure printers
cargo run --example cli_printer_manager

# Full cpdb-text-frontend port — every cpdb-rs API exercised
cargo run --example cpdb-text-frontend
```

## Ownership model

`Printer` carries a lifetime tied to the `Frontend` it came from. Borrowed
printers (those returned by `get_printers`, `get_printer`, `find_printer`,
`get_default_printer`, ...) cannot outlive their frontend — the compiler
checks this for you. Owned printers (`Printer::load_from_file`) have a
`'static` lifetime and are freed when dropped.

`Printer` is intentionally **not** `Send` or `Sync`. cpdb-libs does not
lock internally; if you need cross-thread access, wrap the printer in a
`Mutex` (or, more typically, run your printer operations on a single
thread).

`Frontend` is `Send` but **not** `Sync` — for the same reason.

## Error handling

`CpdbError` is `#[non_exhaustive]`, so always include a wildcard arm:

```rust
use cpdb_rs::CpdbError;

match some_op() {
    Ok(value) => { /* ... */ }
    Err(CpdbError::NullPointer) => eprintln!("null pointer"),
    Err(CpdbError::NotFound(what)) => eprintln!("not found: {what}"),
    Err(CpdbError::JobFailed(msg)) => eprintln!("job failed: {msg}"),
    Err(e) => eprintln!("other: {e}"),
}
```

## Building on macOS

macOS is supported for header parsing and compilation only. Linking
requires a Linux environment with D-Bus. Use `CPDB_NO_LINK=1` to skip
link directives:

```bash
CPDB_NO_LINK=1 cargo build --lib
```

## Testing

```bash
# Tests that do not need a live D-Bus
cargo test

# Integration tests — require a running session bus and cpdb backends
cargo test -- --ignored
```

## Troubleshooting

- **"cpdb-libs not found"** — Install `libcpdb-dev` / `cpdb-libs-devel`
  so pkg-config can locate `cpdb.pc`. Override the discovery path with
  `CPDB_LIBS_PATH=<prefix>` when working against an uninstalled checkout.
- **"D-Bus connection failed"** — Confirm a session bus is running and
  that print backends (CUPS, ...) are active.
- **"No printers found"** — Verify printers are configured and the
  relevant backend services are reachable over D-Bus.
- **Linker errors** — Make sure pkg-config can resolve `cpdb` and
  `cpdb-frontend`; on non-standard installs set
  `PKG_CONFIG_PATH=<prefix>/lib/pkgconfig`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

1. Fork and clone.
2. `cargo build` — verify the toolchain finds cpdb-libs.
3. Make changes, add tests.
4. Ensure `cargo test`, `cargo fmt --check`, and
   `cargo clippy --all-targets -- -D warnings` pass.
5. Open a pull request.

## License

MIT — see [LICENSE](LICENSE).

## Related projects

- [cpdb-libs](https://github.com/OpenPrinting/cpdb-libs) — the C library this crate binds to.
- [OpenPrinting](https://openprinting.org/)
- [CUPS](https://www.cups.org/)

## Support

- [Issues](https://github.com/OpenPrinting/cpdb-rs/issues)
- [Discussions](https://github.com/OpenPrinting/cpdb-rs/discussions)
- [API docs](https://docs.rs/cpdb-rs)

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
