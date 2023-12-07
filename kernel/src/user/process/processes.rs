use hashbrown::HashMap;
use lazy_static::lazy_static;

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use spin::RwLock;

use crate::user::{id_gen::IdGen, Error, process::process};

use super::Process;

lazy_static! {
    pub static ref PROCESSES: Processes = Processes::new();
}

#[derive(Debug)]
pub struct Processes {
  id_gen: IdGen,
  processes: RwLock<HashMap<u32, Weak<Process>>>,
}

impl Processes {
  fn new() -> Self {
      Self {
          id_gen: IdGen::new(),
          processes: RwLock::new(HashMap::new()),
      }
  }

  /// Create a new process
  pub fn create(&self) -> Result<Arc<Process>, Error> {
      self.clean_map();

      let id = self.id_gen.generate();
      let process = process::new(id)?;

      let mut map = self.processes.write();
      assert!(
          map.insert(id, Arc::downgrade(&process)).is_none(),
          "unepxected map overwrite"
      );

      Ok(process)
  }

  /// Find a process by its pid
  pub fn find(&self, pid: u32) -> Option<Arc<Process>> {
      self.clean_map();

      let map = self.processes.read();
      if let Some(weak) = map.get(&pid) {
          return weak.upgrade();
      } else {
          None
      }
  }

  fn clean_map(&self) {
      let map = self.processes.upgradeable_read();

      let mut delete_list = Vec::new();

      for (&pid, weak) in map.iter() {
          if weak.strong_count() == 0 {
              delete_list.push(pid);
          }
      }

      if delete_list.len() > 0 {
          let mut map = map.upgrade();
          for pid in delete_list {
              map.remove(&pid);
          }
      }
  }
}
