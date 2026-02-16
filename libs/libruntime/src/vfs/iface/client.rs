use alloc::{string::String, vec::Vec};

use super::{messages, DirectoryEntry, MountInfo};
use crate::{
    ipc::{self, Handle},
    kobject::KObject,
    vfs::{
        iface::{dentries_block::DentriesBlock, mounts_block::MountsBlock},
        types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions},
    },
};

pub type VfsServerCallError = ipc::CallError<messages::VfsServerError>;

/// Low level VFS client implementation.
#[derive(Debug)]
pub struct Client {
    ipc_client: ipc::Client,
}

impl Client {
    /// Creates a new VFS client.
    pub fn new() -> Self {
        Self {
            ipc_client: ipc::Client::new(messages::PORT_NAME, messages::VERSION),
        }
    }

    /// call ipc Open
    pub fn open(
        &self,
        path: &str,
        r#type: Option<NodeType>,
        mode: OpenMode,
        no_follow: bool,
        permissions: Permissions,
        handle_permissions: HandlePermissions,
    ) -> Result<Handle, VfsServerCallError> {
        let (path_memobj, path_buffer) = ipc::Buffer::new_local(path.as_bytes()).into_shared();

        let query = messages::OpenQueryParameters {
            path: path_buffer,
            r#type,
            mode,
            no_follow,
            permissions,
            handle_permissions,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::OpenQueryParameters::HANDLE_PATH_MOBJ] = path_memobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::OpenQueryParameters, messages::OpenReply, messages::VfsServerError>(
            messages::Type::Open,
            query,
            query_handles,
        )?;

        Ok(reply.handle)
    }

    /// call ipc Close
    pub fn close(&self, handle: Handle) -> Result<(), VfsServerCallError> {
        let query = messages::CloseQueryParameters { handle };

        self.ipc_client.call::<messages::Type, messages::CloseQueryParameters, messages::CloseReply, messages::VfsServerError>(
            messages::Type::Close,
            query,
            ipc::KHandles::new(),
        )?;

        Ok(())
    }

    /// call ipc Stat
    pub fn stat(&self, handle: Handle) -> Result<Metadata, VfsServerCallError> {
        let query = messages::StatQueryParameters { handle };
        let query_handles = ipc::KHandles::new();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::StatQueryParameters, messages::StatReply, messages::VfsServerError>(
            messages::Type::Stat,
            query,
            query_handles,
        )?;

        Ok(reply.metadata)
    }

    /// call ipc SetPermissions
    pub fn set_permissions(
        &self,
        handle: Handle,
        permissions: Permissions,
    ) -> Result<(), VfsServerCallError> {
        let query = messages::SetPermissionsQueryParameters {
            handle,
            permissions,
        };
        let query_handles = ipc::KHandles::new();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::SetPermissionsQueryParameters, messages::SetPermissionsReply, messages::VfsServerError>(
            messages::Type::SetPermissions,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc Read
    pub fn read(
        &self,
        handle: Handle,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<u64, VfsServerCallError> {
        let (buffer_memobj, buffer_buffer) = ipc::Buffer::new_local(buffer).into_shared();

        let query = messages::ReadQueryParameters {
            handle,
            offset,
            buffer: buffer_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::ReadQueryParameters::HANDLE_BUFFER_MOBJ] =
            buffer_memobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::ReadQueryParameters, messages::ReadReply, messages::VfsServerError>(
            messages::Type::Read,
            query,
            query_handles,
        )?;

        Ok(reply.bytes_read)
    }

    /// call ipc Write
    pub fn write(
        &self,
        handle: Handle,
        offset: u64,
        buffer: &[u8],
    ) -> Result<u64, VfsServerCallError> {
        let (buffer_memobj, buffer_buffer) = ipc::Buffer::new_local(buffer).into_shared();

        let query = messages::WriteQueryParameters {
            handle,
            offset,
            buffer: buffer_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::WriteQueryParameters::HANDLE_BUFFER_MOBJ] =
            buffer_memobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::WriteQueryParameters, messages::WriteReply, messages::VfsServerError>(
            messages::Type::Write,
            query,
            query_handles,
        )?;

        Ok(reply.bytes_written)
    }

    /// call ipc Resize
    pub fn resize(&self, handle: Handle, new_size: u64) -> Result<(), VfsServerCallError> {
        let query = messages::ResizeQueryParameters { handle, new_size };
        let query_handles = ipc::KHandles::new();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::ResizeQueryParameters, messages::ResizeReply, messages::VfsServerError>(
            messages::Type::Resize,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc List
    pub fn list(&self, handle: Handle) -> Result<Vec<DirectoryEntry>, VfsServerCallError> {
        // We don't know how many entries there are, so we start with a small buffer and grow it until it's big enough.
        let mut size = 256;

        let allocated_buffer = loop {
            let mut allocated_buffer = {
                let mut vec = Vec::with_capacity(size);
                unsafe { vec.set_len(size) };
                vec
            };

            let (buffer_mobj, buffer) = ipc::Buffer::new_local(&allocated_buffer).into_shared();

            let query = messages::ListQueryParameters { handle, buffer };

            let mut query_handles = ipc::KHandles::new();
            query_handles[messages::ListQueryParameters::HANDLE_BUFFER_MOBJ] =
                buffer_mobj.into_handle();

            let res = self.ipc_client.call::<messages::Type, messages::ListQueryParameters, messages::ListReply, messages::VfsServerError>(
                messages::Type::List,
                query,
                query_handles,
            );

            if let Err(ipc::CallError::ReplyError(messages::VfsServerError::BufferTooSmall)) = res {
                size *= 2;
                continue;
            }

            let (reply, _reply_handles) = res?;

            unsafe { allocated_buffer.set_len(reply.buffer_used_len) };
            break allocated_buffer;
        };

        let entries = DentriesBlock::read(&allocated_buffer)
            .expect("Failed to read dentries block from buffer");

        Ok(entries)
    }

    /// call ipc Move
    pub fn r#move(
        &self,
        old_dir: Handle,
        old_name: &str,
        new_dir: Handle,
        new_name: &str,
    ) -> Result<(), VfsServerCallError> {
        let (old_name_memobj, old_name_buffer) =
            ipc::Buffer::new_local(old_name.as_bytes()).into_shared();
        let (new_name_memobj, new_name_buffer) =
            ipc::Buffer::new_local(new_name.as_bytes()).into_shared();

        let query = messages::MoveQueryParameters {
            old_dir,
            old_name: old_name_buffer,
            new_dir,
            new_name: new_name_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::MoveQueryParameters::HANDLE_OLD_NAME_MOBJ] =
            old_name_memobj.into_handle();
        query_handles[messages::MoveQueryParameters::HANDLE_NEW_NAME_MOBJ] =
            new_name_memobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::MoveQueryParameters, messages::MoveReply, messages::VfsServerError>(
            messages::Type::Move,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc Remove
    pub fn remove(&self, dir: Handle, name: &str) -> Result<(), VfsServerCallError> {
        let (name_memobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::RemoveQueryParameters {
            dir,
            name: name_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::RemoveQueryParameters::HANDLE_NAME_MOBJ] =
            name_memobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::RemoveQueryParameters, messages::RemoveReply, messages::VfsServerError>(
            messages::Type::Remove,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc CreateSymlink
    pub fn create_symlink(&self, path: &str, target: &str) -> Result<(), VfsServerCallError> {
        let (path_memobj, path_buffer) = ipc::Buffer::new_local(path.as_bytes()).into_shared();
        let (target_memobj, target_buffer) =
            ipc::Buffer::new_local(target.as_bytes()).into_shared();

        let query = messages::CreateSymlinkQueryParameters {
            path: path_buffer,
            target: target_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::CreateSymlinkQueryParameters::HANDLE_PATH_MOBJ] =
            path_memobj.into_handle();
        query_handles[messages::CreateSymlinkQueryParameters::HANDLE_TARGET_MOBJ] =
            target_memobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::CreateSymlinkQueryParameters, messages::CreateSymlinkReply, messages::VfsServerError>(
            messages::Type::CreateSymlink,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc ReadSymlink
    pub fn read_symlink(&self, handle: Handle) -> Result<String, VfsServerCallError> {
        // We don't know how long the target path is, so we start with a small buffer and grow it until it's big enough.
        let mut size = 64;

        let allocated_buffer = loop {
            let mut allocated_buffer = {
                let mut vec = Vec::with_capacity(size);
                unsafe { vec.set_len(size) };
                vec
            };

            let (buffer_mobj, buffer) = ipc::Buffer::new_local(&allocated_buffer).into_shared();

            let query = messages::ReadSymlinkQueryParameters { handle, buffer };

            let mut query_handles = ipc::KHandles::new();
            query_handles[messages::ReadSymlinkQueryParameters::HANDLE_BUFFER_MOBJ] =
                buffer_mobj.into_handle();

            let res = self.ipc_client.call::<messages::Type, messages::ReadSymlinkQueryParameters, messages::ReadSymlinkReply, messages::VfsServerError>(
                messages::Type::ReadSymlink,
                query,
                query_handles,
            );

            if let Err(ipc::CallError::ReplyError(messages::VfsServerError::BufferTooSmall)) = res {
                size *= 2;
                continue;
            }

            let (reply, _reply_handles) = res?;

            unsafe { allocated_buffer.set_len(reply.target_length) };
            break allocated_buffer;
        };

        let target = unsafe { String::from_utf8_unchecked(allocated_buffer) };

        Ok(target)
    }

    /// call ipc Mount
    pub fn mount(
        &self,
        mount_point: &str,
        fs_port_name: &str,
        args: &[u8],
    ) -> Result<(), VfsServerCallError> {
        let (mount_point_memobj, mount_point_buffer) =
            ipc::Buffer::new_local(mount_point.as_bytes()).into_shared();
        let (fs_port_name_memobj, fs_port_name_buffer) =
            ipc::Buffer::new_local(fs_port_name.as_bytes()).into_shared();
        let (args_memobj, args_buffer) = ipc::Buffer::new_local(args).into_shared();

        let query = messages::MountQueryParameters {
            mount_point: mount_point_buffer,
            fs_port_name: fs_port_name_buffer,
            args: args_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::MountQueryParameters::HANDLE_MOUNT_POINT_MOBJ] =
            mount_point_memobj.into_handle();
        query_handles[messages::MountQueryParameters::HANDLE_FS_PORT_NAME_MOBJ] =
            fs_port_name_memobj.into_handle();
        query_handles[messages::MountQueryParameters::HANDLE_ARGS_MOBJ] = args_memobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::MountQueryParameters, messages::MountReply, messages::VfsServerError>(
            messages::Type::Mount,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc Unmount
    pub fn unmount(&self, mount_point: &str) -> Result<(), VfsServerCallError> {
        let (mount_point_memobj, mount_point_buffer) =
            ipc::Buffer::new_local(mount_point.as_bytes()).into_shared();

        let query = messages::UnmountQueryParameters {
            mount_point: mount_point_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::UnmountQueryParameters::HANDLE_MOUNT_POINT_MOBJ] =
            mount_point_memobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::UnmountQueryParameters, messages::UnmountReply, messages::VfsServerError>(
            messages::Type::Unmount,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc ListMounts
    pub fn list_mounts(&self) -> Result<Vec<MountInfo>, VfsServerCallError> {
        // We don't know how many mounts there are, so we start with a small buffer and grow it until it's big enough.
        let mut size = 256;

        let allocated_buffer = loop {
            let mut allocated_buffer = {
                let mut vec = Vec::with_capacity(size);
                unsafe { vec.set_len(size) };
                vec
            };

            let (buffer_mobj, buffer) = ipc::Buffer::new_local(&allocated_buffer).into_shared();

            let query = messages::ListMountsQueryParameters { buffer };

            let mut query_handles = ipc::KHandles::new();
            query_handles[messages::ListMountsQueryParameters::HANDLE_BUFFER_MOBJ] =
                buffer_mobj.into_handle();

            let res = self.ipc_client.call::<messages::Type, messages::ListMountsQueryParameters, messages::ListMountsReply, messages::VfsServerError>(
                messages::Type::ListMounts,
                query,
                query_handles,
            );

            if let Err(ipc::CallError::ReplyError(messages::VfsServerError::BufferTooSmall)) = res {
                size *= 2;
                continue;
            }

            let (reply, _reply_handles) = res?;

            unsafe { allocated_buffer.set_len(reply.buffer_used_len) };
            break allocated_buffer;
        };

        let mounts =
            MountsBlock::read(&allocated_buffer).expect("Failed to read mounts list from buffer");

        Ok(mounts)
    }
}
