use alloc::boxed::Box;
use core::fmt::Debug;
use hashbrown::HashSet;

pub trait IdFilter: Debug {
    fn filter(&self, id: u64) -> bool;
}

#[derive(Debug)]
pub struct AllFilter {}

impl AllFilter {
    pub fn new() -> Box<dyn IdFilter> {
        Box::new(Self {})
    }
}

impl IdFilter for AllFilter {
    fn filter(&self, _id: u64) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct ListFilter {
    allowed: HashSet<u64>,
}

impl ListFilter {
    pub fn new(ids: &[u64]) -> Box<dyn IdFilter> {
        let allowed = HashSet::from_iter(ids.iter().copied());

        Box::new(Self { allowed })
    }
}

impl IdFilter for ListFilter {
    fn filter(&self, id: u64) -> bool {
        self.allowed.contains(&id)
    }
}
