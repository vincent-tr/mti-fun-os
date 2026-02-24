use bitflags::bitflags;

bitflags! {
    /// Possible IO memory flags
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct IoMemFlags: u64 {
        /// No flags
        const NONE = 0;

        /// Page is write-through
        const WRITE_THROUGH = 1 << 0;

        /// Page is not cached
        const NO_CACHE = 1 << 1;
    }
}
