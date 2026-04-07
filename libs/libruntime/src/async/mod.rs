mod executor;
mod future;
mod reactor;
pub mod tools;

use core::future::Future;
use executor::Executor;
use future::KWaitableFuture;
use reactor::Reactor;

use crate::kobject;

pub use executor::JoinHandle;

/// Waits for a waitable object to become ready.
pub fn wait<'a, Waitable: kobject::KWaitable + 'a>(
    waitable: &'a Waitable,
) -> KWaitableFuture<'a, Waitable> {
    KWaitableFuture::new(waitable)
}

/// Spawns a new future onto the executor.
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) -> JoinHandle {
    Executor::get().spawn(future)
}

/// Runs the executor until all tasks have completed.
pub fn block_on() {
    loop {
        Executor::get().run_once();

        if Executor::get().is_empty() {
            break;
        }
    }
}
