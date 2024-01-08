use core::mem::size_of;

use syscalls::Message;

use crate::user::handle::Handle;

pub struct MessageBuilder {
    message: Message,
}

impl MessageBuilder {
    pub const fn new() -> Self {
        Self {
            message: Message {
                data: [0; Message::DATA_SIZE],
                handles: [Handle::invalid().as_u64(); Message::HANDLE_COUNT],
            },
        }
    }

    pub fn data_mut<TData>(&mut self) -> &mut TData {
        assert!(size_of::<TData>() <= Message::DATA_SIZE * size_of::<u64>());

        let ptr = self.message.data.as_mut_ptr() as *mut TData;
        unsafe { &mut *ptr }
    }

    pub fn message(self) -> Message {
        self.message
    }
}
