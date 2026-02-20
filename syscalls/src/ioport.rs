use bitflags::bitflags;

bitflags! {
    /// Possible port access
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct PortAccess: u64 {
        /// No access
        const NONE = 0;

        /// Page can be read
        const READ = 1 << 0;

        /// Page can be written
        const WRITE = 1 << 1;

        /// Page can be executed
        const EXECUTE = 1 << 2;
    }
}
