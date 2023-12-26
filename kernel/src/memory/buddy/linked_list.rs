use core::marker::PhantomData;
use core::{fmt, ptr};

use x86_64::VirtAddr;

#[derive(Copy, Clone)]
pub struct LinkedList {
    head: *mut ListNode,
}

unsafe impl Send for LinkedList {}

impl LinkedList {
    /// Create a new LinkedList
    pub const fn new() -> LinkedList {
        LinkedList {
            head: ptr::null_mut(),
        }
    }

    /// Return `true` if the list is empty
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// Push `item` to the front of the list
    pub unsafe fn push(&mut self, item: *mut ListNode) {
        (*item).next = self.head;
        self.head = item;
    }

    /// Try to remove the first item in the list
    pub fn pop(&mut self) -> Option<*mut ListNode> {
        match self.is_empty() {
            true => None,
            false => {
                // Advance head pointer
                let item = self.head;
                self.head = unsafe { (*item).next };
                Some(item)
            }
        }
    }

    /// Remove the node after the given node.
    ///
    /// If the given node is null, remove the first node.
    pub fn remove_after(&mut self, item: *mut ListNode) -> *mut ListNode {
        assert!(!self.is_empty());

        if item.is_null() {
            self.pop().unwrap()
        } else {
            unsafe {
                let res = (*item).next;
                assert!(!res.is_null(), "remove_after called with last node");
                (*item).next = (*res).next;
                (*res).next = ptr::null_mut();

                res
            }
        }
    }

    /// Return an iterator over the items in the list
    pub fn iter(&self) -> Iter {
        Iter {
            curr: self.head,
            list: PhantomData,
        }
    }
}

impl fmt::Debug for LinkedList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// An iterator over the linked list
pub struct Iter<'a> {
    curr: *mut ListNode,
    list: PhantomData<&'a LinkedList>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = *mut ListNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr.is_null() {
            None
        } else {
            let item = self.curr;
            self.curr = unsafe { (*item).next };
            Some(item)
        }
    }
}

/// Represent a mutable node in `LinkedList`
pub struct ListNode {
    value: VirtAddr,
    next: *mut ListNode,
}

impl ListNode {
    /// Initialize a node after allocation
    /// Note: act as "hand-made" in-place constructor
    pub fn init(&mut self, value: VirtAddr) {
        self.value = value;
        self.next = ptr::null_mut();
    }

    /// Returns the pointed address
    pub fn address(&self) -> VirtAddr {
        self.value
    }
}
