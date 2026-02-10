mod executor;
mod future;
mod reactor;

use reactor::Reactor;

use future::KWaitableFuture;

use executor::Executor;

use core::future::Future;

use crate::kobject;

/// Waits for a waitable object to become ready.
pub async fn wait<Waitable: kobject::KWaitable>(waitable: &Waitable) {
    KWaitableFuture::new(waitable).await
}

/// Spawns a new future onto the executor.
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    Executor::get().spawn(future);
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
