use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::printer::Printer;
use std::ptr;

pub struct Frontend {
    raw: *mut ffi::cpdb_frontend_obj_t,
}

unsafe impl Send for Frontend {}
unsafe impl Sync for Frontend {}

impl Frontend {
    #[inline]
    pub(crate) fn as_raw(&self) -> *mut ffi::cpdb_frontend_obj_t {
        self.raw
    }
    pub fn new() -> Result<Self> {
        unsafe {
            let raw_frontend = ffi::cpdbGetNewFrontendObj(None);
            if raw_frontend.is_null() {
                Err(CpdbError::FrontendError("cpdbGetNewFrontendObj returned null".to_string()))
            } else {
                Ok(Self { raw: raw_frontend })
            }
        }
    }

    /// Connects the frontend to D-Bus and activates backends.
    pub fn connect_to_dbus(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null before calling cpdbConnectToDBus".to_string()));
        }
        unsafe {
            ffi::cpdbConnectToDBus(self.raw);
        }
        Ok(())
    }

    /// Disconnects the frontend from D-Bus.
    pub fn disconnect_from_dbus(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null before calling cpdbDisconnectFromDBus".to_string()));
        }
        unsafe {
            ffi::cpdbDisconnectFromDBus(self.raw);
        }
        Ok(())
    }

    /// Starts the printer listing process and returns a new Frontend instance configured for it.
    pub fn start_listing(printer_callback: ffi::cpdb_printer_callback) -> Result<Self> {
        unsafe {
            let new_frontend_ptr = ffi::cpdbStartListingPrinters(printer_callback);
            if new_frontend_ptr.is_null() {
                Err(CpdbError::FrontendError("cpdbStartListingPrinters returned null, failed to start listing".to_string()))
            } else {
                Ok(Frontend { raw: new_frontend_ptr })
            }
        }
    }

    /// Stops the printer listing process for the given frontend object.
    pub fn stop_listing_printers(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null before calling cpdbStopListingPrinters".to_string()));
        }
        unsafe {
            ffi::cpdbStopListingPrinters(self.raw);
        }
        Ok(())
    }

    pub fn get_printers(&self) -> Result<Vec<Printer>> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null for get_printers".to_string()));
        }
        unsafe {
            // Use cpdbGetAllPrinters which doesn't return printers directly
            // Instead, we need to implement a callback-based approach
            ffi::cpdbGetAllPrinters(self.raw);
            
            // For now, return empty vector since cpdbGetAllPrinters uses callbacks
            // In a real implementation, you'd need to set up callbacks to collect printers
            Ok(Vec::new())
        }
    }

    pub fn get_printer(&self, name: &str) -> Result<Printer> {
        // Since cpdbGetPrinter doesn't exist in the actual API,
        // we'll need to implement printer lookup differently
        // For now, return an error indicating this needs to be implemented
        Err(CpdbError::FrontendError(format!("Printer lookup by name '{}' not yet implemented - requires callback-based approach", name)))
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