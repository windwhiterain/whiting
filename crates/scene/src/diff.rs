//! Turning one emitted output into the delta that produces the next.
//!
//! A compiler emits both a complete document and the delta that reaches it from the
//! previous one. The delta is not re-diffed centrally, because the compiler knows which
//! parts of its output it actually recomputed. This module is the generic part of that
//! work: it compares two values at one path and emits the operations that turn the
//! first into the second.
//!
//! # Why a shape change is a replace
//!
//! The operations map holds at most one primitive per path, and the primitives do not
//! cover every change of shape. A list that changes length needs a removal and an
//! append at the same path; a struct that gains a field has no add-field primitive at
//! all. Both are emitted as a whole value replace, which is coarser than necessary but
//! always correct. A compiler that knows better can emit the operations itself.

use std::collections::BTreeMap;

use definition::{Path, PrimitiveDelta, Value};

/// Emits the operations that turn one whole document into the next.
///
/// Each top level field is diffed on its own, so a change to one field never widens into
/// a replace of the whole document.
pub fn document_delta(previous: Option<&Value>, next: &Value) -> BTreeMap<Path, PrimitiveDelta> {
    let Some(after) = next.as_field_map() else {
        return value_delta(&Path::root(), previous, next);
    };
    let before = previous.and_then(Value::as_field_map);

    let mut operations = BTreeMap::new();
    for (name, value) in after {
        operations.extend(value_delta(
            &Path::root().key(name),
            before.and_then(|fields| fields.get(name)),
            value,
        ));
    }

    // A document that loses a top level field has no removal primitive for it either.
    let lost = before.is_some_and(|fields| fields.keys().any(|name| !after.contains_key(name)));
    if lost {
        operations.insert(Path::root(), PrimitiveDelta::Replace(next.clone()));
    }
    operations
}

/// Diffs one value against its previous form, choosing the primitive by shape.
fn value_delta(
    path: &Path,
    previous: Option<&Value>,
    next: &Value,
) -> BTreeMap<Path, PrimitiveDelta> {
    match next {
        Value::List(_) => list_delta(path, previous, Some(next)),
        _ => struct_delta(path, previous, Some(next)),
    }
}

/// Emits the operations that turn `previous` into `next` for a list at `path`.
pub fn list_delta(
    path: &Path,
    previous: Option<&Value>,
    next: Option<&Value>,
) -> BTreeMap<Path, PrimitiveDelta> {
    let mut operations = BTreeMap::new();
    let (Some(Value::List(before)), Some(Value::List(after))) = (previous, next) else {
        return operations;
    };

    if before.len() != after.len() {
        operations.insert(
            path.clone(),
            PrimitiveDelta::Replace(Value::List(after.clone())),
        );
        return operations;
    }

    for (index, (old, new)) in before.iter().zip(after).enumerate() {
        if old != new {
            operations.insert(
                path.clone().index(index),
                PrimitiveDelta::Replace(new.clone()),
            );
        }
    }
    operations
}

/// Emits the operations that turn `previous` into `next` for one value at `path`.
///
/// # Invariants
///
/// Only a dict can gain an entry through [`PrimitiveDelta::AppendEntries`], so a struct
/// that gains a field is replaced whole. A field that already exists is replaced in
/// place at `path.key(field)`, which is what keeps a small edit a small delta.
pub fn struct_delta(
    path: &Path,
    previous: Option<&Value>,
    next: Option<&Value>,
) -> BTreeMap<Path, PrimitiveDelta> {
    let mut operations = BTreeMap::new();
    let (Some(before), Some(after)) = (previous, next) else {
        return operations;
    };

    // A leaf, or an enum: one value in, one value out.
    let Some(after_fields) = after.as_field_map() else {
        if before != after {
            operations.insert(path.clone(), PrimitiveDelta::Replace(after.clone()));
        }
        return operations;
    };
    let Some(before_fields) = before.as_field_map() else {
        return whole_replace(path, after);
    };

    let removed: Vec<String> = before_fields
        .keys()
        .filter(|name| !after_fields.contains_key(*name))
        .cloned()
        .collect();
    let added: Vec<(String, Value)> = after_fields
        .iter()
        .filter(|(name, _)| !before_fields.contains_key(*name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();

    let can_append = matches!(after, Value::Dict(_));
    if !added.is_empty() && (!can_append || !removed.is_empty()) {
        return whole_replace(path, after);
    }
    if !removed.is_empty() {
        operations.insert(path.clone(), PrimitiveDelta::remove_keys(removed));
    }
    if !added.is_empty() {
        operations.insert(path.clone(), PrimitiveDelta::append_entries(added));
    }
    for (name, value) in after_fields {
        if before_fields.get(name).is_some_and(|old| old != value) {
            operations.insert(
                path.clone().key(name),
                PrimitiveDelta::Replace(value.clone()),
            );
        }
    }
    operations
}

/// Returns the single operation that replaces a whole container.
fn whole_replace(path: &Path, value: &Value) -> BTreeMap<Path, PrimitiveDelta> {
    let mut operations = BTreeMap::new();
    operations.insert(path.clone(), PrimitiveDelta::Replace(value.clone()));
    operations
}
