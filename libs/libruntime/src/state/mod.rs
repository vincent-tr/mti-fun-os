use crate::{ipc, kobject::KObject};

mod client;
pub mod messages;

lazy_static::lazy_static! {
    static ref CLIENT: client::Client = client::Client::new();
}

type StateServerError = ipc::CallError<messages::StateServerError>;

pub fn get_state(name: &str) -> ipc::BufferView {
    let mobj = CLIENT.get_state(name).expect("failed to get state");

    ipc::BufferView::new(
        mobj.into_handle(),
        &ipc::buffer_messages::Buffer {
            offset: 0,
            size: messages::STATE_SIZE,
        },
    )
    .expect("failed to create buffer view")
}
