//! A definition once it is loaded: a graph of nodes whose references are resolved.
//!
//! # Why a graph rather than a value
//!
//! The serialized form is a tree of values in which a slot may spell a *reference* — a path
//! into the same document, a path into another one, or a file. Loading resolves every
//! spelling into the node it names, so afterwards there is no reference concept left to
//! interpret: every slot names a node, and reading one is following a pointer rather than
//! walking a path.
//!
//! Two slots that named one path therefore name one node, which is what makes traversal
//! transparent: a change made through either slot is a change to the value both read. That
//! is also the whole of "sharing": a node is shared when a reference names it, and a shared
//! node is written in place rather than swapped, because a swap would leave the other slot
//! reading the old node.
//!
//! # Invariants
//!
//! - A slot always names a node. Nothing in a loaded definition is unresolved, so no access
//!   can fail for want of a target.
//! - A node's children are slots, and every container is a slot apiece, so a document is a
//!   tree of slots with references making it a graph. A cycle is therefore expressible, and
//!   is nothing to collect: a generation owns its nodes and is freed whole.
//! - A slot keeps the spelling it was written with, so a loaded definition writes back as
//!   the document it was read from rather than with every reference expanded.
//!
//! # Example
//!
//! ```
//! use whiting_definition::{Loaded, Node, Path, Reference, Schema, SchemaKind, Value};
//!
//! let slot = Schema::new(SchemaKind::Int).reference();
//! let schema = Schema::new(SchemaKind::structure([
//!     ("left".to_owned(), slot.clone()),
//!     ("right".to_owned(), slot.clone()),
//! ]));
//! let document = Value::structure([
//!     ("left".to_owned(), Value::Int(7)),
//!     (
//!         "right".to_owned(),
//!         Reference::Internal(Path::root().key("left")).to_value(),
//!     ),
//! ]);
//!
//! let loaded = Loaded::load(&document, &schema).expect("the document loads");
//!
//! // Both slots name one node, so a change through either is a change to the value both
//! // read, and the node is marked shared for exactly that reason.
//! let left = loaded.resolve(&Path::root().key("left")).expect("it resolves");
//! let right = loaded.resolve(&Path::root().key("right")).expect("it resolves");
//! assert_eq!(left, right);
//! assert!(loaded.is_shared(left));
//! assert_eq!(loaded.node(left), &Node::Int(7));
//!
//! // A reference is written back as the reference it was, not expanded.
//! assert_eq!(loaded.to_value(), document);
//! ```

use std::collections::{BTreeMap, BTreeSet};

use crate::error::LoadError;
use crate::path::{Path, PathError, PathSegment};
use crate::reference::Reference;
use crate::schema::Schema;
use crate::value::Value;

mod load;
mod write;

/// The address of a node in a generation's arena.
///
/// An address is only meaningful within the generation that issued it, and a generation is
/// immutable for as long as it is held, so an address stays valid for as long as anything
/// can reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

impl NodeId {
    /// Returns the slot this node is stored in.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// What one slot holds: the node it names, and how the document spelled it.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    target: NodeId,
    spelling: Option<Box<Reference>>,
}

impl Slot {
    /// A slot that holds the value it was written with.
    pub(crate) const fn inlined(target: NodeId) -> Self {
        Self {
            target,
            spelling: None,
        }
    }

    /// A slot that was written as a reference to a node.
    pub(crate) fn referenced(spelling: Reference, target: NodeId) -> Self {
        Self {
            target,
            spelling: Some(Box::new(spelling)),
        }
    }

    /// Returns the node this slot names.
    pub const fn target(&self) -> NodeId {
        self.target
    }

    /// Borrows the reference this slot was written as, or `None` when it inlined its value.
    pub fn spelling(&self) -> Option<&Reference> {
        self.spelling.as_deref()
    }

    /// Returns whether this slot was written as a reference.
    pub const fn is_reference(&self) -> bool {
        self.spelling.is_some()
    }
}

/// One node of a loaded definition.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Absence of a value.
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    /// Named fields, closed by the schema that validated them.
    Struct(BTreeMap<String, Slot>),
    /// A variant tag plus the fields that variant carries.
    Enum {
        tag: String,
        fields: BTreeMap<String, Slot>,
    },
    /// An ordered sequence.
    List(Vec<Slot>),
    /// An open map of string keys.
    Dict(BTreeMap<String, Slot>),
}

impl Node {
    /// Borrows the named fields of this node, whether it is a struct, an enum or a dict.
    pub fn fields(&self) -> Option<&BTreeMap<String, Slot>> {
        match self {
            Self::Struct(fields) | Self::Dict(fields) | Self::Enum { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Borrows the elements of this node when it is a list.
    pub fn elements(&self) -> Option<&[Slot]> {
        match self {
            Self::List(elements) => Some(elements),
            _ => None,
        }
    }

    /// Returns the name of this kind of node, for an error message.
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Unit => "a unit",
            Self::Bool(_) => "a bool",
            Self::Int(_) => "an integer",
            Self::Float(_) => "a float",
            Self::String(_) => "a string",
            Self::Bytes(_) => "bytes",
            Self::Struct(_) => "a struct",
            Self::Enum { .. } => "an enum",
            Self::List(_) => "a list",
            Self::Dict(_) => "a dict",
        }
    }
}

/// A definition whose references are resolved: its nodes, its root, and what is shared.
pub struct Loaded {
    nodes: Vec<Node>,
    root: NodeId,
    shared: BTreeSet<NodeId>,
}

impl Loaded {
    /// Loads a document, resolving every reference its schema marked a slot for.
    ///
    /// # Invariants
    ///
    /// The schema decides which slots may be a reference, so loading reads a spelling only
    /// where the schema allows one and treats a value that happens to look like a reference
    /// elsewhere as the data it is.
    ///
    /// # Cycles
    ///
    /// A reference may name an ancestor, which makes a cycle. A cycle is a graph rather than
    /// a leak, because a generation owns its nodes and is freed whole, and it is written back
    /// as the spelling it was, so writing it terminates.
    ///
    /// ```
    /// use whiting_definition::{Loaded, Path, Reference, Schema, SchemaKind, Value};
    ///
    /// // A list whose first element is the list itself.
    /// let element = Schema::new(SchemaKind::Int).reference();
    /// let schema = Schema::new(SchemaKind::list(element));
    /// let document = Value::List(vec![
    ///     Reference::Internal(Path::root()).to_value(),
    ///     Value::Int(2),
    /// ]);
    ///
    /// let loaded = Loaded::load(&document, &schema).expect("a cycle loads");
    /// let first = loaded.resolve(&Path::root().index(0)).expect("it resolves");
    /// assert_eq!(first, loaded.root());
    /// assert!(loaded.is_shared(first));
    /// assert_eq!(loaded.to_value(), document);
    /// ```
    ///
    /// A cycle of references that holds no value at all is an error rather than a graph,
    /// because there is nothing for the cycle to be:
    ///
    /// ```
    /// # use whiting_definition::{Loaded, Path, Reference, Schema, SchemaKind, Value};
    /// let slot = Schema::new(SchemaKind::Int).reference();
    /// let schema = Schema::new(SchemaKind::structure([
    ///     ("left".to_owned(), slot.clone()),
    ///     ("right".to_owned(), slot.clone()),
    /// ]));
    /// let document = Value::structure([
    ///     (
    ///         "left".to_owned(),
    ///         Reference::Internal(Path::root().key("right")).to_value(),
    ///     ),
    ///     (
    ///         "right".to_owned(),
    ///         Reference::Internal(Path::root().key("left")).to_value(),
    ///     ),
    /// ]);
    /// assert!(Loaded::load(&document, &schema).is_err());
    /// ```
    pub fn load(value: &Value, schema: &Schema) -> Result<Self, LoadError> {
        load::load(value, schema)
    }

    /// Returns the node this definition starts at.
    pub const fn root(&self) -> NodeId {
        self.root
    }

    /// Borrows a node.
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.index()]
    }

    /// Returns how many nodes the definition holds.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether the definition holds no node beyond its root.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns whether a reference names this node.
    ///
    /// A shared node is written in place rather than swapped, because another slot reads it
    /// through a reference and a swap would leave that slot reading the node it replaced.
    pub fn is_shared(&self, id: NodeId) -> bool {
        self.shared.contains(&id)
    }

    /// Resolves a path to the node it names, following references as it goes.
    pub fn resolve(&self, path: &Path) -> Result<NodeId, LoadError> {
        let unresolved = |reason: PathError| LoadError::UnresolvedPath {
            path: path.clone(),
            reason,
        };
        let mut current = self.root;
        for segment in path.segments() {
            let node = self.node(current);
            current = match (segment, node) {
                (PathSegment::Key(name), node) if node.fields().is_some() => node
                    .fields()
                    .expect("the guard checked this")
                    .get(name)
                    .ok_or_else(|| unresolved(PathError::AbsentKey(name.clone())))?
                    .target(),
                (PathSegment::Index(index), Node::List(elements)) => elements
                    .get(*index)
                    .ok_or_else(|| {
                        unresolved(PathError::IndexOutOfBounds {
                            index: *index,
                            length: elements.len(),
                        })
                    })?
                    .target(),
                (PathSegment::Key(_), node) => {
                    return Err(unresolved(PathError::NotKeyed(node.kind_name())));
                }
                (PathSegment::Index(_), node) => {
                    return Err(unresolved(PathError::NotIndexed(node.kind_name())));
                }
            };
        }
        Ok(current)
    }
}
