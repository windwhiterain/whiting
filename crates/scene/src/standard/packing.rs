//! Packing scene data into the bytes a buffer carries.
//!
//! # Invariants
//!
//! A buffer is a flat run of little endian four byte floats, because that is the only
//! thing a shader can index. The order is the field order of the schema that describes
//! the data, which is sorted by field name, so the same schema always produces the same
//! bytes. Every point of a mesh occupies the same number of bytes, because a shader
//! reads a fixed stride; a ragged point list is refused rather than packed into a buffer
//! the shader cannot walk.

use definition::{Path, Schema, SchemaKind, Value};

use crate::CompileError;

/// Returns the field order a struct schema lays its data out in.
pub fn layout(schema: &Schema, path: &Path) -> Result<Vec<String>, CompileError> {
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
        let Some(point_fields) = point.as_struct() else {
            return Err(CompileError::Unpackable {
                path: point_path,
                reason: format!("a point must be a struct, found {}", point.kind_name()),
            });
        };
        let start = bytes.len();
        for name in &fields {
            let value = point_fields
                .get(name)
                .ok_or_else(|| CompileError::Unpackable {
                    path: point_path.key(name),
                    reason: "a point is missing a field its own schema declares".to_owned(),
                })?;
            push(&mut bytes, value, &point_path.key(name))?;
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
        push(&mut bytes, field, &path.key(&name))?;
    }
    Ok(bytes)
}

/// Appends one value, flattening a list of numbers into consecutive components.
fn push(bytes: &mut Vec<u8>, value: &Value, path: &Path) -> Result<(), CompileError> {
    match value {
        Value::List(components) => {
            for (index, component) in components.iter().enumerate() {
                push(bytes, component, &path.index(index))?;
            }
            Ok(())
        }
        Value::Float(number) => {
            bytes.extend_from_slice(&number.to_le_bytes());
            Ok(())
        }
        Value::Int(number) => {
            bytes.extend_from_slice(&(*number as f64).to_le_bytes());
            Ok(())
        }
        other => Err(CompileError::Unpackable {
            path: path.clone(),
            reason: format!(
                "a {} is not a number and cannot be packed",
                other.kind_name()
            ),
        }),
    }
}
