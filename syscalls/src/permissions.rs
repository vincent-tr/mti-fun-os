use bitflags::bitflags;

bitflags! {
  /// Possible paging permissions
  #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
  pub struct Permissions: u64 {
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
