use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::printer::Printer;
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::os::raw::c_void;
use crossbeam_channel::Sender;

static FRONTEND: OnceLock<Mutex<Frontend>> = OnceLock::new();

pub struct Frontend {
    raw: *mut ffi::cpdb_frontend_obj_t,
}

// Mark as Send/Sync since CPDB handles thread safety
unsafe impl Send for Frontend {}
unsafe impl Sync for Frontend {}

impl Frontend {
    /// Get global frontend instance (thread-safe)
    pub fn global() -> Result<&'static Mutex<Self>> {
        FRONTEND.get_or_init(|| {
            Mutex::new(Self::new().expect("Failed to create frontend"))
        });
        Ok(FRONTEND.get().unwrap())
    }

    /// Create new frontend instance
    pub fn new() -> Result<Self> {
        unsafe {
            let raw = ffi::cpdb_get_new_frontend_obj();
            if raw.is_null() {
                Err(CpdbError::FrontendError("Failed to create frontend object".into()))
            } else {
                Ok(Self { raw })
            }
        }
    }

    /// Get all available printers
    pub fn get_printers(&self) -> Result<Vec<Printer>> {
        unsafe {
            let mut printers_ptr: *mut *mut ffi::cpdb_printer_obj_t = ptr::null_mut();
            let count = ffi::cpdb_get_printers(
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
            
            // Free the C array (but not the printer objects themselves)
            libc::free(printers_ptr as *mut libc::c_void);
            
            Ok(printers)
        }
    }

    /// Get printer by name
    pub fn get_printer(&self, name: &str) -> Result<Printer> {
        unsafe {
            let c_name = std::ffi::CString::new(name)?;
            let printer_ptr = ffi::cpdb_get_printer(self.raw, c_name.as_ptr());
            
            if printer_ptr.is_null() {
                Err(CpdbError::InvalidPrinter)
            } else {
                Printer::from_raw(printer_ptr)
            }
        }
    }

    /// Asynchronously get printers with callback
    pub fn get_printers_async<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Result<Printer>) + Send + 'static,
    {
        unsafe extern "C" fn c_callback(
            printer_ptr: *mut ffi::cpdb_printer_obj_t,
            user_data: *mut c_void,
        ) {
            // SAFETY: We control the lifetime of this callback
            let callback = unsafe { &*(user_data as *const Box<dyn Fn(Result<Printer>)>) };
            let result = if printer_ptr.is_null() {
                Err(CpdbError::NullPointer)
            } else {
                Printer::from_raw(printer_ptr)
            };
            callback(result);
        }

        let callback_box = Box::new(Box::new(callback) as Box<dyn Fn(Result<Printer>)>);
        let user_data = Box::into_raw(callback_box) as *mut c_void;

        unsafe {
            ffi::cpdb_get_printers_async(
                self.raw,
                Some(c_callback),
                user_data,
            );
        }

        Ok(())
    }

    /// Get printers with channel interface
    pub fn get_printers_channel(&self) -> Result<crossbeam_channel::Receiver<Result<Printer>>> {
        let (tx, rx) = crossbeam_channel::unbounded();
        
        let tx_clone = tx.clone();
        self.get_printers_async(move |printer| {
            tx_clone.send(printer).expect("Channel send failed");
        })?;
        
        Ok(rx)
    }
}

impl Drop for Frontend {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                ffi::cpdb_delete_frontend_obj(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}