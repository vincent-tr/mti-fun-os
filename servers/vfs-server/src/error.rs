use core::fmt;

use libruntime::vfs::iface::VfsServerError;
use log::error;

/// Extension trait for Result to add context
pub trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, VfsServerError>;
    fn runtime_err(self, msg: &'static str) -> Result<T, VfsServerError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, VfsServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            VfsServerError::InvalidArgument
        })
    }

    fn runtime_err(self, msg: &'static str) -> Result<T, VfsServerError> {
        self.map_err(|e| {
            error!("{}: {}", msg, e);
            VfsServerError::RuntimeError
        })
    }
}
