pub mod messages;

use crate::kobject;

#[derive(Debug)]
pub struct Process {
    kobj: kobject::Process,
}

impl Process {
    pub fn spawn() -> Self {
        unimplemented!()
    }
}
