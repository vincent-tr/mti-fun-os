mod error;
mod manager;

use libruntime::kobject;
use manager::Manager;

pub fn start() {
    let mut options = kobject::ThreadOptions::default();
    options.name("state-server");

    kobject::Thread::start(entry, options).expect("failed to start state-server thread");
}

fn entry() {
    let manager = Manager::new().expect("failed to create state-server");

    let server = manager
        .build_ipc_server()
        .expect("failed to build state-server IPC server");

    server.run()
}
