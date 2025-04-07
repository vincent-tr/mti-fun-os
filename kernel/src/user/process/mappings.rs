use core::{
    cell::{Ref, RefCell},
    iter::Step,
    mem::swap,
    ops::{Bound, Range},
    panic,
};

use alloc::{collections::BTreeMap, format, rc::Rc};

use crate::{
    memory::{Permissions, VirtAddr, KERNEL_START, PAGE_SIZE},
    user::{error::out_of_memory, Error},
};

use super::mapping::Mapping;

// Forbid use of NULL as valid address
const USER_SPACE_START: VirtAddr = VirtAddr::new_truncate(PAGE_SIZE as u64);
const USER_SPACE_END: VirtAddr = KERNEL_START;

#[derive(Debug)]
struct Area {
    range: Range<VirtAddr>,
    content: RefCell<AreaContent>,
}

#[derive(Debug)]
enum AreaContent {
    Invalid,
    Boundary,
    Empty,
    Used(Mapping),
}

#[derive(Debug)]
enum AreaContentType {
    Invalid,
    Boundary,
    Empty,
    Used,
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

    pub fn is_valid(&self) -> bool {
        let content = self.content.borrow();
        if let AreaContent::Invalid = *content {
            false
        } else {
            true
        }
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
                    panic!("unexpected enum value {:?}", content);
                }
            });

            Some(mapping_ref)
        } else {
            None
        }
    }

    pub fn r#type(&self) -> AreaContentType {
        match &*self.content.borrow() {
            AreaContent::Invalid => AreaContentType::Invalid,
            AreaContent::Boundary => AreaContentType::Boundary,
            AreaContent::Empty => AreaContentType::Empty,
            AreaContent::Used(_) => AreaContentType::Used,
        }
    }

    pub fn invalidate(&self) {
        *self.content.borrow_mut() = AreaContent::Invalid;
    }

    pub fn take_mapping(&self) -> Mapping {
        let mut content = self.content.borrow_mut();
        let mut swapped_content = AreaContent::Invalid;
        swap(&mut swapped_content, &mut *content);

        if let AreaContent::Used(mapping) = swapped_content {
            mapping
        } else {
            panic!("invalid area type {:?}", swapped_content);
        }
    }
}

#[derive(Clone, Debug)]
struct Node {
    prev: Rc<Area>,
    next: Rc<Area>,
}

impl Node {
    pub const fn new(prev: Rc<Area>, next: Rc<Area>) -> Self {
        Self { prev, next }
    }
}

#[derive(Debug)]
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
        let end = Node::new(initial_area, Area::boundary(USER_SPACE_END));

        nodes.insert(USER_SPACE_START, start);
        nodes.insert(USER_SPACE_END, end);

        let mappings = Self { nodes };

        #[cfg(debug_assertions)]
        mappings.check_consistency();

        mappings
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        let mut len = 0;
        for node in self.nodes.values() {
            let area = &node.next;
            if area.is_used().is_some() {
                len += 1;
            }
        }

        len
    }

    pub fn add(&mut self, mapping: Mapping) {
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
            assert!(node.prev.is_empty());
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

        #[cfg(debug_assertions)]
        self.check_consistency();
    }

    pub fn find_space(&self, size: usize) -> Result<Range<VirtAddr>, Error> {
        for node in self.nodes.values() {
            let area = &node.next;
            if area.is_empty() && area.size() >= size {
                let addr = area.range.start;
                return Ok(addr..addr + (size as u64));
            }
        }

        Err(out_of_memory())
    }

    pub fn overlaps(&self, range: &Range<VirtAddr>) -> bool {
        // find node before range
        let (&before, _) = self
            .nodes
            .range((Bound::Unbounded, Bound::Excluded(&range.start)))
            .last()
            .unwrap();
        // Get an iterator starting from the node before our range
        for (addr, node) in self
            .nodes
            .range((Bound::Included(before), Bound::Unbounded))
        {
            if *addr >= range.end {
                return false;
            }
            let area = &node.next;
            if !area.is_empty() {
                return true;
            }
        }

        return false;
    }

    /// Check that the given range is only part of one mapping area
    pub fn is_contigous_mapping(&self, range: &Range<VirtAddr>) -> bool {
        // Get a cursor starting from the node before our range
        let start_area = self.get(range.start);
        let end_area = self.get(last_page(&range));

        Rc::ptr_eq(&start_area, &end_area) && start_area.is_used().is_some()
    }

    pub fn remove_range(&mut self, range: Range<VirtAddr>) {
        // Make entries fit perfectly on boundaries
        let start_area = self.get(range.start);
        if start_area.range.start < range.start {
            // need to split
            self.split(start_area, range.start);
        }

        let end_area = self.get(last_page(&range));
        if end_area.range.end > range.end {
            // need to split
            self.split(end_area, range.end);
        }

        //start_area = self.get(range.start);
        //end_area = self.get(last_page(&range));

        // Replace all ranges inside
        let mut addr = range.start;
        loop {
            let area = self
                .nodes
                .get(&addr)
                .expect(&format!("missing node {:?}", addr))
                .next
                .clone();
            self.replace(Area::empty(area.range.clone()));

            addr = area.range.end;
            if addr == range.end {
                break;
            }
        }

        // Merge all empty area inside
        loop {
            let start_area = &self
                .nodes
                .get(&range.start)
                .expect(&format!("missing node {:?}", addr))
                .next;

            let addr = start_area.range.end;

            if addr == range.end {
                break;
            }

            self.merge(addr);
        }

        // Check if we can merge with prev/next area
        if self.can_merge(range.start) {
            self.merge(range.start);
        }

        if self.can_merge(range.end) {
            self.merge(range.end);
        }

        #[cfg(debug_assertions)]
        self.check_consistency();
    }

    pub fn update_access_range(&mut self, range: Range<VirtAddr>, perms: Permissions) {
        // Make entries fit perfectly on boundaries
        let mut start_area = self.get(range.start);
        if start_area.range.start < range.start {
            // need to split
            self.split(start_area, range.start);
        }

        let mut end_area = self.get(last_page(&range));
        if end_area.range.end > range.end {
            // need to split
            self.split(end_area, range.end);
        }

        start_area = self.get(range.start);
        end_area = self.get(last_page(&range));

        // Ensure there is only one range inside
        assert!(Rc::ptr_eq(&start_area, &end_area));
        let area = start_area;

        let mut mapping = area.take_mapping();
        mapping.set_permissions(perms);
        // Note: even if permissions update failed, we must still deal with setting the mapping back
        self.replace(Area::from_mapping(mapping));

        // Check if we can merge with prev/next area
        if self.can_merge(range.start) {
            self.merge(range.start);
        }

        if self.can_merge(range.end) {
            self.merge(range.end);
        }

        #[cfg(debug_assertions)]
        self.check_consistency();
    }

    /// Clear all mappings on process terminate
    pub fn clear(&mut self) {
        self.remove_range(USER_SPACE_START..USER_SPACE_END);
    }

    fn split(&mut self, area: Rc<Area>, addr: VirtAddr) {
        assert!(area.range.start < addr);
        assert!(addr < area.range.end);

        let (left_area, right_area) = match area.r#type() {
            AreaContentType::Invalid => panic!("invalid area {:?}", area),
            AreaContentType::Boundary => panic!("Cannot split area boundary"),
            AreaContentType::Empty => (
                Area::empty(area.range.start..addr),
                Area::empty(addr..area.range.end),
            ),
            AreaContentType::Used => {
                let mut left_mapping = area.take_mapping();
                let right_mapping = left_mapping.split(addr);
                (
                    Area::from_mapping(left_mapping),
                    Area::from_mapping(right_mapping),
                )
            }
        };

        area.invalidate();

        let start = self
            .nodes
            .get_mut(&area.range.start)
            .expect(&format!("bad area {:?}", area.range));
        start.next = left_area.clone();

        let middle = Node::new(left_area, right_area.clone());
        self.nodes.insert(addr, middle);

        let end = self
            .nodes
            .get_mut(&area.range.end)
            .expect(&format!("bad area {:?}", area.range));
        end.prev = right_area;
    }

    fn can_merge(&self, addr: VirtAddr) -> bool {
        let node = self
            .nodes
            .get(&addr)
            .expect(&format!("bad address {:?}", addr));

        let left_area = node.prev.clone();
        let right_area = node.next.clone();

        let res = match &*left_area.content.borrow() {
            AreaContent::Invalid => panic!("invalid area {:?}", left_area),
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
        let node = self
            .nodes
            .get(&addr)
            .expect(&format!("bad address {:?}", addr));

        let left_area = node.prev.clone();
        let right_area = node.next.clone();

        self.nodes.remove(&addr);

        let new_area = match left_area.r#type() {
            AreaContentType::Invalid => panic!("invalid area {:?}", left_area),
            AreaContentType::Boundary => panic!("Cannot merge area boundary {:?}", left_area),
            AreaContentType::Empty => {
                assert!(
                    right_area.is_empty(),
                    "area types mismatch {:?} / {:?}",
                    left_area,
                    right_area
                );
                Area::empty(left_area.range.start..right_area.range.end)
            }
            AreaContentType::Used => {
                let mut left_mapping = left_area.take_mapping();
                let right_mapping = right_area.take_mapping();
                assert!(left_mapping.can_merge(&right_mapping));
                unsafe {
                    left_mapping.merge(right_mapping);
                }
                Area::from_mapping(left_mapping)
            }
        };

        let start = self
            .nodes
            .get_mut(&new_area.range.start)
            .expect(&format!("bad area {:?}", new_area.range));
        start.next = new_area.clone();

        let end = self
            .nodes
            .get_mut(&new_area.range.end)
            .expect(&format!("bad area {:?}", new_area.range));
        end.prev = new_area;
    }

    fn replace(&mut self, new_area: Rc<Area>) {
        let start = self
            .nodes
            .get_mut(&new_area.range.start)
            .expect(&format!("bad area {:?}", new_area.range));
        assert!(start.next.range == new_area.range);
        start.next = new_area.clone();

        let end = self
            .nodes
            .get_mut(&new_area.range.end)
            .expect(&format!("bad area {:?}", new_area.range));
        assert!(end.prev.range == new_area.range);
        end.prev = new_area;
    }

    fn get(&self, addr: VirtAddr) -> Rc<Area> {
        let (_, node) = self
            .nodes
            .range((Bound::Unbounded, Bound::Included(&addr)))
            .last()
            .expect(&format!("no area corresponding to address {:?}", addr));
        let area = node.next.clone();
        assert!(area.range.contains(&addr));
        area
    }

    #[cfg(debug_assertions)]
    fn check_consistency(&self) {
        // Check that first and last are boundaries
        let (&first_addr, first_node) = self
            .nodes
            .first_key_value()
            .expect("first boundary missing");
        assert!(first_addr == USER_SPACE_START);
        assert!(first_node.prev.is_bounary());
        assert!(first_node.prev.size() == 0);

        let (&last_addr, last_node) = self.nodes.last_key_value().expect("last boundary missing");
        assert!(last_addr == USER_SPACE_END);
        assert!(last_node.next.is_bounary());
        assert!(last_node.next.size() == 0);

        // Check that each node and areas address matches, and no merge are possible
        for (&addr, node) in self.nodes.iter() {
            let prev = &node.prev;
            assert!(prev.range.end == addr);

            let next = &node.next;
            assert!(next.range.start == addr);

            if addr != last_addr {
                assert!(next.size() > 0);
                assert!(!next.is_bounary());

                let is_invalid = {
                    let content = next.content.borrow();
                    if let AreaContent::Invalid = *content {
                        true
                    } else {
                        false
                    }
                };

                assert!(!is_invalid);
            }
        }

        for &addr in self.nodes.keys() {
            assert!(!self.can_merge(addr));
        }

        // Very verbose: write out a summary of the address space
        use log::trace;

        trace!("BEGIN check_consistency");

        let mut first = true;
        for (&addr, node) in self.nodes.iter() {
            if first {
                let left = &node.prev;
                trace!("    {:?} ({:?})", left.r#type(), left.range);
                first = false
            }

            trace!("  {:?}", addr);

            let right = &node.next;
            trace!("    {:?} ({:?})", right.r#type(), right.range);
        }

        trace!("END check_consistency");
    }
}

fn last_page(range: &Range<VirtAddr>) -> VirtAddr {
    Step::backward(range.end, PAGE_SIZE)
}
