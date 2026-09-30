//! A schema written as a document, so a schema can be embedded in another document.
//!
//! A shader declares the interface of the data it reads, and a mesh declares the shape
//! of its points. Neither is a name in a registry: both are schemas carried by the
//! document that needs them, so `Schema` converts to and from `Value` in the same
//! `.ron`-shaped language as every other document.
//!
//! # Invariants
//!
//! The round trip is exact: `Schema::from_value(&schema.to_value())` is the original
//! schema. Absence is written by leaving a key out rather than by writing a sentinel,
//! so a `default` of `unit` survives the trip and is not read back as no default at
//! all.
//!
//! # Example
//!
//! ```
//! use whiting_definition::{Attributes, Schema, SchemaKind, Value};
//!
//! // A field may declare a default, including a default of `unit`, which is exactly the
//! // case a sentinel encoding would lose.
//! let point = Schema::new(SchemaKind::structure([(
//!     "visible".to_owned(),
//!     Schema::new(SchemaKind::Unit)
//!         .with_attributes(Attributes::new().with_default(Value::Unit)),
//! )]));
//!
//! let document = point.to_value();
//! assert_eq!(
//!     Schema::from_value(&document).expect("a schema document reads"),
//!     point
//! );
//! ```

use crate::error::SchemaError;
use crate::path::Path;
use crate::schema::{Attributes, Schema, SchemaKind, SchemaMap};
use crate::value::{FieldMap, Value};

/// Field holding the shape of a schema document.
const KIND: &str = "kind";

/// Field holding the attributes of a schema document.
const ATTRIBUTES: &str = "attributes";

/// Field holding the prose of a schema document, left out when there is none.
const DOCUMENTATION: &str = "documentation";

/// Field holding the `optional` attribute.
const OPTIONAL: &str = "optional";

/// Field holding the `default` attribute, left out when there is none.
const DEFAULT: &str = "default";

/// Field holding the `extensions` attribute.
const EXTENSIONS: &str = "extensions";

/// Payload field of a `Struct` or `Enum` kind: a map of names to nested schemas.
const FIELDS: &str = "fields";

/// Payload field of an `Enum` kind: a map of variant tags to nested schemas.
const VARIANTS: &str = "variants";

/// Payload field of a `List` kind: its element schema.
const ELEMENT: &str = "element";

/// Payload field of a `Dict` kind: its entry schema.
const VALUE: &str = "value";

/// The tag of every schema kind, with the payload field it carries, if any.
///
/// One table, read by the writer, the reader and [`Schema::document_schema`] alike, so
/// the three descriptions of the language cannot drift apart.
const KIND_TAGS: [(&str, Option<&str>); 11] = [
    ("Any", None),
    ("Unit", None),
    ("Bool", None),
    ("Int", None),
    ("Float", None),
    ("String", None),
    ("Bytes", None),
    ("Struct", Some(FIELDS)),
    ("Enum", Some(VARIANTS)),
    ("List", Some(ELEMENT)),
    ("Dict", Some(VALUE)),
];

impl Schema {
    /// Returns the schema describing how a schema is written as a document.
    ///
    /// A document that carries a schema declares the slot as this schema rather than
    /// leaving it open, so an agent reading the document is told what the slot holds.
    pub fn document_schema() -> Schema {
        Schema::new(SchemaKind::structure([
            (
                KIND.to_owned(),
                kind_document_schema().with_documentation("The shape this schema accepts."),
            ),
            (ATTRIBUTES.to_owned(), attributes_document_schema()),
            (
                DOCUMENTATION.to_owned(),
                Schema::new(SchemaKind::String)
                    .optional()
                    .with_documentation("Prose describing the schema."),
            ),
        ]))
        .with_documentation("A schema, written as a document rather than held in rust.")
    }

    /// Writes this schema as a document.
    pub fn to_value(&self) -> Value {
        let mut fields = vec![
            (KIND.to_owned(), self.kind.to_value()),
            (ATTRIBUTES.to_owned(), attributes_to_value(&self.attributes)),
        ];
        if let Some(documentation) = &self.documentation {
            fields.push((
                DOCUMENTATION.to_owned(),
                Value::String(documentation.clone()),
            ));
        }
        Value::structure(fields)
    }

    /// Reads a schema out of a document.
    pub fn from_value(value: &Value) -> Result<Self, SchemaError> {
        let path = Path::root();
        let Some(fields) = value.as_struct() else {
            return Err(SchemaError::NotADocument {
                path,
                found: value.kind_name(),
            });
        };
        for name in fields.keys() {
            if !matches!(name.as_str(), KIND | ATTRIBUTES | DOCUMENTATION) {
                return Err(SchemaError::UnknownField {
                    path: path.clone(),
                    field: name.clone(),
                });
            }
        }
        let kind_field = path.key(KIND);
        let kind = SchemaKind::from_value(required(fields, KIND, &path)?, &kind_field)?;
        let attributes_path = path.key(ATTRIBUTES);
        let attributes = match fields.get(ATTRIBUTES) {
            Some(value) => attributes_from_value(value, &attributes_path)?,
            None => Attributes::new(),
        };
        let documentation = match fields.get(DOCUMENTATION) {
            Some(Value::String(prose)) => Some(prose.clone()),
            Some(other) => {
                return Err(SchemaError::KindMismatch {
                    path: path.key(DOCUMENTATION),
                    expected: "a string",
                    found: other.kind_name(),
                });
            }
            None => None,
        };
        Ok(Self {
            kind,
            attributes,
            documentation,
        })
    }
}

/// Builds the schema of a kind, from the tag table.
fn kind_document_schema() -> Schema {
    let variants = KIND_TAGS.iter().map(|(tag, payload)| {
        let fields = match payload {
            None => Schema::new(SchemaKind::structure([])),
            Some(name) => {
                let nested = if *name == FIELDS || *name == VARIANTS {
                    map_document_schema()
                } else {
                    Schema::new(SchemaKind::Any).with_documentation("A nested schema document.")
                };
                Schema::new(SchemaKind::structure([(
                    (*name).to_owned(),
                    nested.with_documentation("The payload of this kind."),
                )]))
            }
        };
        ((*tag).to_owned(), fields)
    });
    Schema::new(SchemaKind::enumeration(variants))
}

/// Builds the schema of a map from names to nested schema documents.
fn map_document_schema() -> Schema {
    Schema::new(SchemaKind::dictionary(
        Schema::new(SchemaKind::Any).with_documentation("A nested schema document."),
    ))
}

/// Builds the schema of the attributes block of a schema document.
fn attributes_document_schema() -> Schema {
    let mut fields = vec![
        (
            OPTIONAL.to_owned(),
            Schema::new(SchemaKind::Bool).with_documentation("The field may be absent."),
        ),
        (
            EXTENSIONS.to_owned(),
            Schema::new(SchemaKind::dictionary(Schema::new(SchemaKind::Any)))
                .with_documentation("Open map of domain specific constraints."),
        ),
    ];
    fields.push((
        DEFAULT.to_owned(),
        Schema::new(SchemaKind::Any)
            .optional()
            .with_documentation("Substituted when the field is absent."),
    ));
    Schema::new(SchemaKind::structure(fields))
}

/// Writes the attributes block of a schema document.
fn attributes_to_value(attributes: &Attributes) -> Value {
    let mut fields = vec![
        (OPTIONAL.to_owned(), Value::Bool(attributes.optional)),
        (
            EXTENSIONS.to_owned(),
            Value::Dict(attributes.extensions.clone()),
        ),
    ];
    if let Some(default) = &attributes.default {
        fields.push((DEFAULT.to_owned(), default.clone()));
    }
    Value::structure(fields)
}

/// Reads the attributes block of a schema document.
fn attributes_from_value(value: &Value, path: &Path) -> Result<Attributes, SchemaError> {
    let Some(fields) = value.as_struct() else {
        return Err(SchemaError::NotADocument {
            path: path.clone(),
            found: value.kind_name(),
        });
    };
    for name in fields.keys() {
        if !matches!(name.as_str(), OPTIONAL | DEFAULT | EXTENSIONS) {
            return Err(SchemaError::UnknownField {
                path: path.clone(),
                field: name.clone(),
            });
        }
    }
    let optional_path = path.key(OPTIONAL);
    let optional = match required(fields, OPTIONAL, path)? {
        Value::Bool(flag) => *flag,
        other => {
            return Err(SchemaError::KindMismatch {
                path: optional_path,
                expected: "a bool",
                found: other.kind_name(),
            });
        }
    };
    let extensions_path = path.key(EXTENSIONS);
    let extensions = match required(fields, EXTENSIONS, path)? {
        Value::Dict(entries) => entries.clone(),
        other => {
            return Err(SchemaError::KindMismatch {
                path: extensions_path,
                expected: "a dict",
                found: other.kind_name(),
            });
        }
    };
    let default = match fields.get(DEFAULT) {
        Some(value) => Some(value.clone()),
        None => None,
    };
    Ok(Attributes {
        optional,
        default,
        extensions,
    })
}

/// Reads a required field of a struct shaped document.
fn required<'value>(
    fields: &'value FieldMap,
    name: &str,
    path: &Path,
) -> Result<&'value Value, SchemaError> {
    fields.get(name).ok_or_else(|| SchemaError::MissingField {
        path: path.clone(),
        field: name.to_owned(),
    })
}

/// A schema kind being written or read, kept apart from the recursive call sites.
impl SchemaKind {
    /// Writes a kind as a document.
    fn to_value(&self) -> Value {
        let (tag, payload) = self.written();
        let fields = match payload {
            None => FieldMap::new(),
            Some(Payload::Map { field, entries }) => FieldMap::from([(
                field.to_owned(),
                Value::Dict(
                    entries
                        .iter()
                        .map(|(name, schema)| (name.clone(), schema.to_value()))
                        .collect(),
                ),
            )]),
            Some(Payload::Nested { field, schema }) => {
                FieldMap::from([(field.to_owned(), schema.to_value())])
            }
        };
        Value::Enum {
            tag: tag.to_owned(),
            fields,
        }
    }

    /// Reads a kind out of a document.
    fn from_value(value: &Value, path: &Path) -> Result<Self, SchemaError> {
        let Value::Enum { tag, fields } = value else {
            return Err(SchemaError::NotAKind {
                path: path.clone(),
                found: value.kind_name(),
            });
        };
        if !KIND_TAGS.iter().any(|(name, _)| name == tag) {
            return Err(SchemaError::UnknownKind {
                path: path.clone(),
                tag: tag.clone(),
            });
        }
        let Some(payload) = payload_field(tag) else {
            if let Some(name) = fields.keys().next() {
                return Err(SchemaError::UnexpectedKindField {
                    path: path.clone(),
                    tag: tag.clone(),
                    field: name.clone(),
                });
            }
            return Ok(leaf_kind(tag));
        };

        let payload_path = path.key(payload);
        let value = required(fields, payload, path)?;
        match payload {
            FIELDS => Ok(SchemaKind::Struct {
                fields: read_schema_map(value, &payload_path)?,
            }),
            VARIANTS => Ok(SchemaKind::Enum {
                variants: read_schema_map(value, &payload_path)?,
            }),
            ELEMENT => Ok(SchemaKind::List {
                element: Box::new(Schema::from_value(value)?),
            }),
            _ => Ok(SchemaKind::Dict {
                value: Box::new(Schema::from_value(value)?),
            }),
        }
    }

    /// Returns the tag this kind is written under, with the payload it carries, if any.
    fn written(&self) -> (&'static str, Option<Payload<'_>>) {
        match self {
            Self::Any => ("Any", None),
            Self::Unit => ("Unit", None),
            Self::Bool => ("Bool", None),
            Self::Int => ("Int", None),
            Self::Float => ("Float", None),
            Self::String => ("String", None),
            Self::Bytes => ("Bytes", None),
            Self::Struct { fields } => (
                "Struct",
                Some(Payload::Map {
                    field: FIELDS,
                    entries: fields,
                }),
            ),
            Self::Enum { variants } => (
                "Enum",
                Some(Payload::Map {
                    field: VARIANTS,
                    entries: variants,
                }),
            ),
            Self::List { element } => (
                "List",
                Some(Payload::Nested {
                    field: ELEMENT,
                    schema: element,
                }),
            ),
            Self::Dict { value } => (
                "Dict",
                Some(Payload::Nested {
                    field: VALUE,
                    schema: value,
                }),
            ),
        }
    }
}

/// The payload a kind carries, if any, and the field it is written under.
enum Payload<'schema> {
    /// A map of names to nested schemas.
    Map {
        field: &'static str,
        entries: &'schema SchemaMap,
    },
    /// A single nested schema.
    Nested {
        field: &'static str,
        schema: &'schema Schema,
    },
}

/// Returns the payload field a kind tag carries.
fn payload_field(tag: &str) -> Option<&'static str> {
    KIND_TAGS
        .iter()
        .find(|(name, _)| *name == tag)
        .and_then(|(_, payload)| *payload)
}

/// Reads a map of names to nested schema documents.
fn read_schema_map(value: &Value, path: &Path) -> Result<SchemaMap, SchemaError> {
    let Some(entries) = value.as_dict() else {
        return Err(SchemaError::NotAMap {
            path: path.clone(),
            found: value.kind_name(),
        });
    };
    entries
        .iter()
        .map(|(name, entry)| Ok((name.clone(), Schema::from_value(entry)?)))
        .collect()
}

/// Returns the kind of a tag that carries no payload.
fn leaf_kind(tag: &str) -> SchemaKind {
    match tag {
        "Any" => SchemaKind::Any,
        "Unit" => SchemaKind::Unit,
        "Bool" => SchemaKind::Bool,
        "Int" => SchemaKind::Int,
        "Float" => SchemaKind::Float,
        "String" => SchemaKind::String,
        _ => SchemaKind::Bytes,
    }
}
