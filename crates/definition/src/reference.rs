//! Where a reference points, as a document spells it.
//!
//! # Why a reference is a value rather than a variant of [`Value`]
//!
//! The schema decides which slots are references, so a value only has to say *where* it
//! points. Spelling that as an ordinary value — a variant tag and one field — keeps
//! [`Value`] unchanged, keeps a reference readable in the same way as the data around it,
//! and leaves the schema as the single place that says a slot is a reference. The loader
//! reads the spelling through [`Reference::from_value`] on exactly the slots the schema
//! marked, and a value that happens to look like a reference in an unmarked slot is just
//! data.
//!
//! # Invariants
//!
//! [`Reference::to_value`] and [`Reference::from_value`] are inverses, so a document
//! round trips through its own spelling. A reference names a `path` whatever kind it is:
//! a [`Path`] for the two that address a document, and a [`FilePath`] for the one that
//! addresses a file.
//!
//! # Example
//!
//! ```
//! use whiting_definition::{Reference, Schema, SchemaKind, Value};
//!
//! // A reference is spelled as a variant and one path.
//! let reference = Reference::Internal(whiting_definition::Path::root().key("meshes").index(0));
//! let spelled = reference.to_value();
//! assert_eq!(Reference::from_value(&spelled).expect("it round trips"), reference);
//! assert_eq!(
//!     spelled,
//!     Value::variant(
//!         "InternalReference",
//!         [(
//!             "path".to_owned(),
//!             Value::List(vec![Value::String("meshes".to_owned()), Value::Int(0)]),
//!         )],
//!     ),
//! );
//!
//! // The schema is what says a slot is a reference, and a marked slot holds a union of the
//! // reference and the value, so either is accepted and the declared kind describes what the
//! // value is and what a reference must resolve to.
//! let slot = Schema::new(SchemaKind::String).reference();
//! assert!(slot.is_reference());
//! slot.validate(&spelled).expect("a reference is what a marked slot accepts");
//! slot.validate(&Value::String("inlined".to_owned()))
//!     .expect("a value of the declared kind is accepted too");
//! slot.clone()
//!     .without_reference()
//!     .validate(&Value::String("inlined".to_owned()))
//!     .expect("a reference must resolve to a value of the declared kind");
//!
//! // The marker survives being written as a schema document, which is how a schema is
//! // registered and read back, so a slot cannot quietly stop being a reference.
//! let read = Schema::from_value(&slot.to_value()).expect("the schema reads back");
//! assert!(read.is_reference());
//! ```

use std::fmt;

use thiserror::Error;

use crate::path::Path;
use crate::value::Value;

/// Tag of the reference that addresses a path inside its own definition.
pub const INTERNAL: &str = "InternalReference";

/// Tag of the reference that addresses a path from the registry root.
pub const EXTERNAL: &str = "ExternalReference";

/// Tag of the reference that addresses a file.
pub const FILE: &str = "FilePath";

/// Field holding where a reference points.
pub const PATH: &str = "path";

/// A file a definition or an asset is read from.
///
/// An absolute path is read as itself; a relative one is resolved against the root of the
/// document that refers to it, so a document tree stays movable. What the file holds is
/// decided by the schema of the slot that refers to it: a slot of bytes wants the file as
/// an asset, and a slot of structure wants it parsed as a definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilePath(String);

impl FilePath {
    /// Creates a file path.
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// Borrows the path as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Where a reference points.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reference {
    /// A path inside the definition that refers to it.
    Internal(Path),
    /// A path from the registry root, whose first key names a definition.
    External(Path),
    /// A file.
    File(FilePath),
}

impl Reference {
    /// Returns whether a variant tag names a kind of reference.
    ///
    /// A marked slot holds a union of a reference and a value, so this is what tells the two
    /// apart: a tag that names a reference is read as one, whether or not its path is well
    /// formed, and any other tag is a value of the declared kind.
    pub fn is_tag(tag: &str) -> bool {
        matches!(tag, INTERNAL | EXTERNAL | FILE)
    }

    /// Encodes this reference as the value a document spells it with.
    pub fn to_value(&self) -> Value {
        let (tag, target) = match self {
            Self::Internal(path) => (INTERNAL, path.to_value()),
            Self::External(path) => (EXTERNAL, path.to_value()),
            Self::File(file) => (FILE, Value::String(file.as_str().to_owned())),
        };
        Value::variant(tag, [(PATH.to_owned(), target)])
    }

    /// Decodes the reference a value spells, which [`Reference::to_value`] wrote.
    pub fn from_value(value: &Value) -> Result<Self, ReferenceError> {
        let Some(tag) = value.variant_tag() else {
            return Err(ReferenceError::NotAReference {
                found: value.kind_name(),
            });
        };
        let target = value
            .variant_fields()
            .and_then(|fields| fields.get(PATH))
            .ok_or_else(|| ReferenceError::MissingPath {
                tag: tag.to_owned(),
            })?;
        match tag {
            INTERNAL => Ok(Self::Internal(path(target)?)),
            EXTERNAL => Ok(Self::External(path(target)?)),
            FILE => match target {
                Value::String(file) => Ok(Self::File(FilePath::new(file))),
                found => Err(ReferenceError::NotAFile {
                    found: found.kind_name(),
                }),
            },
            other => Err(ReferenceError::UnknownKind {
                tag: other.to_owned(),
            }),
        }
    }
}

/// Decodes the path a reference carries.
fn path(value: &Value) -> Result<Path, ReferenceError> {
    Path::from_value(value).map_err(|error| ReferenceError::Path {
        reason: error.to_string(),
    })
}

/// A value does not spell a reference.
#[derive(Debug, Error)]
pub enum ReferenceError {
    #[error("a reference is a variant, and this is {found}")]
    NotAReference { found: &'static str },
    #[error("a `{tag}` reference has no `path`")]
    MissingPath { tag: String },
    #[error("`{tag}` is not a kind of reference")]
    UnknownKind { tag: String },
    #[error("a file path is a string, and this is {found}")]
    NotAFile { found: &'static str },
    #[error("the path a reference carries is malformed: {reason}")]
    Path { reason: String },
}
