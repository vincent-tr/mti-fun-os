/// Represents a client handle to a server object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(u64);

impl Handle {
    /// Returns an invalid handle.
    pub const fn invalid() -> Self {
        Handle(0)
    }

    /// Checks if the handle is valid.
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl From<u64> for Handle {
    fn from(value: u64) -> Self {
        Handle(value)
    }
}
