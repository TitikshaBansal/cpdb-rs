use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::printer::Printer;
use std::ffi::CString;
use std::ptr;

pub struct Frontend {
    raw: *mut ffi::cpdb_frontend_obj_t,
}

unsafe impl Send for Frontend {}
unsafe impl Sync for Frontend {}

impl Frontend {
    #[inline]
    pub fn as_raw(&self) -> *mut ffi::cpdb_frontend_obj_t {
        self.raw
    }

    pub fn new() -> Result<Self> {
        unsafe {
            let raw_frontend = ffi::cpdbGetNewFrontendObj(None);
            if raw_frontend.is_null() {
                Err(CpdbError::FrontendError(
                    "cpdbGetNewFrontendObj returned null".to_string(),
                ))
            } else {
                Ok(Self { raw: raw_frontend })
            }
        }
    }

    /// Create a frontend with a custom printer callback.
    pub fn new_with_callback(cb: ffi::cpdb_printer_callback) -> Result<Self> {
        unsafe {
            let raw_frontend = ffi::cpdbGetNewFrontendObj(cb);
            if raw_frontend.is_null() {
                Err(CpdbError::FrontendError(
                    "cpdbGetNewFrontendObj returned null".to_string(),
                ))
            } else {
                Ok(Self { raw: raw_frontend })
            }
        }
    }

    pub fn from_raw(raw: *mut ffi::cpdb_frontend_obj_t) -> Result<Self> {
        if raw.is_null() {
            Err(CpdbError::NullPointer)
        } else {
            Ok(Self { raw })
        }
    }

    /// Ignore previously saved settings (call after new(), before connect_to_dbus()).
    pub fn ignore_last_saved_settings(&self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::cpdbIgnoreLastSavedSettings(self.raw);
            }
        }
    }

    /// Connects the frontend to D-Bus and activates backends.
    pub fn connect_to_dbus(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError(
                "Frontend raw pointer is null before calling cpdbConnectToDBus".to_string(),
            ));
        }
        unsafe {
            ffi::cpdbConnectToDBus(self.raw);
        }
        Ok(())
    }

    /// Disconnects the frontend from D-Bus.
    pub fn disconnect_from_dbus(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError(
                "Frontend raw pointer is null before calling cpdbDisconnectFromDBus".to_string(),
            ));
        }
        unsafe {
            ffi::cpdbDisconnectFromDBus(self.raw);
        }
        Ok(())
    }

    /// Activate backends (rediscover without full disconnect/reconnect)
    pub fn activate_backends(&self) {
        unsafe {
            ffi::cpdbActivateBackends(self.raw);
        }
    }

    /// Start the background thread that periodically checks for new backends.
    pub fn start_backend_list_refreshing(&self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::cpdbStartBackendListRefreshing(self.raw);
            }
        }
    }

    /// Stop the background thread. Blocks until the thread joins.
    pub fn stop_backend_list_refreshing(&self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::cpdbStopBackendListRefreshing(self.raw);
            }
        }
    }

    /// Starts the printer listing process and returns a new Frontend instance configured for it.
    pub fn start_listing(printer_callback: ffi::cpdb_printer_callback) -> Result<Self> {
        unsafe {
            let new_frontend_ptr = ffi::cpdbStartListingPrinters(printer_callback);
            if new_frontend_ptr.is_null() {
                Err(CpdbError::FrontendError(
                    "cpdbStartListingPrinters returned null, failed to start listing".to_string(),
                ))
            } else {
                Ok(Frontend {
                    raw: new_frontend_ptr,
                })
            }
        }
    }

    /// Stops the printer listing process for the given frontend object.
    pub fn stop_listing_printers(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError(
                "Frontend raw pointer is null before calling cpdbStopListingPrinters".to_string(),
            ));
        }
        unsafe {
            ffi::cpdbStopListingPrinters(self.raw);
        }
        Ok(())
    }

    /// Hide remote printers.
    pub fn hide_remote_printers(&self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::cpdbHideRemotePrinters(self.raw);
            }
        }
    }

    /// Unhide remote printers.
    pub fn unhide_remote_printers(&self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::cpdbUnhideRemotePrinters(self.raw);
            }
        }
    }

    /// Hide temporary printers.
    pub fn hide_temporary_printers(&self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::cpdbHideTemporaryPrinters(self.raw);
                (*self.raw).hide_temporary = 1;
            }
        }
    }

    /// Unhide temporary printers.
    pub fn unhide_temporary_printers(&self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::cpdbUnhideTemporaryPrinters(self.raw);
                (*self.raw).hide_temporary = 0;
            }
        }
    }

    /// Find a printer by id and backend name.
    pub fn find_printer(&self, printer_id: &str, backend_name: &str) -> Result<Printer> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Null frontend".to_string()));
        }
        let c_id = CString::new(printer_id)
            .map_err(|_| CpdbError::FrontendError("Invalid printer_id".to_string()))?;
        let c_backend = CString::new(backend_name)
            .map_err(|_| CpdbError::FrontendError("Invalid backend_name".to_string()))?;
        unsafe {
            let raw_printer = ffi::cpdbFindPrinterObj(self.raw, c_id.as_ptr(), c_backend.as_ptr());
            if raw_printer.is_null() {
                Err(CpdbError::PrinterError(format!(
                    "Printer '{}' on backend '{}' not found",
                    printer_id, backend_name
                )))
            } else {
                Printer::from_raw_borrowed(raw_printer)
            }
        }
    }

    /// Get the default printer.
    pub fn get_default_printer(&self) -> Result<Printer> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Null frontend".to_string()));
        }
        unsafe {
            let raw = ffi::cpdbGetDefaultPrinter(self.raw);
            if raw.is_null() {
                Err(CpdbError::FrontendError(
                    "No default printer found".to_string(),
                ))
            } else {
                Printer::from_raw_borrowed(raw)
            }
        }
    }

    /// Get the default printer for a specific backend.
    pub fn get_default_printer_for_backend(&self, backend_name: &str) -> Result<Printer> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Null frontend".to_string()));
        }
        let c_backend = CString::new(backend_name)
            .map_err(|_| CpdbError::FrontendError("Invalid backend_name".to_string()))?;
        unsafe {
            let raw = ffi::cpdbGetDefaultPrinterForBackend(self.raw, c_backend.as_ptr());
            if raw.is_null() {
                Err(CpdbError::FrontendError(format!(
                    "No default printer for backend '{}'",
                    backend_name
                )))
            } else {
                Printer::from_raw_borrowed(raw)
            }
        }
    }

    /// Refresh printers from all backends.
    pub fn get_all_printers(&self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::cpdbGetAllPrinters(self.raw);
            }
        }
    }

    /// Get all currently known printers by iterating the internal hash table.
    pub fn get_printers(&self) -> Result<Vec<Printer>> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError(
                "Frontend raw pointer is null for get_printers".to_string(),
            ));
        }
        unsafe {
            let hash_table = (*self.raw).printer as *mut glib_sys::GHashTable;
            if hash_table.is_null() {
                return Ok(Vec::new());
            }

            let mut printers: Vec<Printer> = Vec::new();
            let mut iter: glib_sys::GHashTableIter = std::mem::zeroed();
            let mut _key: glib_sys::gpointer = std::ptr::null_mut();
            let mut value: glib_sys::gpointer = std::ptr::null_mut();

            glib_sys::g_hash_table_iter_init(&mut iter, hash_table);
            while glib_sys::g_hash_table_iter_next(&mut iter, &mut _key, &mut value)
                != glib_sys::GFALSE
            {
                let raw_printer = value as *mut ffi::cpdb_printer_obj_t;
                if !raw_printer.is_null()
                    && let Ok(p) = Printer::from_raw_borrowed(raw_printer)
                {
                    printers.push(p);
                }
            }
            Ok(printers)
        }
    }

    pub fn get_printer(&self, name: &str) -> Result<Printer> {
        // Since cpdbGetPrinter doesn't exist in the actual API,
        // we'll need to implement printer lookup differently
        // For now, return an error indicating this needs to be implemented
        Err(CpdbError::FrontendError(format!(
            "Printer lookup by name '{}' not yet implemented - requires callback-based approach",
            name
        )))
    }
}

impl Drop for Frontend {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdbDeleteFrontendObj(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}
