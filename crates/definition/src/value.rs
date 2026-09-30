//! The dynamic value tree that every document in the workspace is made of.

use std::collections::BTreeMap;

use facet::Facet;

use crate::path::Path;
use crate::schema::{Schema, SchemaKind};

/// Ordered map of named fields.
///
/// A `BTreeMap` rather than a `HashMap` so that serialization order is stable, which is
/// what makes a structural diff of two equal values reliably empty.
pub type FieldMap = BTreeMap<String, Value>;

/// A general data representation, nested struct-enum-list-dict shaped like `.ron`.
///
/// `Value` carries no domain meaning. Every domain concept in the workspace, such as
/// `Graphics` or `Pass`, is a pattern of `Value` validated by a
/// [`Schema`](crate::Schema), never a distinct rust type.
#[derive(Debug, Clone, PartialEq, Default, Facet)]
#[repr(C)]
pub enum Value {
    /// Absence of a value.
    #[default]
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    /// Named fields, closed by the schema that validates it.
    Struct(FieldMap),
    /// A variant tag plus the fields that variant carries.
    Enum {
        tag: String,
        fields: FieldMap,
    },
    /// An ordered sequence.
    List(Vec<Value>),
    /// An open map of string keys, whose entries an agent may add without editing a schema.
    Dict(FieldMap),
}

impl Value {
    /// Returns the human readable name of this value's kind, for error messages.
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::String(_) => "string",
            Self::Bytes(_) => "bytes",
            Self::Struct(_) => "struct",
            Self::Enum { .. } => "enum",
            Self::List(_) => "list",
            Self::Dict(_) => "dict",
        }
    }

    /// Borrows the fields when this is a struct, otherwise returns `None`.
    pub fn as_struct(&self) -> Option<&FieldMap> {
        match self {
            Self::Struct(fields) => Some(fields),
            _ => None,
        }
    }

    /// Borrows the elements when this is a list, otherwise returns `None`.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Self::List(elements) => Some(elements),
            _ => None,
        }
    }

    /// Mutably borrows the elements when this is a list, otherwise returns `None`.
    pub fn as_list_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Self::List(elements) => Some(elements),
            _ => None,
        }
    }

    /// Borrows the fields when this is a dict, otherwise returns `None`.
    pub fn as_dict(&self) -> Option<&FieldMap> {
        match self {
            Self::Dict(entries) => Some(entries),
            _ => None,
        }
    }

    /// Mutably borrows the fields when this is a dict, otherwise returns `None`.
    pub fn as_dict_mut(&mut self) -> Option<&mut FieldMap> {
        match self {
            Self::Dict(entries) => Some(entries),
            _ => None,
        }
    }

    /// Borrows the variant tag when this is an enum, otherwise returns `None`.
    pub fn variant_tag(&self) -> Option<&str> {
        match self {
            Self::Enum { tag, .. } => Some(tag),
            _ => None,
        }
    }

    /// Borrows the variant fields when this is an enum, otherwise returns `None`.
    pub fn variant_fields(&self) -> Option<&FieldMap> {
        match self {
            Self::Enum { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Borrows the fields of a struct or a dict, otherwise returns `None`.
    pub fn as_field_map(&self) -> Option<&FieldMap> {
        match self {
            Self::Struct(fields) | Self::Dict(fields) => Some(fields),
            _ => None,
        }
    }

    /// Mutably borrows the fields of a struct or a dict, otherwise returns `None`.
    pub fn as_field_map_mut(&mut self) -> Option<&mut FieldMap> {
        match self {
            Self::Struct(fields) | Self::Dict(fields) => Some(fields),
            _ => None,
        }
    }

    /// Borrows the inner value of a string, otherwise returns `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// Borrows the bytes of a byte string, otherwise returns `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// Returns the inner value of an integer, otherwise returns `None`.
    pub const fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(number) => Some(*number),
            _ => None,
        }
    }

    /// Returns the inner value of a float, otherwise returns `None`.
    pub const fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(number) => Some(*number),
            _ => None,
        }
    }

    /// Builds a struct value from an iterator of fields.
    pub fn structure(fields: impl IntoIterator<Item = (String, Value)>) -> Self {
        Self::Struct(fields.into_iter().collect())
    }

    /// Builds an enum value from a variant tag and its fields.
    pub fn variant(
        tag: impl Into<String>,
        fields: impl IntoIterator<Item = (String, Value)>,
    ) -> Self {
        Self::Enum {
            tag: tag.into(),
            fields: fields.into_iter().collect(),
        }
    }

    /// Builds a dict value from an iterator of entries.
    pub fn dictionary(entries: impl IntoIterator<Item = (String, Value)>) -> Self {
        Self::Dict(entries.into_iter().collect())
    }

    /// Returns a short human readable summary, for error messages and logs.
    pub fn summary(&self) -> String {
        match self {
            Self::Unit => "unit".to_owned(),
            Self::Bool(inner) => inner.to_string(),
            Self::Int(inner) => inner.to_string(),
            Self::Float(inner) => inner.to_string(),
            Self::String(inner) => format!("\"{inner}\""),
            Self::Bytes(inner) => format!("{} bytes", inner.len()),
            Self::Struct(fields) => format!("struct with {} fields", fields.len()),
            Self::Enum { tag, fields } => format!("enum {tag} with {} fields", fields.len()),
            Self::List(elements) => format!("list of {}", elements.len()),
            Self::Dict(entries) => format!("dict of {}", entries.len()),
        }
    }

    /// Borrows the value at `path`, returning `None` when the path does not resolve.
    pub fn at(&self, path: &Path) -> Option<&Value> {
        path.resolve(self).ok()
    }

    /// Mutably borrows the value at `path`, returning `None` when it does not resolve.
    pub fn at_mut(&mut self, path: &Path) -> Option<&mut Value> {
        path.resolve_mut(self).ok()
    }

    /// Substitutes the `default` of every absent optional field, recursively.
    ///
    /// A definition is stored as authored. This fills in the defaults a schema declares
    /// so that later path lookups see a fully populated tree, without ever mutating the
    /// authored document itself.
    pub fn defaulted(&self, schema: &Schema) -> Value {
        match (&schema.kind, self) {
            (SchemaKind::Struct { fields }, Self::Struct(values)) => {
                let mut filled = FieldMap::new();
                for (name, field_schema) in fields {
                    match values.get(name) {
                        Some(value) => {
                            filled.insert(name.clone(), value.defaulted(field_schema));
                        }
                        None => {
                            if let Some(default) = &field_schema.attributes.default {
                                filled.insert(name.clone(), default.clone());
                            }
                        }
                    }
                }
                for (name, value) in values {
                    filled.entry(name.clone()).or_insert_with(|| value.clone());
                }
                Self::Struct(filled)
            }
            (
                SchemaKind::Dict {
                    value: entry_schema,
                },
                Self::Dict(entries),
            ) => Self::Dict(
                entries
                    .iter()
                    .map(|(name, value)| (name.clone(), value.defaulted(entry_schema)))
                    .collect(),
            ),
            (SchemaKind::List { element }, Self::List(elements)) => Self::List(
                elements
                    .iter()
                    .map(|element_value| element_value.defaulted(element))
                    .collect(),
            ),
            (SchemaKind::Enum { variants }, Self::Enum { tag, fields }) => {
                let Some(variant_schema) = variants.get(tag) else {
                    return self.clone();
                };
                let SchemaKind::Struct {
                    fields: variant_fields,
                } = &variant_schema.kind
                else {
                    return self.clone();
                };
                let mut filled = FieldMap::new();
                for (name, field_schema) in variant_fields {
                    match fields.get(name) {
                        Some(value) => {
                            filled.insert(name.clone(), value.defaulted(field_schema));
                        }
                        None => {
                            if let Some(default) = &field_schema.attributes.default {
                                filled.insert(name.clone(), default.clone());
                            }
                        }
                    }
                }
                for (name, value) in fields {
                    filled.entry(name.clone()).or_insert_with(|| value.clone());
                }
                Self::Enum {
                    tag: tag.clone(),
                    fields: filled,
                }
            }
            _ => self.clone(),
        }
    }
}
