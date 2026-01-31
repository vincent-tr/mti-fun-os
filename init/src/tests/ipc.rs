use libruntime::{
    ipc::Handles,
    kobject::{self, ThreadOptions},
};
use log::debug;

/// Test basic IPC between threads using ports
#[allow(dead_code)]
pub fn do_ipc() {
    // create thread, send data and wait back

    let (echo_reader, main_sender) = kobject::Port::create(None).expect("failed to create ipc");
    let (main_reader, echo_sender) = kobject::Port::create(None).expect("failed to create ipc");

    let echo = move || {
        let mut message = echo_reader.blocking_receive().expect("receive failed");
        echo_sender.send(&mut message).expect("send failed");
    };

    let mut options = ThreadOptions::default();
    options.name("echo");

    kobject::Thread::start(echo, options).expect("could not create echo thread");

    let mut msg = unsafe { kobject::Message::new::<i32>(&42, Handles::new().into()) };
    main_sender.send(&mut msg).expect("send failed");

    let msg = main_reader.blocking_receive().expect("wait failed");

    assert!(unsafe { *msg.data::<i32>() } == 42);
    debug!("IPC ALL GOOD");
}
