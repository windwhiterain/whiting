//! A structural fingerprint of a `definition::Value`.
//!
//! # Why structural
//!
//! `definition` publishes no serialiser for [`Value`], so a hash of "the bytes a
//! serialiser would write" cannot be computed here. What is available is a
//! **structural** fingerprint: one word folded over every node's kind and every
//! payload it carries. Two values with the same fingerprint differ only by a
//! collision, and two values that differ anywhere differ in their fingerprint.
//!
//! # What it is not
//!
//! It is **not** a hash of a canonical text, and it does not order two values —
//! it answers one question, "are these two the same document", and a host that
//! wants a diff reads [`shape`] beside it.

use std::collections::BTreeMap;

/// The offset basis of the 64-bit FNV-1a the fingerprint folds.
const BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// The prime of the same fold.
const PRIME: u64 = 0x100_0000_01b3;

/// The fingerprint of `value`.
///
/// # Invariant
///
/// **Never `0`.** The fold starts at the FNV basis and every step mixes a kind's
/// bytes, so a value of any shape answers a non-zero word and `0` stays free to
/// mean "no document".
pub fn of(value: &definition::Value) -> u64 {
    let mut state = BASIS;
    fold_value(value, &mut state);
    match state {
        0 => u64::MAX,
        state => state,
    }
}

/// How many nodes of each kind `value` holds.
pub type Shape = BTreeMap<&'static str, u64>;

/// What `value` is made of: the node count of each kind in it.
///
/// # Invariant
///
/// A `Packed` run is **one** node however many bytes it carries, so the shape
/// answers "how much structure", not "how many bytes".
pub fn shape(value: &definition::Value) -> Shape {
    let mut counts = Shape::new();
    count_shape(value, &mut counts);
    counts
}

/// Fold one value's kind and payload into `state`.
fn fold_value(value: &definition::Value, state: &mut u64) {
    match value {
        definition::Value::Unit => tag(state, "unit"),
        definition::Value::Bool(bit) => {
            tag(state, "bool");
            *state = fold(*state, u64::from(*bit));
        }
        definition::Value::Int(number) => {
            tag(state, "int");
            *state = fold(*state, *number as u64);
        }
        definition::Value::Float(number) => {
            // A float compares by its bits, so the fingerprint folds the bits too.
            tag(state, "float");
            *state = fold(*state, number.to_bits());
        }
        definition::Value::String(text) => {
            tag(state, "string");
            fold_bytes(state, text.as_bytes());
        }
        definition::Value::Bytes(bytes) => {
            tag(state, "bytes");
            *state = fold(*state, bytes.len() as u64);
            fold_bytes(state, bytes);
        }
        definition::Value::Struct(fields) => {
            tag(state, "struct");
            fold_fields(state, fields);
        }
        definition::Value::Dict(fields) => {
            tag(state, "dict");
            fold_fields(state, fields);
        }
        definition::Value::Enum { tag: name, fields } => {
            tag(state, "enum");
            fold_bytes(state, name.as_bytes());
            fold_fields(state, fields);
        }
        definition::Value::List(elements) => {
            tag(state, "list");
            *state = fold(*state, elements.len() as u64);
            for element in elements {
                fold_value(element, state);
            }
        }
        definition::Value::Packed(run) => {
            tag(state, "packed");
            *state = fold(*state, run.len() as u64);
            *state = fold(*state, run.layout().stride_bytes() as u64);
            fold_bytes(state, run.bytes());
        }
    }
}

/// Fold a field map: its size, then each name and value in `BTreeMap` order.
fn fold_fields(state: &mut u64, fields: &BTreeMap<String, definition::Value>) {
    *state = fold(*state, fields.len() as u64);
    for (name, value) in fields {
        fold_bytes(state, name.as_bytes());
        fold_value(value, state);
    }
}

/// Fold one kind's name.
fn tag(state: &mut u64, name: &str) {
    fold_bytes(state, name.as_bytes());
}

/// One fold step over a machine word.
fn fold(state: u64, word: u64) -> u64 {
    word.to_le_bytes().into_iter().fold(state, |state, byte| {
        (state ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

/// Fold a run of bytes.
fn fold_bytes(state: &mut u64, bytes: &[u8]) {
    *state = bytes.iter().fold(*state, |state, byte| {
        (state ^ u64::from(*byte)).wrapping_mul(PRIME)
    });
}

/// Count one value's kinds, recursively.
fn count_shape(value: &definition::Value, counts: &mut Shape) {
    *counts.entry(value.kind_name()).or_insert(0) += 1;
    match value {
        definition::Value::Struct(fields) | definition::Value::Dict(fields) => {
            for value in fields.values() {
                count_shape(value, counts);
            }
        }
        definition::Value::Enum { fields, .. } => {
            for value in fields.values() {
                count_shape(value, counts);
            }
        }
        definition::Value::List(elements) => {
            for element in elements {
                count_shape(element, counts);
            }
        }
        _ => {}
    }
}

/// The fingerprint as sixteen lowercase hex digits.
pub fn hex(fingerprint: u64) -> String {
    format!("{fingerprint:016x}")
}
