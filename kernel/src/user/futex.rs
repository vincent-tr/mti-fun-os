use core::ops::Range;

use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use hashbrown::HashMap;

use super::{
    process::{AddressInfo, Process},
    thread::{self, WaitQueue},
    MemoryObject,
};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    /// Global futex wait queues
    static ref FUTEXES: Mutex<Futexes> =
        Mutex::new(Futexes::new());
}

/// Key identifying a futex wait queue
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FutexKey {
    mobj_id: usize,
    offset: usize,
}

impl FutexKey {
    pub fn new(uaddr: AddressInfo) -> Self {
        let mobj = uaddr
            .mobj
            .as_ref()
            .expect("missing MemoryObject in uaddr info");

        Self {
            mobj_id: FutexKey::compute_mobj_id(mobj),
            offset: uaddr.offset,
        }
    }

    pub fn mobj_id(&self) -> usize {
        self.mobj_id
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn compute_mobj_id(mobj: &Arc<MemoryObject>) -> usize {
        // use ptr as id
        mobj.as_ref() as *const MemoryObject as usize
    }
}

/// Structure holding all futex wait queues
#[derive(Debug)]
struct Futexes {
    queues: HashMap<usize, BTreeMap<usize, Arc<WaitQueue>>>,
}

impl Futexes {
    pub fn new() -> Self {
        Self {
            queues: HashMap::new(),
        }
    }

    /// Access (and create if needed) the wait queue for the given key
    pub fn access_queue(&mut self, key: &FutexKey) -> Arc<WaitQueue> {
        let obj_queues = self
            .queues
            .entry(key.mobj_id())
            .or_insert_with(BTreeMap::new);

        obj_queues
            .entry(key.offset())
            .or_insert_with(|| Arc::new(WaitQueue::new()))
            .clone()
    }

    /// Get the wait queue for the given key, if it exists
    pub fn get_queue(&self, key: &FutexKey) -> Option<Arc<WaitQueue>> {
        if let Some(obj_queues) = self.queues.get(&key.mobj_id()) {
            if let Some(queue) = obj_queues.get(&key.offset()) {
                return Some(queue.clone());
            }
        }

        None
    }

    /// Clean up empty wait queues for the given key
    pub fn clean_queue(&mut self, key: &FutexKey) {
        if let Some(obj_queues) = self.queues.get_mut(&key.mobj_id()) {
            if let Some(queue) = obj_queues.get(&key.offset()) {
                if !queue.empty() {
                    return;
                }
            }

            obj_queues.remove(&key.offset());

            if obj_queues.is_empty() {
                self.queues.remove(&key.mobj_id());
            }
        }
    }

    /// Iterate over all wait queues for the given memory object id
    pub fn iterate_mobj_queues(
        &self,
        mobj_id: usize,
    ) -> impl Iterator<Item = (&usize, &Arc<WaitQueue>)> {
        self.queues
            .get(&mobj_id)
            .into_iter()
            .flat_map(|obj_queues| obj_queues.iter())
    }
}

/// Get (and create if needed) the wait queue for the given user address
pub fn get_waitqueue(uaddr: AddressInfo) -> Arc<WaitQueue> {
    let key = FutexKey::new(uaddr);
    let mut futexes = FUTEXES.lock();
    futexes.access_queue(&key)
}

/// Wake up to `max_count` threads waiting on the futex at the given user address.
pub fn wake(uaddr: AddressInfo, max_count: usize) -> usize {
    let key = FutexKey::new(uaddr);
    let mut woken_count = 0;

    let mut futexes = FUTEXES.lock();

    let Some(queue) = futexes.get_queue(&key) else {
        return 0;
    };

    for _ in 0..max_count {
        if !thread::wait_queue_wake_one(&queue) {
            break;
        }
        woken_count += 1;
    }

    futexes.clean_queue(&key);

    woken_count
}

/// Wake all threads from the given process waiting on futexes in the given memory object and range.
///
/// This is used when unmapping a region to ensure no threads remain blocked on futexes in that region.
pub fn wake_process_region(process: &Arc<Process>, mobj: &Arc<MemoryObject>, range: Range<usize>) {
    let mut futexes = FUTEXES.lock();

    let mobj_id = FutexKey::compute_mobj_id(mobj);

    let mut clean_list = Vec::new();

    for (&offset, queue) in futexes.iterate_mobj_queues(mobj_id) {
        if range.contains(&offset) {
            // wake only threads that belong to the given process
            let count = thread::wait_queue_wake_all(&queue, &|thread| {
                Arc::ptr_eq(&thread.process(), process)
            });

            if count > 0 {
                clean_list.push(offset);
            }
        }
    }

    // Clean up possibly empty queues
    for offset in clean_list {
        let key = FutexKey { mobj_id, offset };
        futexes.clean_queue(&key);
    }
}
