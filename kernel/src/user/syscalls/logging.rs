use alloc::{format, string::String};
use log::Level;

use crate::user::{
    Error,
    error::invalid_argument,
    syscalls::{context::Context, helpers::StringReader},
};

pub async fn log(context: Context) -> Result<(), Error> {
    let level = context.arg1();
    let message_ptr = context.arg2();
    let message_len = context.arg3();

    let thread = context.owner();
    let process = thread.process();

    let message_reader = StringReader::new(&context, message_ptr, message_len)?;

    let pid = process.id();
    let tid = thread.id();
    let pname = process.name();
    let tname = thread.name();
    let level = parse_level(level)?;
    let message = message_reader.str()?;

    let thread_suffix = if let Some(name) = &*tname {
        format!(" ({})", name)
    } else {
        String::new()
    };

    log::log!(
        level,
        "(pid={pid} ({pname}), tid={tid}{thread_suffix}): {message}"
    );

    Ok(())
}

fn parse_level(level: usize) -> Result<Level, Error> {
    const ERROR_USIZE: usize = Level::Error as usize;
    const WARN_USIZE: usize = Level::Warn as usize;
    const INFO_USIZE: usize = Level::Info as usize;
    const DEBUG_USIZE: usize = Level::Debug as usize;
    const TRACE_USIZE: usize = Level::Trace as usize;
    match level {
        ERROR_USIZE => Ok(Level::Error),
        WARN_USIZE => Ok(Level::Warn),
        INFO_USIZE => Ok(Level::Info),
        DEBUG_USIZE => Ok(Level::Debug),
        TRACE_USIZE => Ok(Level::Trace),
        _ => Err(invalid_argument()),
    }
}
