//! How a schema lays out the floats a buffer carries.
//!
//! A point schema is already a description of what a vertex is, so the vertex layout a
//! renderer needs is derived from it rather than declared a second time. A field of the
//! schema is one attribute, a list field is as many components as it declares, and the
//! fields are laid out in schema order, which is sorted by name.
//!
//! # Invariants
//!
//! - The compiler that packs a buffer and the renderer that draws it both read the layout
//!   from here, so the two cannot disagree about a stride or an offset.
//! - Every field contributes four bytes per component, little endian, which is what
//!   [`DATA`](crate::binders::DATA) carries. There is no padding between attributes.
//! - A list field says how many components it has in its `components` extension. A list
//!   that does not say has no layout, and a buffer packed from it cannot be drawn.
//! - A field wider than four components is split into consecutive attributes of four,
//!   because a backend reads no more than four at one location.
//!
//! # Example
//!
//! ```
//! use definition::{Schema, SchemaKind};
//! use whiting_graphics::layout::{MAX_COMPONENTS, components_schema, vertex_layout};
//!
//! // A point of three floats is one attribute of three components.
//! let point = Schema::new(SchemaKind::structure([(
//!     "position".to_owned(),
//!     components_schema(3),
//! )]));
//! let layout = vertex_layout(&point).expect("a point schema has a layout");
//! assert_eq!(layout.stride_bytes, 12);
//! assert_eq!(layout.attributes.len(), 1);
//! assert_eq!(layout.attributes[0].components, 3);
//!
//! // A matrix is four attributes of four components, at consecutive locations.
//! let instance = Schema::new(SchemaKind::structure([(
//!     "model".to_owned(),
//!     components_schema(MAX_COMPONENTS),
//! )]));
//! let layout = vertex_layout(&instance).expect("a matrix schema has a layout");
//! assert_eq!(layout.stride_bytes, 64);
//! assert_eq!(layout.attributes.len(), 4);
//! assert_eq!(
//!     layout.attributes.iter().map(|a| a.offset_bytes).collect::<Vec<u64>>(),
//!     [0, 16, 32, 48]
//! );
//!
//! // A list that does not say how wide it is has no layout at all.
//! let vague = Schema::new(SchemaKind::structure([(
//!     "position".to_owned(),
//!     Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float))),
//! )]));
//! assert!(vertex_layout(&vague).is_err());
//! ```

use definition::{Schema, SchemaKind, Value};
use thiserror::Error;

/// Extension on a list field naming how many components it has.
pub const COMPONENTS: &str = "components";

/// The largest number of components one vertex attribute may have.
pub const MAX_ATTRIBUTE_COMPONENTS: usize = 4;

/// The largest number of components one field of a schema may have.
///
/// A field wider than an attribute is split across consecutive attributes, four
/// components each, which is how a backend reads a matrix.
pub const MAX_COMPONENTS: usize = 16;

/// One attribute of a vertex, as a renderer hands it to a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexAttribute {
    /// The location the shader reads this attribute at.
    pub shader_location: u32,
    /// The offset of the attribute from the start of a vertex, in bytes.
    pub offset_bytes: u64,
    /// How many components the attribute has.
    pub components: usize,
}

/// The layout of one vertex, derived from the schema of a point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexLayout {
    /// The distance between two vertices, in bytes.
    pub stride_bytes: u64,
    /// The attributes of a vertex, in the order they are laid out.
    pub attributes: Vec<VertexAttribute>,
}

impl VertexLayout {
    /// Returns how many vertices a buffer of `size_bytes` holds, or `None` when the size
    /// is not a whole number of vertices.
    pub fn vertex_count(&self, size_bytes: u64) -> Option<u64> {
        (self.stride_bytes > 0).then(|| size_bytes / self.stride_bytes)
    }
}

/// A schema does not say how its data is laid out.
#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("a vertex schema is a struct, {0} is not one")]
    NotAStruct(&'static str),
    #[error("the field `{field}` of a vertex schema is a {kind}, which has no float layout")]
    UnpackableField { field: String, kind: &'static str },
    #[error(
        "the field `{field}` of a vertex schema is a list, so it must declare how many \
         components it has in its `{COMPONENTS}` extension"
    )]
    UndeclaredComponents { field: String },
    #[error("the field `{field}` of a vertex schema declares {found} components")]
    BadComponents { field: String, found: String },
    #[error("the value is not what the schema it is packed against describes: {reason}")]
    Mismatch { reason: String },
}

/// Derives the vertex layout of a point schema.
///
/// # Invariants
///
/// A field of four components or fewer is one attribute. A wider field is split into
/// consecutive attributes of four components each, because a backend reads no more than
/// four components at one location and that is how a matrix is described to one.
pub fn vertex_layout(schema: &Schema) -> Result<VertexLayout, LayoutError> {
    let SchemaKind::Struct { fields } = &schema.kind else {
        return Err(LayoutError::NotAStruct(schema.kind_name()));
    };

    let mut attributes = Vec::new();
    let mut stride_bytes = 0u64;
    for (name, field) in fields {
        let components = components(name, field)?;
        for chunk in split(components) {
            attributes.push(VertexAttribute {
                shader_location: attributes.len() as u32,
                offset_bytes: stride_bytes,
                components: chunk,
            });
            stride_bytes += (chunk * 4) as u64;
        }
    }
    Ok(VertexLayout {
        stride_bytes,
        attributes,
    })
}

/// Splits a field wider than an attribute into the attributes it is read as.
fn split(components: usize) -> Vec<usize> {
    if components <= MAX_ATTRIBUTE_COMPONENTS {
        return vec![components];
    }
    (0..components / MAX_ATTRIBUTE_COMPONENTS)
        .map(|_| MAX_ATTRIBUTE_COMPONENTS)
        .collect()
}

/// Returns how many float components a field contributes to a vertex.
pub fn components(name: &str, field: &Schema) -> Result<usize, LayoutError> {
    match &field.kind {
        SchemaKind::Float | SchemaKind::Int => Ok(1),
        SchemaKind::List { .. } => declared_components(name, field),
        _ => Err(LayoutError::UnpackableField {
            field: name.to_owned(),
            kind: field.kind_name(),
        }),
    }
}

/// Returns how many components a list field declares.
fn declared_components(name: &str, field: &Schema) -> Result<usize, LayoutError> {
    let Some(declared) = field.attributes.extensions.get(COMPONENTS) else {
        return Err(LayoutError::UndeclaredComponents {
            field: name.to_owned(),
        });
    };
    let count = declared
        .as_int()
        .ok_or_else(|| LayoutError::BadComponents {
            field: name.to_owned(),
            found: declared.kind_name().to_owned(),
        })?;
    if !(1..=MAX_COMPONENTS as i64).contains(&count) {
        return Err(LayoutError::BadComponents {
            field: name.to_owned(),
            found: count.to_string(),
        });
    }
    if count > MAX_ATTRIBUTE_COMPONENTS as i64 && count % MAX_ATTRIBUTE_COMPONENTS as i64 != 0 {
        return Err(LayoutError::BadComponents {
            field: name.to_owned(),
            found: format!("{count}, which is wider than one attribute"),
        });
    }
    Ok(count as usize)
}

/// Builds a list schema of `count` floats, with the component count declared.
///
/// This is the one a point schema is written with, because a list without it has no
/// layout and a buffer packed from it cannot be drawn.
pub fn components_schema(count: usize) -> Schema {
    let schema = Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float)));
    match count {
        1 => schema,
        _ => schema.with_attributes(
            definition::Attributes::new().with_extension(COMPONENTS, Value::Int(count as i64)),
        ),
    }
}

/// Packs a value into the bytes a buffer carries.
///
/// # Invariants
///
/// The order is field order, which is sorted by name, and every number is four little
/// endian bytes with a list flattened into consecutive components. A struct or a dict is
/// flattened field by field, in the same order, so a nested uniform lands in the order a
/// schema describes it. That is the same rule [`vertex_layout`] derives offsets from, so a
/// compiler that packs with this and a renderer that reads a layout from the schema agree
/// by construction.
pub fn value_bytes(value: &Value) -> Result<Vec<u8>, LayoutError> {
    let mut bytes = Vec::new();
    push(&mut bytes, value)?;
    Ok(bytes)
}

/// Appends one value, flattening a list of numbers into consecutive components.
fn push(bytes: &mut Vec<u8>, value: &Value) -> Result<(), LayoutError> {
    match value {
        Value::List(components) => {
            for component in components {
                push(bytes, component)?;
            }
            Ok(())
        }
        Value::Struct(fields) | Value::Dict(fields) => {
            for field in fields.values() {
                push(bytes, field)?;
            }
            Ok(())
        }
        Value::Float(number) => {
            bytes.extend_from_slice(&(*number as f32).to_le_bytes());
            Ok(())
        }
        Value::Int(number) => {
            // A shader reads a buffer as floats, so an integer component is written the
            // same width as a float one.
            bytes.extend_from_slice(&(*number as f32).to_le_bytes());
            Ok(())
        }
        other => Err(LayoutError::UnpackableField {
            field: other.kind_name().to_owned(),
            kind: "a number, a list of numbers, or a struct of them",
        }),
    }
}

/// Packs a uniform value into the bytes a uniform buffer carries, laid out as its schema
/// says a shader reads it.
///
/// # Why this is not [`value_bytes`]
///
/// A shader declares the interface it reads, and a backend lays that interface out with
/// alignment: a three component vector starts on a sixteen byte boundary and a matrix on
/// one too. Packing the value tightly would put the second field of a struct four bytes
/// after the first, which is not where the shader looks for it. A schema sorts its fields
/// by name, which is the order a shader author writes them in, so laying the value out by
/// the schema is what makes a naturally written struct read correctly.
///
/// # Invariants
///
/// Every field starts at an offset that is a multiple of its alignment, padding between
/// fields is zeroed, and the result is a whole number of sixteen byte blocks. A field the
/// schema cannot lay out, or a value that does not match the schema, is an error rather
/// than a silently short buffer.
pub fn uniform_bytes(schema: &Schema, value: &Value) -> Result<Vec<u8>, LayoutError> {
    schema
        .validate(value)
        .map_err(|error| LayoutError::Mismatch {
            reason: error.to_string(),
        })?;
    let SchemaKind::Struct { fields } = &schema.kind else {
        return Err(LayoutError::NotAStruct(schema.kind_name()));
    };
    let Some(values) = value.as_field_map().or_else(|| value.variant_fields()) else {
        return Err(LayoutError::Mismatch {
            reason: format!("expected a struct to pack, found {}", value.kind_name()),
        });
    };

    let mut bytes = Vec::new();
    for field in uniform_layout(schema)? {
        let value = values
            .get(&field.name)
            .ok_or_else(|| LayoutError::Mismatch {
                reason: format!("the uniform has no `{}`", field.name),
            })?;
        write_at(&mut bytes, field.offset, &fields[&field.name], value)?;
    }
    // A uniform block is a whole number of sixteen byte blocks.
    bytes.resize(align_to(bytes.len(), 16), 0);
    Ok(bytes)
}

/// One field of a uniform layout: where it is written, and what is inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniformField {
    /// The name the field is declared under.
    pub name: String,
    /// The offset the field starts at, in bytes from the start of the block.
    pub offset: usize,
    /// The fields inside it, when it is a struct.
    pub fields: Vec<UniformField>,
}

/// Returns the layout of a uniform schema: where every field of it is written.
///
/// This is what a shader's own declaration is checked against. Two layouts that disagree
/// read the wrong bytes, and nothing else in a pipeline notices.
///
/// # Invariants
///
/// This is the only place the uniform layout is decided. The packer writes at the offsets
/// it returns and the check that a shader agrees reads them, so a change here changes both
/// together.
pub fn uniform_layout(schema: &Schema) -> Result<Vec<UniformField>, LayoutError> {
    let SchemaKind::Struct { fields } = &schema.kind else {
        return Err(LayoutError::NotAStruct(schema.kind_name()));
    };

    let mut laid_out = Vec::with_capacity(fields.len());
    let mut offset = 0usize;
    for (name, field) in fields {
        offset = align_to(offset, alignment_of(name, field)?);
        let inner = if matches!(field.kind, SchemaKind::Struct { .. }) {
            uniform_layout(field)?
        } else {
            Vec::new()
        };
        laid_out.push(UniformField {
            name: name.clone(),
            offset,
            fields: inner,
        });
        offset += size_of(name, field)?;
    }
    Ok(laid_out)
}

/// Returns the size a field occupies, before the next field is aligned past it.
///
/// A struct is padded to its own alignment, because a backend places the field after it on
/// that boundary rather than immediately after the last member.
fn size_of(name: &str, field: &Schema) -> Result<usize, LayoutError> {
    match &field.kind {
        SchemaKind::Float | SchemaKind::Int => Ok(4),
        SchemaKind::List { .. } => Ok(declared_components(name, field)? * 4),
        SchemaKind::Struct { fields } => {
            let mut offset = 0usize;
            let mut end = 0usize;
            for (member, member_schema) in fields {
                offset = align_to(offset, alignment_of(member, member_schema)?);
                end = offset + size_of(member, member_schema)?;
                offset = end;
            }
            Ok(align_to(end, alignment_of(name, field)?))
        }
        _ => Err(LayoutError::UnpackableField {
            field: name.to_owned(),
            kind: field.kind_name(),
        }),
    }
}

/// Writes one field of a uniform at the offset the layout gives it.
fn write_at(
    bytes: &mut Vec<u8>,
    base: usize,
    schema: &Schema,
    value: &Value,
) -> Result<(), LayoutError> {
    if let SchemaKind::Struct { fields } = &schema.kind {
        let Some(values) = value.as_field_map().or_else(|| value.variant_fields()) else {
            return Err(LayoutError::Mismatch {
                reason: format!("expected a struct to pack, found {}", value.kind_name()),
            });
        };
        for field in uniform_layout(schema)? {
            let value = values
                .get(&field.name)
                .ok_or_else(|| LayoutError::Mismatch {
                    reason: format!("the uniform has no `{}`", field.name),
                })?;
            write_at(bytes, base + field.offset, &fields[&field.name], value)?;
        }
        return Ok(());
    }

    let packed = value_bytes(value)?;
    let end = base + packed.len();
    if bytes.len() < end {
        bytes.resize(end, 0);
    }
    bytes[base..end].copy_from_slice(&packed);
    Ok(())
}

/// Returns the alignment a field's first component sits on.
///
/// A nested struct is aligned to sixteen, because a backend lays out a struct in a uniform
/// block on a sixteen byte boundary as a whole.
fn alignment_of(name: &str, schema: &Schema) -> Result<usize, LayoutError> {
    Ok(match &schema.kind {
        SchemaKind::Float | SchemaKind::Int => 4,
        SchemaKind::Struct { fields } => {
            let nested = fields
                .iter()
                .map(|(field, schema)| alignment_of(field, schema))
                .collect::<Result<Vec<usize>, LayoutError>>()?
                .into_iter()
                .max()
                .unwrap_or(4);
            nested.max(16)
        }
        SchemaKind::List { .. } => match declared_components(name, schema)? {
            1 => 4,
            2 => 8,
            _ => 16,
        },
        _ => {
            return Err(LayoutError::UnpackableField {
                field: name.to_owned(),
                kind: schema.kind_name(),
            });
        }
    })
}

/// Rounds an offset up to the next multiple of an alignment.
fn align_to(offset: usize, alignment: usize) -> usize {
    offset.div_ceil(alignment) * alignment
}
