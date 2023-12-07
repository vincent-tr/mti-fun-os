use core::{
    ops::{Bound, Range},
    panic,
};

use alloc::{
    collections::{btree_map::Iter, BTreeMap},
    rc::Rc,
};

use crate::{
    memory::{VirtAddr, KERNEL_START, PAGE_SIZE},
    user::{error::out_of_memory, Error},
};

use super::mapping::Mapping;

// Forbid use of NULL as valid address
const USER_SPACE_START: VirtAddr = VirtAddr::zero() + PAGE_SIZE;
const USER_SPACE_END: VirtAddr = KERNEL_START;

struct Area {
    range: Range<VirtAddr>,
    content: AreaContent,
}

enum AreaContent {
    Boundary,
    Empty,
    Used(Mapping),
}

impl Area {
    pub fn from_mapping(mapping: Mapping) -> Rc<Self> {
        Rc::new(Self {
            range: mapping.range().clone(),
            content: AreaContent::Used(mapping),
        })
    }

    pub fn empty(range: Range<VirtAddr>) -> Rc<Self> {
        Rc::new(Self {
            range,
            content: AreaContent::Empty,
        })
    }

    pub fn boundary(addr: VirtAddr) -> Rc<Self> {
        Rc::new(Self {
            range: addr..addr,
            content: AreaContent::Boundary,
        })
    }

    pub fn size(&self) -> usize {
        (self.range.end - self.range.start) as usize
    }

    pub fn is_empty(&self) -> bool {
        if let AreaContent::Empty = self.content {
            true
        } else {
            false
        }
    }

    pub fn is_bounary(&self) -> bool {
        if let AreaContent::Boundary = self.content {
            true
        } else {
            false
        }
    }

    pub fn is_used(&self) -> Option<&Mapping> {
        if let AreaContent::Used(mapping) = &self.content {
            Some(mapping)
        } else {
            None
        }
    }

    pub fn take_mapping(self) -> Mapping {
        if let AreaContent::Used(mapping) = self.content {
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
        let prev = self.get(mapping.range().start - PAGE_SIZE);
        if let Some(prev_mapping) = prev.is_used()
            && prev_mapping.can_merge(&mapping)
        {
            self.remove(&prev);

            let mut new_mapping = prev.take_mapping();
            unsafe { new_mapping.merge(mapping) };
            mapping = new_mapping;
        }

        let next = self.get(mapping.range().end);
        if let Some(next_mapping) = next.is_used()
            && mapping.can_merge(next_mapping)
        {
            self.remove(&next);

            unsafe { mapping.merge(next.take_mapping()) };
        }

        self.insert(Area::from_mapping(mapping));
    }

    pub fn find_space(&self, size: usize) -> Result<Range<VirtAddr>, Error> {
        for area in self.area_iter() {
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

    fn insert(&mut self, area: Rc<Area>) {
        // Get the gap around
        let mut prev = self
            .nodes
            .upper_bound(Bound::Included(&area.range.start))
            .value()
            .expect("area out of bounds")
            .clone();
        let mut next = self
            .nodes
            .lower_bound(Bound::Included(&area.range.end))
            .value()
            .expect("area out of bounds")
            .clone();

        assert!(Rc::ptr_eq(&prev.next, &next.prev), "area cross boundaries");
        let empty_area = prev.next.clone();
        assert!(area.is_empty());

        // Recreate prev empty area
        if empty_area.range.start < area.range.start {
            let new_area = Area::empty(empty_area.range.start..area.range.start);
            prev.next = new_area;
            self.nodes.insert(empty_area.range.start, prev);

            let new_node = Node::new(new_area.clone(), area.clone());
            self.nodes.insert(area.range.start, new_node);
        } else {
            prev.next = area.clone();
            self.nodes.insert(area.range.start, prev);
        }

        // Recreate next empty area
        if empty_area.range.end > area.range.end {
            let new_area = Area::empty(area.range.end..empty_area.range.end);
            let new_node = Node::new(area.clone(), new_area.clone());
            self.nodes.insert(area.range.end, new_node);

            next.prev = new_area;
            self.nodes.insert(empty_area.range.end, next);
        } else {
            next.prev = area.clone();
            self.nodes.insert(area.range.end, next);
        }
    }

    fn remove(&mut self, area: &Rc<Area>) {
        let new_area = Area::empty(area.range);

        let mut start = self.nodes.get_mut(&area.range.start).expect("bad area");
        assert!(Rc::ptr_eq(area, &start.next));
        start.next = new_area.clone();

        let mut end = self
            .nodes
            .get_mut(&area.range.end)
            .expect("bad area")
            .clone();
        assert!(Rc::ptr_eq(area, &end.prev));
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

    fn area_iter(&self) -> AreaIterator {
        AreaIterator {
            map_iter: self.nodes.iter(),
        }
    }
}

struct AreaIterator<'a> {
    map_iter: Iter<'a, VirtAddr, Node>,
}

impl<'a> Iterator for AreaIterator<'a> {
    type Item = &'a Rc<Area>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((_, node)) = self.map_iter.next() {
            let area = &node.next;
            // Do not iter over boundaries
            if !area.is_bounary() {
                return Some(area);
            }
        }

        None
    }
}
