use libruntime::{r#async, sync::r#async::NotifyOnce};
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
        join_handle.clone().await;
        info!("Nested task completed");
        join_handle.await;
        info!("Nested task still completed");
    }

    // Spawn the test task
    r#async::spawn(run_test());

    // Run the executor until all tasks complete
    r#async::block_on();

    info!("test_spawn_join passed!");
}

#[allow(dead_code)]
pub fn test_notify_once() {
    info!("Testing NotifyOnce...");

    test_notify_once_basic();
    test_notify_once_idempotent();
    test_notify_once_multiple_waiters();

    info!("NotifyOnce tests passed!");
}

/// Test NotifyOnce basic functionality
#[allow(dead_code)]
fn test_notify_once_basic() {
    info!("Testing NotifyOnce basic functionality...");

    async fn run_test() {
        let notify = NotifyOnce::new();

        assert!(
            !notify.is_notified(),
            "Notify should not be signaled initially"
        );

        notify.notify();
        assert!(
            notify.is_notified(),
            "Notify should be signaled after notify()"
        );

        info!("NotifyOnce basic test passed");
    }

    r#async::spawn(run_test());
    r#async::block_on();
}

/// Test NotifyOnce idempotency
#[allow(dead_code)]
fn test_notify_once_idempotent() {
    info!("Testing NotifyOnce idempotency...");

    async fn run_test() {
        let notify = NotifyOnce::new();

        notify.notify();
        notify.notify(); // Should be safe to call multiple times

        assert!(notify.is_notified(), "Notify should remain signaled");

        info!("NotifyOnce idempotent test passed");
    }

    r#async::spawn(run_test());
    r#async::block_on();
}

/// Test NotifyOnce with multiple waiters
#[allow(dead_code)]
fn test_notify_once_multiple_waiters() {
    info!("Testing NotifyOnce with multiple waiters...");

    async fn run_test() {
        let notify = NotifyOnce::new();
        let notify1 = notify.clone();
        let notify2 = notify.clone();

        // Spawn two tasks that wait on the notification
        let handle1 = r#async::spawn(async move {
            info!("Waiter 1 waiting...");
            notify1.wait().await;
            info!("Waiter 1 notified!");
        });

        let handle2 = r#async::spawn(async move {
            info!("Waiter 2 waiting...");
            notify2.wait().await;
            info!("Waiter 2 notified!");
        });

        // Give waiters a chance to register
        // (In a real runtime with yields, this would be automatic)

        // Signal the notification
        notify.notify();
        info!("Notification signaled");

        // Wait for both tasks to complete
        handle1.await;
        handle2.await;

        info!("NotifyOnce multiple waiters test passed");
    }

    r#async::spawn(run_test());
    r#async::block_on();
}
