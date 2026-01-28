use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    code: libsyscalls::Error,
}

impl Error {
    pub const fn new(code: libsyscalls::Error) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> libsyscalls::Error {
        self.code
    }
}

#[allow(non_upper_case_globals)]
impl Error {
    pub const InvalidArgument: Self = Self::new(libsyscalls::Error::InvalidArgument);
    pub const OutOfMemory: Self = Self::new(libsyscalls::Error::OutOfMemory);
    pub const NotSupported: Self = Self::new(libsyscalls::Error::NotSupported);
    pub const MemoryAccessDenied: Self = Self::new(libsyscalls::Error::MemoryAccessDenied);
    pub const ObjectNotFound: Self = Self::new(libsyscalls::Error::ObjectNotFound);
    pub const ObjectNameDuplicate: Self = Self::new(libsyscalls::Error::ObjectNameDuplicate);
    pub const ObjectClosed: Self = Self::new(libsyscalls::Error::ObjectClosed);
    pub const ObjectNotReady: Self = Self::new(libsyscalls::Error::ObjectNotReady);
}

impl From<libsyscalls::Error> for Error {
    fn from(code: libsyscalls::Error) -> Self {
        Self::new(code)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code {
            libsyscalls::Error::InvalidArgument => write!(f, "InvalidArgument"),
            libsyscalls::Error::OutOfMemory => write!(f, "OutOfMemory"),
            libsyscalls::Error::NotSupported => write!(f, "NotSupported"),
            libsyscalls::Error::MemoryAccessDenied => write!(f, "MemoryAccessDenied"),
            libsyscalls::Error::ObjectNotFound => write!(f, "ObjectNotFound"),
            libsyscalls::Error::ObjectNameDuplicate => write!(f, "ObjectNameDuplicate"),
            libsyscalls::Error::ObjectClosed => write!(f, "ObjectClosed"),
            libsyscalls::Error::ObjectNotReady => write!(f, "ObjectNotReady"),
        }
    }
}

impl core::error::Error for Error {}
