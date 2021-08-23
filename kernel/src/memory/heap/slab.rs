use crate::{error::Error, memory::PAGE_SIZE};

pub trait SlabConfig: Copy + Eq + PartialOrd + Ord {
    const OBJECT_SIZE: u64;
    const BLOCK_SIZE: u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlabConfig8 {}

impl SlabConfig for SlabConfig8 {
    const OBJECT_SIZE: u64 = 8;
    const BLOCK_SIZE: u64 = PAGE_SIZE;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlabConfig16 {}

impl SlabConfig for SlabConfig8 {
    const OBJECT_SIZE: u64 = 16;
    const BLOCK_SIZE: u64 = PAGE_SIZE;
}

pub struct Slab<Config: SlabConfig> {
    used: u64;
    free: u64;
    // blocks
}