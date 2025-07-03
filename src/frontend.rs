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
    pub fn new() -> Result<Self> {
        unsafe {
            let raw_frontend = ffi::cpdbGetNewFrontendObj(ptr::null_mut());
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
            let mut printers_ptr: *mut *mut ffi::cpdb_printer_obj_t = ptr::null_mut();
            let count = ffi::cpdbGetPrinters(
                self.raw,
                &mut printers_ptr as *mut *mut *mut ffi::cpdb_printer_obj_t,
            );

            if count < 0 {
                return Err(CpdbError::FrontendError(format!("cpdbGetPrinters failed with code: {}", count)));
            }

            let mut printers = Vec::with_capacity(count as usize);
            if !printers_ptr.is_null() {
                for i in 0..(count as isize) {
                    let printer_ptr = *printers_ptr.offset(i);
                    printers.push(Printer::from_raw(printer_ptr)?);
                }
                libc::free(printers_ptr as *mut libc::c_void);
            }
            Ok(printers)
        }
    }

    pub fn get_printer(&self, name: &str) -> Result<Printer> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null for get_printer".to_string()));
        }
        unsafe {
            let c_name = std::ffi::CString::new(name)?;
            let printer_ptr = ffi::cpdbGetPrinter(self.raw, c_name.as_ptr());
            
            if printer_ptr.is_null() {
                Err(CpdbError::FrontendError(format!("Printer with name '{}' not found", name)))
            } else {
                Printer::from_raw(printer_ptr)
            }
        }
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