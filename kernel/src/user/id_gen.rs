use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct IdGen {
    counter: AtomicU64,
}

impl IdGen {
    pub const fn new() -> Self {
        IdGen { counter: AtomicU64::new(1) }
    }

    pub fn generate(&self) -> u64 {
        let id = self.counter.fetch_add(1, Ordering::Relaxed);
        assert!(id > 0, "counter wrapped");
        id
    }
}
