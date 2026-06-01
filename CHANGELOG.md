# Changelog

All notable changes to cpdb-rs will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `Printer::submit_job` previously discarded its options array (the parameter was
  leading-underscored). Options are now applied via `cpdbAddSettingToPrinter`
  before submission, matching the documented behaviour.
- Replaced `libc::free` with `glib_sys::g_free` for cpdb-libs return values that
  are `g_strdup`'d (fixes undefined behaviour on platforms where
  `g_malloc != malloc`).
- `Printer::get_option` no longer returns the sentinel string `"NA"` —
  unset options now resolve to `Ok(None)`.
- README, CHANGELOG, and example code references to `printer.print_file(...)`,
  `CpdbError::NotFound`, and the option-translation signature now match the
  shipping API.

### Changed

- **BREAKING:** `Printer` now carries a lifetime parameter tied to its
  `Frontend`. Borrowed printers cannot outlive their frontend — the borrow
  checker enforces this. `Printer::load_from_file` returns a `Printer<'static>`.
- **BREAKING:** `Printer::print_single_file` was renamed to
  [`Printer::print_file`] to match `cpdbPrintFile`.
- **BREAKING:** `Printer::submit_job` now returns the job ID string
  (previously returned `()`).
- **BREAKING:** `Printer::get_option`, `Printer::get_media`, and
  `Printer::get_setting` return `Result<Option<String>>` instead of using
  ad-hoc sentinel strings.
- **BREAKING:** `Printer::get_media_size` returns a [`MediaSize`] struct;
  `Printer::get_media_margins` returns a [`Margins`] of [`Margin`]s rather
  than a formatted string. The new types expose every margin entry, not just
  the first one.
- **BREAKING:** `Settings::clear_setting` returns `Result<bool>` —
  `true` when the key existed before this call.
- **BREAKING:** `Settings::serialize_to_gvariant` removed from the public API
  (it leaked a raw `*mut GVariant`).
- **BREAKING:** `Printer::set_user_default` / `set_system_default` now return
  `Result<bool>`.
- **BREAKING:** `Frontend::from_raw` is now `unsafe fn`.
- **BREAKING:** `Frontend::Sync` removed. Methods take `&self` for ergonomics
  but mutate C state; concurrent access is unsound. `Frontend` is still `Send`.
- **BREAKING:** `Printer` no longer implements `Send`/`Sync`.
- **BREAKING:** `CpdbError` gained `NotFound` and `PrinterError` variants; the
  unused `CupsError`, `InvalidStatus`, `Unsupported` variants and the
  misleading `from_status` helper were removed.
- `Frontend::get_printer` now compares names as raw bytes (no
  `to_string_lossy` allocation per printer).

### Added

- `CpdbClient`: The new main entrypoint for the async native D-Bus client.
- `CpdbClient::get_all_printers`: Fetches a snapshot of all active printers across all discovered backends.
- `CpdbClient::get_printer_details`: Fetches options and media sizes for a specific printer.
- `CpdbClient::discovery_stream`: Exposes a native `Stream` of `DiscoveryEvent`s (`PrinterAdded`, `PrinterRemoved`, `PrinterStateChanged`) for live discovery.
- `CpdbClient::keep_alive_all`: Helper method to ping all connected backends and prevent them from auto-exiting due to inactivity timeouts.
- `Frontend::new_with_observer<F: FnMut(&Printer, PrinterUpdate) + Send + 'static>` —
  closure-based registration for the `cpdb_printer_callback`. Backed by a
  process-global pointer-keyed registry and unregistered automatically when
  the [`Frontend`] is dropped.
- `Printer::acquire_details_with` and `Printer::acquire_translations_with` —
  closure-based completion handlers for the two `cpdb_async_callback`-driven
  operations. Closure panics are absorbed by `catch_unwind`.
- `PrinterUpdate` enum (`Added` / `Removed` / `StateChanged`) re-exported from
  the crate root.
- `Printer::translations()` and the [`TranslationMap`] type — owned
  snapshot of a printer's translation hash table. No raw FFI required
  to enumerate translations from user code.
- `Printer::print_fd` and `Printer::print_socket` — safe wrappers around
  `cpdbPrintFD` / `cpdbPrintSocket`. Return `PrintFdHandle`
  (`OwnedFd` + `job_id` + optional `socket_path`) and `PrintSocketHandle`
  respectively.
- `Printer::get_option_translation_from_table` and
  `Printer::get_choice_translation_from_table` — synchronous local-table
  translation lookups (no D-Bus roundtrip).
- `Printer::debug_dump` and `Printer::dump_basic_options` — wrappers for
  `cpdbDebugPrinter` / `cpdbPrintBasicOptions`.
- `Frontend::dbus_connected()` — process-wide D-Bus availability probe.
- Free functions in `cpdb_rs::common` (also re-exported from the crate
  root): `user_config_dir`, `system_config_dir`, `absolute_path`,
  `concat_sep`, `concat_path`, `option_group`.
- `Frontend::add_printer`, `Frontend::remove_printer`,
  `Frontend::refresh_printer_list` — wrappers around the corresponding C
  functions.
- `Frontend::refresh_printers` — renamed wrapper around `cpdbGetAllPrinters`
  (was `get_all_printers`).
- `Margin`, `Margins`, `MediaSize` — structured replacements for the formatted
  strings previously returned by media accessors.

### Removed

- `cpdb_rs::PrintJob` and `cpdb_rs::Backend` stub types. The cpdb-libs C API
  does not expose a separate job or backend type on master; print job
  submission flows through [`Printer::print_file`] / [`Printer::submit_job`].
- Phantom symbols `cpdbNewPrintJob`, `cpdbSubmitPrintJobWithFile`,
  `cpdbCancelJobById`, `cpdbDeletePrintJob`, `cpdbGetNewBackendObj`,
  `cpdbSubmitJob`, `cpdbDeleteBackendObj` removed from the bindgen
  allowlist; they do not exist upstream and bindgen was silently dropping
  them.
- `crossbeam-channel` dependency (unused).

### Infrastructure

- Native Rust D-Bus Client: Completely rewrote the crate to use native Rust D-Bus bindings via `zbus`, dropping the dependency on the `cpdb-libs` C library for the primary API.
- Async API: All frontend APIs are now fully asynchronous and powered by `tokio`.
- Legacy FFI: The old C-FFI bindings are still available but have been moved behind the optional `ffi` feature flag.
- `build.rs` now prefers `pkg-config` over the hard-coded fallback path
  list, drops the architecture-specific `/usr/lib/x86_64-linux-gnu` guess,
  and emits a `cargo:warning` when neither pkg-config nor `CPDB_LIBS_PATH`
  produces a hit.
- `build.rs` detects `DOCS_RS=1` and emits a hand-rolled stub
  `cpdb_sys.rs` so docs.rs builds without cpdb-libs installed.
- `Cargo.toml` declares `links = "cpdb"`, removes the unused
  `frontend`/`backend` features, and adds `docs.rs` metadata.
- CI now runs `cargo fmt --check` and `cargo clippy -D warnings`.
- `release.yml` workflow added: tag-triggered (`v*.*.*`), runs full
  verification, asserts the tag matches `Cargo.toml`, performs
  `cargo publish --dry-run`, and creates a GitHub Release with notes
  auto-extracted from `CHANGELOG.md`.
- `src/lib.rs` upgraded `#![warn(missing_docs)]` to `#![deny(missing_docs)]`.
- `.github/dependabot.yml` (weekly cargo + github-actions updates) and
  `.github/CODEOWNERS` added.
- `deny.toml` + CI job running `cargo deny --all-features check`
  (license allowlist, advisory database, wildcard ban, registry pin).
- CI smoke test: `dbus-launch --exit-with-session cargo test --test
  integration -- --ignored` exercises init → connect → discover →
  teardown against a real session bus.
- Nightly miri job (non-blocking) exercises pure-Rust portions of the
  crate. FFI-touching tests marked `#[cfg_attr(miri, ignore)]`.
- Round-trip tests for `util::to_c_options` covering empty input, single
  and multi-pair builds, interior-NUL rejection on key/value,
  null-init of unused option fields, and pointer stability across move.
- README: Architecture section with ASCII diagram, per-module map,
  and a `Printer::add_setting` vs `Settings::add_setting` scope table.

## [0.1.0] - 2024-01-XX

Initial pre-release. See git history for details.
