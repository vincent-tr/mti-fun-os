use spin::Once;

pub struct OnceLock<T>(Once<T>);

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        OnceLock(Once::new())
    }

    pub fn set(&self, value: T) {
        let mut called = false;
        self.0.call_once(|| {
            called = true;
            value
        });

        assert!(called, "OnceLock already initialized");
    }

    pub fn get(&self) -> &T {
        self.0.get().expect("OnceLock not initialized")
    }
}
