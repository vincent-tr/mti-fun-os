use core::fmt;

use super::messages::{
    KHandles, QueryHeader, QueryMessage, ReplyErrorMessage, ReplyHeader, ReplySuccessMessage,
};
use crate::kobject::{self, KObject};

/// IPC client error.
#[derive(Debug)]
pub enum CallError<ReplyError: fmt::Display + fmt::Debug> {
    KernelError(kobject::Error),
    ReplyError(ReplyError),
}

impl<ReplyError: fmt::Display + fmt::Debug> From<kobject::Error> for CallError<ReplyError> {
    fn from(err: kobject::Error) -> Self {
        CallError::KernelError(err)
    }
}

impl<ReplyError: fmt::Display + fmt::Debug> fmt::Display for CallError<ReplyError> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallError::KernelError(err) => write!(f, "KernelError: {}", err),
            CallError::ReplyError(err) => write!(f, "ReplyError: {}", err),
        }
    }
}

impl<ReplyError: fmt::Display + fmt::Debug> core::error::Error for CallError<ReplyError> {}

/// IPC client.
#[derive(Debug)]
pub struct Client {
    name: &'static str,
    version: u16,
}

impl Client {
    /// Creates a new IPC client connected to the server with the given name and version.
    pub fn new(name: &'static str, version: u16) -> Self {
        Self { name, version }
    }

    /// Calls a message on the server and waits for a reply.
    pub fn call<MessageType, QueryParameters, ReplyContent, ReplyError>(
        &self,
        message_type: MessageType,
        query: QueryParameters,
        mut query_handles: KHandles, // Note: first handle will be overwritten for reply port
    ) -> Result<(ReplyContent, KHandles), CallError<ReplyError>>
    where
        MessageType: Into<u16>,
        QueryParameters: Copy,
        ReplyContent: Copy,
        ReplyError: Copy + fmt::Display + fmt::Debug,
    {
        let port = kobject::Port::open_by_name(self.name)?;
        let (reply_reader, reply_sender) = kobject::Port::create(None)?;

        query_handles[0] = reply_sender.into_handle();

        let data = QueryMessage::<QueryParameters> {
            header: QueryHeader {
                version: self.version,
                r#type: message_type.into(),
                transaction: 0, // One port per reply, no need for transaction ID
                sender_pid: kobject::Process::current().pid(),
            },
            parameters: query,
        };

        let mut message = unsafe { kobject::Message::new(&data, query_handles.into()) };

        port.send(&mut message)?;

        // TODO: add a timeout, if server dies during the query we will block forever here
        let mut reply = reply_reader.blocking_receive()?;

        let header = unsafe { reply.data::<ReplyHeader>() };

        if header.success {
            let handles = reply.take_all_handles();
            let reply_message = unsafe { reply.data::<ReplySuccessMessage<ReplyContent>>() };
            Ok((reply_message.content, handles.into()))
        } else {
            let error_message = unsafe { reply.data::<ReplyErrorMessage<ReplyError>>() };
            Err(CallError::ReplyError(error_message.error))
        }
    }

    /// Emits a message to the server without waiting for a reply.
    pub fn emit<MessageType, QueryParameters>(
        &self,
        message_type: MessageType,
        query: QueryParameters,
        query_handles: KHandles,
    ) -> Result<(), kobject::Error>
    where
        MessageType: Into<u16>,
        QueryParameters: Copy,
    {
        let port = kobject::Port::open_by_name(self.name)?;

        let data = QueryMessage::<QueryParameters> {
            header: QueryHeader {
                version: self.version,
                r#type: message_type.into(),
                transaction: 0, // No transaction for emit
                sender_pid: kobject::Process::current().pid(),
            },
            parameters: query,
        };

        let mut message = unsafe { kobject::Message::new(&data, query_handles.into()) };

        port.send(&mut message)?;

        Ok(())
    }
}
