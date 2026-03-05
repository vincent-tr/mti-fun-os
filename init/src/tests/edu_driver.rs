use libruntime::process;
use log::info;

/// Run educational device tests
#[allow(dead_code)]
pub fn run_edu_driver() {
    info!("Running EDU device tests...");

    let options =
        process::ProcessOptions::from_path("/mnt/archive/servers/drivers/test/edu/edu-server")
            .expect("Failed to load edu-server");
    let process = process::Process::spawn(options).expect("Could not spawn edu server");

    let mut waiter = process
        .create_waiter()
        .expect("Failed to create waiter for EDU server");
    waiter.wait_status();

    info!("EDU device tests completed successfully");
}
