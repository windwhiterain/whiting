//! Writing a loaded definition back as the document it was read from.
//!
//! # Why the spelling is kept
//!
//! A slot that was written as a reference is written back as one, so writing a loaded
//! definition does not expand every reference into a copy of what it names. Without that, a
//! document would grow with every round trip and the sharing an author expressed would be
//! lost the first time it was read and written, even though the graph still held it.

use std::collections::BTreeMap;

use crate::error::LoadError;
use crate::loaded::{Loaded, Node, NodeId, Slot, Target};
use crate::value::Value;

impl Loaded {
    /// Writes this definition as a document, with every reference spelled as it was written.
    pub fn to_value(&self) -> Value {
        self.write_node(self.root)
    }

    /// Writes the node at an address.
    pub(super) fn write_node(&self, id: crate::loaded::NodeId) -> Value {
        match self.node(id) {
            Node::Unit => Value::Unit,
            Node::Bool(flag) => Value::Bool(*flag),
            Node::Int(number) => Value::Int(*number),
            Node::Float(number) => Value::Float(*number),
            Node::String(text) => Value::String(text.clone()),
            Node::Bytes(bytes) => Value::Bytes(bytes.clone()),
            Node::Struct(fields) => Value::Struct(self.write_fields(fields)),
            Node::Dict(entries) => Value::Dict(self.write_fields(entries)),
            Node::Enum { tag, fields } => Value::variant(tag.clone(), self.write_fields(fields)),
            Node::List(elements) => {
                Value::List(elements.iter().map(|slot| self.write_slot(slot)).collect())
            }
        }
    }

    /// Writes the named slots of a node.
    fn write_fields(&self, fields: &BTreeMap<String, Slot>) -> BTreeMap<String, Value> {
        fields
            .iter()
            .map(|(name, slot)| (name.clone(), self.write_slot(slot)))
            .collect()
    }

    /// Writes one slot: the reference it was written as, or the node it holds.
    ///
    /// A slot without a spelling always names a node of this generation, because a reference
    /// to another generation was written as a reference and keeps its spelling.
    pub(super) fn write_slot(&self, slot: &Slot) -> Value {
        match slot.spelling() {
            Some(reference) => reference.to_value(),
            None => self.write_node(slot.target().node()),
        }
    }

    /// Writes this definition as a document with every reference replaced by what it names.
    ///
    /// # Why a reader wants this
    ///
    /// A reader that cannot resolve a reference — a packer filling a buffer, say, since a buffer
    /// cannot hold a path — reads an expanded document as an ordinary one, so a document that
    /// shares its data with another definition still reaches it. The cost is that sharing is
    /// lost in the writing: a node named twice is written twice.
    ///
    /// # Invariants
    ///
    /// A cycle cannot be expanded, because what it names has no finite writing. It is refused
    /// rather than recursed into, and only a cycle is: a node reached twice by different paths
    /// is written twice, which terminates.
    ///
    /// ```
    /// use whiting_definition::{Loaded, Path, Reference, Schema, SchemaKind, Value};
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
    /// let loaded = Loaded::load(&document, &schema).expect("it loads");
    ///
    /// // Both slots hold the value, so a reader that cannot follow a reference can read it.
    /// assert_eq!(
    ///     loaded.expanded().expect("it expands"),
    ///     Value::structure([
    ///         ("left".to_owned(), Value::Int(7)),
    ///         ("right".to_owned(), Value::Int(7)),
    ///     ])
    /// );
    ///
    /// // Writing it back still spells the reference, so expanding is a reading rather than a
    /// // rewriting of the document.
    /// assert_eq!(loaded.to_value(), document);
    /// ```
    pub fn expanded(&self) -> Result<Value, LoadError> {
        let mut open = Vec::new();
        self.expand_node(self.root, &mut open)
    }

    /// Writes a node, refusing to descend into one already being written.
    fn expand_node(&self, id: NodeId, open: &mut Vec<(usize, NodeId)>) -> Result<Value, LoadError> {
        let key = (self as *const Self as usize, id);
        if open.contains(&key) {
            return Err(LoadError::Cycle { node: id.index() });
        }
        open.push(key);
        let value = self.expand_containers(id, open)?;
        open.pop();
        Ok(value)
    }

    /// Writes a node's own shape, expanding the slots it holds.
    fn expand_containers(
        &self,
        id: NodeId,
        open: &mut Vec<(usize, NodeId)>,
    ) -> Result<Value, LoadError> {
        Ok(match self.node(id) {
            Node::Unit => Value::Unit,
            Node::Bool(flag) => Value::Bool(*flag),
            Node::Int(number) => Value::Int(*number),
            Node::Float(number) => Value::Float(*number),
            Node::String(text) => Value::String(text.clone()),
            Node::Bytes(bytes) => Value::Bytes(bytes.clone()),
            Node::Struct(fields) => Value::Struct(self.expand_fields(fields, open)?),
            Node::Dict(entries) => Value::Dict(self.expand_fields(entries, open)?),
            Node::Enum { tag, fields } => {
                Value::variant(tag.clone(), self.expand_fields(fields, open)?)
            }
            Node::List(elements) => {
                let mut written = Vec::with_capacity(elements.len());
                for slot in elements {
                    written.push(self.expand_slot(slot, open)?);
                }
                Value::List(written)
            }
        })
    }

    /// Writes the named slots of a node.
    fn expand_fields(
        &self,
        fields: &BTreeMap<String, Slot>,
        open: &mut Vec<(usize, NodeId)>,
    ) -> Result<BTreeMap<String, Value>, LoadError> {
        let mut written = BTreeMap::new();
        for (name, slot) in fields {
            written.insert(name.clone(), self.expand_slot(slot, open)?);
        }
        Ok(written)
    }

    /// Writes what a slot names, wherever it is, including in another generation.
    fn expand_slot(
        &self,
        slot: &Slot,
        open: &mut Vec<(usize, NodeId)>,
    ) -> Result<Value, LoadError> {
        match slot.target() {
            Target::Local(node) => self.expand_node(*node, open),
            Target::Foreign(generation, node) => generation.expand_node(*node, open),
        }
    }
}
