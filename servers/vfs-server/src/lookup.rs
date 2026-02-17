use alloc::{collections::vec_deque::VecDeque, string::String, vec::Vec};
use hashbrown::HashMap;
use libruntime::vfs::{
    iface::VfsServerError,
    types::{Metadata, NodeType, Permissions},
};

use crate::{mounts::MountTable, vnode::VNode};

#[derive(Debug)]
struct LookupContext {
    metadata_cache: HashMap<VNode, Metadata>,
    lookup_count: usize,
}

impl LookupContext {
    pub fn new() -> Self {
        Self {
            metadata_cache: HashMap::new(),
            lookup_count: 0,
        }
    }

    pub async fn get_metadata(&mut self, node: VNode) -> Result<Metadata, VfsServerError> {
        if let Some(metadata) = self.metadata_cache.get(&node) {
            return Ok(*metadata);
        }

        let metadata = node.metadata().await?;
        self.metadata_cache.insert(node, metadata);
        Ok(metadata)
    }

    pub fn increment_lookup_count(&mut self) -> Result<(), VfsServerError> {
        self.lookup_count += 1;
        if self.lookup_count > 10 {
            return Err(VfsServerError::TooManySymlinks);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct NodeStack(Vec<VNode>);

impl NodeStack {
    pub fn new() -> Result<Self, VfsServerError> {
        let root = MountTable::get().root().ok_or(VfsServerError::NotFound)?;

        let mut stack = Vec::new();
        stack.push(root);

        Ok(Self(stack))
    }

    pub fn current(&self) -> VNode {
        self.0
            .last()
            .copied()
            .expect("NodeStack should never be empty")
    }

    pub fn push(&mut self, node: VNode) {
        self.0.push(node);
    }

    pub fn reset(&mut self) {
        self.0.truncate(1);
    }
}

#[derive(Debug)]
struct SegmentsQueue(VecDeque<String>);

impl SegmentsQueue {
    pub fn new(path: &str) -> Self {
        let segments = path.split('/').map(String::from).collect();
        Self(segments)
    }

    pub fn pop_front(&mut self) -> Option<String> {
        self.0.pop_front()
    }

    pub fn reset(&mut self, path: &str) {
        self.0 = path.split('/').map(String::from).collect();
    }

    pub fn prepend(&mut self, path: &str) {
        let mut new_segments = path.split('/').map(String::from).collect::<VecDeque<_>>();
        new_segments.append(&mut self.0);
        self.0 = new_segments;
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Lookup a vnode by its path.
pub async fn lookup(path: &str, no_follow: bool) -> Result<VNode, VfsServerError> {
    let mut context = LookupContext::new();

    if !path.starts_with('/') {
        return Err(VfsServerError::InvalidArgument);
    }

    let mut segments = SegmentsQueue::new(path);
    let mut node_stack = NodeStack::new()?;

    loop {
        let Some(segment) = segments.pop_front() else {
            break;
        };

        let current_node = node_stack.current();
        let new_node = traverse(&mut context, current_node, &segment).await?;

        let metadata = context.get_metadata(new_node).await?;
        if metadata.r#type == NodeType::Symlink && !(no_follow && segments.is_empty()) {
            resolve_symlink(&mut context, &mut node_stack, &mut segments).await?;
        } else {
            node_stack.push(new_node);
        }
    }

    Ok(node_stack.current())
}

/// Traverses from `node` to its child named `name`, and returns the child node.
async fn traverse(
    context: &mut LookupContext,
    node: VNode,
    name: &str,
) -> Result<VNode, VfsServerError> {
    let metadata = context.get_metadata(node).await?;

    if metadata.r#type != NodeType::Directory {
        return Err(VfsServerError::NotDirectory);
    }

    if !metadata.permissions.contains(Permissions::EXECUTE) {
        return Err(VfsServerError::AccessDenied);
    }

    let child = node.mount().lookup(node.node_id(), name).await?;

    Ok(VNode::new(node.mount_id(), child))
}

async fn resolve_symlink(
    context: &mut LookupContext,
    node_stack: &mut NodeStack,
    segments: &mut SegmentsQueue,
) -> Result<(), VfsServerError> {
    context.increment_lookup_count()?;

    let node = node_stack.current();
    let target_path = node.mount().read_symlink(node.node_id()).await?;

    if target_path.starts_with('/') {
        // Absolute symlink: reset the stack and start from root
        segments.reset(&target_path);
        node_stack.reset();
    } else {
        // Relative symlink: prepend the target path to the remaining segments
        segments.prepend(&target_path);
    }

    Ok(())
}
