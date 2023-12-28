use log::Level;

use crate::user::{
    error::{check_arg_res, invalid_argument},
    syscalls::helpers::StringReader,
    thread, Error,
};

use alloc::str;

pub fn log(
    level: usize,
    message_ptr: usize,
    message_len: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> Result<(), Error> {
    let thread = thread::current_thread();
    let process = thread.process();

    let message_reader = StringReader::new(message_ptr, message_len)?;

    let pid = process.id();
    let tid = thread.id();
    let level = parse_level(level)?;
    let message = message_reader.str()?;

    log::log!(level, "(pid={pid}, tid={tid}): {message}");

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
