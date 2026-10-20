//! Reports what the binary form of a document does with every kind of value.
//!
//! ```text
//! cargo run -p whiting-definition --example binary_probe
//! ```
//!
//! # What it reports
//!
//! 1. the bytes a hand-built value is written as, and what they are made of
//! 2. the round trip: the value that comes back, compared bit for bit
//! 3. the determinism: the same value written twice is the same bytes
//! 4. every refusal the format makes, and the error each one gives
//!
//! # Why an example and not a test
//!
//! The numbers this reports are the record of the format, and they are read rather than
//! gated: a build that reddens because a dependency moved says nothing about whether a
//! document is still written the way it was. The refusal report is the exception that
//! proves the rule, and it is printed for the same reason — a reader who wants to know
//! what a broken file says reads it here.
//!
//! The round trip is compared bit for bit rather than with `PartialEq`, because
//! `PartialEq` says a `NaN` is not itself and that a negative zero is a positive zero.
//! The format promises the bits, so the comparison is made on the bits.

use std::collections::BTreeMap;
use std::time::Instant;

use whiting_definition::binary;
use whiting_definition::{FieldMap, PackedLayout, PackedList, Value};

fn main() {
    println!("== a value with every kind in it ==");
    let value = the_value();
    println!("the value holds {} kinds", kinds(&value));

    let started = Instant::now();
    let bytes = binary::encode(&value);
    let written = started.elapsed();
    println!("encoded: {} bytes, in {written:.2?}", bytes.len());
    println!("the first bytes: {}", head(&bytes));
    println!();

    println!("== the round trip ==");
    let started = Instant::now();
    let read = binary::decode(&bytes);
    let read_time = started.elapsed();
    match read {
        Ok(read) => {
            if same(&value, &read) {
                println!("decoded: {read_time:.2?}, identical bit for bit");
            } else {
                println!("decoded: {read_time:.2?}, DIFFERENT");
                println!("  written: {value:?}");
                println!("  read:    {read:?}");
            }
        }
        Err(error) => println!("decoded: refused: {error}"),
    }
    println!();

    println!("== the determinism ==");
    let again = binary::encode(&value);
    if again == bytes {
        println!("written twice: the same {} bytes", bytes.len());
    } else {
        println!(
            "written twice: DIFFERENT ({} and {} bytes)",
            bytes.len(),
            again.len()
        );
    }
    println!();

    println!("== the refusals ==");
    for (what, bytes) in refusals() {
        match binary::decode(&bytes) {
            Ok(_) => println!("{what}: NOT REFUSED"),
            Err(error) => println!("{what}: {error}"),
        }
    }
}

/// Returns a value that holds every kind of value, and the awkward cases besides.
fn the_value() -> Value {
    let mut variant = FieldMap::new();
    variant.insert("intensity".to_owned(), Value::Float(0.5));
    variant.insert(
        "direction".to_owned(),
        Value::List(vec![
            Value::Float(0.0),
            Value::Float(0.0),
            Value::Float(-1.0),
        ]),
    );

    let mut dictionary = FieldMap::new();
    dictionary.insert("added".to_owned(), Value::Int(7));
    dictionary.insert("note".to_owned(), Value::Bool(false));

    let mut fields = FieldMap::new();
    fields.insert("unit".to_owned(), Value::Unit);
    fields.insert("yes".to_owned(), Value::Bool(true));
    fields.insert("no".to_owned(), Value::Bool(false));
    fields.insert("count".to_owned(), Value::Int(-42));
    fields.insert("largest".to_owned(), Value::Int(i64::MAX));
    fields.insert("smallest".to_owned(), Value::Int(i64::MIN));
    fields.insert("ratio".to_owned(), Value::Float(1.5));
    fields.insert("whole".to_owned(), Value::Float(2.0));
    fields.insert(
        "not_a_number".to_owned(),
        // A NaN with a payload, which is a different number from the quiet NaN.
        Value::Float(f64::from_bits(0x7ff8_0000_0000_0001)),
    );
    fields.insert("negative_zero".to_owned(), Value::Float(-0.0));
    fields.insert("infinite".to_owned(), Value::Float(f64::INFINITY));
    fields.insert("text".to_owned(), Value::String("a scene".to_owned()));
    fields.insert("empty_text".to_owned(), Value::String(String::new()));
    fields.insert("nothing".to_owned(), Value::Bytes(Vec::new()));
    fields.insert("bytes".to_owned(), Value::Bytes(vec![0, 1, 127, 255]));
    fields.insert("points".to_owned(), Value::Packed(small_packed()));
    fields.insert(
        "nested".to_owned(),
        Value::List(vec![Value::Packed(small_packed())]),
    );
    fields.insert(
        "list".to_owned(),
        Value::List(vec![Value::Int(1), Value::Unit]),
    );
    fields.insert("empty_list".to_owned(), Value::List(Vec::new()));
    fields.insert(
        "variant".to_owned(),
        Value::Enum {
            tag: "Point".to_owned(),
            fields: variant,
        },
    );
    fields.insert("empty_dict".to_owned(), Value::Dict(FieldMap::new()));
    fields.insert("dict".to_owned(), Value::Dict(dictionary));
    Value::Struct(fields)
}

/// Returns two points of three components each, packed the way a mesh is packed.
fn small_packed() -> PackedList {
    let layout = PackedLayout::new([("normal".to_owned(), 3), ("position".to_owned(), 3)]);
    let points = vec![
        Value::structure([
            ("normal".to_owned(), Value::List(vec![Value::Float(0.0); 3])),
            (
                "position".to_owned(),
                Value::List(vec![
                    Value::Float(0.0),
                    Value::Float(1.0),
                    Value::Float(2.0),
                ]),
            ),
        ]),
        Value::structure([
            ("normal".to_owned(), Value::List(vec![Value::Float(0.0); 3])),
            (
                "position".to_owned(),
                Value::List(vec![
                    Value::Float(3.0),
                    Value::Float(4.0),
                    Value::Float(5.0),
                ]),
            ),
        ]),
    ];
    PackedList::from_list(&layout, &points).expect("both points are three component runs")
}

/// Returns how many kinds of value a tree holds.
fn kinds(value: &Value) -> usize {
    let mut count = 1;
    match value {
        Value::Struct(fields) | Value::Dict(fields) => {
            for value in fields.values() {
                count += kinds(value);
            }
        }
        Value::Enum { fields, .. } => {
            for value in fields.values() {
                count += kinds(value);
            }
        }
        Value::List(values) => {
            for value in values {
                count += kinds(value);
            }
        }
        _ => {}
    }
    count
}

/// Returns the first bytes of a document as they are written.
fn head(bytes: &[u8]) -> String {
    let shown = bytes
        .iter()
        .take(48)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<String>>()
        .join(" ");
    if bytes.len() > 48 {
        format!("{shown} …")
    } else {
        shown
    }
}

/// Returns whether two values are the same bit for bit.
fn same(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Unit, Value::Unit) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Int(left), Value::Int(right)) => left == right,
        // The bits, not the number: a NaN is not itself to `PartialEq`, and a negative zero
        // is a positive zero to it.
        (Value::Float(left), Value::Float(right)) => left.to_bits() == right.to_bits(),
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Bytes(left), Value::Bytes(right)) => left == right,
        (Value::Struct(left), Value::Struct(right)) => same_fields(left, right),
        (Value::Dict(left), Value::Dict(right)) => same_fields(left, right),
        (
            Value::Enum {
                tag: left_tag,
                fields: left_fields,
            },
            Value::Enum {
                tag: right_tag,
                fields: right_fields,
            },
        ) => left_tag == right_tag && same_fields(left_fields, right_fields),
        (Value::List(left), Value::List(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| same(left, right))
        }
        (Value::Packed(left), Value::Packed(right)) => {
            left.layout() == right.layout() && left.bytes() == right.bytes()
        }
        _ => false,
    }
}

/// Returns whether two field maps hold the same fields with the same values.
fn same_fields(left: &BTreeMap<String, Value>, right: &BTreeMap<String, Value>) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|((left_name, left), (right_name, right))| {
                left_name == right_name && same(left, right)
            })
}

/// Returns the bytes of a document that is not this format, one for each refusal.
fn refusals() -> Vec<(&'static str, Vec<u8>)> {
    let mut refusals = Vec::new();

    refusals.push((
        "a file that is not a document",
        b"{\"name\": \"hall\"}".to_vec(),
    ));
    refusals.push(("an empty file", Vec::new()));

    let mut wrong_version = header();
    // The version sits at the head, so it is overwritten rather than appended.
    wrong_version[8..16].copy_from_slice(&(binary::VERSION + 1).to_le_bytes());
    wrong_version.push(0x01);
    refusals.push(("a version this crate does not read", wrong_version));

    let mut truncated = header();
    truncated.push(0x05);
    truncated.extend_from_slice(&100u64.to_le_bytes());
    refusals.push(("a payload that ends early", truncated));

    let mut unknown_tag = header();
    unknown_tag.push(0x7f);
    refusals.push(("a tag this format does not define", unknown_tag));

    let mut bad_utf8 = header();
    bad_utf8.push(0x05);
    bad_utf8.extend_from_slice(&2u64.to_le_bytes());
    bad_utf8.extend_from_slice(&[0xff, 0xfe]);
    refusals.push(("text that is not utf-8", bad_utf8));

    let mut bad_boolean = header();
    bad_boolean.push(0x02);
    bad_boolean.push(2);
    refusals.push(("a boolean that is not a boolean", bad_boolean));

    let mut duplicate = header();
    duplicate.push(0x07);
    duplicate.extend_from_slice(&2u64.to_le_bytes());
    for _ in 0..2 {
        duplicate.extend_from_slice(&1u64.to_le_bytes());
        duplicate.push(b'a');
        duplicate.push(0x03);
        duplicate.extend_from_slice(&1i64.to_le_bytes());
    }
    refusals.push(("a field map that holds one name twice", duplicate));

    let mut odd_run = header();
    odd_run.push(0x0a);
    odd_run.extend_from_slice(&1u64.to_le_bytes());
    odd_run.extend_from_slice(&8u64.to_le_bytes());
    odd_run.extend_from_slice(b"position");
    odd_run.extend_from_slice(&3u64.to_le_bytes());
    odd_run.extend_from_slice(&5u64.to_le_bytes());
    odd_run.extend_from_slice(&[1, 2, 3, 4, 5]);
    refusals.push(("a run that is not a whole number of strides", odd_run));

    let mut empty_layout = header();
    empty_layout.push(0x0a);
    empty_layout.extend_from_slice(&1u64.to_le_bytes());
    empty_layout.extend_from_slice(&8u64.to_le_bytes());
    empty_layout.extend_from_slice(b"position");
    empty_layout.extend_from_slice(&0u64.to_le_bytes());
    empty_layout.extend_from_slice(&0u64.to_le_bytes());
    refusals.push(("a layout with no components", empty_layout));

    let mut trailing = header();
    trailing.push(0x01);
    trailing.push(0xff);
    refusals.push(("bytes after the document", trailing));

    refusals
}

/// Returns the head of a document: the magic and the version.
fn header() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&binary::MAGIC);
    bytes.extend_from_slice(&binary::VERSION.to_le_bytes());
    bytes
}
