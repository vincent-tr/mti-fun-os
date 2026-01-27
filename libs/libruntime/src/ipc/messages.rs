use crate::kobject;

/// Type alias for array of handles extracted from a message.
pub type Handles = [kobject::Handle; kobject::Message::HANDLE_COUNT];

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
