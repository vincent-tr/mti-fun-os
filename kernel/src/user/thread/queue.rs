use core::ptr::null_mut;

use alloc::{boxed::Box, sync::Arc};
use hashbrown::HashMap;

use super::Thread;

// Linked list implementation with accessible nodes
#[derive(Debug)]
struct Node {
    prev: NodePtr,
    next: NodePtr,
    thread: Arc<Thread>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct NodePtr(*mut Node);

impl NodePtr {
    pub const fn null() -> Self {
        NodePtr(null_mut())
    }

    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn as_ref(&self) -> Option<&Node> {
        if self.is_null() {
            None
        } else {
            Some(unsafe { &*self.0 })
        }
    }

    pub fn as_mut_ref(&self) -> Option<&mut Node> {
        if self.is_null() {
            None
        } else {
            Some(unsafe { &mut *self.0 })
        }
    }
}

impl From<Box<Node>> for NodePtr {
    fn from(value: Box<Node>) -> Self {
        Self(Box::leak(value))
    }
}

impl Into<Box<Node>> for NodePtr {
    fn into(self) -> Box<Node> {
        unsafe { Box::from_raw(self.0) }
    }
}

unsafe impl Sync for NodePtr {}

/// Queue implementation with thread fast removal
#[derive(Debug)]
pub struct Queue {
    head: NodePtr,
    tail: NodePtr,
    map: HashMap<u64, NodePtr>,
}

unsafe impl Send for Queue {}

impl Queue {
    pub fn new() -> Self {
        Self {
            head: NodePtr::null(),
            tail: NodePtr::null(),
            map: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Add a new thread to this queue
    pub fn add(&mut self, thread: Arc<Thread>) {
        let id = thread.id();
        assert!(!self.map.contains_key(&id));

        let new_node = Box::new(Node {
            prev: NodePtr::null(),
            next: self.head,
            thread,
        });

        // Move the node out of Box.
        // It will get back in at deletion time
        let new_node_ptr = NodePtr::from(new_node);

        if let Some(head) = self.head.as_mut_ref() {
            head.prev = new_node_ptr;
        } else {
            // no node in the queue, add tail too
            self.tail = new_node_ptr;
        }

        self.head = new_node_ptr;

        self.map.insert(id, new_node_ptr);
    }

    /// Remove a thread from the queue
    pub fn remove(&mut self, thread: &Arc<Thread>) -> bool {
        let id = thread.id();

        if let Some(node_ptr) = self.map.remove(&id) {
            self.remove_node(node_ptr);
            true
        } else {
            false
        }
    }

    /// Pop the next thread from the queue
    pub fn pop(&mut self) -> Option<Arc<Thread>> {
        if self.tail.is_null() {
            return None;
        }

        let thread = self.remove_node(self.tail);
        self.map.remove(&thread.id());

        Some(thread)
    }

    fn remove_node(&mut self, node_ptr: NodePtr) -> Arc<Thread> {
        // Get back the node into Box
        let node: Box<Node> = node_ptr.into();

        if let Some(prev) = node.prev.as_mut_ref() {
            prev.next = node.next;
        } else {
            // current node is head
            self.head = node.next;
        }

        if let Some(next) = node.next.as_mut_ref() {
            next.prev = node.prev;
        } else {
            // current node is tail
            self.tail = node.prev;
        }

        node.thread
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        while !self.tail.is_null() {
            self.remove_node(self.tail);
        }

        self.map.clear();
    }
}
