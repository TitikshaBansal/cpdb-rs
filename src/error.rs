use std::ffi::NulError;
use thiserror::Error;
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum CpdbError {
    #[error("Null pointer encountered")]
    NullPointer,
    #[error("Invalid printer object")]
    InvalidPrinter,
    #[error("Print error: {0}")]
    PrintError(String),
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
    #[error("Invalid UTF-8 string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("Nul byte in string: {0}")]
    NulError(#[from] NulError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid status code: {0}")]
    InvalidStatus(i32),
    #[error("Unsupported operation")]
    Unsupported,
}

pub type Result<T> = std::result::Result<T, CpdbError>;

impl CpdbError {
    pub fn from_status(status: i32, context: &str) -> Self {
        match status {
            0 => CpdbError::NullPointer,
            1 => CpdbError::InvalidPrinter,
            2 => CpdbError::JobFailed(context.to_string()),
            _ => CpdbError::BackendError(format!("Unknown error ({}): {}", status, context)),
        }
    }
}
