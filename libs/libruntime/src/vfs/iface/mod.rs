mod client;
mod dentries_block;
mod messages;
mod mounts_block;
mod server;

pub use dentries_block::DirectoryEntry;
pub use messages::{VfsServerError, PORT_NAME};
pub use mounts_block::MountInfo;
use server::Server;
pub use server::VfsServer;

use crate::{ipc, kobject};

pub fn build_ipc_server<Impl: VfsServer + 'static>(
    inner: Impl,
) -> Result<ipc::AsyncServer, kobject::Error> {
    let server = Server::new(inner);
    server.build_ipc_server()
}
