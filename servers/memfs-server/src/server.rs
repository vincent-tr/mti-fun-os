use log::{error, info};

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use async_trait::async_trait;
use hashbrown::HashMap;
use libruntime::{
    ipc::Handle,
    sync::RwLock,
    vfs::{
        fs::iface::{FileSystem, FsServerError},
        iface::DirectoryEntry,
        types::{HandlePermissions, Metadata, NodeId, NodeType, Permissions},
    },
};

use crate::{instance::FsInstance, state::State};

/// The main server structure
#[derive(Debug)]
pub struct Server {
    instances: RwLock<HashMap<Handle, Arc<RwLock<FsInstance>>>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    fn get_instance(&self, mount_handle: Handle) -> Result<Arc<RwLock<FsInstance>>, FsServerError> {
        self.instances
            .read()
            .get(&mount_handle)
            .cloned()
            .ok_or_else(|| {
                error!("Invalid mount handle: {:?}", mount_handle);
                FsServerError::InvalidArgument
            })
    }

    fn new_handle() -> Handle {
        State::get().handle_generator().generate()
    }
}

#[async_trait]
impl FileSystem for Server {
    type Error = FsServerError;

    async fn lookup(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
    ) -> Result<NodeId, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let node_id = instance.read().lookup(parent, name)?;

        Ok(node_id)
    }

    async fn create(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
        node_type: NodeType,
        permissions: Permissions,
    ) -> Result<NodeId, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let node_id = instance
            .write()
            .create(parent, name, node_type, permissions)?;

        Ok(node_id)
    }

    async fn remove(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
    ) -> Result<(), Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        instance.write().remove(parent, name)?;

        Ok(())
    }

    async fn r#move(
        &self,
        mount_handle: Handle,
        src_parent: NodeId,
        src_name: &str,
        dst_parent: NodeId,
        dst_name: &str,
    ) -> Result<(), Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        instance
            .write()
            .r#move(src_parent, src_name, dst_parent, dst_name)?;

        Ok(())
    }

    async fn get_metadata(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<Metadata, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let metadata = instance.read().get_metadata(node_id)?;

        Ok(metadata)
    }

    async fn set_metadata(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
        permissions: Option<Permissions>,
        size: Option<usize>,
        created: Option<u64>,
        modified: Option<u64>,
    ) -> Result<(), Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        instance
            .write()
            .set_metadata(node_id, permissions, size, created, modified)?;

        Ok(())
    }

    async fn open_file(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
        open_permissions: HandlePermissions,
    ) -> Result<Handle, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let file_handle = instance.write().open_file(node_id, open_permissions)?;

        Ok(file_handle)
    }

    async fn close_file(&self, mount_handle: Handle, handle: Handle) -> Result<(), Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        instance.write().close_file(handle)?;

        Ok(())
    }

    async fn read_file(
        &self,
        mount_handle: Handle,
        handle: Handle,
        buffer: &mut [u8],
        offset: usize,
    ) -> Result<usize, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let bytes_read = instance.read().read_file(handle, buffer, offset)?;

        Ok(bytes_read)
    }

    async fn write_file(
        &self,
        mount_handle: Handle,
        handle: Handle,
        buffer: &[u8],
        offset: usize,
    ) -> Result<usize, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let bytes_written = instance.write().write_file(handle, buffer, offset)?;

        Ok(bytes_written)
    }

    async fn open_dir(&self, mount_handle: Handle, node_id: NodeId) -> Result<Handle, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let dir_handle = instance.write().open_dir(node_id)?;

        Ok(dir_handle)
    }

    async fn close_dir(&self, mount_handle: Handle, handle: Handle) -> Result<(), Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        instance.write().close_dir(handle)?;

        Ok(())
    }

    async fn list_dir(
        &self,
        mount_handle: Handle,
        handle: Handle,
    ) -> Result<Vec<DirectoryEntry>, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let entries = instance.read().list_dir(handle)?;

        Ok(entries)
    }

    async fn create_symlink(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
        target: &str,
    ) -> Result<NodeId, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let node_id = instance.write().create_symlink(parent, name, target)?;

        Ok(node_id)
    }

    async fn read_symlink(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<String, Self::Error> {
        let instance = self.get_instance(mount_handle)?;

        let target = instance.read().read_symlink(node_id)?;

        Ok(target)
    }

    async fn mount(&self, _args: &[u8]) -> Result<(Handle, NodeId), Self::Error> {
        let instance = Arc::new(RwLock::new(FsInstance::new()));
        let mount_handle = Self::new_handle();
        let root_node_id = instance.read().get_root();

        self.instances.write().insert(mount_handle, instance);

        info!(
            "Mounted new instance with handle {:?} and root node ID {:?}",
            mount_handle, root_node_id
        );

        Ok((mount_handle, root_node_id))
    }

    async fn unmount(&self, mount_handle: Handle) -> Result<(), Self::Error> {
        self.instances
            .write()
            .remove(&mount_handle)
            .ok_or_else(|| {
                error!("Invalid mount handle for unmount: {:?}", mount_handle);
                FsServerError::InvalidArgument
            })?;

        info!("Unmounted instance with handle {:?}", mount_handle);

        Ok(())
    }
}
