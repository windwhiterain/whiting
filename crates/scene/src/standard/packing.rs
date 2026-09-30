//! Packing scene data into the bytes a buffer carries.
//!
//! # Invariants
//!
//! A buffer is a flat run of little endian four byte floats, because that is the only
//! thing a shader can index, and the order is the one
//! [`value_bytes`](graphics::value_bytes) writes, which is the order
//! [`vertex_layout`](graphics::vertex_layout) derives offsets from. Every point of a mesh
//! occupies the same number of bytes, because a shader reads a fixed stride; a ragged
//! point list is refused rather than packed into a buffer the shader cannot walk.

use definition::{Path, Schema, SchemaKind, Value};
use graphics::value_bytes;

use crate::CompileError;

/// Packs the points of a mesh into one buffer.
pub fn pack_points(
    schema: &Schema,
    points: &[Value],
    path: &Path,
) -> Result<Vec<u8>, CompileError> {
    let fields = layout(schema, path)?;
    let mut bytes = Vec::new();
    let mut stride = None;

    for (index, point) in points.iter().enumerate() {
        let point_path = path.index(index);
        let point_fields = point.as_struct().ok_or_else(|| CompileError::Unpackable {
            path: point_path.clone(),
            reason: format!("a point must be a struct, found {}", point.kind_name()),
        })?;
        let start = bytes.len();
        for name in &fields {
            let field = point_fields
                .get(name)
                .ok_or_else(|| CompileError::Unpackable {
                    path: point_path.key(name),
                    reason: "a point is missing a field its own schema declares".to_owned(),
                })?;
            bytes.extend_from_slice(&value_bytes(field).map_err(|error| {
                CompileError::Unpackable {
                    path: point_path.key(name),
                    reason: error.to_string(),
                }
            })?);
        }
        let width = bytes.len() - start;
        match stride {
            None => stride = Some(width),
            Some(expected) if expected != width => {
                return Err(CompileError::Unpackable {
                    path: point_path,
                    reason: format!(
                        "every point of a mesh must occupy the same number of bytes, found \
                         {width} and {expected}"
                    ),
                });
            }
            Some(_) => {}
        }
    }
    Ok(bytes)
}

/// Packs one struct of numbers into a buffer, using the field order of its schema.
pub fn pack_struct(schema: &Schema, value: &Value, path: &Path) -> Result<Vec<u8>, CompileError> {
    let fields = layout(schema, path)?;
    let value_fields = value.as_struct().ok_or_else(|| CompileError::Unpackable {
        path: path.clone(),
        reason: format!("expected a struct to pack, found {}", value.kind_name()),
    })?;

    let mut bytes = Vec::new();
    for name in fields {
        let field = value_fields
            .get(&name)
            .ok_or_else(|| CompileError::Unpackable {
                path: path.key(&name),
                reason: "the data is missing a field its own schema declares".to_owned(),
            })?;
        bytes.extend_from_slice(
            &value_bytes(field).map_err(|error| CompileError::Unpackable {
                path: path.key(&name),
                reason: error.to_string(),
            })?,
        );
    }
    Ok(bytes)
}

/// Returns the field order a struct schema lays its data out in.
fn layout(schema: &Schema, path: &Path) -> Result<Vec<String>, CompileError> {
    match &schema.kind {
        SchemaKind::Struct { fields } => Ok(fields.keys().cloned().collect()),
        _ => Err(CompileError::Unpackable {
            path: path.clone(),
            reason: "a standard shader must declare a struct schema, because the data is \
                     packed according to it"
                .to_owned(),
        }),
    }
}
