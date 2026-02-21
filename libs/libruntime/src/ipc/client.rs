use core::fmt;

use super::messages::{
    KHandles, QueryHeader, QueryMessage, ReplyErrorMessage, ReplyHeader, ReplySuccessMessage,
};
use crate::{
    r#async,
    kobject::{self, KObject},
};

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
pub struct Client<'a> {
    name: &'a str,
    version: u16,
}

impl<'a> Client<'a> {
    /// Creates a new IPC client connected to the server with the given name and version.
    pub fn new(name: &'a str, version: u16) -> Self {
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
        let (reply_reader, reply_sender) = kobject::Port::create(None)?;
        query_handles[0] = reply_sender.into_handle();

        self.send_query(message_type, query, query_handles)?;

        let reply = reply_reader.blocking_receive()?;

        self.process_reply(reply)
    }

    /// Asynchronously calls a message on the server and waits for a reply.
    pub async fn async_call<MessageType, QueryParameters, ReplyContent, ReplyError>(
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
        let (reply_reader, reply_sender) = kobject::Port::create(None)?;
        query_handles[0] = reply_sender.into_handle();

        self.send_query(message_type, query, query_handles)?;

        let reply = loop {
            r#async::wait(&reply_reader).await;

            let res = reply_reader.receive();

            if let Err(kobject::Error::ObjectNotReady) = res {
                continue;
                // Not ready yet, wait again
            }

            break res;
        }?;

        self.process_reply(reply)
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
        self.send_query(message_type, query, query_handles)
    }

    fn send_query<MessageType, QueryParameters>(
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

    fn process_reply<ReplyContent, ReplyError>(
        &self,
        mut reply: kobject::Message,
    ) -> Result<(ReplyContent, KHandles), CallError<ReplyError>>
    where
        ReplyContent: Copy,
        ReplyError: Copy + fmt::Display + fmt::Debug,
    {
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
}
