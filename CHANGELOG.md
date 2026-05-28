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

- `build.rs` now prefers `pkg-config` over the hard-coded fallback path
  list, drops the architecture-specific `/usr/lib/x86_64-linux-gnu` guess,
  and emits a `cargo:warning` when neither pkg-config nor `CPDB_LIBS_PATH`
  produces a hit.
- `Cargo.toml` declares `links = "cpdb"`, removes the unused
  `frontend`/`backend` features, and adds `docs.rs` metadata.
- CI now runs `cargo fmt --check` and `cargo clippy -D warnings`.

## [0.1.0] - 2024-01-XX

Initial pre-release. See git history for details.
