use alloc::string::String;
use alloc::vec::Vec;

use super::iface::{Client, DirectoryEntry, VfsServerCallError};
use super::types::{HandlePermissions, NodeType, OpenMode, Permissions};

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
    fn r#type(&self) -> NodeType;
}

/// Represents an opened file
#[derive(Debug)]
pub struct File {
    handle: VfsHandle,
}

impl VfsObject for File {
    fn r#type(&self) -> NodeType {
        NodeType::File
    }
}

impl File {
    /// Opens a file at the given path with the specified mode and permissions.
    pub fn open(
        path: &str,
        mode: OpenMode,
        perms: HandlePermissions,
    ) -> Result<Self, VfsServerCallError> {
        let handle = CLIENT.open(
            path,
            Some(NodeType::File),
            mode,
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
        CLIENT.read(self.handle.value(), offset, buffer)
    }

    /// Writes data to the file at the given offset from the provided buffer.
    pub fn write(&self, offset: usize, buffer: &[u8]) -> Result<usize, VfsServerCallError> {
        CLIENT.write(self.handle.value(), offset, buffer)
    }

    /// Resizes the file to the new size.
    pub fn resisze(&self, new_size: usize) -> Result<(), VfsServerCallError> {
        CLIENT.resize(self.handle.value(), new_size)
    }
}

/// Represents an opened directory
#[derive(Debug)]
pub struct Directory {
    handle: VfsHandle,
}

impl VfsObject for Directory {
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
            OpenMode::CreateNew,
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
        CLIENT.list(self.handle.value())
    }
}

/// Represents an opened symbolic link
#[derive(Debug)]
pub struct Symlink {
    handle: VfsHandle,
}

impl VfsObject for Symlink {
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
        CLIENT.read_symlink(self.handle.value())
    }
}
