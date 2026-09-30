//! Writing a loaded definition back as the document it was read from.
//!
//! # Why the spelling is kept
//!
//! A slot that was written as a reference is written back as one, so writing a loaded
//! definition does not expand every reference into a copy of what it names. Without that, a
//! document would grow with every round trip and the sharing an author expressed would be
//! lost the first time it was read and written, even though the graph still held it.

use std::collections::BTreeMap;

use crate::loaded::{Loaded, Node, Slot};
use crate::value::Value;

impl Loaded {
    /// Writes this definition as a document, with every reference spelled as it was written.
    pub fn to_value(&self) -> Value {
        self.write_node(self.root)
    }

    /// Writes the node at an address.
    fn write_node(&self, id: crate::loaded::NodeId) -> Value {
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
    fn write_slot(&self, slot: &Slot) -> Value {
        match slot.spelling() {
            Some(reference) => reference.to_value(),
            None => self.write_node(slot.target()),
        }
    }
}
