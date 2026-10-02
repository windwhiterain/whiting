//! Reports what a byte sequence costs to transport, and what a reference to a file buys.
//!
//! ```text
//! cargo run -p whiting-device --example bytes_format_probe
//! ```
//!
//! # Why an example and not a test
//!
//! What is measured here is a property of the document spelling this workspace chooses, and
//! the numbers move with the machine and with the build profile. What is wanted is a printed
//! measurement beside the alternatives, not a gate that reddens a build the day a striding
//! loop gets slower. The probe still exits non-zero when a document stops round-tripping or
//! when a slot resolves to something other than the bytes that went in, so it reads as a
//! check as much as a report.
//!
//! # What it reports, in order
//!
//! 1. the codec's round trip over every byte sequence of nought, one and two bytes, a sample of
//!    longer ones, and the malformed spellings it refuses
//! 2. the json a packed run and a byte string are spelled as, and whether each reads back
//! 3. what a run of 20,000 points costs to write and to read, against the decimal spelling
//!    of the same run, which is measured here rather than quoted
//! 4. the same for 480,000 bytes of an image
//! 5. a snapshot holding both, which is the envelope the device writes its whole state with
//! 6. whether a `Reference::File` reaches a slot whose schema marks it as a reference, and
//!    what a store has to hand back for a list-shaped slot rather than a bytes-shaped one
//! 7. whether the `points` slot of the standard mesh schema is marked as a reference at all

use std::rc::Rc;
use std::time::Instant;

use definition::{
    DefinitionIdentifier, FileContent, FilePath, LoadError, Loaded, Origin, PackedBuilder,
    PackedLayout, PackedList, Path, Reference, Schema, SchemaKind, Store, Value, Wanted,
    bytes::{decode_bytes, encode_bytes},
};
use facet::Facet;
use whiting_device::{NamedDefinition, Snapshot};

/// The layout the measured mesh is built with: two three-component fields, so a 24 byte stride.
fn mesh_layout() -> PackedLayout {
    PackedLayout::new([("normal".to_owned(), 3), ("position".to_owned(), 3)])
}

/// A run of `points` points at that layout.
fn run(points: usize) -> PackedList {
    let mut builder = PackedBuilder::new(mesh_layout());
    builder.reserve(points);
    for index in 0..points {
        let x = index as f64;
        builder
            .field("normal", &[0.0, 1.0, 0.0])
            .expect("the layout declares normal first");
        builder
            .field("position", &[x, x, x])
            .expect("the layout declares position second");
    }
    builder.finish().expect("twenty thousand whole points")
}

fn main() {
    let mut failures = Vec::new();
    failures.extend(codec());
    failures.extend(spelling());
    failures.extend(run_cost());
    failures.extend(bytes_cost());
    failures.extend(snapshot_envelope());
    failures.extend(file_reference());
    failures.extend(mesh_points_slot());

    println!();
    if failures.is_empty() {
        println!("probe finished: nothing disagreed with what went in");
    } else {
        for failure in &failures {
            println!("DISCREPANCY: {failure}");
        }
        println!("probe finished: {} discrepancies", failures.len());
        std::process::exit(1);
    }
}

/// Whether the whole-device envelope carries both kinds of byte sequence, both ways.
///
/// A snapshot is the one document that holds every definition at once, written and read by the
/// device's own code rather than by a client, so it is the widest check that the spelling a
/// document is stored with is one the device can also read back.
fn snapshot_envelope() -> Vec<String> {
    println!("== the snapshot envelope ==");
    let mut failures = Vec::new();
    let snapshot = Snapshot {
        schemas: Vec::new(),
        definitions: vec![NamedDefinition {
            name: "probe.document".to_owned(),
            schema: "probe.any".to_owned(),
            version: 1,
            value: Value::structure([
                ("points".to_owned(), Value::Packed(run(4))),
                ("pixels".to_owned(), Value::Bytes(vec![0, 1, 127, 255])),
            ]),
        }],
    };
    let text = facet_json::to_string_pretty(&snapshot).expect("a snapshot writes");
    println!(
        "a snapshot holding a run of 4 points and 4 bytes of pixels is {} bytes",
        text.len()
    );
    match facet_json::from_str::<Snapshot>(&text) {
        Ok(read) if read == snapshot => println!("reads back: identical"),
        Ok(_) => failures.push("a snapshot does not round trip".to_owned()),
        Err(error) => failures.push(format!("a snapshot does not read back: {error}")),
    }
    println!();
    failures
}

/// Whether the codec is the pair of inverses it claims to be.
///
/// Every sequence of no, one and two bytes, which is every padding case there is, and a
/// sampled spread of longer ones. This is a check rather than a measurement, and it is here
/// because the codec is the one thing in this document spelling that is written rather than
/// borrowed.
fn codec() -> Vec<String> {
    println!("== the byte codec ==");
    let mut failures = Vec::new();
    let mut cases: Vec<Vec<u8>> = vec![Vec::new()];
    for first in 0..=255u8 {
        cases.push(vec![first]);
        for second in 0..=255u8 {
            cases.push(vec![first, second]);
        }
    }
    // A simple generator rather than a random one, so a failure is reproducible from the
    // probe alone.
    let mut state = 0x2545_f491_4f6c_dd1du64;
    for length in 3..=8 {
        for _ in 0..256 {
            let mut bytes = Vec::with_capacity(length);
            for _ in 0..length {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                bytes.push((state >> 33) as u8);
            }
            cases.push(bytes);
        }
    }

    let mut wrong = 0;
    for bytes in &cases {
        let text = encode_bytes(bytes);
        match decode_bytes(&text) {
            Ok(read) if read == *bytes => {}
            Ok(read) => {
                wrong += 1;
                if wrong < 4 {
                    println!("{bytes:?} spelled {text} reads back as {read:?}");
                }
            }
            Err(error) => {
                wrong += 1;
                if wrong < 4 {
                    println!("{bytes:?} spelled {text} does not read back: {error}");
                }
            }
        }
    }
    println!(
        "{} byte sequences round trip through the spelling, {wrong} do not",
        cases.len()
    );
    if wrong > 0 {
        failures.push(format!("{wrong} byte sequences do not round trip"));
    }
    // The spelling is canonical, so one byte sequence is one text and not several.
    for text in [
        "A", "AAAAA", "AA=A", "=AAA", "A===", "AB==", "AAH/x", "AAH//w=", "AAH//x==", "AAH]",
    ] {
        match decode_bytes(text) {
            Ok(bytes) => {
                println!(
                    "`{text}` is accepted as {bytes:?}, and it is not a spelling this crate writes"
                );
                failures.push(format!("`{text}` was accepted"));
            }
            Err(error) => println!("`{text}` is refused: {error}"),
        }
    }
    println!();
    failures
}

/// The spelling of a byte sequence, printed.
fn spelling() -> Vec<String> {
    println!("== how a byte sequence is spelled ==");
    let mut failures = Vec::new();
    for (label, value) in [
        ("Value::Bytes", Value::Bytes(vec![0, 1, 127, 255])),
        (
            "Value::Packed",
            Value::Packed(
                PackedList::from_list(
                    &PackedLayout::new([("position".to_owned(), 3)]),
                    &[Value::structure([(
                        "position".to_owned(),
                        Value::List(vec![Value::Float(1.0); 3]),
                    )])],
                )
                .expect("one whole point"),
            ),
        ),
    ] {
        let text = facet_json::to_string(&value).expect("a byte sequence spells");
        println!("--- {label} ---");
        println!("{text}");
        match facet_json::from_str::<Value>(&text) {
            Ok(read) if read == value => println!("reads back: identical"),
            Ok(read) => {
                println!("reads back DIFFERENT: {read:?}");
                failures.push(format!("{label} does not read back"));
            }
            Err(error) => {
                println!("reads back: from_str failed: {error}");
                failures.push(format!("{label} does not read back: {error}"));
            }
        }
    }
    println!();
    failures
}

/// What a packed run costs, against the decimal spelling of the same run.
fn run_cost() -> Vec<String> {
    println!("== what a packed run costs on the wire ==");
    let mut failures = Vec::new();
    let points = 20_000;
    let list = run(points);
    let value = Value::Packed(list.clone());
    let mut read_seconds = f64::NAN;
    println!(
        "a run of {points} points: stride {} bytes, {} bytes of run",
        list.layout().stride_bytes(),
        list.bytes().len()
    );

    let started = Instant::now();
    let compact = facet_json::to_string(&value).expect("a packed run serializes");
    let write = started.elapsed();
    println!(
        "to_string:        {} bytes, x{:.2} the run, in {write:.2?}",
        compact.len(),
        compact.len() as f64 / list.bytes().len() as f64
    );

    let started = Instant::now();
    let pretty = facet_json::to_string_pretty(&value).expect("a packed run pretty prints");
    println!(
        "to_string_pretty: {} bytes, x{:.2} the run, in {:.2?}",
        pretty.len(),
        pretty.len() as f64 / list.bytes().len() as f64,
        started.elapsed()
    );

    let started = Instant::now();
    match facet_json::from_str::<Value>(&compact) {
        Ok(read) if read == value => {
            read_seconds = started.elapsed().as_secs_f64();
            println!(
                "from_str:         {} bytes in {:.2?}, identical",
                compact.len(),
                started.elapsed()
            )
        }
        Ok(read) => {
            println!("from_str:         NOT identical: {read:?}");
            failures.push("a packed run does not round trip".to_owned());
        }
        Err(error) => {
            println!("from_str:         failed: {error}");
            failures.push(format!("a packed run does not read back: {error}"));
        }
    }

    // The spelling this replaced: the run as a decimal octet per byte. This is that spelling
    // measured rather than quoted, on the same machine and the same build, by a mirror of the
    // shape a packed list had before the run had a codec — a struct of the layout and a plain
    // `Vec<u8>` field, which is exactly what the serializer walked then.
    let before = DecimalRun {
        layout: list.layout().clone(),
        bytes: list.bytes().to_vec(),
    };
    let started = Instant::now();
    let decimal = facet_json::to_string(&before).expect("the decimal spelling writes");
    let decimal_write = started.elapsed();
    let started = Instant::now();
    let read = facet_json::from_str::<DecimalRun>(&decimal).expect("the decimal spelling reads");
    let decimal_read = started.elapsed();
    println!(
        "the same run as decimal octets:    {} bytes, x{:.2} the run, in {decimal_write:.2?} to \
         write and {decimal_read:.2?} to read",
        decimal.len(),
        decimal.len() as f64 / list.bytes().len() as f64,
    );
    if read.bytes != list.bytes() {
        failures.push("the decimal spelling of the run does not read back".to_owned());
    }
    println!(
        "so the spelling is x{:.2} the run against x{:.2}, and a read is x{:.1} faster",
        compact.len() as f64 / list.bytes().len() as f64,
        decimal.len() as f64 / list.bytes().len() as f64,
        decimal_read.as_secs_f64() / read_seconds
    );

    println!(
        "so a hundred thousand points is {:.1} MB of run and {:.1} MB of json",
        (100_000.0 * 24.0) / (1024.0 * 1024.0),
        (100_000.0 * 24.0 * compact.len() as f64 / list.bytes().len() as f64) / (1024.0 * 1024.0)
    );
    println!();
    failures
}

/// The shape a packed list had before the run was spelled as text: a layout and a plain byte
/// sequence, which a document wrote as one decimal number per octet.
#[derive(Debug, Facet)]
struct DecimalRun {
    layout: PackedLayout,
    bytes: Vec<u8>,
}

/// What an image's bytes cost, which is the other bulk payload a document carries.
fn bytes_cost() -> Vec<String> {
    println!("== what a byte string costs on the wire ==");
    let mut failures = Vec::new();
    let pixels: Vec<u8> = (0..480_000).map(|index| (index % 256) as u8).collect();
    let value = Value::Bytes(pixels.clone());
    let started = Instant::now();
    let compact = facet_json::to_string(&value).expect("bytes serialize");
    let write = started.elapsed();
    println!(
        "{} bytes as Value::Bytes: {} bytes of json, x{:.2}, in {write:.2?}",
        pixels.len(),
        compact.len(),
        compact.len() as f64 / pixels.len() as f64
    );
    let started = Instant::now();
    match facet_json::from_str::<Value>(&compact) {
        Ok(read) if read == value => {
            println!("reads back in {:.2?}, identical", started.elapsed())
        }
        Ok(_) => failures.push("a byte string does not round trip".to_owned()),
        Err(error) => failures.push(format!("a byte string does not read back: {error}")),
    }

    // And the same bytes as the bare byte sequence the variant used to hold.
    let started = Instant::now();
    let decimal = facet_json::to_string(&pixels).expect("the decimal array writes");
    let decimal_write = started.elapsed();
    let started = Instant::now();
    let read = facet_json::from_str::<Vec<u8>>(&decimal).expect("the decimal array reads");
    let decimal_read = started.elapsed();
    println!(
        "the same bytes as decimal octets: {} bytes, x{:.2}, in {decimal_write:.2?} to write and \
         {decimal_read:.2?} to read",
        decimal.len(),
        decimal.len() as f64 / pixels.len() as f64
    );
    if read != pixels {
        failures.push("the decimal spelling of a byte string does not read back".to_owned());
    }
    println!();
    failures
}

/// Whether a marked slot takes a file reference, and what a store must hand back for one.
fn file_reference() -> Vec<String> {
    println!("== a Reference::File at a marked slot ==");
    let mut failures = Vec::new();

    // A list-shaped slot that the schema marks as a reference, which is the shape a packed run
    // sits in: `SchemaKind::List`, not `SchemaKind::Bytes`.
    let element = Schema::new(SchemaKind::structure([(
        "position".to_owned(),
        Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float))),
    )]));
    let list_slot = Schema::new(SchemaKind::list(element)).reference();
    let document = Value::structure([(
        "points".to_owned(),
        Reference::File(FilePath::new("points.json")).to_value(),
    )]);
    let schema = Schema::new(SchemaKind::structure([(
        "points".to_owned(),
        list_slot.clone(),
    )]));
    schema
        .validate(&document)
        .expect("a marked slot accepts the spelling of a file");

    let packed = run(2);
    for (label, content) in [
        (
            "a document holding a packed run",
            FileContent::Document(Value::Packed(packed.clone())),
        ),
        (
            "a document holding the same points as a tree",
            FileContent::Document(packed.to_list()),
        ),
        (
            "raw asset bytes",
            FileContent::Bytes(packed.bytes().to_vec()),
        ),
    ] {
        let store = Files {
            content: content.clone(),
        };
        let origin = Origin::Definition(DefinitionIdentifier::new("scene"));
        match Loaded::load_in(&document, &schema, &origin, &store) {
            Ok(loaded) => {
                let points = loaded
                    .resolve(&Path::root().key("points"))
                    .expect("the slot resolved when the document loaded");
                let found = match points.node_of(&loaded) {
                    definition::Node::Packed(list) => {
                        format!("a packed run of {} points", list.len())
                    }
                    definition::Node::List(elements) => format!("a list of {}", elements.len()),
                    definition::Node::Bytes(bytes) => format!("{} bytes", bytes.len()),
                    other => other.kind_name().to_owned(),
                };
                println!("store hands back {label}: loaded, the slot holds {found}");
            }
            Err(error) => println!("store hands back {label}: refused: {error}"),
        }
    }

    // What the probe printed above says which of the three a slot of this shape accepts. The
    // raw bytes are the one that cannot reach a list-shaped slot, which is the whole question.
    let store = Files {
        content: FileContent::Bytes(packed.bytes().to_vec()),
    };
    let origin = Origin::Definition(DefinitionIdentifier::new("scene"));
    let mut accepted = false;
    if Loaded::load_in(&document, &schema, &origin, &store).is_ok() {
        accepted = true;
    }
    if accepted {
        failures.push(
            "raw asset bytes reached a list-shaped slot, which the schema kind should forbid"
                .to_owned(),
        );
    }
    println!();
    failures
}

/// Whether the standard mesh schema marks its `points` slot as a reference.
fn mesh_points_slot() -> Vec<String> {
    println!("== the points slot of the standard mesh schema ==");
    let mut failures = Vec::new();
    let schema = scene::standard::mesh_schema();
    let points = schema
        .struct_field("points")
        .expect("a mesh declares points");
    println!(
        "mesh.points is a reference slot: {} (its kind is {})",
        points.is_reference(),
        points.kind_name()
    );

    let document = Value::structure([
        ("label".to_owned(), Value::String("probe".to_owned())),
        ("topology".to_owned(), Value::variant("Triangles", [])),
        (
            "point_schema".to_owned(),
            Schema::new(SchemaKind::structure([(
                "position".to_owned(),
                graphics::layout::components_schema(3),
            )]))
            .to_value(),
        ),
        (
            "points".to_owned(),
            Reference::File(FilePath::new("points.bin")).to_value(),
        ),
    ]);
    match schema.validate(&document) {
        Ok(()) => {
            println!("a mesh whose points name a file validates: yes");
            if !points.is_reference() {
                failures.push(
                    "an unmarked points slot accepted a file reference, which the schema \
                     documentation says it should not"
                        .to_owned(),
                );
            }
        }
        Err(error) => {
            println!("a mesh whose points name a file validates: no: {error}");
            if points.is_reference() {
                failures.push(
                    "a marked points slot refused a file reference, which it should accept"
                        .to_owned(),
                );
            }
        }
    }
    println!();
    failures
}

/// A store that hands back one prepared answer for any file.
struct Files {
    content: FileContent,
}

impl Store for Files {
    fn definition(&self, name: &str) -> Result<Rc<Loaded>, LoadError> {
        Err(LoadError::Store {
            reason: format!("the probe holds no definition named `{name}`"),
        })
    }

    fn file(
        &self,
        _from: &Origin,
        _path: &FilePath,
        _wanted: Wanted,
    ) -> Result<FileContent, LoadError> {
        Ok(self.content.clone())
    }
}
