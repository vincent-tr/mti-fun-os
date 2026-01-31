use core::{
    mem,
    ops::{Index, IndexMut},
};

use crate::kobject;

/// Array of IPC message handles.
#[derive(Debug)]
pub struct Handles([kobject::Handle; kobject::Message::HANDLE_COUNT]);

impl Handles {
    /// Creates a new Handles array initialized with invalid handles.
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

impl Index<usize> for Handles {
    type Output = kobject::Handle;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Handles {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Into<[kobject::Handle; kobject::Message::HANDLE_COUNT]> for Handles {
    fn into(self) -> [kobject::Handle; kobject::Message::HANDLE_COUNT] {
        self.0
    }
}

impl Into<Handles> for [kobject::Handle; kobject::Message::HANDLE_COUNT] {
    fn into(self) -> Handles {
        Handles(self)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct QueryHeader {
    pub version: u16,
    pub r#type: u16,
    pub transaction: u32,
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
