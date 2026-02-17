use core::sync::atomic::{AtomicU32, Ordering};

use lazy_static::lazy_static;

use libruntime::{ipc, state};

const UNSET_VERSION: u32 = 0;
const STATE_VERSION: u32 = 1;

const STATE_NAME: &str = "vfs-server";

/// Global state of the vfs server, including all processes and related information.
#[derive(Debug)]
#[repr(C)]
pub struct State {
    version: AtomicU32,
    handle_generator: ipc::HandleGenerator,
}

impl State {
    /// Get the global state instance, initializing it if necessary.
    pub fn get() -> &'static Self {
        lazy_static! {
            static ref STATE_VIEW: state::StateView<State> = {
                let view = state::StateView::<State>::open(STATE_NAME);

                let state = unsafe { view.as_ref() };

                match state.version.load(Ordering::Acquire) {
                    UNSET_VERSION => {
                        // Initialize the state if it's not set
                        // state.handle_generator is zero-initialized, this is fine.
                        state.version.store(STATE_VERSION, Ordering::Release);
                    }
                    STATE_VERSION => {
                        // State is already initialized, nothing to do
                    }
                    other => panic!("unexpected state version: {}", other),
                };

                view
            };
        };

        unsafe { STATE_VIEW.as_ref() }
    }

    /// Get the handle generator for this state, which can be used to create new handles for processes.
    pub fn handle_generator(&self) -> &ipc::HandleGenerator {
        &self.handle_generator
    }
}
