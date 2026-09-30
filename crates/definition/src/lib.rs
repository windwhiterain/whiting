//! Schema-checked dynamic data, deltas, and undo-redo history.
//!
//! This crate is the foundation of the workspace. It knows nothing about graphics: it
//! provides a value tree, a schema language that verifies that tree, sparse versioned
//! changes to it, and the replay system that undoes those changes.
//!
//! # Why the domain has no rust types
//!
//! Every document in this workspace is a [`Value`], and every document is described by a
//! [`Schema`] that is loaded and replaced at runtime. A concept such as a graphics
//! instruction set is therefore a pattern of [`Value`] plus the [`Schema`] that accepts
//! it, never a `struct` or an `enum` in this codebase. That is what lets a schema change
//! take effect with no recompilation.
//!
//! # Example
//!
//! ```
//! use std::collections::BTreeMap;
//! use whiting_definition::{
//!     Delta, Definition, DefinitionIdentifier, Path, PrimitiveDelta, Registry, Schema,
//!     SchemaIdentifier, SchemaKind, Value,
//! };
//!
//! let mut registry = Registry::new();
//! registry.put_schema(
//!     SchemaIdentifier::new("counter"),
//!     Schema::new(SchemaKind::structure([(
//!         "steps".to_owned(),
//!         Schema::new(SchemaKind::list(Schema::new(SchemaKind::Int))),
//!     )])),
//! );
//!
//! let identifier = DefinitionIdentifier::new("walk");
//! registry
//!     .insert_definition(Definition::new(
//!         identifier.clone(),
//!         SchemaIdentifier::new("counter"),
//!         Value::structure([(
//!             "steps".to_owned(),
//!             Value::List(vec![Value::Int(1)]),
//!         )]),
//!     ))
//!     .expect("the definition is new");
//!
//! let mut operations = BTreeMap::new();
//! operations.insert(
//!     Path::root().key("steps"),
//!     PrimitiveDelta::Append {
//!         index: 1,
//!         values: vec![Value::Int(2)],
//!     },
//! );
//! let delta = Delta::new(identifier.clone(), 0u64.into(), operations);
//!
//! let inverse = registry.apply(&delta).expect("the delta applies");
//! registry.undo(&identifier).expect("the apply is undoable");
//! registry.redo(&identifier).expect("the undo is redoable");
//!
//! // The inverse of an indexed append removes exactly the inserted range.
//! let primitive = inverse.operations.get(&Path::root().key("steps"));
//! assert_eq!(
//!     primitive,
//!     Some(&PrimitiveDelta::RemoveIndices { start: 1, count: 1 })
//! );
//! ```

mod definition;
mod delta;
mod error;
mod history;
mod identifier;
mod path;
mod registry;
mod schema;
mod value;
mod version;

pub use definition::Definition;
pub use delta::{Delta, PrimitiveDelta};
pub use error::{DeltaError, HistoryError, RegistryError, ValidationError};
pub use history::History;
pub use identifier::{DefinitionIdentifier, SchemaIdentifier};
pub use path::{Path, PathError, PathSegment};
pub use registry::Registry;
pub use schema::{Attributes, Schema, SchemaKind, SchemaMap};
pub use value::{FieldMap, Value};
pub use version::Version;
