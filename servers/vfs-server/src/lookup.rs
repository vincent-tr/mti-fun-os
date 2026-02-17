use alloc::{collections::vec_deque::VecDeque, string::String, vec::Vec};
use hashbrown::HashMap;
use libruntime::vfs::{
    iface::VfsServerError,
    types::{Metadata, NodeType, Permissions},
};

use crate::{mounts::MountTable, vnode::VNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupMode {
    /// Follow all symlinks, resolve everything
    Full,

    /// Don't follow the final symlink (for lstat, lchown, etc.)
    NoFollowLast,

    /// Resolve parent directory, return (parent_vnode, filename)
    /// Used for create, unlink, rename operations
    Parent,
}

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
struct NodeStack(Vec<(VNode, String)>);

impl NodeStack {
    pub fn new() -> Result<Self, VfsServerError> {
        let root = MountTable::get().root().ok_or(VfsServerError::NotFound)?;

        let mut stack = Vec::new();
        stack.push((root, String::new()));

        Ok(Self(stack))
    }

    pub fn current_node(&self) -> VNode {
        let (node, _) = self.0.last().expect("NodeStack should never be empty");
        *node
    }

    pub fn current_path(&self) -> String {
        // Skip the first segment since it's always empty (the root)
        let segments: Vec<&str> = self
            .0
            .iter()
            .skip(1)
            .map(|(_, segment)| segment.as_str())
            .collect();

        segments.join("/")
    }

    pub fn push(&mut self, node: VNode, segment: String) {
        self.0.push((node, segment));
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

    pub fn prepend(&mut self, path: &str) {
        let mut new_segments = path.split('/').map(String::from).collect::<VecDeque<_>>();
        new_segments.append(&mut self.0);
        self.0 = new_segments;
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn pop_last(&mut self) -> String {
        self.0.pop_back().expect("Segments queue empty")
    }
}

#[derive(Debug)]
pub struct LookupResult {
    /// The vnode of the looked up node.
    pub node: VNode,

    /// The canonical path of the looked up node (i.e. the path with all symlinks and .. resolved).
    pub canonical_path: String,

    /// The last segment of the path (i.e. the filename).
    pub last_segment: Option<String>,
}

/// Lookup a vnode by its path.
///
/// Also provide the cannonical path of the vnode (i.e. the path with all symlinks and .. resolved).
pub async fn lookup(path: &str, mode: LookupMode) -> Result<LookupResult, VfsServerError> {
    let mut context = LookupContext::new();

    if !path.starts_with('/') {
        return Err(VfsServerError::InvalidArgument);
    }

    if mode == LookupMode::Parent
        && (path.ends_with('/') || path.ends_with("/.") || path.ends_with("/.."))
    {
        return Err(VfsServerError::InvalidArgument);
    }

    let mut segments = SegmentsQueue::new(path);
    let mut node_stack = NodeStack::new()?;

    let mut last_segment = None;
    if mode == LookupMode::Parent {
        last_segment = Some(segments.pop_last());
    }

    loop {
        let Some(segment) = segments.pop_front() else {
            break;
        };

        let current_node = node_stack.current_node();
        let mut new_node = traverse(&mut context, current_node, &segment).await?;

        let metadata = context.get_metadata(new_node).await?;
        if metadata.r#type == NodeType::Symlink
            && !(mode == LookupMode::NoFollowLast && segments.is_empty())
        {
            resolve_symlink(&mut context, &mut node_stack, &mut segments).await?;
            continue;
        }

        // If newnode is a mountpoint, we need to switch to the root of the mounted filesystem.
        if metadata.r#type == NodeType::Directory
            && let Some(mount) = MountTable::get().get_mountpoint(&new_node)
        {
            new_node = mount.root();
        }

        node_stack.push(new_node, segment);
    }

    // With parent lookup, we ensure that the last node is a directory, so that the caller can perform create/unlink/rename operations on it.
    if mode == LookupMode::Parent {
        let metadata = context.get_metadata(node_stack.current_node()).await?;
        if metadata.r#type != NodeType::Directory {
            return Err(VfsServerError::NotDirectory);
        }

        if !metadata.permissions.contains(Permissions::EXECUTE) {
            return Err(VfsServerError::AccessDenied);
        }
    }

    Ok(LookupResult {
        node: node_stack.current_node(),
        canonical_path: node_stack.current_path(),
        last_segment,
    })
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

    let node = node_stack.current_node();
    let target_path = node.mount().read_symlink(node.node_id()).await?;

    segments.prepend(&target_path);

    if target_path.starts_with('/') {
        // Absolute symlink: reset the stack
        node_stack.reset();
    }

    Ok(())
}
