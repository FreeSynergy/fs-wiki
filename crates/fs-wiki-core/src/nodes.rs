//! Spaces, folders and pages as nodes of a tree (`text.space`).
//!
//! Pattern: Value object — plain data. A folder owns the path prefix `path/`; a page's parent is the
//! folder with the longest such prefix. Only the order among siblings is state of its own.

use fs_objects::ObjectId;

/// What a node is (`NodeKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A text.
    Page,
    /// A folder below a space or another folder.
    Folder,
    /// A top-level folder.
    Space,
}

/// A node of the tree (`Node`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// The text id for a page, the folder id otherwise.
    pub id: ObjectId,
    /// What it is.
    pub kind: NodeKind,
    /// Display name.
    pub name: String,
    /// A folder's prefix without a trailing `/`, a page's path.
    pub path: String,
    /// The folder it is in; none = top level.
    pub parent: Option<ObjectId>,
    /// 0-based index among the siblings as of this answer.
    pub position: u32,
    /// Pages only.
    pub template: bool,
    /// Pages only.
    pub redirect: bool,
}

/// Where `reorder` puts a node among its siblings (`Placement` without `unspecified`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// At the start.
    First,
    /// At the end.
    Last,
    /// Right before the anchor.
    Before,
    /// Right after the anchor.
    After,
}

/// A page of children, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodePage {
    /// At most the size asked for.
    pub items: Vec<Node>,
    /// Whether children follow.
    pub more: bool,
}
