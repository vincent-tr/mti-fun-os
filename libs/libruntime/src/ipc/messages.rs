use core::{
    mem,
    ops::{Index, IndexMut},
};

use crate::kobject;

/// Array of IPC message kernel handles.
#[derive(Debug)]
pub struct KHandles([kobject::Handle; kobject::Message::HANDLE_COUNT]);

impl KHandles {
    /// Creates a new KHandles array initialized with invalid handles.
    pub const fn new() -> Self {
        Self([const { kobject::Handle::invalid() }; kobject::Message::HANDLE_COUNT])
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub fn take(&mut self, index: usize) -> kobject::Handle {
        mem::replace(&mut self.0[index], kobject::Handle::invalid())
    }
}

impl Index<usize> for KHandles {
    type Output = kobject::Handle;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for KHandles {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Into<[kobject::Handle; kobject::Message::HANDLE_COUNT]> for KHandles {
    fn into(self) -> [kobject::Handle; kobject::Message::HANDLE_COUNT] {
        self.0
    }
}

impl Into<KHandles> for [kobject::Handle; kobject::Message::HANDLE_COUNT] {
    fn into(self) -> KHandles {
        KHandles(self)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct QueryHeader {
    pub version: u16,
    pub r#type: u16,
    pub transaction: u32,
    pub sender_pid: u64,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct QueryMessage<QueryParameters> {
    pub header: QueryHeader,
    pub parameters: QueryParameters,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReplyHeader {
    pub transaction: u32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReplySuccessMessage<Content> {
    pub header: ReplyHeader,
    pub content: Content,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReplyErrorMessage<Error> {
    pub header: ReplyHeader,
    pub error: Error,
}
