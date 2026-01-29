use core::ops::Range;

use alloc::sync::Arc;
use hashbrown::HashMap;

use super::{process::AddressInfo, thread::WaitQueue, MemoryObject};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref FUTEX_WAIT_QUEUES: Mutex<HashMap<FutexKey, Arc<WaitQueue>>> =
        Mutex::new(HashMap::new());
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FutexKey {
    mem_obj_ptr: usize,
    offset: usize,
}

impl FutexKey {
    pub fn new(uaddr: AddressInfo) -> Self {
        let mem_obj = uaddr
            .mobj
            .as_ref()
            .expect("missing MemoryObject in uaddr info");
        let mem_obj_ptr = mem_obj.as_ref() as *const MemoryObject as usize;

        Self {
            mem_obj_ptr,
            offset: uaddr.offset,
        }
    }
}

pub fn get_waitqueue(uaddr: AddressInfo) -> Arc<WaitQueue> {
    let key = FutexKey::new(uaddr);
    let mut queues = FUTEX_WAIT_QUEUES.lock();

    if let Some(queue) = queues.get(&key) {
        return queue.clone();
    }

    let queue = Arc::new(WaitQueue::new());
    queues.insert(key, queue.clone());
    queue
}

pub fn wake(uaddr: AddressInfo, max_count: usize) -> usize {
    let key = FutexKey::new(uaddr);
    let mut woken_count = 0;

    let mut queues = FUTEX_WAIT_QUEUES.lock();

    let Some(queue) = queues.get(&key) else {
        return 0;
    };

    for _ in 0..max_count {
        if queue.wake().is_none() {
            break;
        }
        woken_count += 1;
    }

    // Clean empty queue
    if queue.empty() {
        queues.remove(&key);
    }

    woken_count
}

pub fn wake_all_region(mobj: &Arc<MemoryObject>, range: Range<usize>) {}
