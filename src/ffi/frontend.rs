//! Safe wrapper around `cpdb_frontend_obj_t`.
//!
//! A [`Frontend`] is the central object for printer discovery. It manages
//! a D-Bus connection, holds the backend list, and owns the hash table
//! of discovered printers.
//!
//! # Threading
//!
//! [`Frontend`] is [`Send`] but **not** [`Sync`]. Most methods take
//! `&self` for ergonomics but mutate the underlying C state, and
//! cpdb-libs does not lock internally. If you need concurrent access,
//! wrap the frontend in a [`std::sync::Mutex`].

use super::bindings as ffi;
use super::printer::Printer;
use super::callbacks::{self, PrinterUpdate};
use crate::error::{CpdbError, Result};
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::ptr::NonNull;

/// Safe wrapper around `cpdb_frontend_obj_t`.
pub struct Frontend {
    raw: NonNull<ffi::cpdb_frontend_obj_t>,
}

// SAFETY: `Frontend` owns its `cpdb_frontend_obj_t *`. Moving it across
// threads is fine; concurrent access from multiple threads is not, so we
// deliberately omit `Sync`.
unsafe impl Send for Frontend {}

impl Frontend {
    // ─── Construction ────────────────────────────────────────────────────────

    /// Creates a new frontend with the default printer-update callback.
    pub fn new() -> Result<Self> {
        Self::new_internal(None)
    }

    /// Creates a new frontend with a raw printer-update callback.
    ///
    /// Prefer [`Frontend::new_with_observer`] for closure-based callbacks.
    /// This entry point exists for callers that need to interoperate with a
    /// hand-written `extern "C"` callback.
    ///
    /// The callback is invoked from the cpdb-libs internal D-Bus listener
    /// thread when printers are added, removed, or change state. It must be
    /// thread-safe and must not call back into this `Frontend`.
    pub fn new_with_callback(cb: ffi::cpdb_printer_callback) -> Result<Self> {
        Self::new_internal(cb)
    }

    /// Creates a new frontend with a closure-based printer observer.
    ///
    /// The observer is invoked from cpdb-libs' internal D-Bus listener
    /// thread when printers are added, removed, or change state. Because
    /// `cpdb_printer_callback` carries no `user_data`, the closure is
    /// stored in a process-global registry keyed by the frontend pointer
    /// and removed automatically when the [`Frontend`] is dropped.
    ///
    /// The closure must be `Send + 'static`. Panics inside the closure are
    /// caught by `catch_unwind` and absorbed; do not rely on panic-based
    /// flow control inside callbacks.
    pub fn new_with_observer<F>(observer: F) -> Result<Self>
    where
        F: FnMut(&Printer<'_>, PrinterUpdate) + Send + 'static,
    {
        let cb: ffi::cpdb_printer_callback = Some(callbacks::printer_trampoline);
        let frontend = Self::new_internal(cb)?;
        callbacks::register_printer_observer(frontend.raw.as_ptr(), Box::new(observer));
        Ok(frontend)
    }

    fn new_internal(cb: ffi::cpdb_printer_callback) -> Result<Self> {
        // SAFETY: `cpdbGetNewFrontendObj` is a constructor; the callback may
        // be null.
        let raw = unsafe { ffi::cpdbGetNewFrontendObj(cb) };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(|| CpdbError::FrontendError("cpdbGetNewFrontendObj returned null".into()))
    }

    /// Wraps an already-allocated frontend object.
    ///
    /// # Safety
    /// `raw` must be a valid pointer to a `cpdb_frontend_obj_t` obtained from
    /// cpdb-libs, not aliased by any other Rust handle, and not yet freed.
    /// Ownership transfers to the returned `Frontend`.
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_frontend_obj_t) -> Result<Self> {
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or(CpdbError::NullPointer)
    }

    /// Returns the raw pointer for use within this crate.
    #[doc(hidden)]
    pub fn as_raw(&self) -> *mut ffi::cpdb_frontend_obj_t {
        self.raw.as_ptr()
    }

    // ─── Lifecycle ───────────────────────────────────────────────────────────

    /// Tells the frontend to ignore the previously saved settings file.
    ///
    /// Must be called before [`Frontend::connect_to_dbus`] to take effect.
    pub fn ignore_last_saved_settings(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbIgnoreLastSavedSettings(self.raw.as_ptr()) };
    }

    /// Connects to the session D-Bus and activates the print backends.
    pub fn connect_to_dbus(&self) -> Result<()> {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbConnectToDBus(self.raw.as_ptr()) };
        Ok(())
    }

    /// Returns `true` when cpdb-libs currently holds a live D-Bus connection.
    ///
    /// This consults `cpdbGetDbusConnection`, which is a process-global —
    /// not per-frontend — query, so the result reflects the overall
    /// cpdb-libs state rather than this specific [`Frontend`].
    pub fn dbus_connected() -> bool {
        // SAFETY: no arguments; returns either a live `GDBusConnection *`
        // or null.
        !unsafe { ffi::cpdbGetDbusConnection() }.is_null()
    }

    /// Disconnects from D-Bus.
    pub fn disconnect_from_dbus(&self) -> Result<()> {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbDisconnectFromDBus(self.raw.as_ptr()) };
        Ok(())
    }

    /// Re-activates the backends (rediscovers printers without disconnecting).
    pub fn activate_backends(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbActivateBackends(self.raw.as_ptr()) };
    }

    /// Starts the background thread that periodically refreshes the backend list.
    pub fn start_backend_list_refreshing(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbStartBackendListRefreshing(self.raw.as_ptr()) };
    }

    /// Stops the backend-list refreshing thread; blocks until it joins.
    pub fn stop_backend_list_refreshing(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbStopBackendListRefreshing(self.raw.as_ptr()) };
    }

    /// Starts the printer-listing flow (`cpdbStartListingPrinters`), creating
    /// a fresh frontend bound to the supplied callback.
    pub fn start_listing(cb: ffi::cpdb_printer_callback) -> Result<Self> {
        // SAFETY: callback may be null per upstream docs.
        let raw = unsafe { ffi::cpdbStartListingPrinters(cb) };
        NonNull::new(raw).map(|raw| Self { raw }).ok_or_else(|| {
            CpdbError::FrontendError("cpdbStartListingPrinters returned null".into())
        })
    }

    /// Stops the printer-listing flow.
    pub fn stop_listing_printers(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbStopListingPrinters(self.raw.as_ptr()) };
    }

    // ─── Visibility toggles ──────────────────────────────────────────────────

    /// Hides remote printers from subsequent listings.
    pub fn hide_remote_printers(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbHideRemotePrinters(self.raw.as_ptr()) };
    }

    /// Unhides remote printers.
    pub fn unhide_remote_printers(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbUnhideRemotePrinters(self.raw.as_ptr()) };
    }

    /// Hides temporary printers from subsequent listings.
    pub fn hide_temporary_printers(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbHideTemporaryPrinters(self.raw.as_ptr()) };
    }

    /// Unhides temporary printers.
    pub fn unhide_temporary_printers(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbUnhideTemporaryPrinters(self.raw.as_ptr()) };
    }

    // ─── Lookup ──────────────────────────────────────────────────────────────

    /// Finds a printer by `(id, backend)`.
    ///
    /// The returned [`Printer`] borrows from `self`.
    pub fn find_printer<'f>(&'f self, printer_id: &str, backend_name: &str) -> Result<Printer<'f>> {
        let c_id = CString::new(printer_id)?;
        let c_backend = CString::new(backend_name)?;
        // SAFETY: pointers are non-null; the CStrings outlive the call.
        let raw = unsafe {
            ffi::cpdbFindPrinterObj(self.raw.as_ptr(), c_id.as_ptr(), c_backend.as_ptr())
        };
        if raw.is_null() {
            Err(CpdbError::NotFound(format!(
                "printer '{printer_id}' on backend '{backend_name}'"
            )))
        } else {
            Printer::from_raw_borrowed(raw)
        }
    }

    /// Returns the user-default printer, if one is set.
    pub fn get_default_printer(&self) -> Result<Printer<'_>> {
        // SAFETY: pointer is non-null.
        let raw = unsafe { ffi::cpdbGetDefaultPrinter(self.raw.as_ptr()) };
        if raw.is_null() {
            Err(CpdbError::NotFound("default printer".into()))
        } else {
            Printer::from_raw_borrowed(raw)
        }
    }

    /// Returns the default printer for a specific backend, if one is set.
    pub fn get_default_printer_for_backend(&self, backend_name: &str) -> Result<Printer<'_>> {
        let c_backend = CString::new(backend_name)?;
        // SAFETY: pointer is non-null; the CString outlives the call.
        let raw =
            unsafe { ffi::cpdbGetDefaultPrinterForBackend(self.raw.as_ptr(), c_backend.as_ptr()) };
        if raw.is_null() {
            Err(CpdbError::NotFound(format!(
                "default printer for backend '{backend_name}'"
            )))
        } else {
            Printer::from_raw_borrowed(raw)
        }
    }

    /// Asks every backend to refresh its printer list.
    pub fn refresh_printers(&self) {
        // SAFETY: pointer is non-null.
        unsafe { ffi::cpdbGetAllPrinters(self.raw.as_ptr()) };
    }

    /// Adds an owned printer to the frontend's table.
    ///
    /// Takes ownership of the printer — cpdb-libs becomes responsible for
    /// freeing it. The argument must be an owned [`Printer`] (e.g. from
    /// [`Printer::load_from_file`]); attempting to insert a borrowed
    /// printer is rejected to prevent the same pointer ending up in two
    /// hash tables.
    ///
    /// Returns `Ok(true)` when the printer was inserted; `Ok(false)` when
    /// it was rejected by the backend.
    ///
    /// [`Printer::load_from_file`]: crate::Printer::load_from_file
    pub fn add_printer(&self, printer: Printer<'static>) -> Result<bool> {
        if !printer.is_owned() {
            return Err(CpdbError::PrinterError(
                "add_printer requires an owned Printer".into(),
            ));
        }
        // SAFETY: ownership of the raw pointer is transferred to cpdb-libs
        // by forgetting the Rust `Printer` (so its Drop does not run).
        let raw = printer.as_raw();
        std::mem::forget(printer);
        let added = unsafe { ffi::cpdbAddPrinter(self.raw.as_ptr(), raw) };
        Ok(added != 0)
    }

    /// Removes a printer from the frontend's table by `(id, backend)`.
    ///
    /// Returns the *owned* removed printer when present, so the caller can
    /// inspect it before dropping (cpdb-libs hands ownership back).
    pub fn remove_printer(
        &self,
        printer_id: &str,
        backend_name: &str,
    ) -> Result<Option<Printer<'static>>> {
        let c_id = CString::new(printer_id)?;
        let c_backend = CString::new(backend_name)?;
        // SAFETY: pointers are non-null; the CStrings outlive the call.
        let raw =
            unsafe { ffi::cpdbRemovePrinter(self.raw.as_ptr(), c_id.as_ptr(), c_backend.as_ptr()) };
        if raw.is_null() {
            Ok(None)
        } else {
            Printer::from_raw_owned(raw).map(Some)
        }
    }

    /// Asks a specific backend to refresh its printer list. Returns `true` on success.
    pub fn refresh_printer_list(&self, backend_name: &str) -> Result<bool> {
        let c_backend = CString::new(backend_name)?;
        // SAFETY: pointers are non-null; the CString outlives the call.
        let ok = unsafe { ffi::cpdbRefreshPrinterList(self.raw.as_ptr(), c_backend.as_ptr()) };
        Ok(ok)
    }

    /// Returns every printer currently known by walking the internal hash table.
    ///
    /// The returned printers borrow from `self`.
    pub fn get_printers(&self) -> Result<Vec<Printer<'_>>> {
        // SAFETY: dereferencing the printer table field is sound; we only
        // read borrowed pointers and never write through them.
        let table = unsafe { (*self.raw.as_ptr()).printer } as *mut glib_sys::GHashTable;
        if table.is_null() {
            return Ok(Vec::new());
        }

        let mut printers: Vec<Printer<'_>> = Vec::new();
        // SAFETY: iterator is initialised on the stack and iterated
        // synchronously; the table is not mutated during this loop.
        unsafe {
            let mut iter = MaybeUninit::<glib_sys::GHashTableIter>::uninit();
            glib_sys::g_hash_table_iter_init(iter.as_mut_ptr(), table);
            let mut iter = iter.assume_init();

            let mut key: glib_sys::gpointer = std::ptr::null_mut();
            let mut value: glib_sys::gpointer = std::ptr::null_mut();
            while glib_sys::g_hash_table_iter_next(&mut iter, &mut key, &mut value) != 0 {
                let raw = value as *mut ffi::cpdb_printer_obj_t;
                if let Ok(p) = Printer::from_raw_borrowed(raw) {
                    printers.push(p);
                }
            }
        }
        Ok(printers)
    }

    /// Looks up the first printer whose `name` field equals the argument.
    ///
    /// When multiple printers share a name across backends, the first one
    /// encountered during hash-table iteration wins. Prefer
    /// [`Frontend::find_printer`] when you can supply a backend name.
    pub fn get_printer<'f>(&'f self, name: &str) -> Result<Printer<'f>> {
        // SAFETY: dereferencing the printer table field is sound.
        let table = unsafe { (*self.raw.as_ptr()).printer } as *mut glib_sys::GHashTable;
        if table.is_null() {
            return Err(CpdbError::NotFound(format!("printer '{name}'")));
        }
        let needle = name.as_bytes();

        // SAFETY: see `get_printers`.
        unsafe {
            let mut iter = MaybeUninit::<glib_sys::GHashTableIter>::uninit();
            glib_sys::g_hash_table_iter_init(iter.as_mut_ptr(), table);
            let mut iter = iter.assume_init();

            let mut key: glib_sys::gpointer = std::ptr::null_mut();
            let mut value: glib_sys::gpointer = std::ptr::null_mut();
            while glib_sys::g_hash_table_iter_next(&mut iter, &mut key, &mut value) != 0 {
                let raw = value as *mut ffi::cpdb_printer_obj_t;
                if raw.is_null() {
                    continue;
                }
                let name_ptr = (*raw).name;
                if name_ptr.is_null() {
                    continue;
                }
                if CStr::from_ptr(name_ptr).to_bytes() == needle {
                    return Printer::from_raw_borrowed(raw);
                }
            }
        }
        Err(CpdbError::NotFound(format!("printer '{name}'")))
    }
}

impl Drop for Frontend {
    fn drop(&mut self) {
        // Unregister any observer FIRST so an in-flight callback from
        // cpdb-libs' D-Bus thread finds an empty slot and bails out
        // instead of touching a half-freed object.
        callbacks::unregister_printer_observer(self.raw.as_ptr());
        // SAFETY: we own the pointer.
        unsafe { ffi::cpdbDeleteFrontendObj(self.raw.as_ptr()) };
    }
}
