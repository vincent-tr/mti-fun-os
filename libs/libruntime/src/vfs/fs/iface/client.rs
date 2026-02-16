use core::pin::Pin;

use alloc::{boxed::Box, string::String, vec::Vec};

use crate::{
    ipc::{self, Handle},
    kobject::KObject,
    vfs::types::{Metadata, NodeId, NodeType, Permissions},
};

use super::{messages, DentriesBlock, DirectoryEntry, FsServerError};

pub type FsServerCallError = ipc::CallError<FsServerError>;

/// Low level FS client implementation.
#[derive(Debug)]
pub struct Client<'a> {
    _port_name: Pin<Box<str>>,
    ipc_client: ipc::Client<'a>,
}

impl Client<'_> {
    /// Creates a new FS client.
    pub fn new(port: &str) -> Self {
        let port_name: Box<str> = port.into();
        let port_name = Box::into_pin(port_name);

        // Safety: The port_name is owned by this Client and will not be modified or dropped while ipc_client is using it.
        let port_name_ref = unsafe { &*(Pin::get_ref(port_name.as_ref()) as *const str) };

        Self {
            _port_name: port_name,
            ipc_client: ipc::Client::new(port_name_ref, messages::VERSION),
        }
    }

    /// call ipc Lookup
    pub async fn lookup(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
    ) -> Result<NodeId, FsServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::LookupQueryParameters {
            mount_handle,
            parent,
            name: name_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::LookupQueryParameters::HANDLE_NAME_MOBJ] = name_mobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::LookupQueryParameters, messages::LookupReply, FsServerError>(
            messages::Type::Lookup,
            query,
            query_handles,
        )?;

        Ok(reply.node_id)
    }

    /// call ipc Create
    pub async fn create(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
        r#type: NodeType,
        permissions: Permissions,
    ) -> Result<NodeId, FsServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::CreateQueryParameters {
            mount_handle,
            parent,
            name: name_buffer,
            r#type,
            permissions,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::CreateQueryParameters::HANDLE_NAME_MOBJ] = name_mobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::CreateQueryParameters, messages::CreateReply, FsServerError>(
            messages::Type::Create,
            query,
            query_handles,
        )?;

        Ok(reply.node_id)
    }

    /// call ipc Remove
    pub async fn remove(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
    ) -> Result<(), FsServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();

        let query = messages::RemoveQueryParameters {
            mount_handle,
            parent,
            name: name_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::RemoveQueryParameters::HANDLE_NAME_MOBJ] = name_mobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::RemoveQueryParameters, messages::RemoveReply, FsServerError>(
            messages::Type::Remove,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc Move
    pub async fn r#move(
        &self,
        mount_handle: Handle,
        src_parent: NodeId,
        src_name: &str,
        dst_parent: NodeId,
        dst_name: &str,
    ) -> Result<(), FsServerCallError> {
        let (src_name_mobj, src_name_buffer) =
            ipc::Buffer::new_local(src_name.as_bytes()).into_shared();
        let (dst_name_mobj, dst_name_buffer) =
            ipc::Buffer::new_local(dst_name.as_bytes()).into_shared();

        let query = messages::MoveQueryParameters {
            mount_handle,
            src_parent,
            src_name: src_name_buffer,
            dst_parent,
            dst_name: dst_name_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::MoveQueryParameters::HANDLE_SRC_NAME_MOBJ] =
            src_name_mobj.into_handle();
        query_handles[messages::MoveQueryParameters::HANDLE_DST_NAME_MOBJ] =
            dst_name_mobj.into_handle();

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::MoveQueryParameters, messages::MoveReply, FsServerError>(
            messages::Type::Move,
            query,
            query_handles,
        )?;

        Ok(())
    }

    /// call ipc GetMetadata
    pub async fn get_metadata(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<Metadata, FsServerCallError> {
        let query = messages::GetMetadataQueryParameters {
            mount_handle,
            node_id,
        };

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::GetMetadataQueryParameters, messages::GetMetadataReply, FsServerError>(
            messages::Type::GetMetadata,
            query,
            ipc::KHandles::new(),
        )?;

        Ok(reply.metadata)
    }

    /// call ipc SetMetadata
    pub async fn set_metadata(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
        permissions: Option<Permissions>,
        size: Option<usize>,
        created: Option<u64>,
        modified: Option<u64>,
    ) -> Result<(), FsServerCallError> {
        let query = messages::SetMetadataQueryParameters {
            mount_handle,
            node_id,
            permissions,
            size,
            created,
            modified,
        };

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::SetMetadataQueryParameters, messages::SetMetadataReply, FsServerError>(
            messages::Type::SetMetadata,
            query,
            ipc::KHandles::new(),
        )?;

        Ok(())
    }

    /// call ipc OpenFile
    pub async fn open_file(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
        open_permissions: Permissions,
    ) -> Result<Handle, FsServerCallError> {
        let query = messages::OpenFileQueryParameters {
            mount_handle,
            node_id,
            open_permissions,
        };

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::OpenFileQueryParameters, messages::OpenFileReply, FsServerError>(
            messages::Type::OpenFile,
            query,
            ipc::KHandles::new(),
        )?;

        Ok(reply.handle)
    }

    /// call ipc CloseFile
    pub async fn close_file(
        &self,
        mount_handle: Handle,
        handle: Handle,
    ) -> Result<(), FsServerCallError> {
        let query = messages::CloseFileQueryParameters {
            mount_handle,
            handle,
        };

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::CloseFileQueryParameters, messages::CloseFileReply, FsServerError>(
            messages::Type::CloseFile,
            query,
            ipc::KHandles::new(),
        )?;

        Ok(())
    }

    /// call ipc ReadFile
    pub async fn read_file(
        &self,
        mount_handle: Handle,
        handle: Handle,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, FsServerCallError> {
        let (buffer_mobj, buffer_buffer) = ipc::Buffer::new_local(buffer).into_shared();

        let query = messages::ReadFileQueryParameters {
            mount_handle,
            handle,
            offset,
            buffer: buffer_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::ReadFileQueryParameters::HANDLE_BUFFER_MOBJ] =
            buffer_mobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::ReadFileQueryParameters, messages::ReadFileReply, FsServerError>(
            messages::Type::ReadFile,
            query,
            query_handles,
        )?;

        Ok(reply.bytes_read)
    }

    /// call ipc WriteFile
    pub async fn write_file(
        &self,
        mount_handle: Handle,
        handle: Handle,
        offset: usize,
        buffer: &[u8],
    ) -> Result<usize, FsServerCallError> {
        let (buffer_mobj, buffer_buffer) = ipc::Buffer::new_local(buffer).into_shared();

        let query = messages::WriteFileQueryParameters {
            mount_handle,
            handle,
            offset,
            buffer: buffer_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::WriteFileQueryParameters::HANDLE_BUFFER_MOBJ] =
            buffer_mobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::WriteFileQueryParameters, messages::WriteFileReply, FsServerError>(
            messages::Type::WriteFile,
            query,
            query_handles,
        )?;

        Ok(reply.bytes_written)
    }

    /// call ipc OpenDir
    pub async fn open_dir(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<Handle, FsServerCallError> {
        let query = messages::OpenDirQueryParameters {
            mount_handle,
            node_id,
        };

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::OpenDirQueryParameters, messages::OpenDirReply, FsServerError>(
            messages::Type::OpenDir,
            query,
            ipc::KHandles::new(),
        )?;

        Ok(reply.handle)
    }

    /// call ipc CloseDir
    pub async fn close_dir(
        &self,
        mount_handle: Handle,
        handle: Handle,
    ) -> Result<(), FsServerCallError> {
        let query = messages::CloseDirQueryParameters {
            mount_handle,
            handle,
        };

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::CloseDirQueryParameters, messages::CloseDirReply, FsServerError>(
            messages::Type::CloseDir,
            query,
            ipc::KHandles::new(),
        )?;

        Ok(())
    }

    /// call ipc ListDir
    pub async fn list_dir(
        &self,
        mount_handle: Handle,
        handle: Handle,
    ) -> Result<Vec<DirectoryEntry>, FsServerCallError> {
        // We don't know how many entries there are, so we start with a small buffer and grow it until it's big enough.
        let mut size = 256;

        let allocated_buffer = loop {
            let mut allocated_buffer = {
                let mut vec = Vec::with_capacity(size);
                unsafe { vec.set_len(size) };
                vec
            };

            let (buffer_mobj, buffer) = ipc::Buffer::new_local(&allocated_buffer).into_shared();

            let query = messages::ListDirQueryParameters {
                mount_handle,
                handle,
                buffer,
            };

            let mut query_handles = ipc::KHandles::new();
            query_handles[messages::ListDirQueryParameters::HANDLE_BUFFER_MOBJ] =
                buffer_mobj.into_handle();

            let res = self.ipc_client.call::<messages::Type, messages::ListDirQueryParameters, messages::ListDirReply, messages::FsServerError>(
                messages::Type::ListDir,
                query,
                query_handles,
            );

            if let Err(ipc::CallError::ReplyError(messages::FsServerError::BufferTooSmall)) = res {
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

    /// call ipc CreateSymlink
    pub async fn create_symlink(
        &self,
        mount_handle: Handle,
        parent: NodeId,
        name: &str,
        target: &str,
    ) -> Result<NodeId, FsServerCallError> {
        let (name_mobj, name_buffer) = ipc::Buffer::new_local(name.as_bytes()).into_shared();
        let (target_mobj, target_buffer) = ipc::Buffer::new_local(target.as_bytes()).into_shared();

        let query = messages::CreateSymlinkQueryParameters {
            mount_handle,
            parent,
            name: name_buffer,
            target: target_buffer,
        };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::CreateSymlinkQueryParameters::HANDLE_NAME_MOBJ] =
            name_mobj.into_handle();
        query_handles[messages::CreateSymlinkQueryParameters::HANDLE_TARGET_MOBJ] =
            target_mobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::CreateSymlinkQueryParameters, messages::CreateSymlinkReply, FsServerError>(
            messages::Type::CreateSymlink,
            query,
            query_handles,
        )?;

        Ok(reply.node_id)
    }

    /// call ipc ReadSymlink
    pub async fn read_symlink(
        &self,
        mount_handle: Handle,
        node_id: NodeId,
    ) -> Result<String, FsServerCallError> {
        // We don't know how long the target path is, so we start with a small buffer and grow it until it's big enough.
        let mut size = 64;

        let allocated_target = loop {
            let mut allocated_target = {
                let mut vec = Vec::with_capacity(size);
                unsafe { vec.set_len(size) };
                vec
            };

            let (target_mobj, target) = ipc::Buffer::new_local(&allocated_target).into_shared();

            let query = messages::ReadSymlinkQueryParameters {
                mount_handle,
                node_id,
                target,
            };

            let mut query_handles = ipc::KHandles::new();
            query_handles[messages::ReadSymlinkQueryParameters::HANDLE_TARGET_MOBJ] =
                target_mobj.into_handle();

            let res = self.ipc_client.call::<messages::Type, messages::ReadSymlinkQueryParameters, messages::ReadSymlinkReply, messages::FsServerError>(
                messages::Type::ReadSymlink,
                query,
                query_handles,
            );

            if let Err(ipc::CallError::ReplyError(messages::FsServerError::BufferTooSmall)) = res {
                size *= 2;
                continue;
            }

            let (reply, _reply_handles) = res?;

            unsafe { allocated_target.set_len(reply.target_len) };
            break allocated_target;
        };

        let target = unsafe { String::from_utf8_unchecked(allocated_target) };

        Ok(target)
    }

    /// call ipc Mount
    pub async fn mount(&self, args: &[u8]) -> Result<(Handle, NodeId), FsServerCallError> {
        let (args_mobj, args_buffer) = ipc::Buffer::new_local(args).into_shared();

        let query = messages::MountQueryParameters { args: args_buffer };

        let mut query_handles = ipc::KHandles::new();
        query_handles[messages::MountQueryParameters::HANDLE_ARGS_MOBJ] = args_mobj.into_handle();

        let (reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::MountQueryParameters, messages::MountReply, FsServerError>(
            messages::Type::Mount,
            query,
            query_handles,
        )?;

        Ok((reply.mount_handle, reply.root_node_id))
    }

    /// call ipc Unmount
    pub async fn unmount(&self, mount_handle: Handle) -> Result<(), FsServerCallError> {
        let query = messages::UnmountQueryParameters { mount_handle };

        let (_reply, _reply_handles) = self.ipc_client.call::<messages::Type, messages::UnmountQueryParameters, messages::UnmountReply, FsServerError>(
                messages::Type::Unmount,
                query,
                ipc::KHandles::new(),
            )?;

        Ok(())
    }
}
