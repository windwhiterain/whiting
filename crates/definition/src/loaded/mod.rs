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
//! reading the node it replaced.
//!
//! # Where a reference can point
//!
//! A path spelled inside a document is relative to that document's root. A path spelled
//! against the registry starts with the name of a definition and continues inside it, so the
//! address of anything in a document is `(definition, path)`. A file path is absolute, or
//! relative to the location of the document that spells it, which is the document itself
//! rather than the registry: a definition stored in `mesh/quad.ron` that names `points.bin`
//! means `mesh/points.bin`.
//!
//! # Invariants
//!
//! - A slot always names a node. Nothing in a loaded definition is unresolved, so no access
//!   can fail for want of a target.
//! - A node's children are slots, and every container is a slot apiece, so a document is a
//!   tree of slots with references making it a graph. A cycle is therefore expressible, and
//!   is nothing to collect: a generation owns its nodes and is freed whole.
//! - A reference into another generation keeps that generation alive, so a definition
//!   referenced by a live one is loaded and stays loaded. Definitions therefore refer to each
//!   other in a graph rather than in a cycle, which is the store's invariant: a definition
//!   already being loaded is refused rather than loaded again, or two generations would hold
//!   each other alive.
//! - A slot keeps the spelling it was written with, so a loaded definition writes back as the
//!   document it was read from rather than with every reference expanded.
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
//! assert_eq!(left.node(), right.node());
//! assert!(left.generation().is_none(), "a path inside this document stays here");
//! assert!(loaded.is_shared(left.node()));
//! assert_eq!(left.node_of(&loaded), &Node::Int(7));
//!
//! // A reference is written back as the reference it was, not expanded.
//! assert_eq!(loaded.to_value(), document);
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::error::LoadError;
use crate::path::{Path, PathError, PathSegment};
use crate::schema::Schema;
use crate::value::Value;

mod load;
mod patch;
mod store;
mod write;

pub use store::{FileContent, Origin, RegistryStore, Store, Wanted};

/// The address of a node in a generation's arena.
///
/// An address is only meaningful within the generation that issued it, and a generation is
/// kept alive by everything that refers to it, so an address stays valid for as long as
/// anything can reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

impl NodeId {
    /// Returns the slot this node is stored in.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// The node a reference points at, and the generation that owns it.
///
/// A reference inside a document points at a node of that document, and a reference out of it
/// points at a node of another generation, which this one keeps alive for as long as it holds
/// the reference.
#[derive(Clone)]
pub enum Target {
    /// A node of the generation holding the reference.
    Local(NodeId),
    /// A node of another generation.
    Foreign(Rc<Loaded>, NodeId),
}

impl std::fmt::Debug for Target {
    /// Writes the address, and for a node of another generation its size rather than its
    /// contents, because a generation is a graph and could be this one.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(id) => write!(formatter, "Local({})", id.index()),
            Self::Foreign(generation, id) => write!(
                formatter,
                "Foreign(node {} of a generation of {})",
                id.index(),
                generation.len()
            ),
        }
    }
}

impl PartialEq for Target {
    /// Compares addresses, and generations by identity rather than by contents.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Local(left), Self::Local(right)) => left == right,
            (Self::Foreign(left, left_id), Self::Foreign(right, right_id)) => {
                Rc::ptr_eq(left, right) && left_id == right_id
            }
            _ => false,
        }
    }
}

impl Target {
    /// Returns the address of the node this target names.
    pub const fn node(&self) -> NodeId {
        match self {
            Self::Local(id) | Self::Foreign(_, id) => *id,
        }
    }

    /// Borrows the generation this target lives in, or `None` when it is this one.
    pub fn generation(&self) -> Option<&Rc<Loaded>> {
        match self {
            Self::Local(_) => None,
            Self::Foreign(generation, _) => Some(generation),
        }
    }

    /// Returns whether this target is a node of the generation the reference is in.
    pub const fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    /// Borrows the node this target names, given the generation a local target belongs to.
    pub fn node_of<'target>(&'target self, home: &'target Loaded) -> &'target Node {
        match self {
            Self::Local(id) => home.node(*id),
            Self::Foreign(generation, id) => generation.node(*id),
        }
    }
}

/// What one slot holds: the node it names, and how the document spelled it.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    target: Target,
    spelling: Option<Box<crate::reference::Reference>>,
}

impl Slot {
    /// A slot that holds the value it was written with.
    pub(crate) const fn inlined(target: Target) -> Self {
        Self {
            target,
            spelling: None,
        }
    }

    /// A slot that was written as a reference to a node.
    pub(crate) fn referenced(spelling: crate::reference::Reference, target: Target) -> Self {
        Self {
            target,
            spelling: Some(Box::new(spelling)),
        }
    }

    /// Borrows the node this slot names.
    pub const fn target(&self) -> &Target {
        &self.target
    }

    /// Borrows the reference this slot was written as, or `None` when it inlined its value.
    pub fn spelling(&self) -> Option<&crate::reference::Reference> {
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
    /// Loads a document that holds everything it points at.
    ///
    /// This is the load for a document whose references point inside itself. A document that
    /// may name another definition or a file needs [`Loaded::load_in`], which is given
    /// somewhere to find them.
    pub fn load(value: &Value, schema: &Schema) -> Result<Self, LoadError> {
        load::load(value, schema, None)
    }

    /// Loads a document, resolving what it points at through a store.
    ///
    /// `origin` is what the document is: a definition the registry holds, or a file, which is
    /// what a relative path inside it resolves against.
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
    /// assert_eq!(first.node(), loaded.root());
    /// assert!(loaded.is_shared(first.node()));
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
    pub fn load_in(
        value: &Value,
        schema: &Schema,
        origin: &Origin,
        store: &dyn Store,
    ) -> Result<Self, LoadError> {
        load::load(value, schema, Some((origin, store)))
    }

    /// # Pointing out of the document
    ///
    /// A document that names another definition, or a file, is loaded through a store. An
    /// external path starts with the name of the definition and continues inside it, and a
    /// file path is relative to the document that spells it. A file wanted as an asset becomes
    /// the bytes it holds, and one wanted as a document becomes a generation of its own, which
    /// is what makes a path inside it relative to the file.
    ///
    /// ```
    /// use std::rc::Rc;
    ///
    /// use whiting_definition::{
    ///     DefinitionIdentifier, FileContent, FilePath, LoadError, Loaded, Node, Origin, Path,
    ///     Reference, Schema, SchemaKind, Store, Value, Wanted,
    /// };
    ///
    /// // A store holding one definition, and handing back bytes for any file.
    /// struct Held {
    ///     mesh: Value,
    ///     mesh_schema: Schema,
    /// }
    ///
    /// impl Store for Held {
    ///     fn definition(&self, name: &str) -> Result<Rc<Loaded>, LoadError> {
    ///         if name != "mesh" {
    ///             return Err(LoadError::Store {
    ///                 reason: format!("there is no definition `{name}`"),
    ///             });
    ///         }
    ///         Ok(Rc::new(Loaded::load(&self.mesh, &self.mesh_schema)?))
    ///     }
    ///
    ///     fn file(
    ///         &self,
    ///         _from: &Origin,
    ///         path: &FilePath,
    ///         _wanted: Wanted,
    ///     ) -> Result<FileContent, LoadError> {
    ///         Ok(FileContent::Bytes(path.as_str().as_bytes().to_vec()))
    ///     }
    /// }
    ///
    /// let mesh_schema = Schema::new(SchemaKind::structure([(
    ///     "points".to_owned(),
    ///     Schema::new(SchemaKind::list(Schema::new(SchemaKind::Int))),
    /// )]));
    /// let store = Held {
    ///     mesh: Value::structure([(
    ///         "points".to_owned(),
    ///         Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
    ///     )]),
    ///     mesh_schema,
    /// };
    ///
    /// // A scene naming the other definition, a file, and a value of its own.
    /// let schema = Schema::new(SchemaKind::structure([
    ///     (
    ///         "mesh".to_owned(),
    ///         Schema::new(SchemaKind::list(Schema::new(SchemaKind::Int))).reference(),
    ///     ),
    ///     ("asset".to_owned(), Schema::new(SchemaKind::Bytes).reference()),
    ///     ("weight".to_owned(), Schema::new(SchemaKind::Float)),
    /// ]));
    /// let document = Value::structure([
    ///     (
    ///         "mesh".to_owned(),
    ///         Reference::External(Path::root().key("mesh").key("points")).to_value(),
    ///     ),
    ///     (
    ///         "asset".to_owned(),
    ///         Reference::File(FilePath::new("points.bin")).to_value(),
    ///     ),
    ///     ("weight".to_owned(), Value::Float(2.5)),
    /// ]);
    ///
    /// let origin = Origin::Definition(DefinitionIdentifier::new("scene"));
    /// let scene = Loaded::load_in(&document, &schema, &origin, &store).expect("it loads");
    ///
    /// // The definition it named is another generation, which the scene now keeps alive.
    /// let mesh = scene.resolve(&Path::root().key("mesh")).expect("it resolves");
    /// let generation = mesh.generation().expect("it left this document");
    /// let again = scene.resolve(&Path::root().key("mesh")).expect("it resolves");
    /// assert!(Rc::ptr_eq(
    ///     generation,
    ///     again.generation().expect("it left this document")
    /// ));
    ///
    /// // The file it wanted as a value of bytes became those bytes, in this document.
    /// let asset = scene.resolve(&Path::root().key("asset")).expect("it resolves");
    /// assert!(asset.generation().is_none());
    /// match asset.node_of(&scene) {
    ///     Node::Bytes(bytes) => assert_eq!(bytes, b"points.bin"),
    ///     other => panic!("a file wanted as an asset is bytes, and this is {other:?}"),
    /// }
    ///
    /// // Both references are written back as the references they were.
    /// assert_eq!(scene.to_value(), document);
    /// ```

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

    /// Returns the path of the first reference a document spells, when it spells one.
    ///
    /// # Invariants
    ///
    /// The schema decides which slots may hold a reference, so a value that looks like a
    /// reference in an unmarked slot is the data it is, and a document that only inlines its
    /// values has none. This is what tells whether a document's paths mean anything before it is
    /// loaded: a path that runs through a reference names the node the reference points at, and
    /// only a loaded generation holds those.
    pub fn spelled_reference(value: &Value, schema: &Schema) -> Option<Path> {
        find_reference(value, schema, &Path::root())
    }

    /// Returns the marked slot a path reaches or runs through that holds a reference.
    ///
    /// # Why the answer is per path
    ///
    /// A document may name something and still be patched where it names nothing: a scene whose
    /// mesh is another definition can have an instance moved without touching the name. Only a path
    /// that arrives at a reference, or descends through one, is ambiguous on the serialized form,
    /// which is where a reference is a spelling rather than the node it names.
    pub fn crossed_reference(value: &Value, schema: &Schema, path: &Path) -> Option<Path> {
        let mut at = Path::root();
        let mut current = value;
        let mut current_schema = schema;
        if current_schema.is_reference() && load::spelled(current).is_some() {
            return Some(at);
        }
        for segment in path.segments() {
            let container = current;
            current = match (segment, current) {
                (PathSegment::Key(name), Value::Struct(fields))
                | (PathSegment::Key(name), Value::Dict(fields))
                | (PathSegment::Key(name), Value::Enum { fields, .. }) => fields.get(name)?,
                (PathSegment::Index(index), Value::List(elements)) => elements.get(*index)?,
                _ => return None,
            };
            current_schema = match (segment, current_schema, container) {
                (PathSegment::Key(name), schema, Value::Enum { tag, .. }) => {
                    load::field_schema(load::variant_schema(schema, tag), name)
                }
                (PathSegment::Key(name), schema, _) => load::field_schema(schema, name),
                (PathSegment::Index(_), schema, _) => load::element_schema(schema),
            };
            at = at.child(segment.clone());
            if current_schema.is_reference() && load::spelled(current).is_some() {
                return Some(at);
            }
        }
        None
    }

    /// Resolves a path to the node it names, following references as it goes.
    ///
    /// A path that runs out of this document into another one answers with a target in that
    /// generation, which this one holds alive for as long as the reference does.
    pub fn resolve(&self, path: &Path) -> Result<Target, LoadError> {
        let unresolved = |reason: PathError| LoadError::UnresolvedPath {
            path: path.clone(),
            reason,
        };
        let mut current = Target::Local(self.root);
        for segment in path.segments() {
            let node = current.node_of(self);
            current = match (segment, node) {
                (PathSegment::Key(name), node) if node.fields().is_some() => node
                    .fields()
                    .expect("the guard checked this")
                    .get(name)
                    .ok_or_else(|| unresolved(PathError::AbsentKey(name.clone())))?
                    .target()
                    .clone(),
                (PathSegment::Index(index), Node::List(elements)) => elements
                    .get(*index)
                    .ok_or_else(|| {
                        unresolved(PathError::IndexOutOfBounds {
                            index: *index,
                            length: elements.len(),
                        })
                    })?
                    .target()
                    .clone(),
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

/// Walks a document for the first reference it spells where the schema marks a slot.
fn find_reference(value: &Value, schema: &Schema, at: &Path) -> Option<Path> {
    if schema.is_reference() && load::spelled(value).is_some() {
        return Some(at.clone());
    }
    match value {
        Value::Struct(fields) | Value::Dict(fields) => fields.iter().find_map(|(name, value)| {
            find_reference(
                value,
                load::field_schema(schema, name),
                &at.key(name.clone()),
            )
        }),
        Value::Enum { tag, fields } => {
            let variant = load::variant_schema(schema, tag);
            fields.iter().find_map(|(name, value)| {
                find_reference(
                    value,
                    load::field_schema(variant, name),
                    &at.key(name.clone()),
                )
            })
        }
        Value::List(elements) => elements.iter().enumerate().find_map(|(index, value)| {
            find_reference(value, load::element_schema(schema), &at.index(index))
        }),
        _ => None,
    }
}
