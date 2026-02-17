use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use async_trait::async_trait;
use libruntime::{
    ipc::{self, Handle},
    kobject::{self, KObject},
    sync::RwLock,
    vfs::{
        iface::{DirectoryEntry, MountInfo, VfsServer, VfsServerError},
        types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions},
    },
};
use log::{debug, info, warn};

use crate::{mounts::MountTable, vnode::VNode};

/// The main server structure
#[derive(Debug)]
pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Self {}
    }

    /// Lookup a vnode by its path.
    fn lookup(&self, path: &str, no_follow: bool) -> Result<VNode, VfsServerError> {
        if !path.starts_with('/') {
            return Err(VfsServerError::InvalidArgument);
        }

        let mut current = MountTable::get().root().ok_or(VfsServerError::NotFound)?;

        for part in path.split('/') {
            if part.is_empty() {
                continue;
            }
        }

        Ok(current)
    }
}

#[async_trait]
impl VfsServer for Server {
    type Error = VfsServerError;

    async fn process_terminated(&self, pid: u64) {
        todo!()
    }

    async fn open(
        &self,
        sender_id: u64,
        path: &str,
        r#type: Option<NodeType>,
        mode: OpenMode,
        no_follow: bool,
        permissions: Permissions,
        handle_permissions: HandlePermissions,
    ) -> Result<Handle, Self::Error> {
        todo!()
    }

    async fn close(&self, sender_id: u64, handle: Handle) -> Result<(), Self::Error> {
        todo!()
    }

    async fn stat(&self, sender_id: u64, handle: Handle) -> Result<Metadata, Self::Error> {
        todo!()
    }

    async fn set_permissions(
        &self,
        sender_id: u64,
        handle: Handle,
        permissions: Permissions,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn read(
        &self,
        sender_id: u64,
        handle: Handle,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        todo!()
    }

    async fn write(
        &self,
        sender_id: u64,
        handle: Handle,
        offset: usize,
        buffer: &[u8],
    ) -> Result<usize, Self::Error> {
        todo!()
    }

    async fn resize(
        &self,
        sender_id: u64,
        handle: Handle,
        new_size: usize,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn list(
        &self,
        sender_id: u64,
        handle: Handle,
    ) -> Result<Vec<DirectoryEntry>, Self::Error> {
        todo!()
    }

    async fn r#move(
        &self,
        sender_id: u64,
        old_dir: Handle,
        old_name: &str,
        new_dir: Handle,
        new_name: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn remove(&self, sender_id: u64, dir: Handle, name: &str) -> Result<(), Self::Error> {
        todo!()
    }

    async fn create_symlink(
        &self,
        sender_id: u64,
        path: &str,
        target: &str,
    ) -> Result<Handle, Self::Error> {
        todo!()
    }

    async fn read_symlink(&self, sender_id: u64, handle: Handle) -> Result<String, Self::Error> {
        todo!()
    }

    async fn mount(
        &self,
        sender_id: u64,
        mount_point: &str,
        fs_port_name: &str,
        args: &[u8],
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn unmount(&self, sender_id: u64, mount_point: &str) -> Result<(), Self::Error> {
        todo!()
    }

    async fn list_mounts(&self, sender_id: u64) -> Result<Vec<MountInfo>, Self::Error> {
        todo!()
    }
}
