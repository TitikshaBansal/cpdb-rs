# cpdb-rs

[![Crates.io](https://img.shields.io/crates/v/cpdb-rs.svg)](https://crates.io/crates/cpdb-rs)
[![Documentation](https://docs.rs/cpdb-rs/badge.svg)](https://docs.rs/cpdb-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Safe, native Rust async bindings for the Common Print Dialog Backends ([CPDB](https://github.com/OpenPrinting/cpdb-libs)) D-Bus interface.

## Overview

cpdb-rs lets Rust applications communicate with CPDB print backends (like `cpdb-backend-cups`) directly over D-Bus without requiring any C dependencies.

The library uses [`zbus`](https://crates.io/crates/zbus) and [`tokio`](https://crates.io/crates/tokio) to provide a fully asynchronous, memory-safe, and pure-Rust implementation of the CPDB client protocol.

## Features

- **Pure Rust D-Bus Client:** No `libcpdb-dev` or C compiler needed! Everything runs natively over D-Bus using `zbus`.
- **Async First:** All methods are `async` and powered by Tokio.
- **Live Discovery:** Subscribe to a native Rust `Stream` for real-time printer additions, removals, and state changes.
- **Robust Auto-Retry:** Handles D-Bus activation race conditions automatically.
- **Keep-Alive Management:** Automatically pings backends to keep them active in the background.
- *(Optional)* **Legacy C-FFI bindings** available via the `ffi` feature flag.

## Prerequisites

Because `cpdb-rs` communicates directly over D-Bus, you do not need to install the `cpdb-libs` C development headers to build this project.

However, your system must have CPDB backends installed to actually discover any printers:

```bash
# Debian / Ubuntu
sudo apt-get install cpdb-backend-cups

# Fedora / RHEL / CentOS
sudo dnf install cpdb-backend-cups
```

## Installation

```toml
[dependencies]
cpdb-rs = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Quick start

```rust
use cpdb_rs::CpdbClient;

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    // Connect to D-Bus and auto-activate available CPDB backends
    let client = CpdbClient::new().await?;
    println!("Connected to {} backend(s).\n", client.backend_count());

    // Retrieve all active printers
    let printers = client.get_all_printers().await?;
    for p in &printers {
        println!("Printer: {} (ID: {})", p.name, p.id);
        println!("  Make & Model: {}", p.make_model);
        println!("  State: {}", p.state);
        println!("  Accepts Jobs: {}", p.accepting_jobs);
    }

    Ok(())
}
```

## Examples

### Fetching Printer Options and Media Sizes

```rust
use cpdb_rs::CpdbClient;

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let client = CpdbClient::new().await?;
    let printers = client.get_all_printers().await?;

    if let Some(p) = printers.first() {
        // Fetch specific details using the printer's ID and backend name
        let (options, media) = client.get_printer_details(&p.id, &p.backend).await?;

        println!("Options for {}:", p.name);
        for opt in options {
            println!("  {}: default='{}', choices=[{}]", 
                opt.name, opt.default_value, opt.supported_values.join(", "));
        }
    }
    Ok(())
}
```

### Live Discovery Event Stream

Watch for new printers appearing and disappearing in real-time.

```rust
use cpdb_rs::{CpdbClient, DiscoveryEvent};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let client = CpdbClient::new().await?;
    
    // Spawn a background task to keep the backends from automatically
    // timing out and exiting after 30 seconds of inactivity.
    let keep_alive_client = client.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            keep_alive_client.keep_alive_all().await;
        }
    });

    let mut stream = client.discovery_stream().await?;
    println!("Listening for printer changes...");

    while let Some(event) = stream.next().await {
        match event {
            DiscoveryEvent::PrinterAdded(snap) => {
                println!("+ Added: {} ({})", snap.name, snap.backend);
            }
            DiscoveryEvent::PrinterRemoved { id, backend } => {
                println!("- Removed: {} ({})", id, backend);
            }
            DiscoveryEvent::PrinterStateChanged { id, state, accepting_jobs, .. } => {
                println!("~ State Changed: {} is now {} (accepting={})", id, state, accepting_jobs);
            }
        }
    }

    Ok(())
}
```

You can run the full interactive test example using:

```bash
cargo run --example zbus_test
```

## Architecture (zbus Backend)

Instead of linking against `libcpdb.so` and using `bindgen` (which required unsafe C memory management, callbacks, and manual lifetime tracking), `cpdb-rs` now uses `zbus` to speak the D-Bus protocol directly to the print backends. This provides massive benefits:

- **100% Safe Rust:** No raw pointers, no manual memory management, no undefined behavior.
- **Zero C Dependencies:** You don't need `libcpdb-dev` to compile.
- **Async Tokio Integration:** `zbus` integrates perfectly with Tokio, allowing you to await D-Bus calls and use Rust `Stream`s for live discovery events.
- **Startup Stability:** Implements internal retry loops to gracefully handle `UnknownMethod` race conditions when systemd auto-activates D-Bus backends.

## Legacy Architecture (C-FFI)

If you have legacy code that still requires the synchronous C-FFI wrappers around `cpdb-libs`, they are still available by enabling the `ffi` feature flag in your `Cargo.toml`. See the `ffi` module documentation for details.

```
            ┌───────────────────────────────────────────┐
            │            cpdb_rs::Frontend              │
            │  (D-Bus connection, backend list, hash    │
            │   table of discovered printers)           │
            └─────┬─────────────────────────────────┬───┘
                  │ borrowed                        │ owned
                  │ (lifetime tied to &Frontend)    │ (Drop frees)
                  ▼                                 ▼
       ┌────────────────────┐            ┌──────────────────────┐
       │ Printer<'frontend> │            │ Printer<'static>     │
       │ from get_printer / │            │ from load_from_file  │
       │ find_printer / ... │            └──────────────────────┘
       └────────┬───────────┘
                │
                ├── per-printer settings  (add_setting / clear_setting)
                ├── option lookups        (get_default / get_current / get_option)
                ├── translations         ─ TranslationMap (owned snapshot)
                ├── media                ─ MediaSize, Margins
                └── job submission       (print_file / submit_job / print_fd / print_socket)


                       ┌──────────────────────────┐
                       │     cpdb_rs::Settings     │
                       │  (free-standing serial-   │
                       │   isable settings object) │
                       └──────────────────────────┘
                            ▲
                            │ persisted via save_to_disk / read_from_disk
                            ▼
                   ~/.config/cpdb/ (cpdb-libs-managed location)
```

### Two `add_setting` methods, two scopes (C-FFI)

| Method                    | Scope                                              | Persists across runs?               |
|---------------------------|----------------------------------------------------|-------------------------------------|
| `Printer::add_setting`    | This printer only, in-memory on the printer object | Only if you re-add on each run      |
| `Settings::add_setting`   | Free-standing settings collection                  | Yes, via `Settings::save_to_disk()` |

`Printer::add_setting` is the per-job knob: tweak `copies`, `sides`, etc.
before calling `print_file` / `submit_job`. `Settings` is the global,
serialisable view that cpdb-libs reads back from disk on startup.

### Module map

| Module                | What lives here                                                     |
|-----------------------|----------------------------------------------------------------------|
| `cpdb_rs::client`     | **(zbus)** `CpdbClient` — Main async D-Bus client & discovery logic |
| `cpdb_rs::events`     | **(zbus)** `DiscoveryEvent`, `PrinterSnapshot` for async streams    |
| `cpdb_rs::media`      | **(zbus)** `MediaCollection`, `MediaInfo`, `MarginInfo`             |
| `cpdb_rs::config`     | **(zbus)** `PrinterConfig` for job submission configuration         |
| `cpdb_rs::options`    | `OptionInfo`, `OptionsCollection` (shared across both implementations)|
| `cpdb_rs::error`      | `CpdbError` and the crate-wide `Result` alias                       |
| `cpdb_rs::proxy`      | **(zbus)** Auto-generated zbus proxy trait `PrintBackend`           |
| `cpdb_rs::frontend`   | *(ffi)* `Frontend` — D-Bus lifecycle, discovery, default printer    |
| `cpdb_rs::printer`    | *(ffi)* `Printer`, `Margin/Margins`, `MediaSize`, `TranslationMap`  |
| `cpdb_rs::settings`   | *(ffi)* `Settings`, `Options`, `Media`                              |
| `cpdb_rs::callbacks`  | *(ffi)* Closure trampolines + `PrinterUpdate` enum                  |
| `cpdb_rs::common`     | *(ffi)* `init`, `version`, path/config helpers                      |
| `cpdb_rs::util`       | *(ffi)* Internal `CStr` helpers + the `COptions` C-array builder    |
| `cpdb_rs::ffi`        | *(ffi)* Raw bindgen output; everything `unsafe`                     |

## Ownership model (C-FFI)

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
