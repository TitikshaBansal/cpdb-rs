use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::printer::Printer;
use std::ptr;

pub struct Frontend {
    raw: *mut ffi::cpdb_frontend_obj_t,
}
// Safety: The raw pointer is managed by the C library, and we ensure it is valid
unsafe impl Send for Frontend {}
unsafe impl Sync for Frontend {}

impl Frontend {
    /// Creates a new frontend object.
    ///
    /// Initializes a connection to the CPDB system. A `cpdb_printer_callback`
    /// is needed by the C function; here, `null_mut()` is passed, which typically
    /// means a default callback or no specific callback behavior is registered at this stage.
    pub fn new() -> Result<Self> {
        unsafe {
            // Pass null_mut() for the cpdb_printer_callback
            let raw_frontend = ffi::cpdbGetNewFrontendObj(ptr::null_mut());
            if raw_frontend.is_null() {
                Err(CpdbError::FrontendError("cpdbGetNewFrontendObj returned null".into()))
            } else {
                Ok(Self { raw: raw_frontend })
            }
        }
    }

    /// Connects the frontend to D-Bus and activates backends.
    ///
    /// This function must be called after creating a `Frontend` object and before
    /// attempting to list or interact with printers. The underlying C function
    /// `cpdbConnectToDBus` returns `void`.
    pub fn connect_to_dbus(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null before calling cpdbConnectToDBus".into()));
        }
        unsafe {
            ffi::cpdbConnectToDBus(self.raw);
        }
        Ok(())
    }

    /// Disconnects the frontend from D-Bus.
    ///
    /// The underlying C function `cpdbDisconnectFromDBus` returns `void`.
    pub fn disconnect_from_dbus(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null before calling cpdbDisconnectFromDBus".into()));
        }
        unsafe {
            ffi::cpdbDisconnectFromDBus(self.raw);
        }
        Ok(())
    }

    /// Starts the process of listing/discovering available printers.
    ///

    pub fn start_listing_printers(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null before calling cpdbStartListingPrinters".into()));
        }
        unsafe {
            ffi::cpdbStartListingPrinters(self.raw, ptr::null_mut()); 
        }
        Ok(())
    }

    /// Stops the printer listing process.
    ///

    pub fn stop_listing_printers(&self) -> Result<()> {
        if self.raw.is_null() {
            return Err(CpdbError::FrontendError("Frontend raw pointer is null before calling cpdbStopListingPrinters".into()));
        }
        unsafe {
            ffi::cpdbStopListingPrinters(self.raw);
        }
        Ok(())
    }

    pub fn get_printers(&self) -> Result<Vec<Printer>> {
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
        unsafe {
            let c_name = std::ffi::CString::new(name)?;
            let printer_ptr = ffi::cpdbGetPrinter(self.raw, c_name.as_ptr());
            
            if printer_ptr.is_null() {
                Err(CpdbError::ObjectNotFound{ object_description: format!("Printer with name '{}'", name) })
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