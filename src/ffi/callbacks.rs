//! Closure-friendly wrappers over the two cpdb-libs C callback shapes.
//!
//! cpdb-libs has two callback types:
//!
//! 1. `cpdb_printer_callback` — fires when a printer is added, removed, or
//!    changes state. It carries no `user_data`, so a thin-pointer Box
//!    trampoline cannot work. We use a global registry keyed on the
//!    frontend pointer; the trampoline looks the closure up by that key.
//!
//! 2. `cpdb_async_callback` — completion for `cpdbAcquireDetails` and
//!    `cpdbAcquireTranslations`. It does carry `user_data`, so we use the
//!    standard `Box<Box<dyn FnOnce>>` thin-pointer trampoline.
//!
//! Both trampolines wrap the user closure in `catch_unwind` so a Rust
//! panic does not unwind across the FFI boundary (which is UB).

use crate::ffi;
use crate::ffi::printer::Printer;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, OnceLock};

// ─── PrinterUpdate enum ──────────────────────────────────────────────────────

/// The change reported by a printer callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterUpdate {
    /// A new printer was discovered.
    Added,
    /// A previously known printer was removed.
    Removed,
    /// An existing printer's state field changed.
    StateChanged,
}

impl PrinterUpdate {
    /// Maps the raw `cpdb_printer_update_t` value to the safe enum.
    pub(crate) fn from_raw(update: ffi::cpdb_printer_update_t) -> Option<Self> {
        match update as i64 {
            0 => Some(Self::Added),
            1 => Some(Self::Removed),
            2 => Some(Self::StateChanged),
            _ => None,
        }
    }
}

// ─── Printer observer (no user_data) ─────────────────────────────────────────

/// A boxed FnMut closure invoked for every printer-update event.
type PrinterObserver = dyn FnMut(&Printer<'_>, PrinterUpdate) + Send;

/// One slot in the registry — wrapped in an Arc so the trampoline can
/// release the global lock before invoking the user's closure.
type ObserverSlot = Arc<Mutex<Box<PrinterObserver>>>;

/// The frontend-pointer-keyed observer registry. Globals are unavoidable
/// here because `cpdb_printer_callback` carries no `user_data`.
fn registry() -> &'static Mutex<HashMap<usize, ObserverSlot>> {
    static R: OnceLock<Mutex<HashMap<usize, ObserverSlot>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_registry() -> std::sync::MutexGuard<'static, HashMap<usize, ObserverSlot>> {
    // Recover from poisoning — a panicking trampoline must not permanently
    // disable registration for the rest of the process.
    registry().lock().unwrap_or_else(|p| p.into_inner())
}

/// Inserts `observer` into the registry keyed by `frontend`.
///
/// Replaces any previously registered observer for that frontend.
pub(crate) fn register_printer_observer(
    frontend: *mut ffi::cpdb_frontend_obj_t,
    observer: Box<PrinterObserver>,
) {
    lock_registry().insert(frontend as usize, Arc::new(Mutex::new(observer)));
}

/// Removes the observer for `frontend`, if any. Idempotent.
pub(crate) fn unregister_printer_observer(frontend: *mut ffi::cpdb_frontend_obj_t) {
    lock_registry().remove(&(frontend as usize));
}

/// `extern "C"` trampoline plugged into `cpdbGetNewFrontendObj`.
///
/// # Safety
/// cpdb-libs guarantees `frontend` and `printer` are valid pointers for the
/// duration of this call. The trampoline must not unwind across the FFI
/// boundary — panics are absorbed by `catch_unwind`.
pub(crate) unsafe extern "C" fn printer_trampoline(
    frontend: *mut ffi::cpdb_frontend_obj_t,
    printer: *mut ffi::cpdb_printer_obj_t,
    update: ffi::cpdb_printer_update_t,
) {
    let slot = {
        let map = lock_registry();
        match map.get(&(frontend as usize)) {
            Some(slot) => Arc::clone(slot),
            None => return,
        }
    };
    let Some(update) = PrinterUpdate::from_raw(update) else {
        return;
    };
    let Ok(printer) = Printer::from_raw_borrowed(printer) else {
        return;
    };

    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        // Poisoning is benign here — the closure will get a fresh frame on
        // the next event; previous panic state cannot affect this call.
        let mut closure = slot.lock().unwrap_or_else(|p| p.into_inner());
        closure(&printer, update);
    }));
}

// ─── Async completion (with user_data) ───────────────────────────────────────

/// A boxed FnOnce closure invoked exactly once when an acquire completes.
pub(crate) type AcquireCompletion = dyn FnOnce(&Printer<'_>, bool) + Send;

/// Converts a boxed completion into a raw `user_data` pointer suitable for
/// `cpdbAcquireDetails` / `cpdbAcquireTranslations`.
///
/// The thin-pointer Box ensures we can round-trip through `*mut c_void`.
pub(crate) fn into_completion_user_data(completion: Box<AcquireCompletion>) -> *mut libc::c_void {
    Box::into_raw(Box::new(completion)) as *mut libc::c_void
}

/// `extern "C"` trampoline for `cpdb_async_callback`.
///
/// # Safety
/// `user_data` must be a pointer returned by [`into_completion_user_data`]
/// for this exact callback firing. cpdb-libs guarantees the firing happens
/// at most once.
pub(crate) unsafe extern "C" fn acquire_trampoline(
    printer: *mut ffi::cpdb_printer_obj_t,
    status: libc::c_int,
    user_data: *mut libc::c_void,
) {
    if user_data.is_null() {
        return;
    }
    // SAFETY: caller contract — `user_data` was created by
    // `into_completion_user_data` and is owned by us now.
    let outer: Box<Box<AcquireCompletion>> =
        unsafe { Box::from_raw(user_data as *mut Box<AcquireCompletion>) };

    let Ok(printer) = Printer::from_raw_borrowed(printer) else {
        return;
    };
    let closure: Box<AcquireCompletion> = *outer;

    let _ = std::panic::catch_unwind(AssertUnwindSafe(move || {
        closure(&printer, status != 0);
    }));
}
