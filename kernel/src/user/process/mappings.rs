use core::{
    mem::swap,
    ops::{Bound, Range},
    panic, cell::{RefCell, Ref}, borrow::BorrowMut,
};

use alloc::{
    collections::{btree_map, BTreeMap},
    rc::Rc,
};

use crate::{
    memory::{VirtAddr, KERNEL_START, PAGE_SIZE},
    user::{error::out_of_memory, Error},
};

use super::mapping::Mapping;

// Forbid use of NULL as valid address
const USER_SPACE_START: VirtAddr = VirtAddr::new_truncate(PAGE_SIZE as u64);
const USER_SPACE_END: VirtAddr = KERNEL_START;

struct Area {
    range: Range<VirtAddr>,
    content: RefCell<AreaContent>,
}

enum AreaContent {
    Invalid,
    Boundary,
    Empty,
    Used(Mapping),
}

impl Area {
    pub fn from_mapping(mapping: Mapping) -> Rc<Self> {
        Rc::new(Self {
            range: mapping.range().clone(),
            content: RefCell::new(AreaContent::Used(mapping)),
        })
    }

    pub fn empty(range: Range<VirtAddr>) -> Rc<Self> {
        Rc::new(Self {
            range,
            content: RefCell::new(AreaContent::Empty),
        })
    }

    pub fn boundary(addr: VirtAddr) -> Rc<Self> {
        Rc::new(Self {
            range: addr..addr,
            content: RefCell::new(AreaContent::Boundary),
        })
    }

    pub fn size(&self) -> usize {
        (self.range.end - self.range.start) as usize
    }

    pub fn is_empty(&self) -> bool {
        let content = self.content.borrow();
        if let AreaContent::Empty = *content {
            true
        } else {
            false
        }
    }

    pub fn is_bounary(&self) -> bool {
        let content = self.content.borrow();
        if let AreaContent::Boundary = *content {
            true
        } else {
            false
        }
    }

    pub fn is_used(&self) -> Option<Ref<Mapping>> {
        let content = self.content.borrow();
        if let AreaContent::Used(_) = &*content {
            let mapping_ref = Ref::map(content, |content| {
                if let AreaContent::Used(mapping) = content {
                    mapping
                } else {
                    panic!("unexpected enum value");
                }
            });

            Some(mapping_ref)
        } else {
            None
        }
    }

    pub fn invalidate(&self) {
        *self.content.borrow_mut() = AreaContent::Invalid;
    }

    pub fn take_mapping(&self) -> Mapping {
        let mut content = self.content.borrow_mut();
        let mut swapped_content = AreaContent::Invalid;
        swap(&mut swapped_content, &mut *content.borrow_mut());

        if let AreaContent::Used(mapping) = swapped_content {
            mapping
        } else {
            panic!("invalid area type");
        }
    }
}

#[derive(Clone)]
struct Node {
    prev: Rc<Area>,
    next: Rc<Area>,
}

impl Node {
    pub const fn new(prev: Rc<Area>, next: Rc<Area>) -> Self {
        Self { prev, next }
    }
}

pub struct Mappings {
    nodes: BTreeMap<VirtAddr, Node>,
}

unsafe impl Sync for Mappings {}
unsafe impl Send for Mappings {}

impl Mappings {
    pub fn new() -> Self {
        let mut nodes = BTreeMap::new();

        let initial_area = Area::empty(USER_SPACE_START..USER_SPACE_END);
        let start = Node::new(Area::boundary(USER_SPACE_START), initial_area.clone());
        let end = Node::new(Area::boundary(USER_SPACE_END), initial_area);

        nodes.insert(USER_SPACE_START, start);
        nodes.insert(USER_SPACE_END, end);

        Self { nodes }
    }

    pub fn add(&mut self, mut mapping: Mapping) {
        let new_area = Area::from_mapping(mapping);

        if let Some(node) = self.nodes.get(&new_area.range.start) {
            // exactly on node
            assert!(node.next.is_empty());
        } else {
            // need to split prev area
            let prev_area = self.get(new_area.range.start);
            assert!(prev_area.is_empty());
            self.split(prev_area, new_area.range.start);
        }

        if let Some(node) = self.nodes.get(&new_area.range.end) {
            // exactly on node
            assert!(node.next.is_empty());
        } else {
            // need to split next area
            let next_area = self.get(new_area.range.end);
            assert!(next_area.is_empty());
            self.split(next_area, new_area.range.end);
        }

        self.replace(new_area.clone());

        if self.can_merge(new_area.range.start) {
            self.merge(new_area.range.start);
        }

        if self.can_merge(new_area.range.end) {
            self.merge(new_area.range.end);
        }
    }

    pub fn find_space(&self, size: usize) -> Result<Range<VirtAddr>, Error> {
        for node in self.nodes.values() {
            let area = &node.next;
            if area.is_empty() && area.size() >= size {
                let addr = area.range.start;
                return Ok(addr..addr + size);
            }
        }

        Err(out_of_memory())
    }

    pub fn overlaps(&self, range: &Range<VirtAddr>) -> bool {
        // Get a cursor starting from the node before our range
        let cursor = self.nodes.upper_bound(Bound::Included(&range.start));
        loop {
            if cursor.value().is_none() {
                return false;
            }

            let (addr, node) = cursor.key_value().unwrap(); // checked above
            if *addr >= range.end {
                return false;
            }
            let area = &node.next;
            if !area.is_empty() {
                return true;
            }
        }
    }

    pub fn remove_range(&mut self, range: Range<VirtAddr>) {

        // Make entries fit perfectly on boundaries
        let mut start_area = self.get(range.start);
        if start_area.range.start < range.start {
            // need to split
            self.split(start_area, range.start);
            start_area = self.get(range.start);
        }

        let mut end_area = self.get(range.end - PAGE_SIZE);
        if end_area.range.end > range.end {
            // need to split
            self.split(end_area, range.end);
            end_area = self.get(range.end - PAGE_SIZE);
        }

        // Replace all ranges inside
        let mut addr = range.start;
        loop {
            let area = self.nodes.get(&addr).expect("missing area").next.clone();
            self.replace(Area::empty(area.range.clone()));

            addr = area.range.end;
            if addr == range.end {
                break;
            }
        }

        // Merge all empty area inside
        let mut addr = range.start;
        loop {
            let area = self.nodes.get(&addr).expect("missing area").next.clone();

            self.merge(addr);

            addr = area.range.end;
            if addr == range.end {
                break;
            }
        }

        // Check if we can merge with prev/next area
        if self.can_merge(range.start) {
            self.merge(range.start);
        }

        if self.can_merge(range.end) {
            self.merge(range.end);
        }
    }

    fn split(&mut self, area: Rc<Area>, addr: VirtAddr) {
        assert!(area.range.start < addr);
        assert!(addr < area.range.end);

        let (left_area, right_area) = match &*area.content.borrow() {
            AreaContent::Invalid => panic!("invalid area"),
            AreaContent::Boundary => panic!("Cannot split area boundary"),
            AreaContent::Empty => (
                Area::empty(area.range.start..addr),
                Area::empty(addr..area.range.end),
            ),
            AreaContent::Used(_) => {
                let mut left_mapping = area.take_mapping();
                let right_mapping = left_mapping.split(addr);
                (
                    Area::from_mapping(left_mapping),
                    Area::from_mapping(right_mapping),
                )
            }
        };

        area.invalidate();

        let start = self.nodes.get_mut(&area.range.start).expect("bad area");
        start.next = left_area.clone();

        let middle = Node::new(left_area, right_area.clone());
        self.nodes.insert(addr, middle);

        let end = self.nodes.get_mut(&area.range.end).expect("bad area");
        end.prev = right_area;
    }

    fn can_merge(&mut self, addr: VirtAddr) -> bool {
        let node = self.nodes.get_mut(&addr).expect("bad address");

        let left_area = node.prev.clone();
        let right_area = node.next.clone();

        let res = match &*left_area.content.borrow() {
            AreaContent::Invalid => panic!("invalid area"),
            AreaContent::Boundary => false,
            AreaContent::Empty => right_area.is_empty(),
            AreaContent::Used(left_mapping) => {
                if let Some(right_mapping) = right_area.is_used() {
                    left_mapping.can_merge(&*right_mapping)
                } else {
                    false
                }
            }
        };

        res
    }

    fn merge(&mut self, addr: VirtAddr) {
        let node = self.nodes.get(&addr).expect("bad address");

        let left_area = node.prev.clone();
        let right_area = node.next.clone();

        self.nodes.remove(&addr);

        let new_area = match &*left_area.content.borrow() {
            AreaContent::Invalid => panic!("invalid area"),
            AreaContent::Boundary => panic!("Cannot merge area boundary"),
            AreaContent::Empty => {
                assert!(right_area.is_empty(), "area types mismatch");
                Area::empty(left_area.range.start..right_area.range.end)
            }
            AreaContent::Used(_) => {
                let mut left_mapping = left_area.take_mapping();
                let right_mapping = right_area.take_mapping();
                assert!(left_mapping.can_merge(&right_mapping));
                unsafe {
                    left_mapping.merge(right_mapping);
                }
                Area::from_mapping(left_mapping)
            }
        };

        let start = self.nodes.get_mut(&new_area.range.start).expect("bad area");
        start.next = new_area.clone();

        let end = self.nodes.get_mut(&new_area.range.end).expect("bad area");
        end.prev = new_area;
    }

    fn replace(&mut self, new_area: Rc<Area>) {
        let start = self.nodes.get_mut(&new_area.range.start).expect("bad area");
        assert!(start.next.range == new_area.range);
        start.next = new_area.clone();

        let end = self.nodes.get_mut(&new_area.range.end).expect("bad area");
        assert!(end.prev.range == new_area.range);
        end.prev = new_area;
    }

    fn get(&self, addr: VirtAddr) -> Rc<Area> {
        let area = self
            .nodes
            .upper_bound(Bound::Included(&addr))
            .value()
            .expect("no area corresponding to address")
            .next
            .clone();
        assert!(area.range.contains(&addr));
        area
    }

}