//! Schema documents: the recursive description that verifies a [`Value`].

use std::collections::BTreeMap;

use facet::Facet;

use crate::error::ValidationError;
use crate::path::{Path, PathSegment};
use crate::value::{FieldMap, Value};

/// Map of named entries to a nested schema, used for both struct fields and enum variants.
pub type SchemaMap = BTreeMap<String, Schema>;

/// A general schema representation to verify a [`Value`](crate::Value).
///
/// A schema is a document, not a compile-time artifact. It is loaded and replaced at
/// runtime through a [`Registry`](crate::Registry), so a new schema can take effect with
/// no recompilation of this crate or of anything downstream of it.
#[derive(Debug, Clone, PartialEq, Default, Facet)]
pub struct Schema {
    /// The shape this schema accepts.
    pub kind: SchemaKind,
    /// Constraints and metadata attached to this node.
    pub attributes: Attributes,
    /// Optional prose describing the node.
    pub documentation: Option<String>,
}

impl Schema {
    /// Creates a schema of the given kind with no attributes.
    pub const fn new(kind: SchemaKind) -> Self {
        Self {
            kind,
            attributes: Attributes::new(),
            documentation: None,
        }
    }

    /// Adds attributes to this schema.
    #[must_use]
    pub fn with_attributes(mut self, attributes: Attributes) -> Self {
        self.attributes = attributes;
        self
    }

    /// Attaches documentation to this schema.
    #[must_use]
    pub fn with_documentation(mut self, documentation: impl Into<String>) -> Self {
        self.documentation = Some(documentation.into());
        self
    }

    /// Returns the human readable name of this schema's kind, for error messages.
    pub const fn kind_name(&self) -> &'static str {
        match &self.kind {
            SchemaKind::Any => "any",
            SchemaKind::Unit => "unit",
            SchemaKind::Bool => "bool",
            SchemaKind::Int => "int",
            SchemaKind::Float => "float",
            SchemaKind::String => "string",
            SchemaKind::Bytes => "bytes",
            SchemaKind::Struct { .. } => "struct",
            SchemaKind::Enum { .. } => "enum",
            SchemaKind::List { .. } => "list",
            SchemaKind::Dict { .. } => "dict",
        }
    }

    /// Verifies that `value` satisfies this schema, reporting the root path.
    pub fn validate(&self, value: &Value) -> Result<(), ValidationError> {
        self.validate_at(&Path::root(), value)
    }

    fn validate_at(&self, path: &Path, value: &Value) -> Result<(), ValidationError> {
        // `Int` and `Float` stay distinct, so this match is on the pair of kinds.
        match (&self.kind, value) {
            (SchemaKind::Any, _) => Ok(()),

            (SchemaKind::Unit, Value::Unit) => Ok(()),
            (SchemaKind::Bool, Value::Bool(_)) => Ok(()),
            (SchemaKind::Int, Value::Int(_)) => Ok(()),
            (SchemaKind::Float, Value::Float(_)) => Ok(()),
            (SchemaKind::String, Value::String(_)) => Ok(()),
            (SchemaKind::Bytes, Value::Bytes(_)) => Ok(()),

            (SchemaKind::Struct { fields }, Value::Struct(values)) => {
                validate_fields(path, fields, values)
            }

            (SchemaKind::Enum { variants }, Value::Enum { tag, fields }) => {
                let variant_schema =
                    variants
                        .get(tag)
                        .ok_or_else(|| ValidationError::UnknownVariant {
                            path: path.clone(),
                            tag: tag.clone(),
                        })?;
                match &variant_schema.kind {
                    SchemaKind::Struct {
                        fields: variant_fields,
                    } => validate_fields(path, variant_fields, fields),
                    _ => Err(ValidationError::KindMismatch {
                        path: path.clone(),
                        expected: "a struct shaped variant".to_owned(),
                        found: variant_schema.kind_name().to_owned(),
                    }),
                }
            }
            (SchemaKind::List { element }, Value::List(elements)) => elements
                .iter()
                .enumerate()
                .try_for_each(|(index, element_value)| {
                    element.validate_at(&path.child(PathSegment::Index(index)), element_value)
                }),

            (SchemaKind::Dict { value: entry }, Value::Dict(entries)) => {
                entries.iter().try_for_each(|(name, entry_value)| {
                    entry.validate_at(&path.child(PathSegment::Key(name.clone())), entry_value)
                })
            }

            _ => Err(ValidationError::KindMismatch {
                path: path.clone(),
                expected: self.kind_name().to_owned(),
                found: value.kind_name().to_owned(),
            }),
        }
    }
}

/// A struct is closed and a dict is open; both check presence and value the same way.
fn validate_fields(
    path: &Path,
    declared: &SchemaMap,
    present: &FieldMap,
) -> Result<(), ValidationError> {
    for (name, field_schema) in declared {
        let field_path = path.child(PathSegment::Key(name.clone()));
        match present.get(name) {
            Some(field_value) => field_schema.validate_at(&field_path, field_value)?,
            None if field_schema.attributes.optional
                || field_schema.attributes.default.is_some() => {}
            None => {
                return Err(ValidationError::MissingField {
                    path: field_path,
                    field: name.clone(),
                });
            }
        }
    }

    for name in present.keys() {
        if !declared.contains_key(name) {
            return Err(ValidationError::UnknownField {
                path: path.clone(),
                field: name.clone(),
            });
        }
    }

    Ok(())
}

/// The shape a [`Schema`] accepts.
#[derive(Debug, Clone, PartialEq, Default, Facet)]
#[repr(C)]
pub enum SchemaKind {
    /// Accepts any value.
    #[default]
    Any,
    Unit,
    Bool,
    Int,
    Float,
    String,
    Bytes,
    /// Named fields. Closed: a field absent from this map is an error.
    Struct {
        fields: SchemaMap,
    },
    /// Variant tag to the fields that variant carries.
    Enum {
        variants: SchemaMap,
    },
    /// An ordered sequence of a single element shape.
    List {
        element: Box<Schema>,
    },
    /// An open map of string keys. Entries may be added without editing the schema.
    Dict {
        value: Box<Schema>,
    },
}

impl SchemaKind {
    /// Builds a struct kind from an iterator of field names and schemas.
    pub fn structure(fields: impl IntoIterator<Item = (String, Schema)>) -> Self {
        Self::Struct {
            fields: fields.into_iter().collect(),
        }
    }

    /// Builds an enum kind from an iterator of variant tags and their schemas.
    ///
    /// Each variant schema is expected to be a struct, which is what carries the fields
    /// that variant holds.
    pub fn enumeration(variants: impl IntoIterator<Item = (String, Schema)>) -> Self {
        Self::Enum {
            variants: variants.into_iter().collect(),
        }
    }

    /// Builds a list kind of the given element schema.
    pub fn list(element: Schema) -> Self {
        Self::List {
            element: Box::new(element),
        }
    }

    /// Builds a dict kind of the given entry schema.
    pub fn dictionary(value: Schema) -> Self {
        Self::Dict {
            value: Box::new(value),
        }
    }
}

/// Per-node constraints and metadata.
#[derive(Debug, Clone, PartialEq, Default, Facet)]
pub struct Attributes {
    /// The field may be absent.
    pub optional: bool,
    /// Value substituted when the field is absent.
    pub default: Option<Value>,
    /// Open map for domain-specific constraints.
    pub extensions: FieldMap,
}

impl Attributes {
    /// Creates attributes with no constraint set.
    pub const fn new() -> Self {
        Self {
            optional: false,
            default: None,
            extensions: FieldMap::new(),
        }
    }

    /// Marks the field as optional.
    #[must_use]
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Declares the value substituted when the field is absent.
    #[must_use]
    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }

    /// Records a domain-specific constraint.
    #[must_use]
    pub fn with_extension(mut self, name: impl Into<String>, value: Value) -> Self {
        self.extensions.insert(name.into(), value);
        self
    }
}
