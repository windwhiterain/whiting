//! Reading the scene document.
//!
//! The document is already schema-verified by the time an emitter sees it, so nothing here
//! re-validates structure. What it does do is refuse a value it cannot turn into a number
//! or a string, because "the schema said this is a float and it is not" is a bug worth
//! surfacing rather than a case to paper over with a default.

use super::error::{EmitError, EmitResult};
use definition::{FieldMap, Value};

/// Borrows a field from a struct or dict.
pub fn field<'a>(fields: &'a FieldMap, name: &str) -> Option<&'a Value> {
    fields.get(name)
}

/// Borrows a field, or reports which one was missing.
///
/// The context string names the thing the field belongs to, so the message reads "light
/// 'key' has no color" rather than "no color", which is a message nobody can act on.
pub fn require<'a>(fields: &'a FieldMap, name: &str, context: &str) -> EmitResult<&'a Value> {
    fields.get(name).ok_or_else(|| EmitError::Malformed {
        detail: format!("{context} has no field '{name}'"),
    })
}

/// Reads a string field.
pub fn text(value: &Value, context: &str) -> EmitResult<String> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| EmitError::Malformed {
            detail: format!("{context} is not a string"),
        })
}

/// Reads a number, accepting an integer where a float is expected.
///
/// The document language distinguishes `Int` and `Float`, and an author writing `1` gets an
/// `Int`. Rejecting that would make `energy: 5` a malformed scene, which is not what the
/// author meant and not what the schema is about.
pub fn number(value: &Value, context: &str) -> EmitResult<f64> {
    match value {
        Value::Float(number) => Ok(*number),
        Value::Int(number) => Ok(*number as f64),
        Value::Bool(flag) => Ok(f64::from(u8::from(*flag))),
        _ => Err(EmitError::Malformed {
            detail: format!("{context} is not a number"),
        }),
    }
}

/// Reads a string field by name.
pub fn text_field(fields: &FieldMap, name: &str, context: &str) -> EmitResult<String> {
    text(
        require(fields, name, context)?,
        &format!("{context}'s '{name}'"),
    )
}

/// Reads a list of numbers, flattening the structure the document uses for a triple.
///
/// A colour is written `[1.0, 0.95, 0.9]`, a list of three floats. A packed list, which is
/// what the compiled pipeline produces, is also accepted because a document that has been
/// through the compiler carries its points in packed form and re-emitting it should not
/// require unpacking first.
pub fn numbers(value: &Value, context: &str) -> EmitResult<Vec<f64>> {
    if let Some(elements) = value.as_list() {
        return elements
            .iter()
            .enumerate()
            .map(|(index, element)| number(element, &format!("{context}[{index}]")))
            .collect();
    }
    if let Some(packed) = value.as_packed() {
        return packed_floats(packed);
    }
    Err(EmitError::Malformed {
        detail: format!("{context} is not a list of numbers"),
    })
}

/// Reads exactly three numbers, which is what a colour and a position both are.
pub fn triple(value: &Value, context: &str) -> EmitResult<[f64; 3]> {
    let values = numbers(value, context)?;
    if values.len() != 3 {
        return Err(EmitError::Malformed {
            detail: format!("{context} has {} components, expected 3", values.len()),
        });
    }
    Ok([values[0], values[1], values[2]])
}

/// Reads the elements of a list field, treating an absent field as empty.
///
/// A scene's `lights`, `meshes` and `mesh_materials` are all optional, and an empty list and
/// an absent field mean the same thing to a renderer. Anything that is *not* optional goes
/// through [`require`] instead, so that a field the schema says must be there and is not
/// still fails.
pub fn list_or_empty<'a>(
    fields: &'a FieldMap,
    name: &str,
    context: &str,
) -> EmitResult<&'a [Value]> {
    match fields.get(name) {
        Some(value) => value.as_list().ok_or_else(|| EmitError::Malformed {
            detail: format!("{context}'s '{name}' is not a list"),
        }),
        None => Ok(&[]),
    }
}

/// The floats inside a packed run, decoded one element at a time.
///
/// A packed list is a buffer rather than a tree of `Value`, so reading it means decoding
/// each element against the run's own layout. `PackedList::element` is used rather than
/// reinterpreting the bytes, so the stride and the field layout stay the run's business and
/// a layout that is not four-byte floats is reported rather than misread.
fn packed_floats(packed: &definition::PackedList) -> EmitResult<Vec<f64>> {
    let mut out = Vec::with_capacity(packed.len());
    for index in 0..packed.len() {
        let element = packed
            .element(index)
            .map_err(|error| EmitError::Malformed {
                detail: format!("packed element {index}: {error}"),
            })?;
        let fields = element.as_field_map().ok_or_else(|| EmitError::Malformed {
            detail: format!("packed element {index} is not a structure"),
        })?;
        for (name, value) in fields {
            out.extend(numbers(
                value,
                &format!("packed element {index}'s '{name}'"),
            )?);
        }
    }
    Ok(out)
}
