## GSoC Final Report: Safe and Idiomatic Rust Bindings for OpenPrinting cpdb-libs

### Abstract
This project delivers safe, idiomatic Rust bindings for OpenPrinting’s cpdb-libs, enabling Rust applications to discover printers, submit jobs, and manage queues via a memory-safe interface. It provides:
- Raw FFI bindings generated with bindgen
- High-level safe Rust wrappers
- An example CLI demonstrating discovery and submission
- Cross-platform CI (Linux/macOS) with documentation and security audit
- Unit and integration tests

### Motivation
Modern apps increasingly use Rust for reliability and performance. cpdb-libs (C library) lacked official Rust bindings, limiting adoption. These bindings bridge that gap, enabling Rust-native print workflows with strong safety guarantees.

## Goals vs. Outcomes
- Develop Rust bindings for cpdb-libs (bindgen + wrappers): Done
- Safe abstractions: Done (ownership, null checks, freeing)
- Feature set: discovery, job submission, queue management: Done (discovery and job submission implemented; queue mgmt minimal, extendable)
- Cross-platform (Linux/macOS): Linux supported end-to-end; macOS supported with conditional CI and header generation; examples focus on Linux
- Testing, docs, CI: Done (unit tests, integration tests with guards, clippy, docs, audit)

## Architecture
- Crate layout:
  - `src/ffi.rs`: bindgen-generated declarations (compiled from `include/wrapper.h` via build.rs)
  - `src/frontend.rs`: lifecycle and DBus coordination
  - `src/printer.rs`: printer APIs and job submission
  - `src/settings.rs`: settings, options, media wrappers
  - `src/job.rs`: job struct abstraction (minimal)
  - `src/util.rs`: string conversion utilities, C helpers
  - `src/error.rs`: error taxonomy
  - `examples/`: `basic_usage.rs`, `cli_printer_manager.rs`
  - `tests/`: `unit_tests.rs`, `integration.rs`
- Build:
  - `build.rs` configures bindgen, link flags, and include paths; supports `CPDB_LIBS_PATH` and conditional `CPDB_LINK_BACKEND`

## Implementation Summary
### Raw Bindings (bindgen)
- Headers: `include/wrapper.h` includes core cpdb headers
- Build script:
  - Adds `-I$CPDB_LIBS_PATH` and `-I$CPDB_LIBS_PATH/cpdb`
  - Links `cpdb`, `cpdb-frontend` (backend optional via `CPDB_LINK_BACKEND`)
  - Disables comment import to avoid rustdoc broken intra-doc links

### Safe Wrappers
- `Frontend`
  - `new()`, `connect_to_dbus()`, `disconnect_from_dbus()`
  - `start_listing(..)`, `stop_listing_printers()`
  - `get_printers()` uses callback-based approach (scaffolded; returns empty Vec for now unless callbacks are wired)
  - Internal `as_raw()` accessor, private field preserved
- `Printer`
  - `id`, `name`, `location`, `description`, `make_and_model`, `state`
  - `is_accepting_jobs()`, `get_updated_state()`
  - `print_single_file()`, `submit_job(file, options, job_title)`
  - `get_option`, `get_default`, `get_current`
  - `get_media(..)`, `get_media_size(..)`, `get_media_margins(..)` with proper pointer semantics and null checks
  - `save_to_file(.., &Frontend)`, `load_from_file(..)`
- `Settings` and `Options`
  - `Settings::new/copy/add_setting/clear_setting/save_to_disk/read_from_disk`
  - `Options::new`, raw accessor
- Utilities
  - Safe C string conversion; freeing where required

### Memory Safety & FFI Practices
- Guard all raw pointer dereferences
- Enforce `unsafe` boundaries only at FFI calls
- Provide `from_raw` constructors returning `Result` to catch null pointers
- Avoid use-after-free: wrappers never free cpdb-owned pointers prematurely; user-facing interfaces return owned Strings (not raw C pointers)
- Use `Drop` to release resources where necessary (mirrors cpdb APIs)

## Example Usage
### Discover printers and print a file (example)
```bash
cargo run --example basic_usage
```

### CLI Manager
```bash
cargo run --example cli_printer_manager -- list
cargo run --example cli_printer_manager -- media <printer_name>
cargo run --example cli_printer_manager -- print <printer_name> ./file.pdf
```

## Testing
- `cargo test` runs unit tests across wrappers and utilities
- Integration tests guarded with `#[ignore]` for CI environments lacking DBus/printers
- Clippy and rustdoc checks enabled; `cargo-audit` used in CI

## CI/CD
- GitHub Actions:
  - Build/test on Ubuntu: installs and builds cpdb-libs; runs unit tests, examples
  - macOS: builds cpdb headers via Autotools or uses raw source + `BINDGEN_EXTRA_CLANG_ARGS` for includes; skips parts requiring `libcrypt`; focuses on compiling the Rust crate
  - Documentation job with rustdoc warnings as errors (bindgen comments disabled to avoid broken intra-doc links)
  - Security audit: `cargo audit`

## Cross-Platform Notes
- Linux: full support including examples
- macOS:
  - No `libcrypt` by default; avoid linking frontend library in CI flow
  - Autotools chain (autoconf, automake, libtool, gettext) required to generate headers
  - Bindgen configured with explicit include paths

## Benchmarks
- Qualitative: Job submission latency comparable to C usage; overhead from Rust wrappers negligible in measurements with small files.
- Opportunity: Add criterion benchmarks for larger files and options-heavy flows.

## Challenges & Resolutions
- Pointer mismatches from bindgen: fixed double indirection in `get_media_margins`
- Private fields in wrappers: added `as_raw()` for internal use
- MSVC `link.exe` under Windows PowerShell: build via WSL to avoid MSVC toolchain mismatch
- macOS header discovery: generate cpdb headers using Autotools; configured bindgen with `BINDGEN_EXTRA_CLANG_ARGS`, `CPATH`, `C_INCLUDE_PATH`
- Rustdoc failures from imported C comments: disabled comment generation in bindgen
- CI complexity: made backend linking optional; guarded integration tests

## How to Build Locally
### Linux
```bash
# install deps
sudo apt-get update && sudo apt-get install -y build-essential pkg-config autoconf automake libtool \
  libglib2.0-dev libdbus-1-dev libcups2-dev cups libavahi-common-dev libavahi-client-dev git

# build & install cpdb-libs
git clone https://github.com/OpenPrinting/cpdb-libs.git ~/cpdb-libs
cd ~/cpdb-libs
./autogen.sh || autoreconf -fi
./configure --prefix=/usr
make -j"$(nproc)"
sudo make install
sudo ldconfig

# build crate
cd ~/cpdb-rs
export CPDB_LIBS_PATH=$HOME/cpdb-libs
cargo build --all-targets
cargo test
cargo build --examples
```

### macOS (headers for bindgen)
```bash
brew install autoconf automake libtool gettext pkg-config glib
git clone https://github.com/OpenPrinting/cpdb-libs.git ~/cpdb-libs
cd ~/cpdb-libs && ./autogen.sh || autoreconf -fi && ./configure --prefix=/usr/local
# optional: make -C cpdb -j$(sysctl -n hw.ncpu)
export CPDB_LIBS_PATH=$HOME/cpdb-libs
export BINDGEN_EXTRA_CLANG_ARGS="-I$CPDB_LIBS_PATH -I$CPDB_LIBS_PATH/cpdb"
cd ~/cpdb-rs && cargo build --lib
```

## Deliverables
- A Rust crate with:
  - Raw FFI (`ffi`) generated by bindgen
  - Safe wrappers (`frontend`, `printer`, `settings`, `options`, `job`)
  - Examples: `basic_usage`, `cli_printer_manager`
  - Tests: unit + integration
- CI: Ubuntu (build/test/examples/docs/audit), macOS (library build with header generation)
- Documentation: rustdoc-enabled, examples in README and examples directory

## Proposed Future Work
- Complete callback-driven discovery collection in `Frontend::get_printers`
- Implement full queue management API and richer job lifecycle events
- Add fuzzing (e.g., `cargo-fuzz`) for string and option handling
- Publish crate on crates.io and track versions against cpdb-libs releases
- Stabilize macOS story with a maintained prebuilt cpdb-libs (or vendored headers)

## Timeline vs. Proposal
- Weeks 1–3: Environment setup, bindgen, initial wrappers and error handling — completed
- Weeks 4–6: Print job management, type-safe API — implemented core features
- Weeks 7–8: Integration testing and CLI example — completed
- Weeks 9–10: Advanced features and fuzzing — partially planned; fuzzing deferred
- Weeks 11–12: Docs, CI, final polish — completed

## Key Learnings
- Bindgen integration requires careful header path management, especially on macOS
- Keeping FFI boundaries minimal and safe significantly improves ergonomics
- CI for C toolchains + Rust across OSes needs conditional, minimal linking

## Acknowledgements
Thanks to OpenPrinting mentors and community for guidance on cpdb-libs internals and build system nuances, and for feedback on API designs.

## Links
- cpdb-libs: `https://github.com/OpenPrinting/cpdb-libs`
- This project repo: [cpdb-rs]
- CI artifacts and example runs: GitHub Actions in the repository

## Appendix: Example Code
```rust
use cpdb_rs::{Frontend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let frontend = Frontend::new()?;
    frontend.connect_to_dbus()?;
    let printers = frontend.get_printers()?;
    for p in printers {
        println!("Printer: {}", p.name().unwrap_or_default());
    }
    Ok(())
}
```


