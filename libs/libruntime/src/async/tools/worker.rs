use crate::{
    r#async::{self, JoinHandle},
    sync::r#async::NotifyOnce,
};

/// A simple worker abstraction that allows you to run an async function in a separate task and provides a way to gracefully terminate it.
#[derive(Debug)]
pub struct Worker {
    exit_signal: NotifyOnce,
    handle: JoinHandle,
}

impl Worker {
    /// Spawns a new worker task that runs the given async function.
    pub fn spawn<Fut: Future<Output = ()> + Send + 'static>(
        implementation: impl FnOnce(NotifyOnce) -> Fut + Send + 'static,
    ) -> Self {
        let exit_signal = NotifyOnce::new();

        let handle = r#async::spawn({
            let exit_signal = exit_signal.clone();
            async move {
                implementation(exit_signal).await;
            }
        });

        Self {
            exit_signal,
            handle,
        }
    }

    /// Gracefully terminates the worker.
    pub async fn terminate(&self) {
        self.exit_signal.notify();
        self.handle.clone().await;
    }

    /// Checks if the worker has terminated.
    pub fn is_terminated(&self) -> bool {
        self.exit_signal.is_notified()
    }
}
