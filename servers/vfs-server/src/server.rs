use alloc::{boxed::Box, string::String, vec::Vec};
use async_trait::async_trait;
use libruntime::{
    ipc::Handle,
    vfs::{
        iface::{DirectoryEntry, MountInfo, VfsServer, VfsServerError},
        types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions},
    },
};

use crate::{lookup, mounts::MountTable};

/// The main server structure
#[derive(Debug)]
pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl VfsServer for Server {
    type Error = VfsServerError;

    async fn process_terminated(&self, pid: u64) {
        let _ = pid;
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
        let _ = sender_id;
        let _ = path;
        let _ = r#type;
        let _ = mode;
        let _ = no_follow;
        let _ = permissions;
        let _ = handle_permissions;
        todo!()
    }

    async fn close(&self, sender_id: u64, handle: Handle) -> Result<(), Self::Error> {
        let _ = sender_id;
        let _ = handle;
        todo!()
    }

    async fn stat(&self, sender_id: u64, handle: Handle) -> Result<Metadata, Self::Error> {
        let _ = sender_id;
        let _ = handle;
        todo!()
    }

    async fn set_permissions(
        &self,
        sender_id: u64,
        handle: Handle,
        permissions: Permissions,
    ) -> Result<(), Self::Error> {
        let _ = sender_id;
        let _ = handle;
        let _ = permissions;
        todo!()
    }

    async fn read(
        &self,
        sender_id: u64,
        handle: Handle,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let _ = sender_id;
        let _ = handle;
        let _ = offset;
        let _ = buffer;
        todo!()
    }

    async fn write(
        &self,
        sender_id: u64,
        handle: Handle,
        offset: usize,
        buffer: &[u8],
    ) -> Result<usize, Self::Error> {
        let _ = sender_id;
        let _ = handle;
        let _ = offset;
        let _ = buffer;
        todo!()
    }

    async fn resize(
        &self,
        sender_id: u64,
        handle: Handle,
        new_size: usize,
    ) -> Result<(), Self::Error> {
        let _ = sender_id;
        let _ = handle;
        let _ = new_size;
        todo!()
    }

    async fn list(
        &self,
        sender_id: u64,
        handle: Handle,
    ) -> Result<Vec<DirectoryEntry>, Self::Error> {
        let _ = sender_id;
        let _ = handle;
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
        let _ = sender_id;
        let _ = old_dir;
        let _ = old_name;
        let _ = new_dir;
        let _ = new_name;
        todo!()
    }

    async fn remove(&self, sender_id: u64, dir: Handle, name: &str) -> Result<(), Self::Error> {
        let _ = sender_id;
        let _ = dir;
        let _ = name;
        todo!()
    }

    async fn create_symlink(
        &self,
        sender_id: u64,
        path: &str,
        target: &str,
    ) -> Result<Handle, Self::Error> {
        let _ = sender_id;
        let _ = path;
        let _ = target;
        todo!()
    }

    async fn read_symlink(&self, sender_id: u64, handle: Handle) -> Result<String, Self::Error> {
        let _ = sender_id;
        let _ = handle;
        todo!()
    }

    async fn mount(
        &self,
        _sender_id: u64,
        mount_point: &str,
        fs_port_name: &str,
        args: &[u8],
    ) -> Result<(), Self::Error> {
        let mount_point = lookup::lookup(mount_point, false).await?;

        MountTable::get()
            .mount(&mount_point, fs_port_name, args)
            .await?;

        Ok(())
    }

    async fn unmount(&self, _sender_id: u64, mount_point: &str) -> Result<(), Self::Error> {
        let mount_point = lookup::lookup(mount_point, false).await?;

        MountTable::get().unmount(&mount_point).await?;

        Ok(())
    }

    async fn list_mounts(&self, sender_id: u64) -> Result<Vec<MountInfo>, Self::Error> {
        let _ = sender_id;
        todo!()
    }
}
