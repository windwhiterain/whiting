//! The dynamic value tree that every document in the workspace is made of.

use std::collections::BTreeMap;

use facet::Facet;

use crate::packed::PackedList;
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
///
/// # The packed form of a list
///
/// A [`Value::List`] whose elements all have one structure may be carried as
/// [`Value::Packed`] instead: a flat run of bytes, one stride per element, described by the
/// layout its element schema implies. The two are the same data — a packed list expands back
/// into the list exactly — and they are the same *kind*, so a schema accepts either. What differs
/// is what reading one costs: a tree is walked, and a run is indexed.
///
/// The choice is the author's or a compiler's, per list. Nothing in the framework converts one
/// into the other behind an author's back, because expanding a run to read one element would undo
/// the reason for having it.
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
    /// A list whose elements all have one structure, carried as a flat run of bytes.
    ///
    /// This is the same kind as [`Value::List`] and satisfies the same schema; it is a
    /// representation, not a different thing. See [`crate::PackedList`].
    Packed(PackedList),
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
            // A packed list is a list; saying otherwise would make a schema that accepts one
            // report a kind mismatch to a caller that did nothing wrong.
            Self::Packed(_) => "list",
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

    /// Borrows the packed run when this is a packed list, otherwise returns `None`.
    pub fn as_packed(&self) -> Option<&PackedList> {
        match self {
            Self::Packed(list) => Some(list),
            _ => None,
        }
    }

    /// Mutably borrows the packed run when this is a packed list, otherwise returns `None`.
    pub fn as_packed_mut(&mut self) -> Option<&mut PackedList> {
        match self {
            Self::Packed(list) => Some(list),
            _ => None,
        }
    }

    /// Returns whether this value is a list in either of its two representations.
    ///
    /// A schema, a path and a reader all need to ask "is this a sequence" without caring how it
    /// is carried, and this is how they ask. Expanding a packed list to answer a question about
    /// its shape would cost the whole run, which is the one thing the run exists to avoid.
    pub const fn is_list(&self) -> bool {
        matches!(self, Self::List(_) | Self::Packed(_))
    }

    /// Returns how many elements this list holds, in either representation.
    ///
    /// This is a `len` for a tree and a division for a run, and it is exact for both because a
    /// packed list's byte length is always a whole number of strides.
    pub fn list_len(&self) -> Option<usize> {
        match self {
            Self::List(elements) => Some(elements.len()),
            Self::Packed(list) => Some(list.len()),
            _ => None,
        }
    }

    /// Expands a list into the tree form, leaving anything else as it is.
    ///
    /// This is the reading that undoes the packing, and it is deliberately explicit: nothing
    /// calls it on a path walk, because doing so would turn one element's read into a whole run's
    /// copy. A caller that genuinely needs the tree asks for it.
    pub fn expanded(&self) -> Value {
        match self {
            Self::Packed(list) => list.to_list(),
            other => other.clone(),
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

    /// Returns the inner value of a boolean, otherwise returns `None`.
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(flag) => Some(*flag),
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

    /// Inserts whole elements before `index`, in either representation.
    ///
    /// # Invariants
    ///
    /// A packed list keeps its representation: inserting into a run rebuilds the run rather than
    /// expanding it, because a list that was packed for its size should not grow into a tree the
    /// moment something is added to it. An element appended to a run is encoded against the run's
    /// own layout, so the run stays uniform.
    pub fn list_insert(&mut self, index: usize, values: Vec<Value>) -> Option<()> {
        match self {
            Self::List(elements) => {
                if index > elements.len() {
                    return None;
                }
                for (offset, value) in values.into_iter().enumerate() {
                    elements.insert(index + offset, value);
                }
                Some(())
            }
            Self::Packed(list) => list.insert(index, &values).ok(),
            _ => None,
        }
    }

    /// Removes a contiguous range of whole elements and returns them, in either representation.
    ///
    /// This is the exact inverse of [`Value::list_insert`] over the same range, which is what lets
    /// a delta invert a list mutation minimally in packed form as well as in tree form.
    pub fn list_remove(&mut self, start: usize, count: usize) -> Option<Vec<Value>> {
        match self {
            Self::List(elements) => {
                let end = start
                    .checked_add(count)
                    .filter(|end| *end <= elements.len())?;
                Some(elements.drain(start..end).collect())
            }
            Self::Packed(list) => list.remove(start, count).ok(),
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
            Self::Packed(list) => format!("list of {}", list.len()),
            Self::Dict(entries) => format!("dict of {}", entries.len()),
        }
    }

    /// Borrows the value at `path`, returning `None` when the path does not resolve.
    ///
    /// A path into a packed list answers with a decoded element, which is why this returns a
    /// [`Resolved`] rather than a plain borrow: see [`Resolved`](crate::Resolved).
    pub fn at(&self, path: &Path) -> Option<crate::path::Resolved<'_>> {
        path.resolve(self).ok()
    }

    /// Applies an edit to the value at `path`, returning `None` when the path does not resolve.
    ///
    /// # Invariants
    ///
    /// The edit runs at most once, and only after the whole path has resolved, so a rejected
    /// operation leaves the document untouched.
    pub fn update_at<R>(&mut self, path: &Path, edit: impl FnOnce(&mut Value) -> R) -> Option<R> {
        path.update(self, edit).ok()
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
            // A packed list is filled in the same way a tree list is and is packed again rather
            // than expanded: filling defaults must not turn a run back into fifty thousand maps,
            // which is the cost the run was chosen to avoid. A run that will not pack again was
            // already valid, so it is returned rather than mangled.
            (SchemaKind::List { element }, Self::Packed(list)) => {
                let filled: Vec<Value> = (0..list.len())
                    .map(|index| {
                        list.element(index)
                            .map_or(Value::Unit, |value| value.defaulted(element))
                    })
                    .collect();
                Self::Packed(
                    PackedList::from_list(list.layout(), &filled).unwrap_or_else(|_| list.clone()),
                )
            }
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
