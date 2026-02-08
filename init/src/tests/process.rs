use alloc::format;
use libruntime::process;
use log::debug;

/// List all processes with their information
#[allow(dead_code)]
pub fn list_processes() {
    let processes = process::Process::list().expect("Could not list processes");
    debug!("Processes:");
    debug!("  PID   PPID  Status       Name");
    for proc in processes {
        debug!(
            "  {:5} {:5} {:12} {}",
            proc.pid,
            proc.ppid,
            format!("{:?}", proc.status),
            proc.name
        );
    }
}
