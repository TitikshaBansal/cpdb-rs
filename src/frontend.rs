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
            let raw = ffi::cpdbGetNewFrontendObj();
            if raw.is_null() {
                Err(CpdbError::FrontendError("Failed to create frontend object".into()))
            } else {
                Ok(Self { raw })
            }
        }
    }

    pub fn get_printers(&self) -> Result<Vec<Printer>> {
        unsafe {
            let mut printers_ptr: *mut *mut ffi::cpdb_printer_obj_t = ptr::null_mut();
            let count = ffi::cpdbGetPrinters(
                self.raw,
                &mut printers_ptr as *mut *mut *mut ffi::cpdb_printer_obj_t,
            );

            if count < 0 {
                return Err(CpdbError::FrontendError("Failed to get printers".into()));
            }

            let mut printers = Vec::with_capacity(count as usize);
            for i in 0..count {
                let printer_ptr = *printers_ptr.offset(i as isize);
                printers.push(Printer::from_raw(printer_ptr)?);
            }
            
            libc::free(printers_ptr as *mut libc::c_void);
            
            Ok(printers)
        }
    }

    pub fn get_printer(&self, name: &str) -> Result<Printer> {
        unsafe {
            let c_name = std::ffi::CString::new(name)?;
            let printer_ptr = ffi::cpdbGetPrinter(self.raw, c_name.as_ptr());
            
            if printer_ptr.is_null() {
                Err(CpdbError::InvalidPrinter)
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