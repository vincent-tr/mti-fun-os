use libruntime::r#async;
use log::info;

/// Test the async framework with spawn and block_on
#[allow(dead_code)]
pub fn test_spawn_join() {
    info!("Testing spawn, await, and block_on...");

    // Inner async function that does the actual work
    async fn run_test() {
        info!("Started async test task");

        // Spawn a nested task
        async fn nested_task() {
            info!("Nested task running");
        }

        let join_handle = r#async::spawn(nested_task());
        info!("Awaiting nested task...");
        join_handle.await;
        info!("Nested task completed");
    }

    // Spawn the test task
    r#async::spawn(run_test());

    // Run the executor until all tasks complete
    r#async::block_on();

    info!("test_spawn_join passed!");
}
