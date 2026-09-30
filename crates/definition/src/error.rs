//! Error types for schema validation, path resolution, delta application, and history.

use thiserror::Error;

use crate::identifier::{DefinitionIdentifier, SchemaIdentifier};
use crate::path::{Path, PathError};
use crate::version::Version;

/// A [`Value`](crate::Value) does not satisfy its [`Schema`](crate::Schema).
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("schema `{0}` is not registered")]
    UnknownSchema(SchemaIdentifier),
    #[error("at {path}: expected {expected}, found {found}")]
    KindMismatch {
        path: Path,
        expected: String,
        found: String,
    },
    #[error("at {path}: field `{field}` is required")]
    MissingField { path: Path, field: String },
    #[error("at {path}: field `{field}` is not declared by the schema")]
    UnknownField { path: Path, field: String },
    #[error("at {path}: unknown variant `{tag}`")]
    UnknownVariant { path: Path, tag: String },
    #[error("at {path}: the schema declares a reference here, and {reason}")]
    MalformedReference { path: Path, reason: String },
}

/// A value is not a readable [`Schema`](crate::Schema) document.
///
/// This is a read error rather than a [`ValidationError`], because it is the schema
/// that is malformed rather than the value being wrong: a document that carries a
/// schema is the thing at fault.
#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("at {path}: a schema document is a struct, found {found}")]
    NotADocument { path: Path, found: &'static str },
    #[error("at {path}: field `{field}` is required in a schema document")]
    MissingField { path: Path, field: String },
    #[error("at {path}: field `{field}` is not a schema document field")]
    UnknownField { path: Path, field: String },
    #[error("at {path}: a schema kind is an enum tag, found {found}")]
    NotAKind { path: Path, found: &'static str },
    #[error("at {path}: unknown schema kind `{tag}`")]
    UnknownKind { path: Path, tag: String },
    #[error("at {path}: the `{tag}` kind needs a `{field}` field")]
    MissingKindField {
        path: Path,
        tag: String,
        field: &'static str,
    },
    #[error("at {path}: the `{tag}` kind carries no fields, found `{field}`")]
    UnexpectedKindField {
        path: Path,
        tag: String,
        field: String,
    },
    #[error("at {path}: expected {expected}, found {found}")]
    KindMismatch {
        path: Path,
        expected: &'static str,
        found: &'static str,
    },
    #[error("at {path}: a map of named schemas is a dict, found {found}")]
    NotAMap { path: Path, found: &'static str },
}

/// A [`Delta`](crate::Delta) could not be applied.
#[derive(Debug, Error)]
pub enum DeltaError {
    #[error("delta targets definition `{expected}` but was applied to `{actual}`")]
    WrongDefinition {
        expected: DefinitionIdentifier,
        actual: DefinitionIdentifier,
    },
    #[error(
        "delta was computed against version {expected} but the definition is at version {actual}"
    )]
    Conflict { expected: Version, actual: Version },
    #[error("at {path}: {reason}")]
    UnresolvedPath { path: Path, reason: String },
    #[error("at {path}: {reason}")]
    InvalidOperation { path: Path, reason: String },
    #[error(
        "at {path}: the delta reaches a reference, and a reference is only traversed once the \
         document is loaded, so this delta belongs on a loaded generation"
    )]
    ReferenceInSerialized { path: Path },
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

/// A [`History`](crate::History) has nothing left to replay.
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
    #[error(transparent)]
    Delta(#[from] DeltaError),
}

/// A [`Registry`](crate::Registry) lookup or mutation failed.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("definition `{0}` is not registered")]
    UnknownDefinition(DefinitionIdentifier),
    #[error("definition `{0}` is already registered")]
    DuplicateDefinition(DefinitionIdentifier),
    #[error(transparent)]
    Delta(#[from] DeltaError),
    #[error(transparent)]
    History(#[from] HistoryError),
}

/// A document could not be loaded, because a reference in it names nothing.
///
/// A load failure is its own kind of error rather than a validation failure: the document
/// satisfied its schema, and what is missing is a target, or a store to find one in.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error(
        "the reference `{reference}` names a definition or a file, and this load was given no store to find it in"
    )]
    NoStore { reference: String },
    #[error("a load could not find what a reference named: {reason}")]
    Store { reason: String },
    #[error("at {path}: {reason}")]
    UnresolvedPath { path: Path, reason: PathError },
    #[error("the references {references:?} name each other and no value")]
    ValueLessCycle { references: Vec<String> },
    #[error("the reference `{reference}` was never resolved")]
    Unresolved { reference: String },
    #[error("the document cannot be expanded into a tree, because it reaches node {node} again")]
    Cycle { node: usize },
}
