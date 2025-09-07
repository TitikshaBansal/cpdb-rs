use crate::error::{CpdbError, Result};

// Note: The actual cpdb-libs API doesn't have a separate backend object
// Backend functionality is integrated into the frontend and printer objects
// This module is kept for future compatibility but currently not functional

pub struct Backend {
    // Placeholder - no actual backend object in cpdb-libs API
}

impl Backend {
    pub fn new(_backend_name: &str) -> Result<Self> {
        Err(CpdbError::BackendError("Backend objects not supported in cpdb-libs API - use Frontend and Printer instead".into()))
    }

    pub fn submit_job(
        &self,
        _printer_name: &str,
        _file_path: &str,
        _options: &[(&str, &str)],
        _job_name: &str,
    ) -> Result<()> {
        Err(CpdbError::BackendError("Backend job submission not supported - use Printer::print_single_file instead".into()))
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        // No cleanup needed since there's no actual backend object
    }
}