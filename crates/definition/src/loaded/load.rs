//! Building a loaded definition: read the tree, then resolve what it points at.
//!
//! # Why two phases
//!
//! A reference may point forward, at a node the reader has not reached yet, and it may point
//! at an ancestor, which makes a cycle. So the document is read into a raw arena first, with
//! a slot that spelled a reference inside this document left unresolved but remembering its
//! spelling, and only then is each of those resolved by walking its path over the raw arena.
//!
//! A reference out of the document needs no second phase: nothing here can hold its target, so
//! it is resolved while the tree is being read, and its target is a node of the generation it
//! names.
//!
//! # Why resolving is a fixpoint
//!
//! One reference may name a path that runs *through* another reference, so a pass may be
//! unable to resolve a slot until another slot has been resolved. Resolving what can be
//! resolved and repeating terminates, because each round either resolves a slot or
//! establishes that the remainder point only at each other. A remainder like that is a
//! reference cycle holding no value at all, which is an error rather than a graph.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::error::LoadError;
use crate::loaded::store::{FileContent, Origin, Store, Wanted};
use crate::loaded::{Loaded, Node, NodeId, Slot, Target};
use crate::path::{Path, PathError, PathSegment};
use crate::reference::{FilePath, Reference};
use crate::schema::{Schema, SchemaKind};
use crate::value::Value;

/// A loaded definition is built by reading, and then by resolving what it points at.
///
/// The context is absent for a load that was given no store, in which case a reference out of
/// the document is an error rather than something to look up.
pub(super) fn load(
    value: &Value,
    schema: &Schema,
    context: Option<(&Origin, &dyn Store)>,
) -> Result<Loaded, LoadError> {
    let mut builder = Builder {
        raw: Vec::new(),
        pending: Vec::new(),
        context,
    };
    let root = builder.read(value, schema)?;
    builder.resolve(root)
}

/// A slot of the raw arena.
#[derive(Debug, Clone)]
enum RawSlot {
    /// The node this slot names.
    Node(usize),
    /// A reference inside this document, whose target this document's own nodes hold.
    Internal(Reference),
    /// A reference out of this document, resolved as it was read.
    Resolved {
        reference: Reference,
        target: Target,
    },
}

/// A node of the raw arena, which is a [`Node`] whose slots may still be unresolved.
#[derive(Debug, Clone)]
enum RawNode {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Struct(BTreeMap<String, RawSlot>),
    Enum {
        tag: String,
        fields: BTreeMap<String, RawSlot>,
    },
    List(Vec<RawSlot>),
    Dict(BTreeMap<String, RawSlot>),
}

/// Which slot of a node a reference was found in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SlotKey {
    /// A field of a struct, an enum's fields, or an entry of a dict.
    Key(String),
    /// An element of a list.
    Index(usize),
}

/// Reads a document into a raw arena and then resolves the references in it.
struct Builder<'context> {
    raw: Vec<RawNode>,
    /// Every slot that spelled a reference inside this document: the node holding it, which
    /// slot, and the spelling.
    pending: Vec<(usize, SlotKey, Reference)>,
    context: Option<(&'context Origin, &'context dyn Store)>,
}

impl Builder<'_> {
    /// Reads a value into a node, returning the address of the node it became.
    ///
    /// Children are read before the node that holds them, so a node's address is above all
    /// of its children's, and a reference recorded against a node is recorded once the node
    /// has an address to be recorded against.
    fn read(&mut self, value: &Value, schema: &Schema) -> Result<usize, LoadError> {
        let (node, found) = self.read_node(value, schema)?;
        let id = self.raw.len();
        self.raw.push(node);
        for (key, reference) in found {
            self.pending.push((id, key, reference));
        }
        Ok(id)
    }

    /// Reads the node a value is, with the references among its own slots.
    fn read_node(
        &mut self,
        value: &Value,
        schema: &Schema,
    ) -> Result<(RawNode, Vec<(SlotKey, Reference)>), LoadError> {
        let mut pending = Vec::new();
        let node = match value {
            Value::Unit => RawNode::Unit,
            Value::Bool(flag) => RawNode::Bool(*flag),
            Value::Int(number) => RawNode::Int(*number),
            Value::Float(number) => RawNode::Float(*number),
            Value::String(text) => RawNode::String(text.clone()),
            Value::Bytes(bytes) => RawNode::Bytes(bytes.clone()),
            Value::Struct(fields) => {
                let (read, found) = self.read_fields(fields, schema)?;
                pending.extend(found);
                RawNode::Struct(read)
            }
            Value::Dict(entries) => {
                let (read, found) = self.read_fields(entries, schema)?;
                pending.extend(found);
                RawNode::Dict(read)
            }
            Value::Enum { tag, fields } => {
                let (read, found) = self.read_fields(fields, variant_schema(schema, tag))?;
                pending.extend(found);
                RawNode::Enum {
                    tag: tag.clone(),
                    fields: read,
                }
            }
            Value::List(elements) => {
                let element_schema = element_schema(schema);
                let mut read = Vec::with_capacity(elements.len());
                for (position, element) in elements.iter().enumerate() {
                    let slot = self.read_slot(element, element_schema)?;
                    if let RawSlot::Internal(reference) = &slot {
                        pending.push((SlotKey::Index(position), reference.clone()));
                    }
                    read.push(slot);
                }
                RawNode::List(read)
            }
        };
        Ok((node, pending))
    }

    /// Reads one slot: the reference it spells, or the node it holds.
    ///
    /// # Invariants
    ///
    /// A schema marked as a reference makes its slot a union, so a value that spells a
    /// reference is read as one and anything else is read as a value of the declared kind.
    /// An unmarked slot is never read as a reference, however it is spelled.
    ///
    /// A reference inside this document is the only kind left unresolved, because it is the
    /// only kind whose target this document's own nodes hold. A reference out of it is
    /// resolved here, and its spelling travels with it.
    fn read_slot(&mut self, value: &Value, schema: &Schema) -> Result<RawSlot, LoadError> {
        if schema.is_reference() {
            if let Some(reference) = spelled(value) {
                return Ok(match &reference {
                    Reference::Internal(_) => RawSlot::Internal(reference),
                    Reference::External(path) => RawSlot::Resolved {
                        target: self.external(path)?,
                        reference,
                    },
                    Reference::File(path) => RawSlot::Resolved {
                        target: self.file(path, schema)?,
                        reference,
                    },
                });
            }
        }
        Ok(RawSlot::Node(self.read(value, schema)?))
    }

    /// Reads the named slots of a struct, a dict or an enum variant.
    fn read_fields(
        &mut self,
        fields: &BTreeMap<String, Value>,
        schema: &Schema,
    ) -> Result<(BTreeMap<String, RawSlot>, Vec<(SlotKey, Reference)>), LoadError> {
        let mut read = BTreeMap::new();
        let mut pending = Vec::new();
        for (name, value) in fields {
            let slot = self.read_slot(value, field_schema(schema, name))?;
            if let RawSlot::Internal(reference) = &slot {
                pending.push((SlotKey::Key(name.clone()), reference.clone()));
            }
            read.insert(name.clone(), slot);
        }
        Ok((read, pending))
    }

    /// Resolves a path from the registry root, whose first segment names a definition.
    fn external(&self, path: &Path) -> Result<Target, LoadError> {
        let (_, store) = self.context.ok_or_else(|| self.needs_store(path))?;
        let (name, inside) = path
            .segments()
            .split_first()
            .ok_or_else(|| LoadError::Store {
                reason: "an external reference names no definition".to_owned(),
            })?;
        let PathSegment::Key(name) = name else {
            return Err(LoadError::Store {
                reason: "an external reference starts with the name of a definition".to_owned(),
            });
        };
        let generation = store.definition(name)?;
        let inside = Path::new(inside.iter().cloned());
        match generation.resolve(&inside)? {
            Target::Local(node) => Ok(Target::Foreign(generation, node)),
            Target::Foreign(other, node) => Ok(Target::Foreign(other, node)),
        }
    }

    /// Resolves a file: its bytes when the slot wants an asset, its document otherwise.
    ///
    /// A document a file held is a document in its own right, so it is loaded as a generation
    /// of its own and the slot refers to it. That is what makes a path inside it relative to
    /// the file rather than to the document that named the file.
    fn file(&mut self, path: &FilePath, schema: &Schema) -> Result<Target, LoadError> {
        let (origin, store) = self.context.ok_or_else(|| self.needs_store(path))?;
        let wanted = match &schema.kind {
            SchemaKind::Bytes => Wanted::Asset,
            _ => Wanted::Document,
        };
        match (store.file(origin, path, wanted)?, wanted) {
            (FileContent::Bytes(bytes), Wanted::Asset) => {
                let id = self.raw.len();
                self.raw.push(RawNode::Bytes(bytes));
                Ok(Target::Local(NodeId(id)))
            }
            (FileContent::Document(document), Wanted::Document) => {
                let inner = schema.clone().without_reference();
                let generation =
                    Loaded::load_in(&document, &inner, &Origin::File(path.clone()), store)?;
                let root = generation.root();
                Ok(Target::Foreign(Rc::new(generation), root))
            }
            (FileContent::Bytes(_), Wanted::Document) => Err(LoadError::Store {
                reason: format!("`{path}` holds bytes, and this slot wants a document"),
            }),
            (FileContent::Document(_), Wanted::Asset) => Err(LoadError::Store {
                reason: format!("`{path}` holds a document, and this slot wants bytes"),
            }),
        }
    }

    /// Builds the error for a reference that needs somewhere to look and has none.
    fn needs_store(&self, reference: impl std::fmt::Debug) -> LoadError {
        LoadError::NoStore {
            reference: format!("{reference:?}"),
        }
    }

    /// Resolves every pending reference, then materialises the loaded definition.
    fn resolve(&mut self, root: usize) -> Result<Loaded, LoadError> {
        let mut resolved: BTreeMap<(usize, SlotKey), usize> = BTreeMap::new();
        let mut waiting = std::mem::take(&mut self.pending);

        // Resolve what can be resolved until a round resolves nothing, which is what breaks a
        // cycle of references that names no value.
        while !waiting.is_empty() {
            let mut progress = false;
            let mut still_waiting = Vec::new();
            for (parent, key, reference) in waiting {
                let Reference::Internal(path) = &reference else {
                    unreachable!("a reference out of the document is resolved as it is read");
                };
                match self.walk(root, path)? {
                    Some(target) => {
                        resolved.insert((parent, key), target);
                        progress = true;
                    }
                    None => still_waiting.push((parent, key, reference)),
                }
            }
            waiting = still_waiting;
            if !progress && !waiting.is_empty() {
                return Err(LoadError::ValueLessCycle {
                    references: waiting
                        .iter()
                        .map(|(_, _, reference)| format!("{reference:?}"))
                        .collect(),
                });
            }
        }

        self.materialise(root, &resolved)
    }

    /// Walks a path over the raw arena, which is `None` while it runs through a slot that is
    /// not resolved yet.
    fn walk(&self, root: usize, path: &Path) -> Result<Option<usize>, LoadError> {
        let unresolved = |reason: PathError| LoadError::UnresolvedPath {
            path: path.clone(),
            reason,
        };
        let mut current = Some(root);
        for segment in path.segments() {
            let Some(id) = current else {
                return Ok(None);
            };
            let slot = match (segment, &self.raw[id]) {
                (PathSegment::Key(name), RawNode::Struct(fields))
                | (PathSegment::Key(name), RawNode::Dict(fields))
                | (PathSegment::Key(name), RawNode::Enum { fields, .. }) => fields
                    .get(name)
                    .ok_or_else(|| unresolved(PathError::AbsentKey(name.clone())))?,
                (PathSegment::Index(index), RawNode::List(elements)) => {
                    elements.get(*index).ok_or_else(|| {
                        unresolved(PathError::IndexOutOfBounds {
                            index: *index,
                            length: elements.len(),
                        })
                    })?
                }
                (PathSegment::Key(_), node) => {
                    return Err(unresolved(PathError::NotKeyed(node.kind_name())));
                }
                (PathSegment::Index(_), node) => {
                    return Err(unresolved(PathError::NotIndexed(node.kind_name())));
                }
            };
            current = match slot {
                RawSlot::Node(id) => Some(*id),
                RawSlot::Internal(_) | RawSlot::Resolved { .. } => None,
            };
        }
        Ok(current)
    }

    /// Turns the raw arena into nodes whose every slot names a node.
    fn materialise(
        &self,
        root: usize,
        resolved: &BTreeMap<(usize, SlotKey), usize>,
    ) -> Result<Loaded, LoadError> {
        let mut nodes = Vec::with_capacity(self.raw.len());
        let mut shared = BTreeSet::new();
        for (index, raw) in self.raw.iter().enumerate() {
            let node = match raw {
                RawNode::Unit => Node::Unit,
                RawNode::Bool(flag) => Node::Bool(*flag),
                RawNode::Int(number) => Node::Int(*number),
                RawNode::Float(number) => Node::Float(*number),
                RawNode::String(text) => Node::String(text.clone()),
                RawNode::Bytes(bytes) => Node::Bytes(bytes.clone()),
                RawNode::Struct(fields) => {
                    Node::Struct(self.materialise_fields(index, fields, resolved, &mut shared)?)
                }
                RawNode::Dict(entries) => {
                    Node::Dict(self.materialise_fields(index, entries, resolved, &mut shared)?)
                }
                RawNode::Enum { tag, fields } => Node::Enum {
                    tag: tag.clone(),
                    fields: self.materialise_fields(index, fields, resolved, &mut shared)?,
                },
                RawNode::List(elements) => {
                    let mut read = Vec::with_capacity(elements.len());
                    for (position, slot) in elements.iter().enumerate() {
                        let key = SlotKey::Index(position);
                        read.push(self.materialise_slot(
                            index,
                            &key,
                            slot,
                            resolved,
                            &mut shared,
                        )?);
                    }
                    Node::List(read)
                }
            };
            nodes.push(node);
        }
        Ok(Loaded {
            nodes,
            root: NodeId(root),
            shared,
        })
    }

    /// Turns the named slots of a raw node into slots that name a node.
    fn materialise_fields(
        &self,
        index: usize,
        fields: &BTreeMap<String, RawSlot>,
        resolved: &BTreeMap<(usize, SlotKey), usize>,
        shared: &mut BTreeSet<NodeId>,
    ) -> Result<BTreeMap<String, Slot>, LoadError> {
        let mut read = BTreeMap::new();
        for (name, slot) in fields {
            let key = SlotKey::Key(name.clone());
            read.insert(
                name.clone(),
                self.materialise_slot(index, &key, slot, resolved, shared)?,
            );
        }
        Ok(read)
    }

    /// Turns one raw slot into a slot that names a node, marking a reference's target shared.
    ///
    /// A reference into another generation marks nothing here: the node it names belongs to a
    /// document that is not this one, and a delta of this document cannot reach it.
    fn materialise_slot(
        &self,
        index: usize,
        key: &SlotKey,
        slot: &RawSlot,
        resolved: &BTreeMap<(usize, SlotKey), usize>,
        shared: &mut BTreeSet<NodeId>,
    ) -> Result<Slot, LoadError> {
        match slot {
            RawSlot::Node(id) => Ok(Slot::inlined(Target::Local(NodeId(*id)))),
            RawSlot::Resolved { reference, target } => {
                if let Target::Local(id) = target {
                    shared.insert(*id);
                }
                Ok(Slot::referenced(reference.clone(), target.clone()))
            }
            RawSlot::Internal(reference) => {
                let target =
                    resolved
                        .get(&(index, key.clone()))
                        .ok_or_else(|| LoadError::Unresolved {
                            reference: format!("{reference:?}"),
                        })?;
                let id = NodeId(*target);
                shared.insert(id);
                Ok(Slot::referenced(reference.clone(), Target::Local(id)))
            }
        }
    }
}

impl RawNode {
    /// Returns the name of this kind of node, for an error message.
    const fn kind_name(&self) -> &'static str {
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

/// Returns the reference a value spells, when it spells one at all.
fn spelled(value: &Value) -> Option<Reference> {
    let tag = value.variant_tag()?;
    if !Reference::is_tag(tag) {
        return None;
    }
    Reference::from_value(value).ok()
}

/// Returns the schema a field is verified against, which is an open one when the schema is.
fn field_schema<'schema>(schema: &'schema Schema, name: &str) -> &'schema Schema {
    match &schema.kind {
        SchemaKind::Struct { fields } => fields.get(name).unwrap_or_else(|| open_schema()),
        SchemaKind::Enum { variants } => variants.get(name).unwrap_or_else(|| open_schema()),
        SchemaKind::Dict { .. } | SchemaKind::Any => open_schema(),
        _ => schema,
    }
}

/// Returns the schema a variant's fields are verified against.
fn variant_schema<'schema>(schema: &'schema Schema, tag: &str) -> &'schema Schema {
    match &schema.kind {
        SchemaKind::Enum { variants } => variants.get(tag).unwrap_or_else(|| open_schema()),
        _ => schema,
    }
}

/// Returns the schema an element of a list is verified against.
fn element_schema(schema: &Schema) -> &Schema {
    match &schema.kind {
        SchemaKind::List { element } => element.as_ref(),
        _ => schema,
    }
}

/// A schema that accepts anything, for a slot whose schema does not describe it.
fn open_schema() -> &'static Schema {
    static OPEN: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    OPEN.get_or_init(|| Schema::new(SchemaKind::Any))
}
