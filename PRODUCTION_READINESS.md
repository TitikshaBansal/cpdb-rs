# cpdb-rs Production-Readiness Checklist

A living tracking document for the work between today and a publishable 0.1.0
release on crates.io. Every item below has a stable ID (e.g. `B-1.1`) so it
can be referenced in commits, PRs, and issue trackers.

Audited against upstream `OpenPrinting/cpdb-libs` (master branch).

## Legend

| Marker | Meaning |
|---|---|
| `[BLOCKER]` | Must be fixed before any release. Soundness, UB, or "the documented API doesn't work" bugs. |
| `[HIGH]`    | Must land before 1.0. Production users will hit these. |
| `[MED]`     | Should land in 0.1.x. Quality / completeness. |
| `[LOW]`     | Polish. Track but not blocking. |

## Status legend

- `[ ]` Not started
- `[~]` In progress
- `[x]` Done
- `[-]` Decided not to do (record why in the notes column)

---

## 1. Correctness bugs

| ID | Sev | Status | Item | Location | Notes |
|---|---|---|---|---|---|
| B-1.1 | BLOCKER | [x] | `Printer::submit_job` silently discards the options array. **FIXED:** now applies each `(name, value)` pair via `cpdbAddSettingToPrinter` before `cpdbPrintFileWithJobTitle` and returns the job ID. | `src/printer.rs` | |
| B-1.2 | BLOCKER | [x] | Wrong allocator: `libc::free` on a `g_strdup`'d pointer is UB. **FIXED:** all GLib-allocated strings now go through `glib_sys::g_free` via `util::cstr_to_string_and_g_free`. | `src/printer.rs`, `src/util.rs` | |
| B-1.3 | BLOCKER | [x] | Method renamed to `Printer::print_file` and every doc/example/test updated. | `src/printer.rs`, `README.md`, examples, tests | |
| B-1.4 | BLOCKER | [x] | `CpdbError::NotFound` and `CpdbError::PrinterError` added; documented variants now match the implementation. | `src/error.rs` | |
| B-1.5 | BLOCKER | [x] | `Printer` no longer implements `Send`/`Sync`. Borrowed printers carry a lifetime `'frontend`. | `src/printer.rs` | |
| B-1.6 | BLOCKER | [x] | `Frontend` no longer implements `Sync` (kept `Send`). Concurrent callers must wrap in a `Mutex`. | `src/frontend.rs` | |
| B-1.7 | BLOCKER | [x] | All `Frontend::*` printer-returning methods now return `Printer<'_>` tied to `&self`; the borrow checker prevents the printer from outliving the frontend. | `src/frontend.rs`, `src/printer.rs` | |
| B-1.8 | HIGH | [x] | `Frontend::from_raw` is now `unsafe fn`. | `src/frontend.rs` | |
| B-1.9 | HIGH | [x] | `Settings::serialize_to_gvariant` removed from the public API. | `src/settings.rs` | |
| B-1.10 | HIGH | [x] | `Settings::clear_setting` and `Printer::clear_setting` return `Result<bool>` indicating whether the key existed. | `src/settings.rs`, `src/printer.rs` | |
| B-1.11 | HIGH | [x] | `Printer::get_option` and `Printer::get_setting` now return `Result<Option<String>>`. | `src/printer.rs` | |
| B-1.12 | HIGH | [x] | `CpdbError::from_status` removed. Variants are constructed explicitly at error sites. | `src/error.rs` | |

---

## 2. Upstream API coverage gaps

These exist in `cpdb/cpdb.h` / `cpdb/cpdb-frontend.h` on master but have no
safe Rust wrapper yet.

| ID | Sev | Status | Missing surface | Notes |
|---|---|---|---|---|
| C-2.1 | HIGH | [x] | Phantom symbols removed from `build.rs` allowlist. | `build.rs` |
| C-2.2 | HIGH | [x] | `src/job.rs` and `src/backend.rs` deleted; re-exports removed from `src/lib.rs`. | |
| C-2.3 | HIGH | [ ] | Closure-friendly wrapper for `cpdb_printer_callback`. Current `Frontend::new_with_callback` still takes a raw `unsafe extern "C" fn`. | |
| C-2.4 | HIGH | [ ] | Closure-friendly wrapper for `cpdb_async_callback` used by `cpdbAcquireDetails` / `cpdbAcquireTranslations`. | |
| C-2.5 | MED | [x] | Wrappers added: `Frontend::add_printer`, `Frontend::remove_printer`, `Frontend::refresh_printer_list`. | `src/frontend.rs` |
| C-2.6 | MED | [ ] | Wrap `cpdbPrintFD` and `cpdbPrintSocket` for the FD/socket print paths. | |
| C-2.7 | MED | [ ] | Wrap translation `*FromTable` variants. | |
| C-2.8 | MED | [x] | `Margin` / `Margins` / `MediaSize` structs added; `get_media_margins` returns the full array, not just `[0]`. | `src/printer.rs` |
| C-2.9 | MED | [ ] | Debug bridges: `cpdbFDebugPrintf`, `cpdbBDebugPrintf`, `CpdbDebugLevel`, `cpdbDebugPrinter`, `cpdbPrintBasicOptions`, `cpdbFillBasicOptions`. | |
| C-2.10 | MED | [ ] | GVariant helpers: `cpdbPackStringArray`, `cpdbUnpackStringArray`, `cpdbPackMediaArray`. Allowlisted in bindgen, no Rust wrappers yet. | |
| C-2.11 | MED | [ ] | Path/config helpers: `cpdbGetUserConfDir`, `cpdbGetSysConfDir`, `cpdbGetAbsolutePath`, `cpdbConcatSep`, `cpdbConcatPath`, `cpdbGetGroup`. | |
| C-2.12 | MED | [ ] | Backend creation: `cpdbCreateBackend`, `cpdbGetDbusConnection`. | |

Reference (do NOT try to wrap — upstream-absent on master): `cpdb_job_t`,
`cpdb_print_job_t`, `cpdb_async_obj_t`, `cpdbGetActiveJobsCount`,
`cpdbGetAllJobs`, `cpdbCancelJob`, `cpdbNewPrintJob`, `cpdbDeletePrintJob`,
`cpdb_backend_obj_t` (as a public type). The headers `backend.h` and
`frontend.h` are just `#include` shims for `cpdb.h` / `cpdb-frontend.h`.

---

## 3. Build / FFI / packaging

| ID | Sev | Status | Item | Location | Notes |
|---|---|---|---|---|---|
| D-3.1 | HIGH | [x] | `build.rs` rewritten to use `pkg-config --variable=libdir glib-2.0` and `pkg-config cpdb`. Architecture-specific path guesses removed. | `build.rs` |
| D-3.2 | HIGH | [x] | pkg-config is now the primary discovery path; `CPDB_LIBS_PATH` is an explicit escape hatch. A warning is emitted when neither hits. | `build.rs` |
| D-3.3 | HIGH | [x] | Unused `frontend`/`backend` features removed from `Cargo.toml`. | `Cargo.toml` |
| D-3.4 | HIGH | [x] | Global `dead_code` / `non_snake_case` allows scoped to `src/ffi.rs` only. `src/lib.rs` now uses `#![warn(missing_docs)]`. | `src/lib.rs`, `src/ffi.rs` |
| D-3.5 | MED | [x] | Added `links = "cpdb"` to `[package]`. | `Cargo.toml` |
| D-3.6 | MED | [x] | `CONTRIBUTING.md` updated to Rust 1.85 / 2024 edition to match `Cargo.toml`. | `CONTRIBUTING.md` |
| D-3.7 | MED | [ ] | Consider splitting bindgen output into a `cpdb-sys` crate. Cleaner layering; defer to 0.2. | |

---

## 4. CI / release workflow

| ID | Sev | Status | Item | Location |
|---|---|---|---|---|
| E-4.1 | HIGH | [x] | `cargo clippy --all-targets -- -D warnings` job added. | `.github/workflows/ci.yml` |
| E-4.2 | HIGH | [x] | `cargo fmt --all -- --check` job added. | `.github/workflows/ci.yml` |
| E-4.3 | HIGH | [ ] | Add `release` workflow (tag-triggered): `cargo publish --dry-run`, build docs, attach release artefacts. | |
| E-4.4 | HIGH | [~] | `[package.metadata.docs.rs]` added pinning the build to `x86_64-unknown-linux-gnu` and forwarding a `docsrs` cfg. A vendored-header escape for the actual docs.rs builder is still TODO. | `Cargo.toml`, `build.rs` |
| E-4.5 | MED | [ ] | Add `dependabot.yml` and `CODEOWNERS`. | |
| E-4.6 | MED | [ ] | Add `cargo deny check` (licenses, advisories, bans). | |
| E-4.7 | LOW | [ ] | Minimal D-Bus + dummy-CUPS integration smoke test that runs at least one `#[ignore]`-flagged test in CI. | |

---

## 5. API ergonomics / code quality

| ID | Sev | Status | Item | Location |
|---|---|---|---|---|
| F-5.1 | HIGH | [x] | `Settings`, `Options`, `Media`, `Frontend`, `Printer` all hold `NonNull<T>`; per-method `is_null()` guards removed. | every module |
| F-5.2 | HIGH | [x] | `Frontend::get_printer` now compares raw bytes via `CStr::from_ptr(...).to_bytes()`. | `src/frontend.rs` |
| F-5.3 | HIGH | [x] | `impl Clone for Settings` removed; replaced with explicit `Settings::try_clone() -> Result<Self>`. | `src/settings.rs` |
| F-5.4 | HIGH | [x] | `Printer::accepts_pdf` removed. Callers should inspect the `document-format` option instead. | `src/printer.rs` |
| F-5.5 | MED | [x] | `OptionsCollection::from_raw` now takes `NonNull<...>` and returns `Self`. | `src/options.rs` |
| F-5.6 | MED | [x] | `CpdbError` trimmed; `NotFound` and `PrinterError` added; unused variants removed. | `src/error.rs` |
| F-5.7 | LOW | [x] | `set_user_default` and `set_system_default` now return `Result<bool>`. | `src/printer.rs` |
| F-5.8 | LOW | [x] | `util::to_c_options` returns a `COptions` whose storage is `Box<[CString]>` — invariant enforced statically. | `src/util.rs` |

---

## 6. Tests

| ID | Sev | Status | Item | Location |
|---|---|---|---|---|
| G-6.1 | HIGH | [x] | `tests/unit_tests.rs` rewritten: real assertions on Settings lifecycle, `try_clone` independence, `clear_setting` return value, util string helpers, error formatting. No more dishonest `println!("expected in test environment")` arms. | `tests/unit_tests.rs` |
| G-6.2 | HIGH | [x] | `tests/integration.rs` rewritten: removed dead `PrintJob` references; new `submit_job` test verifies options application and asserts a non-empty job id. | `tests/integration.rs` |
| G-6.3 | MED | [ ] | Add a `cargo miri test` job covering the safe portions. | |
| G-6.4 | MED | [ ] | Add a property/round-trip test for `util::to_c_options`. | |

---

## 7. Documentation

| ID | Sev | Status | Item | Location |
|---|---|---|---|---|
| H-7.1 | HIGH | [x] | README rewritten end-to-end against the shipping API. | `README.md` |
| H-7.2 | HIGH | [~] | `#![warn(missing_docs)]` added to `src/lib.rs`. Every `pub` item now has at least a one-line doc comment. Consider escalating to `deny` once the upstream callback wrappers (C-2.3 / C-2.4) land. | `src/lib.rs` |
| H-7.3 | HIGH | [~] | The cpdb-text-frontend example now uses safe `Printer::get_media_margins` / `MediaSize`. A `print_translations` helper still iterates the raw `translations` GHashTable; promote that to a safe `TranslationMap` in 0.1.x. | `examples/cpdb-text-frontend.rs` |
| H-7.4 | MED | [x] | Every `unsafe impl Send/Sync` now carries a `SAFETY:` comment justifying it. | `src/settings.rs`, `src/frontend.rs` |
| H-7.5 | MED | [x] | CONTRIBUTING.md updated to "Rust 1.85+ (2024 edition)" and the placeholder username fixed. | `CONTRIBUTING.md` |
| H-7.6 | LOW | [x] | README now includes a dedicated `find_printer(id, backend)` example. | `README.md` |
| H-7.7 | LOW | [ ] | Add a short architecture / module map to the README. | `README.md` |

---

## 8. Release plan

### Pre-0.1.0 (must land)

All `BLOCKER` items, plus the user-facing `HIGH` items that touch the public
API surface or docs: B-1.1 .. B-1.12, C-2.1, C-2.2, D-3.1, D-3.3, D-3.4,
G-6.1, G-6.2, H-7.1.

### 0.1.x (should land after first publish)

C-2.3, C-2.4, D-3.2, D-3.5, D-3.6, E-4.1, E-4.2, E-4.3, E-4.4, F-5.1, F-5.2,
F-5.4, F-5.6, H-7.2, H-7.3.

### Track for 0.2

The remaining items in section 2 (broader API surface), an async-friendly
acquire-details API, the FD/socket print paths, the GVariant helpers, and a
potential `cpdb-sys` split (D-3.7).

### The two single most damaging items to fix first

- **B-1.1** — `submit_job` silently drops the options array. Users following
  the README example will silently get jobs with wrong settings.
- **B-1.7** — borrowed-printer lifetimes are not enforced by the type
  system. The first user who keeps a `Printer` past its `Frontend` gets
  use-after-free.

Fix those two plus the README/method-name mismatch in **B-1.3** and the
crate is in shouting distance of an honest preview release.

---

## Maintenance notes

- When you complete an item, change `[ ]` to `[x]`.
- When the scope shifts, add a row rather than editing IDs — outside links
  reference them.
- Severity is a hint, not a contract: feel free to bump items up or down as
  you learn more, but record the reason in the notes column.
