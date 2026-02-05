use core::{marker::PhantomData, mem};

use crate::{ipc, kobject::KObject};

mod client;
pub mod messages;

lazy_static::lazy_static! {
    static ref CLIENT: client::Client = client::Client::new();
}

/// A view into a state object, providing typed access to the underlying buffer
#[derive(Debug)]
pub struct StateView<T> {
    buffer_view: ipc::BufferView,
    _phantom: PhantomData<T>,
}

impl<T> StateView<T> {
    /// Open a state by name, returning a view that allows access to its buffer as type T
    pub fn open(name: &str) -> Self {
        // Compile-time assertion to ensure T fits within the STATE_SIZE limit
        const {
            assert!(
                mem::size_of::<T>() <= messages::STATE_SIZE,
                "State type size exceeds STATE_SIZE limit"
            )
        };

        let mobj = CLIENT.get_state(name).expect("failed to get state");

        let buffer_view = ipc::BufferView::new(
            mobj.into_handle(),
            &ipc::buffer_messages::Buffer {
                offset: 0,
                size: messages::STATE_SIZE,
            },
        )
        .expect("failed to create buffer view");

        Self {
            buffer_view,
            _phantom: PhantomData,
        }
    }

    /// Get a reference to the state data as type T.
    ///
    /// Safety:
    /// - The caller must ensure that the buffer layout matches T.
    /// - The buffer can be uninitialized (all zeros), so T must be able to handle that if necessary.
    /// - The buffer is shared, so concurrent access must be properly synchronized by the caller if needed.
    pub unsafe fn as_ref(&self) -> &T {
        let data = self.buffer_view.buffer();
        unsafe { &*(data.as_ptr() as *const _) }
    }
}
