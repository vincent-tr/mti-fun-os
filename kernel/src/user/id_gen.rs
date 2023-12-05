use core::sync::atomic::{AtomicU32, Ordering};

pub struct IdGen {
    counter: AtomicU32,
}

impl IdGen {
    pub const fn new() -> Self {
        IdGen { counter: AtomicU32::new(1) }
    }

    pub fn generate(&mut self) -> u32 {
        let id = self.counter.fetch_add(1, Ordering::Relaxed);
        assert!(id > 0, "counter wrapped");
        id
    }
}
