use core::fmt;

use alloc::boxed::Box;
use libruntime::{kobject, process::iface::ProcessServerError};
use log::error;

/// Internal error type with context
pub struct InternalError {
    target: ProcessServerError,
    context: &'static str,
    source: Option<Box<dyn fmt::Display>>,
}

impl InternalError {
    pub fn invalid_argument(context: &'static str) -> Self {
        Self {
            target: ProcessServerError::InvalidArgument,
            context,
            source: None,
        }
    }

    pub fn invalid_binary(context: &'static str) -> Self {
        Self {
            target: ProcessServerError::InvalidBinaryFormat,
            context,
            source: None,
        }
    }

    pub fn runtime_error(context: &'static str) -> Self {
        Self {
            target: ProcessServerError::RuntimeError,
            context,
            source: None,
        }
    }

    pub fn buffer_too_small(context: &'static str) -> Self {
        Self {
            target: ProcessServerError::BufferTooSmall,
            context,
            source: None,
        }
    }

    pub fn process_already_terminated(context: &'static str) -> Self {
        Self {
            target: ProcessServerError::ProcessNotRunning,
            context,
            source: None,
        }
    }

    pub fn with_source<E: fmt::Display + 'static>(mut self, err: E) -> Self {
        self.source = Some(Box::new(err));
        self
    }
}

impl Into<ProcessServerError> for InternalError {
    fn into(self) -> ProcessServerError {
        if let Some(source) = &self.source {
            error!("{}: {}", self.context, source);
        } else {
            error!("{}", self.context);
        }

        self.target
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.context)?;
        if let Some(source) = &self.source {
            write!(f, ": {}", source)?;
        }
        Ok(())
    }
}

impl fmt::Debug for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("InternalError");
        debug
            .field("target", &self.target)
            .field("context", &self.context);

        if let Some(source) = &self.source {
            debug.field("source", &format_args!("{}", source));
        }

        debug.finish()
    }
}

impl core::error::Error for InternalError {}

impl From<kobject::Error> for InternalError {
    fn from(err: kobject::Error) -> Self {
        InternalError::runtime_error("Kernel object operation failed").with_source(err)
    }
}

/// Extension trait for Result to add context
pub trait ResultExt<T> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, InternalError>;
    fn invalid_binary(self, msg: &'static str) -> Result<T, InternalError>;
    fn runtime_err(self, msg: &'static str) -> Result<T, InternalError>;
}

impl<T, E: fmt::Display + 'static> ResultExt<T> for Result<T, E> {
    fn invalid_arg(self, msg: &'static str) -> Result<T, InternalError> {
        self.map_err(|e| InternalError::invalid_argument(msg).with_source(e))
    }

    fn invalid_binary(self, msg: &'static str) -> Result<T, InternalError> {
        self.map_err(|e| InternalError::invalid_binary(msg).with_source(e))
    }

    fn runtime_err(self, msg: &'static str) -> Result<T, InternalError> {
        self.map_err(|e| InternalError::runtime_error(msg).with_source(e))
    }
}
