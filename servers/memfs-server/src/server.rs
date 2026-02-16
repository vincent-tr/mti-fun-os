use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use libruntime::{
    ipc::Handle,
    vfs::{
        fs::iface::{FileSystem, FsServerError, NodeId},
        iface::DirectoryEntry,
        types::{Metadata, NodeType, Permissions},
    },
};

/// The main server structure
#[derive(Debug)]
pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl FileSystem for Server {
    type Error = FsServerError;

    async fn process_terminated(&self, pid: u64) {
        todo!()
    }

    async fn lookup(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
    ) -> Result<NodeId, Self::Error> {
        todo!()
    }

    async fn create(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
        node_type: NodeType,
        permissions: Permissions,
    ) -> Result<NodeId, Self::Error> {
        todo!()
    }

    async fn remove(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn r#move(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        src_parent: NodeId,
        src_name: &str,
        dst_parent: NodeId,
        dst_name: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn get_metadata(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<Metadata, Self::Error> {
        todo!()
    }

    async fn set_metadata(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        node_id: NodeId,
        permissions: Option<Permissions>,
        size: Option<usize>,
        created: Option<u64>,
        modified: Option<u64>,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn open_file(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        node_id: NodeId,
        open_permissions: Permissions,
    ) -> Result<Handle, Self::Error> {
        todo!()
    }

    async fn close_file(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        handle: Handle,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn read_file(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        handle: Handle,
        buffer: &mut [u8],
        offset: usize,
    ) -> Result<usize, Self::Error> {
        todo!()
    }

    async fn write_file(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        handle: Handle,
        buffer: &[u8],
        offset: usize,
    ) -> Result<usize, Self::Error> {
        todo!()
    }

    async fn open_dir(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<Handle, Self::Error> {
        todo!()
    }

    async fn close_dir(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        handle: Handle,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn list_dir(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        handle: Handle,
    ) -> Result<Vec<DirectoryEntry>, Self::Error> {
        todo!()
    }

    async fn create_symlink(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
        target: &str,
    ) -> Result<NodeId, Self::Error> {
        todo!()
    }

    async fn read_symlink(
        &self,
        sender_id: u64,
        mount_handle: Handle,
        node_id: NodeId,
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        todo!()
    }

    async fn mount(&self, sender_id: u64, args: &[u8]) -> Result<(Handle, NodeId), Self::Error> {
        todo!()
    }

    async fn unmount(&self, sender_id: u64, mount_handle: Handle) -> Result<(), Self::Error> {
        todo!()
    }
}
