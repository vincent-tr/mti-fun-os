use alloc::sync::Arc;

use crate::{ipc, kobject};

use super::FileSystem;

/// The main manager structure
#[derive(Debug)]
pub struct Manager<FS: FileSystem> {
    /// The filesystem implementation.
    pub fs: FS,
}

impl<FS: FileSystem> Manager<FS> {
    pub fn new(fs: FS) -> Result<Arc<Self>, kobject::Error> {
        let manager = Self { fs };

        Ok(Arc::new(manager))
    }

    pub fn build_ipc_server(self: &Arc<Self>) -> Result<ipc::AsyncServer, kobject::Error> {
        panic!("TODO: implement IPC server for VFS manager");
    }
}
