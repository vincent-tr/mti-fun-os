use spin::Mutex;

pub struct LazyInit<T> (
    Mutex<Option<T>>,
);

impl<T> LazyInit<T> {
    pub fn new() -> LazyInit<T> {
        LazyInit (
             Mutex::new(Option::None),
        )
    }

    pub fn init(self, value: T) {
        let mut inner_value = self.0.lock();

        if let Some(_) = *inner_value {
            panic!("Init called twice");
        } else {
            *inner_value = Some(value);
        }
    }

    pub fn get(&self) -> &mut T {
        if let Some(ref mut value) = *self.0.lock() {
            return value;
        } else {
            panic!("Init not called");
        }
    }
}
