use thiserror::Error;
use std::ffi::{CStr, NulError};

#[derive(Error, Debug)]
pub enum CpdbError {
    #[error("Null pointer encountered")]
    NullPointer,
    #[error("Invalid printer object")]
    InvalidPrinter,
    #[error("Print job failed: {0}")]
    JobFailed(String),
    #[error("Backend error: {0}")]
    BackendError(String),
    #[error("Frontend error: {0}")]
    FrontendError(String),
    #[error("Option parsing error: {0}")]
    OptionError(String),
    #[error("CUPS error: {0}")]
    CupsError(i32),
    #[error("Invalid UTF-8 string")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("String contains null byte")]
    NulError(#[from] NulError),
    #[error("Unsupported operation")]
    Unsupported,
    #[error("Channel communication error")]
    ChannelError,
}

pub type Result<T> = std::result::Result<T, CpdbError>;

impl CpdbError {
    /// Convert from cpdb status codes
    pub fn from_status(status: i32, context: &str) -> Self {
        match status {
            0 => CpdbError::NullPointer,
            1 => CpdbError::InvalidPrinter,
            2 => CpdbError::JobFailed(context.to_string()),
            _ => CpdbError::BackendError(format!("Unknown error: {}", status)),
        }
    }
    
    /// Convert C string to Rust String with error handling
    pub unsafe fn cstr_to_string(ptr: *const libc::c_char) -> Result<String> {
        if ptr.is_null() {
            return Err(CpdbError::NullPointer);
        }
        // SAFETY: ptr is checked for null
        let cstr = unsafe { CStr::from_ptr(ptr) };
        cstr.to_str()
            .map(|s| s.to_string())
            .map_err(Into::into)
    }
}