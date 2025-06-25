use crate::error::{CpdbError, Result};
use crate::ffi;
use crate::printer::Printer;
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::os::raw::c_void;

static FRONTEND: OnceLock<Mutex<Frontend>> = OnceLock::new();

pub struct Frontend {
    raw: *mut ffi::cpdb_frontend_obj_t,
}

unsafe impl Send for Frontend {}
unsafe impl Sync for Frontend {}

impl Frontend {
    pub fn global() -> Result<&'static Mutex<Self>> {
        FRONTEND.get_or_init(|| {
            Mutex::new(Self::new().expect("Failed to create frontend"))
        });
        Ok(FRONTEND.get().unwrap())
    }

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

    pub fn get_printers_async<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Result<Printer>) + Send + 'static,
    {
        unsafe extern "C" fn c_callback(
            printer_ptr: *mut ffi::cpdb_printer_obj_t,
            user_data: *mut c_void,
        ) {
            let callback = &*(user_data as *const Box<dyn Fn(Result<Printer>)>);
            let result = if printer_ptr.is_null() {
                Err(CpdbError::NullPointer)
            } else {
                unsafe { Printer::from_raw(printer_ptr) }
            };
            callback(result);
        }

        let callback_box = Box::new(Box::new(callback) as Box<dyn Fn(Result<Printer>)>);
        let user_data = Box::into_raw(callback_box) as *mut c_void;

        unsafe {
            ffi::cpdbGetPrintersAsync(
                self.raw,
                Some(c_callback),
                user_data,
            );
        }

        Ok(())
    }

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
                ffi::cpdbDeleteFrontendObj(self.raw);
                self.raw = ptr::null_mut();
            }
        }
    }
}