use futures::future::join_all;
use log::error;

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use async_trait::async_trait;
use libruntime::{
    ipc::{self, Handle},
    vfs::{
        iface::{DirectoryEntry, MountInfo, VfsServer, VfsServerError},
        types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions},
    },
};

use crate::{
    lookup::{self, LookupResult},
    mounts::MountTable,
    opened_node::OpenedNode,
    state::State,
    vnode::VNode,
};

/// The main server structure
#[derive(Debug)]
pub struct Server {
    handles: ipc::HandleTable<'static, OpenedNode>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            handles: ipc::HandleTable::new(State::get().handle_generator()),
        }
    }

    async fn open_file(
        &self,
        sender_id: u64,
        vnode: VNode,
        handle_permissions: HandlePermissions,
    ) -> Result<Handle, VfsServerError> {
        let fs_handle = vnode.open_file(handle_permissions).await?;

        let opened_node = Arc::new(
            OpenedNode::new(vnode, NodeType::File, handle_permissions, Some(fs_handle)).await,
        );

        Ok(self.handles.open(sender_id, opened_node))
    }

    async fn open_dir(
        &self,
        sender_id: u64,
        vnode: VNode,
        handle_permissions: HandlePermissions,
    ) -> Result<Handle, VfsServerError> {
        let fs_handle = vnode.open_dir().await?;

        let opened_node = Arc::new(
            OpenedNode::new(
                vnode,
                NodeType::Directory,
                handle_permissions,
                Some(fs_handle),
            )
            .await,
        );

        Ok(self.handles.open(sender_id, opened_node))
    }

    async fn open_symlink(
        &self,
        sender_id: u64,
        vnode: VNode,
        handle_permissions: HandlePermissions,
    ) -> Result<Handle, VfsServerError> {
        let opened_node = Arc::new(
            OpenedNode::new(
                vnode,
                NodeType::Symlink,
                handle_permissions,
                None, // Symlinks are not opened on fs
            )
            .await,
        );

        Ok(self.handles.open(sender_id, opened_node))
    }

    async fn check_open_permissions(
        &self,
        vnode: VNode,
        required_permissions: HandlePermissions,
    ) -> Result<(), VfsServerError> {
        let node_permissions = vnode.permissions().await?;

        if required_permissions.contains(HandlePermissions::READ)
            && !node_permissions.contains(Permissions::READ)
        {
            return Err(VfsServerError::AccessDenied);
        }

        if required_permissions.contains(HandlePermissions::WRITE)
            && !node_permissions.contains(Permissions::WRITE)
        {
            return Err(VfsServerError::AccessDenied);
        }

        Ok(())
    }

    async fn open_node(
        &self,
        sender_id: u64,
        vnode: VNode,
        handle_permissions: HandlePermissions,
    ) -> Result<Handle, VfsServerError> {
        self.check_open_permissions(vnode, handle_permissions)
            .await?;

        let handle = match vnode.r#type().await? {
            NodeType::File => self.open_file(sender_id, vnode, handle_permissions).await?,
            NodeType::Directory => self.open_dir(sender_id, vnode, handle_permissions).await?,
            NodeType::Symlink => {
                self.open_symlink(sender_id, vnode, handle_permissions)
                    .await?
            }
        };

        Ok(handle)
    }

    fn get_opened_node(
        &self,
        sender_id: u64,
        handle: Handle,
    ) -> Result<Arc<OpenedNode>, VfsServerError> {
        self.handles
            .read(sender_id, handle)
            .ok_or(VfsServerError::InvalidArgument)
    }

    fn get_opened_file(
        &self,
        sender_id: u64,
        handle: Handle,
    ) -> Result<Arc<OpenedNode>, VfsServerError> {
        let opened_node = self.get_opened_node(sender_id, handle)?;
        opened_node.check_type(NodeType::File)?;

        Ok(opened_node)
    }

    fn get_opened_dir(
        &self,
        sender_id: u64,
        handle: Handle,
    ) -> Result<Arc<OpenedNode>, VfsServerError> {
        let opened_node = self.get_opened_node(sender_id, handle)?;
        opened_node.check_type(NodeType::Directory)?;

        Ok(opened_node)
    }

    fn get_opened_symlink(
        &self,
        sender_id: u64,
        handle: Handle,
    ) -> Result<Arc<OpenedNode>, VfsServerError> {
        let opened_node = self.get_opened_node(sender_id, handle)?;
        opened_node.check_type(NodeType::Symlink)?;

        Ok(opened_node)
    }

    async fn close_opened_node(&self, opened_node: Arc<OpenedNode>) {
        match opened_node.r#type() {
            NodeType::File => {
                let fs_handle = opened_node
                    .fs_handle()
                    .expect("Opened file without fs handle");
                let file = opened_node.vnode();

                file.close_file(fs_handle).await.unwrap_or_else(|e| {
                    error!("Failed to close opened file: {:?}", e);
                });
            }
            NodeType::Directory => {
                let fs_handle = opened_node
                    .fs_handle()
                    .expect("Opened directory without fs handle");
                let dir = opened_node.vnode();

                dir.close_dir(fs_handle).await.unwrap_or_else(|e| {
                    error!("Failed to close opened directory: {:?}", e);
                });
            }
            NodeType::Symlink => {
                // No special handling needed for symlinks
            }
        }

        opened_node.mark_closed().await;
    }
}

#[async_trait]
impl VfsServer for Server {
    type Error = VfsServerError;

    async fn process_terminated(&self, pid: u64) {
        let opened_nodes = self.handles.process_terminated(pid);

        let futures = opened_nodes
            .into_iter()
            .map(|opened_node| self.close_opened_node(opened_node));

        join_all(futures).await;
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
        match mode {
            OpenMode::OpenExisting => {
                let mut lookup_mode = lookup::LookupMode::Full;
                if no_follow {
                    lookup_mode = lookup::LookupMode::NoFollowLast;
                }

                let LookupResult {
                    node,
                    canonical_path: _,
                    last_segment: _,
                } = lookup::lookup(path, lookup_mode).await?;

                let actual_type = node.r#type().await?;
                if let Some(expected_type) = r#type {
                    if actual_type != expected_type {
                        return Err(VfsServerError::BadType);
                    }
                }

                let handle = self.open_node(sender_id, node, handle_permissions).await?;

                Ok(handle)
            }
            OpenMode::CreateNew => {
                // CreateNew: Create a new file/directory, error if it already exists
                let Some(r#type) = r#type else {
                    // Cannot create without knowing the type
                    return Err(VfsServerError::InvalidArgument);
                };

                if r#type == NodeType::Symlink {
                    // Cannot create symlinks with open()
                    return Err(VfsServerError::InvalidArgument);
                }

                let LookupResult {
                    node: parent_node,
                    canonical_path: _,
                    last_segment,
                } = lookup::lookup(path, lookup::LookupMode::Parent).await?;

                let name = last_segment
                    .as_ref()
                    .expect("Parent mode without last segment");

                let node = parent_node.create(&name, r#type, permissions).await?;

                let handle = self.open_node(sender_id, node, handle_permissions).await?;

                Ok(handle)
            }
            OpenMode::OpenAlways => {
                // OpenAlways: Open if exists, create if doesn't
                let Some(r#type) = r#type else {
                    // Cannot create without knowing the type
                    return Err(VfsServerError::InvalidArgument);
                };

                if r#type == NodeType::Symlink {
                    // Cannot create symlinks with open()
                    return Err(VfsServerError::InvalidArgument);
                }

                // Try to open existing first
                let mut lookup_mode = lookup::LookupMode::Full;
                if no_follow {
                    lookup_mode = lookup::LookupMode::NoFollowLast;
                }

                let lookup_result = lookup::lookup(path, lookup_mode).await;

                match lookup_result {
                    Ok(LookupResult {
                        node,
                        canonical_path: _,
                        last_segment: _,
                    }) => {
                        // Node exists, verify type and open it
                        let node_type = node.r#type().await?;
                        if node_type != r#type {
                            return Err(VfsServerError::BadType);
                        }

                        let handle = self.open_node(sender_id, node, handle_permissions).await?;

                        Ok(handle)
                    }
                    Err(VfsServerError::NotFound) => {
                        // Node doesn't exist, create it
                        let LookupResult {
                            node: parent_node,
                            canonical_path: _,
                            last_segment,
                        } = lookup::lookup(path, lookup::LookupMode::Parent).await?;

                        let name = last_segment
                            .as_ref()
                            .expect("Parent mode without last segment");

                        let node = parent_node.create(&name, r#type, permissions).await?;

                        let handle = self.open_node(sender_id, node, handle_permissions).await?;

                        Ok(handle)
                    }
                    Err(e) => Err(e),
                }
            }
            OpenMode::CreateAlways => {
                // CreateAlways: For files, create new or truncate existing. For directories, error if exists.
                let Some(r#type) = r#type else {
                    // Cannot create without knowing the type
                    return Err(VfsServerError::InvalidArgument);
                };

                if r#type == NodeType::Symlink {
                    // Cannot create symlinks with open()
                    return Err(VfsServerError::InvalidArgument);
                }

                // Try to lookup the node first
                let mut lookup_mode = lookup::LookupMode::Full;
                if no_follow {
                    lookup_mode = lookup::LookupMode::NoFollowLast;
                }

                let lookup_result = lookup::lookup(path, lookup_mode).await;

                match (lookup_result, r#type) {
                    (Ok(LookupResult { node, .. }), NodeType::File) => {
                        // File exists, verify it's a file and truncate it
                        let node_type = node.r#type().await?;
                        if node_type != NodeType::File {
                            return Err(VfsServerError::BadType);
                        }

                        // Truncate the file to size 0
                        node.set_metadata(None, Some(0), None, None).await?;

                        let handle = self.open_node(sender_id, node, handle_permissions).await?;

                        Ok(handle)
                    }
                    (Ok(_), NodeType::Directory) => {
                        // Directory exists, return error (cannot truncate/recreate directories)
                        Err(VfsServerError::AlreadyExists)
                    }
                    (Err(VfsServerError::NotFound), _) => {
                        // Node doesn't exist, create it
                        let LookupResult {
                            node: parent_node,
                            canonical_path: _,
                            last_segment,
                        } = lookup::lookup(path, lookup::LookupMode::Parent).await?;

                        let name = last_segment
                            .as_ref()
                            .expect("Parent mode without last segment");

                        let node = parent_node.create(&name, r#type, permissions).await?;

                        let handle = self.open_node(sender_id, node, handle_permissions).await?;

                        Ok(handle)
                    }
                    (Err(e), _) => Err(e),
                    _ => unreachable!(),
                }
            }
        }
    }

    async fn close(&self, sender_id: u64, handle: Handle) -> Result<(), Self::Error> {
        let Some(opened_node) = self.handles.close(sender_id, handle) else {
            return Err(VfsServerError::InvalidArgument);
        };

        self.close_opened_node(opened_node).await;

        Ok(())
    }

    async fn stat(&self, sender_id: u64, handle: Handle) -> Result<Metadata, Self::Error> {
        let opened_node = self.get_opened_node(sender_id, handle)?;

        opened_node.check_read()?;
        let node = opened_node.vnode();
        let metadata = node.get_metadata().await?;

        Ok(metadata)
    }

    async fn set_permissions(
        &self,
        sender_id: u64,
        handle: Handle,
        permissions: Permissions,
    ) -> Result<(), Self::Error> {
        let opened_node = self.get_opened_node(sender_id, handle)?;

        if opened_node.r#type() == NodeType::Symlink {
            return Err(VfsServerError::BadType);
        }

        opened_node.check_write()?;
        let node = opened_node.vnode();
        node.set_metadata(Some(permissions), None, None, None)
            .await?;

        Ok(())
    }

    async fn read(
        &self,
        sender_id: u64,
        handle: Handle,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let opened_file = self.get_opened_file(sender_id, handle)?;

        opened_file.check_read()?;
        let handle = opened_file
            .fs_handle()
            .expect("Opened file without fs handle");
        let file = opened_file.vnode();

        let byte_read = file.read_file(handle, offset, buffer).await?;

        Ok(byte_read)
    }

    async fn write(
        &self,
        sender_id: u64,
        handle: Handle,
        offset: usize,
        buffer: &[u8],
    ) -> Result<usize, Self::Error> {
        let opened_file = self.get_opened_file(sender_id, handle)?;

        opened_file.check_write()?;
        let handle = opened_file
            .fs_handle()
            .expect("Opened file without fs handle");
        let file = opened_file.vnode();

        let byte_written = file.write_file(handle, offset, buffer).await?;

        Ok(byte_written)
    }

    async fn resize(
        &self,
        sender_id: u64,
        handle: Handle,
        new_size: usize,
    ) -> Result<(), Self::Error> {
        let opened_file = self.get_opened_file(sender_id, handle)?;

        opened_file.check_write()?;
        let file = opened_file.vnode();

        file.set_metadata(None, Some(new_size), None, None).await?;

        Ok(())
    }

    async fn list(
        &self,
        sender_id: u64,
        handle: Handle,
    ) -> Result<Vec<DirectoryEntry>, Self::Error> {
        let opened_dir = self.get_opened_dir(sender_id, handle)?;

        opened_dir.check_read()?;
        let handle = opened_dir
            .fs_handle()
            .expect("Opened directory without fs handle");
        let dir = opened_dir.vnode();

        let entries = dir.list_dir(handle).await?;

        Ok(entries)
    }

    async fn r#move(
        &self,
        sender_id: u64,
        old_dir: Handle,
        old_name: &str,
        new_dir: Handle,
        new_name: &str,
    ) -> Result<(), Self::Error> {
        let opened_old_dir = self.get_opened_dir(sender_id, old_dir)?;
        let opened_new_dir = self.get_opened_dir(sender_id, new_dir)?;

        opened_old_dir.check_write()?;
        opened_new_dir.check_write()?;

        let old_dir = opened_old_dir.vnode();
        let new_dir = opened_new_dir.vnode();

        if old_dir.mount_id() != new_dir.mount_id() {
            error!(
                "Move across mounts is not supported ('{}':{:?} -> '{}':{:?})",
                old_name,
                old_dir.mount_id(),
                new_name,
                new_dir.mount_id()
            );
            return Err(VfsServerError::NotSupported);
        }

        old_dir
            .r#move(old_name, new_dir.node_id(), new_name)
            .await?;

        Ok(())
    }

    async fn remove(&self, sender_id: u64, dir: Handle, name: &str) -> Result<(), Self::Error> {
        let opened_dir = self.get_opened_dir(sender_id, dir)?;

        opened_dir.check_write()?;
        let dir = opened_dir.vnode();

        dir.remove(name).await?;

        Ok(())
    }

    async fn create_symlink(
        &self,
        sender_id: u64,
        path: &str,
        target: &str,
    ) -> Result<Handle, Self::Error> {
        let LookupResult {
            node,
            canonical_path: _,
            last_segment,
        } = lookup::lookup(path, lookup::LookupMode::Parent).await?;

        let name = last_segment.expect("Did not get last segment in parent mode");

        let node = node.create_symlink(&name, target).await?;

        let handle = self
            .open_symlink(
                sender_id,
                node,
                HandlePermissions::READ | HandlePermissions::WRITE,
            )
            .await?;

        Ok(handle)
    }

    async fn read_symlink(&self, sender_id: u64, handle: Handle) -> Result<String, Self::Error> {
        let opened_link = self.get_opened_symlink(sender_id, handle)?;

        opened_link.check_type(NodeType::Symlink)?;
        opened_link.check_read()?;
        let link = opened_link.vnode();
        let target = link.read_symlink().await?;

        Ok(target)
    }

    async fn mount(
        &self,
        _sender_id: u64,
        mount_point: &str,
        fs_port_name: &str,
        args: &[u8],
    ) -> Result<(), Self::Error> {
        let LookupResult {
            node: mount_point,
            canonical_path: path,
            last_segment: _,
        } = lookup::lookup(mount_point, lookup::LookupMode::NoMountpointLast).await?;

        MountTable::get()
            .mount(&mount_point, path, fs_port_name, args)
            .await?;

        Ok(())
    }

    async fn unmount(&self, _sender_id: u64, mount_point: &str) -> Result<(), Self::Error> {
        let LookupResult {
            node: mount_point,
            canonical_path: _,
            last_segment: _,
        } = lookup::lookup(mount_point, lookup::LookupMode::NoMountpointLast).await?;

        MountTable::get().unmount(&mount_point).await?;

        Ok(())
    }

    async fn list_mounts(&self, _sender_id: u64) -> Result<Vec<MountInfo>, Self::Error> {
        Ok(MountTable::get().info().await)
    }
}
