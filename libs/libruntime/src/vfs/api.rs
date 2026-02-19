use alloc::string::String;
use alloc::vec::Vec;

use super::iface::{Client, DirectoryEntry, MountInfo, VfsServerCallError, VfsServerError};
use super::types::{HandlePermissions, Metadata, NodeType, OpenMode, Permissions};

use crate::ipc::Handle;

lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::new();
}

/// A handle to a VFS object.
#[derive(Debug)]
struct VfsHandle {
    handle: Handle,
}

impl VfsHandle {
    /// Returns the raw handle value for IPC communication.
    pub fn value(&self) -> Handle {
        self.handle
    }
}

impl From<Handle> for VfsHandle {
    fn from(handle: Handle) -> Self {
        Self { handle }
    }
}

impl Drop for VfsHandle {
    fn drop(&mut self) {
        CLIENT
            .close(self.handle)
            .expect("Failed to close vfs handle");
    }
}

/// Trait representing a VFS object (file, directory, or symlink).
pub trait VfsObject {
    /// Returns a reference to the underlying VFS handle for IPC communication.
    fn handle(&self) -> Handle;

    /// Returns the type of the VFS object (file, directory, or symlink).
    fn r#type(&self) -> NodeType;

    /// Gets the metadata of the object, including type, permissions, size, and timestamps.
    fn stat(&self) -> Result<Metadata, VfsServerCallError> {
        CLIENT.stat(self.handle())
    }

    /// Sets the permissions of the object.
    fn set_permissions(&self, perms: Permissions) -> Result<(), VfsServerCallError> {
        CLIENT.set_permissions(self.handle(), perms)
    }
}

/// Represents an opened file
#[derive(Debug)]
pub struct File {
    handle: VfsHandle,
}

impl VfsObject for File {
    fn handle(&self) -> Handle {
        self.handle.value()
    }

    fn r#type(&self) -> NodeType {
        NodeType::File
    }
}

impl File {
    /// Opens a file at the given path with the specified mode and permissions.
    pub fn open(
        path: &str,
        perms: HandlePermissions,
    ) -> Result<Self, VfsServerCallError> {
        let handle = CLIENT.open(
            path,
            Some(NodeType::File),
            OpenMode::OpenExisting,
            false,
            Permissions::NONE,
            perms,
        )?;

        Ok(Self {
            handle: VfsHandle::from(handle),
        })
    }

    /// Creates a new file at the given path with the specified permissions.
    pub fn create(path: &str, perms: Permissions) -> Result<Self, VfsServerCallError> {
        let handle = CLIENT.open(
            path,
            Some(NodeType::File),
            OpenMode::CreateNew,
            false,
            perms,
            HandlePermissions::READ | HandlePermissions::WRITE,
        )?;

        Ok(Self {
            handle: VfsHandle::from(handle),
        })
    }

    /// Reads data from the file at the given offset into the provided buffer.
    pub fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, VfsServerCallError> {
        CLIENT.read(self.handle(), offset, buffer)
    }

    /// Writes data to the file at the given offset from the provided buffer.
    pub fn write(&self, offset: usize, buffer: &[u8]) -> Result<usize, VfsServerCallError> {
        CLIENT.write(self.handle(), offset, buffer)
    }

    /// Resizes the file to the new size.
    pub fn resisze(&self, new_size: usize) -> Result<(), VfsServerCallError> {
        CLIENT.resize(self.handle(), new_size)
    }
}

/// Represents an opened directory
#[derive(Debug)]
pub struct Directory {
    handle: VfsHandle,
}

impl VfsObject for Directory {
    fn handle(&self) -> Handle {
        self.handle.value()
    }

    fn r#type(&self) -> NodeType {
        NodeType::Directory
    }
}

impl Directory {
    /// Opens a directory at the given path.
    pub fn open(path: &str) -> Result<Self, VfsServerCallError> {
        let handle = CLIENT.open(
            path,
            Some(NodeType::Directory),
            OpenMode::OpenExisting,
            false,
            Permissions::NONE,
            HandlePermissions::READ,
        )?;

        Ok(Self {
            handle: VfsHandle::from(handle),
        })
    }

    /// Creates a new directory at the given path.
    pub fn create(path: &str, perms: Permissions) -> Result<Self, VfsServerCallError> {
        let handle = CLIENT.open(
            path,
            Some(NodeType::Directory),
            OpenMode::CreateNew,
            false,
            perms,
            HandlePermissions::READ,
        )?;

        Ok(Self {
            handle: VfsHandle::from(handle),
        })
    }

    /// Lists the entries in the directory, returning a vector of DirectoryEntry.
    pub fn list(&self) -> Result<Vec<DirectoryEntry>, VfsServerCallError> {
        CLIENT.list(self.handle())
    }

    /// Remove a file or directory with the given name from this directory.
    pub fn remove(&self, name: &str) -> Result<(), VfsServerCallError> {
        CLIENT.remove(self.handle(), name)
    }
}

/// Represents an opened symbolic link
#[derive(Debug)]
pub struct Symlink {
    handle: VfsHandle,
}

impl VfsObject for Symlink {
    fn handle(&self) -> Handle {
        self.handle.value()
    }

    fn r#type(&self) -> NodeType {
        NodeType::Symlink
    }
}

impl Symlink {
    /// Opens a symbolic link at the given path.
    pub fn open(path: &str) -> Result<Self, VfsServerCallError> {
        let handle = CLIENT.open(
            path,
            Some(NodeType::Symlink),
            OpenMode::OpenExisting,
            true, // else we cannot target the symlink itself
            Permissions::NONE,
            HandlePermissions::READ,
        )?;

        Ok(Self {
            handle: VfsHandle::from(handle),
        })
    }

    /// Creates a new symbolic link at the given path pointing to the target path.
    pub fn create(path: &str, target: &str) -> Result<(), VfsServerCallError> {
        CLIENT.create_symlink(path, target)
    }

    /// Reads the target path of the symlink into the provided buffer.
    pub fn target(&self) -> Result<String, VfsServerCallError> {
        CLIENT.read_symlink(self.handle())
    }
}

/// Mounts a filesystem.
pub fn mount(mount_point: &str, fs_port_name: &str, args: &[u8]) -> Result<(), VfsServerCallError> {
    CLIENT.mount(mount_point, fs_port_name, args)
}

/// Unmounts a filesystem.
pub fn unmount(mount_point: &str) -> Result<(), VfsServerCallError> {
    CLIENT.unmount(mount_point)
}

/// Gets the list of mounted filesystems.
pub fn list_mounts() -> Result<Vec<MountInfo>, VfsServerCallError> {
    CLIENT.list_mounts()
}

/// Gets the metadata of the object at the given path.
pub fn stat(path: &str) -> Result<Metadata, VfsServerCallError> {
    let handle = VfsHandle::from(CLIENT.open(
        path,
        None,
        OpenMode::OpenExisting,
        false,
        Permissions::NONE,
        HandlePermissions::READ,
    )?);

    CLIENT.stat(handle.value())
}

/// Moves a object from the source path to the destination path.
pub fn r#move(src: &str, dst: &str) -> Result<(), VfsServerCallError> {
    let (src_parent, src_name) = split_path(src)?;
    let (dst_parent, dst_name) = split_path(dst)?;

    let src_parent = Directory::open(src_parent)?;
    let dst_parent = Directory::open(dst_parent)?;

    CLIENT.r#move(
        src_parent.handle.value(),
        src_name,
        dst_parent.handle.value(),
        dst_name,
    )
}

/// Removes the object at the given path.
pub fn remove(path: &str) -> Result<(), VfsServerCallError> {
    let (parent, name) = split_path(path)?;
    let parent = Directory::open(parent)?;

    CLIENT.remove(parent.handle.value(), name)
}

fn split_path(path: &str) -> Result<(&str, &str), VfsServerCallError> {
    let Some(pos) = path.rfind('/') else {
        return Err(VfsServerCallError::ReplyError(
            VfsServerError::InvalidArgument,
        ));
    };

    let mut parent = &path[..pos];
    let name = &path[pos + 1..];

    if parent == "" {
        parent = "/";
    }

    if name == "" || name == "." || name == ".." {
        return Err(VfsServerCallError::ReplyError(
            VfsServerError::InvalidArgument,
        ));
    }

    Ok((parent, name))
}

// TODO: better error wrapping
