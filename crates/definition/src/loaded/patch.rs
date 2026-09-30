//! Patching a loaded generation, which is where a delta's paths mean something.
//!
//! # Why a delta is applied here and not to the serialized form
//!
//! A path is resolved transparently, so a path that runs through a reference names the node
//! the reference points at. The serialized form holds spellings rather than the nodes they
//! name, so a path reaching through one has nothing to arrive at; only a loaded generation has
//! both a slot and its target, and only there does the path mean what it says.
//!
//! # Why a patch writes in place
//!
//! A node another slot also names is not swapped, because every slot that named it would go on
//! reading the node it replaced. The patch writes into the node itself, so a change through one
//! reference is a change to the value every referrer reads — which is what transparent
//! traversal promises, and it holds for a reference from another document too.
//!
//! # Why a patch cannot run out of the document
//!
//! A delta is against one definition at one version, so a path that arrives in another
//! definition names a version this delta never saw. That is refused rather than applied.
//!
//! # Atomicity
//!
//! Everything that can fail happens while planning: the paths are resolved, the target's kind
//! and bounds are checked, and the content a patch introduces is read into nodes. Writing then
//! cannot fail, so a patch either applies completely or leaves the document as it was. Reading
//! content pushes nodes into the arena, so a patch that fails may leave nodes nothing reaches;
//! they are unreachable rather than visible, and a generation is freed whole anyway.

use std::collections::BTreeMap;

use crate::delta::{Appended, PrimitiveDelta, Removed};
use crate::error::DeltaError;
use crate::loaded::load::{element_schema, field_schema, spelled, variant_schema};
use crate::loaded::{Loaded, Node, NodeId, Slot, Target};
use crate::path::{Path, PathSegment};
use crate::reference::Reference;
use crate::schema::{Schema, SchemaKind};
use crate::value::Value;

impl Loaded {
    /// Applies operations to this generation, writing into the nodes they name.
    ///
    /// The schema is what says which slots may hold a reference, so the content a patch
    /// introduces is read with the same rules a load uses: a reference the content spells is
    /// resolved, and anything else is a value of the declared kind.
    ///
    /// The returned operations undo what was applied, keyed by the path each was applied at.
    /// # Invariants
    ///
    /// Undo is a stack, so an inverse is applied before the inverse of whatever followed it.
    /// That is what lets an inverse name the path it was applied at rather than the node it
    /// reached: a reference the patch ran through is restored before the change that ran
    /// through it is undone.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// use whiting_definition::{
    ///     Loaded, Node, Path, PrimitiveDelta, Reference, Schema, SchemaKind, Value,
    /// };
    ///
    /// let slot = Schema::new(SchemaKind::Int).reference();
    /// let schema = Schema::new(SchemaKind::structure([
    ///     ("left".to_owned(), slot.clone()),
    ///     ("right".to_owned(), slot.clone()),
    /// ]));
    /// let document = Value::structure([
    ///     ("left".to_owned(), Value::Int(7)),
    ///     (
    ///         "right".to_owned(),
    ///         Reference::Internal(Path::root().key("left")).to_value(),
    ///     ),
    /// ]);
    /// let mut loaded = Loaded::load(&document, &schema).expect("it loads");
    ///
    /// // A patch through either slot writes the node both of them name.
    /// let mut operations = BTreeMap::new();
    /// operations.insert(
    ///     Path::root().key("right"),
    ///     PrimitiveDelta::Replace(Value::Int(9)),
    /// );
    /// let inverse = loaded.patch(&operations, &schema).expect("it applies");
    ///
    /// let left = loaded.resolve(&Path::root().key("left")).expect("it resolves");
    /// assert_eq!(loaded.node(left.node()), &Node::Int(9));
    ///
    /// // The inverse restores it, and the document still writes back with its spelling
    /// // rather than having lost it to the patch.
    /// assert_eq!(
    ///     inverse.get(&Path::root().key("right")),
    ///     Some(&PrimitiveDelta::Replace(Value::Int(7)))
    /// );
    /// loaded.patch(&inverse, &schema).expect("it applies");
    /// assert_eq!(loaded.node(left.node()), &Node::Int(7));
    /// assert_eq!(loaded.to_value(), document);
    /// ```
    pub fn patch(
        &mut self,
        operations: &BTreeMap<Path, PrimitiveDelta>,
        schema: &Schema,
    ) -> Result<BTreeMap<Path, PrimitiveDelta>, DeltaError> {
        let planned = self.plan(operations, schema)?;
        Ok(self.write(planned))
    }

    /// Resolves a path that must stay in this document.
    fn local(&self, path: &Path) -> Result<NodeId, DeltaError> {
        match self
            .resolve(path)
            .map_err(|reason| DeltaError::UnresolvedPath {
                path: path.clone(),
                reason: reason.to_string(),
            })? {
            Target::Local(node) => Ok(node),
            Target::Foreign(..) => Err(DeltaError::InvalidOperation {
                path: path.clone(),
                reason: "a delta cannot reach into another definition".to_owned(),
            }),
        }
    }

    /// Resolves, checks and reads everything a patch needs, so that writing cannot fail.
    fn plan<'operation>(
        &mut self,
        operations: &'operation BTreeMap<Path, PrimitiveDelta>,
        schema: &Schema,
    ) -> Result<Vec<Planned<'operation>>, DeltaError> {
        let mut planned = Vec::with_capacity(operations.len());
        for (path, primitive) in operations {
            let node = self.local(path)?;
            let at = schema_at(schema, path);
            let action = match primitive {
                PrimitiveDelta::Replace(value) => {
                    Action::Replace(self.read(value, &at.clone().without_reference(), path)?)
                }
                PrimitiveDelta::Append(Appended::Values { index, values }) => {
                    let element = element_schema(&at);
                    let mut read = Vec::with_capacity(values.len());
                    for value in values {
                        read.push(self.read(value, element, path)?);
                    }
                    let length = self
                        .elements(node)
                        .ok_or_else(|| self.not_a_list(path))?
                        .len();
                    if *index > length {
                        return Err(self.beyond_the_end(path));
                    }
                    Action::Values {
                        index: *index,
                        nodes: read,
                    }
                }
                PrimitiveDelta::Append(Appended::Entries { entries }) => {
                    let value_schema = match &at.kind {
                        SchemaKind::Dict { value } => value.as_ref(),
                        _ => &at,
                    };
                    let mut read = Vec::with_capacity(entries.len());
                    for (name, value) in entries {
                        read.push((name.clone(), self.read(value, value_schema, path)?));
                    }
                    let map = self.entries(node).ok_or_else(|| self.not_a_dict(path))?;
                    if let Some(name) = read.iter().find(|(name, _)| map.contains_key(name)) {
                        return Err(DeltaError::InvalidOperation {
                            path: path.clone(),
                            reason: format!("the entry `{}` is already present", name.0),
                        });
                    }
                    Action::Entries { entries: read }
                }
                PrimitiveDelta::Remove(Removed::Indices { start, count }) => {
                    let length = self
                        .elements(node)
                        .ok_or_else(|| self.not_a_list(path))?
                        .len();
                    let end = start.checked_add(*count).filter(|end| *end <= length);
                    if end.is_none() {
                        return Err(self.beyond_the_end(path));
                    }
                    Action::Indices {
                        start: *start,
                        count: *count,
                    }
                }
                PrimitiveDelta::Remove(Removed::Keys { keys }) => {
                    let map = self.entries(node).ok_or_else(|| self.not_a_dict(path))?;
                    if let Some(name) = keys.iter().find(|name| !map.contains_key(*name)) {
                        return Err(DeltaError::InvalidOperation {
                            path: path.clone(),
                            reason: format!("the key `{name}` is absent"),
                        });
                    }
                    Action::Keys { keys: keys.clone() }
                }
            };
            planned.push(Planned { path, node, action });
        }
        Ok(planned)
    }

    /// Writes the planned changes and returns what undoes them.
    fn write(&mut self, planned: Vec<Planned<'_>>) -> BTreeMap<Path, PrimitiveDelta> {
        let mut inverse = BTreeMap::new();
        for Planned { path, node, action } in planned {
            let undo = match action {
                Action::Replace(new) => {
                    let previous = self.write_node(node);
                    self.nodes[node.index()] =
                        std::mem::replace(&mut self.nodes[new.index()], Node::Unit);
                    PrimitiveDelta::Replace(previous)
                }
                Action::Values { index, nodes } => {
                    let count = nodes.len();
                    if let Node::List(elements) = &mut self.nodes[node.index()] {
                        for (offset, node) in nodes.into_iter().enumerate() {
                            elements.insert(index + offset, Slot::inlined(Target::Local(node)));
                        }
                    }
                    PrimitiveDelta::remove_indices(index, count)
                }
                Action::Entries { entries } => {
                    let names = entries.iter().map(|(name, _)| name.clone());
                    let undo = PrimitiveDelta::remove_keys(names);
                    if let Node::Dict(map) = &mut self.nodes[node.index()] {
                        for (name, value) in entries {
                            map.insert(name, Slot::inlined(Target::Local(value)));
                        }
                    }
                    undo
                }
                Action::Indices { start, count } => {
                    let mut removed = Vec::with_capacity(count);
                    if let Node::List(elements) = &mut self.nodes[node.index()] {
                        for _ in 0..count {
                            removed.push(elements.remove(start));
                        }
                    }
                    let values = removed
                        .iter()
                        .map(|slot| self.write_slot(slot))
                        .collect::<Vec<Value>>();
                    PrimitiveDelta::append_values(start, values)
                }
                Action::Keys { keys } => {
                    let mut taken = Vec::new();
                    if let Some(map) = self.map_mut(node) {
                        for key in keys {
                            if let Some(slot) = map.remove(&key) {
                                taken.push((key, slot));
                            }
                        }
                    }
                    let removed = taken
                        .into_iter()
                        .map(|(key, slot)| (key, self.write_slot(&slot)))
                        .collect::<Vec<(String, Value)>>();
                    PrimitiveDelta::append_entries(removed)
                }
            };
            inverse.insert(path.clone(), undo);
        }
        inverse
    }

    /// Borrows the elements of a node when it is a list.
    fn elements(&self, node: NodeId) -> Option<&[Slot]> {
        self.node(node).elements()
    }

    /// Borrows the named slots of a node when it has any.
    fn entries(&self, node: NodeId) -> Option<&BTreeMap<String, Slot>> {
        self.node(node).fields()
    }

    /// Borrows the named slots of a node mutably, whether it is a struct, an enum or a dict.
    fn map_mut(&mut self, node: NodeId) -> Option<&mut BTreeMap<String, Slot>> {
        match &mut self.nodes[node.index()] {
            Node::Struct(fields) | Node::Dict(fields) | Node::Enum { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Reads a value into this document as a node the patch introduces.
    ///
    /// The content a patch introduces is read against this document, so a reference it spells
    /// resolves here and must stay here: a path out of the document needs a load, because
    /// resolving it needs somewhere to look.
    fn read(&mut self, value: &Value, schema: &Schema, at: &Path) -> Result<NodeId, DeltaError> {
        let node = self.read_node(value, schema, at)?;
        self.nodes.push(node);
        Ok(NodeId(self.nodes.len() - 1))
    }

    /// Reads the node a value is.
    fn read_node(&mut self, value: &Value, schema: &Schema, at: &Path) -> Result<Node, DeltaError> {
        Ok(match value {
            Value::Unit => Node::Unit,
            Value::Bool(flag) => Node::Bool(*flag),
            Value::Int(number) => Node::Int(*number),
            Value::Float(number) => Node::Float(*number),
            Value::String(text) => Node::String(text.clone()),
            Value::Bytes(bytes) => Node::Bytes(bytes.clone()),
            Value::Struct(fields) => Node::Struct(self.read_fields(fields, schema, at)?),
            Value::Dict(entries) => Node::Dict(self.read_fields(entries, schema, at)?),
            Value::Enum { tag, fields } => Node::Enum {
                tag: tag.clone(),
                fields: self.read_fields(fields, variant_schema(schema, tag), at)?,
            },
            Value::List(elements) => {
                let element = element_schema(schema);
                let mut read = Vec::with_capacity(elements.len());
                for value in elements {
                    read.push(self.read_slot(value, element, at)?);
                }
                Node::List(read)
            }
            // A run holds no slots, so a patch that introduces one reads it as the run it is.
            Value::Packed(list) => Node::Packed(list.clone()),
        })
    }

    /// Reads the named slots of a struct, a dict or an enum variant.
    fn read_fields(
        &mut self,
        fields: &BTreeMap<String, Value>,
        schema: &Schema,
        at: &Path,
    ) -> Result<BTreeMap<String, Slot>, DeltaError> {
        let mut read = BTreeMap::new();
        for (name, value) in fields {
            read.insert(
                name.clone(),
                self.read_slot(value, field_schema(schema, name), at)?,
            );
        }
        Ok(read)
    }

    /// Reads one slot: the reference it spells, or the node it holds.
    fn read_slot(&mut self, value: &Value, schema: &Schema, at: &Path) -> Result<Slot, DeltaError> {
        if schema.is_reference() {
            if let Some(reference) = spelled(value) {
                let target = self.target(&reference, at)?;
                return Ok(Slot::referenced(reference, target));
            }
        }
        Ok(Slot::inlined(Target::Local(self.read(value, schema, at)?)))
    }

    /// Resolves a reference the introduced content spells, which must stay in this document.
    fn target(&self, reference: &Reference, at: &Path) -> Result<Target, DeltaError> {
        let out_of_document = |at: &Path| DeltaError::InvalidOperation {
            path: at.clone(),
            reason: "a patch cannot introduce a reference out of the document, which needs a \
                     load rather than a patch"
                .to_owned(),
        };
        match reference {
            Reference::Internal(path) => match self.resolve(path) {
                Ok(Target::Local(node)) => Ok(Target::Local(node)),
                Ok(Target::Foreign(..)) | Err(..) => Err(out_of_document(at)),
            },
            Reference::External(_) | Reference::File(_) => Err(out_of_document(at)),
        }
    }

    /// Builds the error for an operation whose target is not a list.
    fn not_a_list(&self, path: &Path) -> DeltaError {
        DeltaError::InvalidOperation {
            path: path.clone(),
            reason: "this operation needs a list".to_owned(),
        }
    }

    /// Builds the error for an operation whose target has no named entries.
    fn not_a_dict(&self, path: &Path) -> DeltaError {
        DeltaError::InvalidOperation {
            path: path.clone(),
            reason: "this operation needs a dict or a struct".to_owned(),
        }
    }

    /// Builds the error for an operation reaching outside what the value holds.
    fn beyond_the_end(&self, path: &Path) -> DeltaError {
        DeltaError::InvalidOperation {
            path: path.clone(),
            reason: "this operation reaches outside the value".to_owned(),
        }
    }
}

/// One operation, with the node it names and the work that could fail already done.
struct Planned<'operation> {
    path: &'operation Path,
    node: NodeId,
    action: Action,
}

/// What an operation does to the node it names, with content already read into the document.
enum Action {
    /// Writes a node into the one the path names.
    Replace(NodeId),
    /// Inserts nodes before an index of a list.
    Values { index: usize, nodes: Vec<NodeId> },
    /// Adds entries to a dict.
    Entries { entries: Vec<(String, NodeId)> },
    /// Removes a range of a list.
    Indices { start: usize, count: usize },
    /// Removes keys.
    Keys { keys: Vec<String> },
}

/// Returns the schema of the value a path names.
///
/// # Invariants
///
/// A key addresses a field of a struct, an entry of a dict, or one of the fields of an enum's
/// variant. A path cannot name a variant, so where the variants disagree about a field the
/// schema of it is open: nothing is known about the value, and a patch reading content there
/// treats a spelling as the data it is. A list's elements share one schema.
fn schema_at(schema: &Schema, path: &Path) -> Schema {
    let mut at = schema.clone();
    for segment in path.segments() {
        at = match segment {
            PathSegment::Key(name) => match &at.kind {
                SchemaKind::Struct { fields } => fields.get(name).cloned().unwrap_or_default(),
                SchemaKind::Enum { variants } => variants
                    .values()
                    .find_map(|variant| match &variant.kind {
                        SchemaKind::Struct { fields } => fields.get(name).cloned(),
                        _ => None,
                    })
                    .unwrap_or_default(),
                _ => Schema::default(),
            },
            PathSegment::Index(_) => element_schema(&at).clone(),
        };
    }
    at
}
