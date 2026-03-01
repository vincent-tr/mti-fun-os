use core::fmt;

use libruntime::process::iface::ProcessServerError;
use log::error;

/// Extension trait for Result to add context
pub trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, ProcessServerError>;
    fn invalid_binary(self, msg: &'static str) -> Result<T, ProcessServerError>;
    fn runtime_err(self, msg: &'static str) -> Result<T, ProcessServerError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, ProcessServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            ProcessServerError::InvalidArgument
        })
    }

    fn invalid_binary(self, msg: &'static str) -> Result<T, ProcessServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            ProcessServerError::InvalidBinaryFormat
        })
    }

    fn runtime_err(self, msg: &'static str) -> Result<T, ProcessServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            ProcessServerError::RuntimeError
        })
    }
}

pub fn invalid_binary(msg: &'static str) -> ProcessServerError {
    error!("Invalid binary: {}", msg);
    ProcessServerError::InvalidBinaryFormat
}
