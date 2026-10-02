//! Reports the json that `facet_json` gives every variant of `definition::Value`.
//!
//! ```text
//! cargo run -p whiting-device --example facet_value_probe
//! ```
//!
//! # Why an example and not a test
//!
//! The encoding is a property of the pinned `facet-json` version rather than of this workspace,
//! so what is wanted is a full printed report of what the encoder does, not a gate that reddens
//! a build the day a dependency moves. The probe still exits non-zero when the json that comes
//! back is not the value that went in, or when a live submission is refused, so it reads as a
//! check as much as a report. `cargo test --workspace` builds it, which is also why it carries
//! no `required-features`: nothing here needs a gpu adapter.
//!
//! # What it reports, in order
//!
//! 1. the pretty json of every `Value` variant, and whether `from_str` reads it back identical
//! 2. the fidelity of a few `f64` values, including the ones json cannot spell
//! 3. the two byte encodings this version offers, and which of them reads back
//! 4. what a packed run costs on the wire, against the expanded tree of the same data
//! 5. the exact schema identifiers `register_standard_scene_schema` registers
//! 6. a `Snapshot` envelope, and a live `PUT /definition/{name}` and `POST /restore` against a
//!    real device on a real socket, through the real router
//!
//! # The one place this gets out of the reader's way
//!
//! Reading the device's own snapshot back is the one step that does not run on the thread it was
//! called from. A snapshot of a few schemas is six hundred kilobytes that nest forty levels, and
//! `from_str` needs more than the megabyte a Windows main thread has by default: the first run of
//! this probe died with a stack overflow before it could finish the report. The parse therefore
//! runs on a thread with [`snapshot_stack_bytes`], which is a probe decision and not a claim that
//! the default is enough. Set `FACET_PROBE_STACK` to measure the depth this reader actually needs.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use definition::{
    DefinitionIdentifier, FieldMap, PackedBuilder, PackedLayout, PackedList, Registry, Schema,
    SchemaIdentifier, SchemaKind, Value,
};
use facet::Facet;
use scene::{ReferenceScene, register_standard_scene_schema};
use whiting_device::{Device, NamedDefinition, NamedSchema, NoopRenderer, Snapshot};

/// The body `PUT /definition/{name}` deserializes.
///
/// A copy of the route's own `PutDefinition`, because the types an http handler takes are
/// private to the crate that serves them. The field names, order and types are the route's, so
/// the text this prints is the text the route reads.
#[derive(Debug, Facet)]
struct PutDefinitionBody {
    schema: String,
    value: Value,
}

/// The body `PUT /definition/{name}` answers with.
#[derive(Debug, Facet)]
struct DefinitionReportBody {
    name: String,
    schema: String,
    version: u64,
    value: Value,
}

/// The body `POST /snapshot` and `POST /restore` deserialize.
#[derive(Debug, Facet)]
struct SnapshotRequestBody {
    path: String,
}

#[tokio::main]
async fn main() {
    let mut failures = Vec::new();
    failures.extend(variants());
    failures.extend(packed_expansion());
    failures.extend(float_fidelity());
    failures.extend(byte_encodings());
    failures.extend(packed_cost());
    failures.extend(schema_identifiers());
    failures.extend(submission().await);

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

/// Every variant of `Value`, printed, then read back.
fn variants() -> Vec<String> {
    println!("== every variant of Value ==");
    println!();
    let mut failures = Vec::new();
    for (label, value) in cases() {
        println!("--- {label} ---");
        println!("rust: {}", value.summary());
        match facet_json::to_string_pretty(&value) {
            Ok(text) => {
                println!("json:\n{text}");
                match facet_json::from_str::<Value>(&text) {
                    Ok(read) if read == value => {
                        println!("read back as: Value::{}", variant_name(&read));
                        println!("round trip: identical");
                    }
                    Ok(read) => {
                        println!("read back as: Value::{}", variant_name(&read));
                        println!("round trip: DIFFERENT");
                        println!("  original:  {value:?}");
                        println!("  read back: {read:?}");
                        match facet_json::to_string_pretty(&read) {
                            Ok(again) => println!("  re-encoded:\n{again}"),
                            Err(error) => println!("  re-encoding failed: {error}"),
                        }
                        failures.push(format!("Value::{label} does not round trip"));
                    }
                    Err(error) => {
                        println!("round trip: from_str failed: {error}");
                        failures.push(format!("Value::{label} does not read back: {error}"));
                    }
                }
            }
            Err(error) => {
                println!("to_string_pretty failed: {error}");
                failures.push(format!("Value::{label} does not serialize: {error}"));
            }
        }
        println!();
    }
    failures
}

/// One sample of each variant, the packed one built the way a mesh is built.
fn cases() -> Vec<(&'static str, Value)> {
    let mut fields = FieldMap::new();
    fields.insert("height".to_owned(), Value::Float(2.0));
    fields.insert("name".to_owned(), Value::String("block".to_owned()));

    let mut variant_fields = FieldMap::new();
    variant_fields.insert("intensity".to_owned(), Value::Float(0.5));
    variant_fields.insert(
        "direction".to_owned(),
        Value::List(vec![
            Value::Float(0.0),
            Value::Float(0.0),
            Value::Float(-1.0),
        ]),
    );

    let mut dict = FieldMap::new();
    dict.insert("added".to_owned(), Value::Int(7));
    dict.insert("note".to_owned(), Value::Bool(false));

    vec![
        ("Unit", Value::Unit),
        ("Bool", Value::Bool(true)),
        ("Int", Value::Int(-42)),
        ("Float", Value::Float(1.5)),
        ("String", Value::String("scene".to_owned())),
        ("Bytes", Value::Bytes(vec![0, 1, 127, 255])),
        ("Struct", Value::Struct(fields)),
        (
            "Enum",
            Value::Enum {
                tag: "Point".to_owned(),
                fields: variant_fields,
            },
        ),
        (
            "List",
            Value::List(vec![
                Value::Int(1),
                Value::String("two".to_owned()),
                Value::Unit,
            ]),
        ),
        ("Packed", Value::Packed(small_packed())),
        ("Dict", Value::Dict(dict)),
    ]
}

/// Two points of three components, packed by the constructor a compiler uses.
fn small_packed() -> PackedList {
    let layout = PackedLayout::new([("position".to_owned(), 3)]);
    let points = vec![
        Value::structure([(
            "position".to_owned(),
            Value::List(vec![
                Value::Float(0.0),
                Value::Float(0.0),
                Value::Float(0.0),
            ]),
        )]),
        Value::structure([(
            "position".to_owned(),
            Value::List(vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ]),
        )]),
    ];
    PackedList::from_list(&layout, &points).expect("both points are three component runs")
}

/// The variant a value came back as, for a report that has to name it.
fn variant_name(value: &Value) -> &'static str {
    match value {
        Value::Unit => "Unit",
        Value::Bool(_) => "Bool",
        Value::Int(_) => "Int",
        Value::Float(_) => "Float",
        Value::String(_) => "String",
        Value::Bytes(_) => "Bytes",
        Value::Struct(_) => "Struct",
        Value::Enum { .. } => "Enum",
        Value::List(_) => "List",
        Value::Packed(_) => "Packed",
        Value::Dict(_) => "Dict",
    }
}

/// What a packed run is a representation *of*, which is the question a reader of the json asks.
fn packed_expansion() -> Vec<String> {
    println!("== what a packed list expands into ==");
    let list = small_packed();
    println!(
        "PackedList::element(1) decodes one point without touching the other: {:?}",
        list.element(1).expect("the run holds two points")
    );
    println!("PackedList::to_list() expands the whole run:");
    println!(
        "{}",
        facet_json::to_string_pretty(&list.to_list()).expect("a list serializes")
    );
    println!(
        "and that tree satisfies the same schema as the run, so a reader that wants a `Value::List` gets one only by asking for it"
    );
    println!();
    Vec::new()
}

/// How exactly one float survives the trip, bit for bit.
fn float_fidelity() -> Vec<String> {
    println!("== f64 fidelity ==");
    let mut failures = Vec::new();
    for number in [1.5f64, 0.1, 0.1 + 0.2, 1.0 / 3.0, -0.0, 5e-324, -2.5e-8] {
        let text = facet_json::to_string(&Value::Float(number)).expect("a float serializes");
        match facet_json::from_str::<Value>(&text) {
            Ok(Value::Float(read)) => {
                let exact = read.to_bits() == number.to_bits();
                println!(
                    "{number:?} -> {} -> {read:?}  value equal: {}, bits equal: {exact}",
                    shown(&text),
                    read == number
                );
                if !exact {
                    failures.push(format!(
                        "Float({number:?}) reads back as {read:?}, which is not the same bits"
                    ));
                }
            }
            Ok(other) => {
                println!("{number:?} -> {} -> not a float: {other:?}", shown(&text));
                failures.push(format!("Float({number:?}) reads back as {other:?}"));
            }
            Err(error) => {
                println!("{number:?} -> {} -> from_str failed: {error}", shown(&text));
                failures.push(format!("Float({number:?}) does not read back: {error}"));
            }
        }
    }

    println!();
    println!("and across magnitudes, where the writer stops writing a form the reader takes:");
    for exponent in [15, 30, 38, 39, 40, 300] {
        let number = 10f64.powi(exponent);
        let text = facet_json::to_string(&Value::Float(number)).expect("a float serializes");
        match facet_json::from_str::<Value>(&text) {
            Ok(Value::Float(read)) if read.to_bits() == number.to_bits() => {
                println!("1e{exponent}: {} -> identical", shown(&text))
            }
            Ok(Value::Float(read)) => {
                println!(
                    "1e{exponent}: {} -> {read:?}, not the same bits",
                    shown(&text)
                );
                failures.push(format!("Float(1e{exponent}) does not survive the trip"));
            }
            Ok(other) => {
                println!("1e{exponent}: {} -> not a float: {other:?}", shown(&text));
                failures.push(format!("Float(1e{exponent}) reads back as {other:?}"));
            }
            Err(error) => {
                println!("1e{exponent}: {} -> from_str failed: {error}", shown(&text));
                failures.push(format!("Float(1e{exponent}) does not read back: {error}"));
            }
        }
    }

    println!();
    println!("and the three values json has no spelling for:");
    for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        match facet_json::to_string(&Value::Float(number)) {
            Ok(text) => match facet_json::from_str::<Value>(&text) {
                Ok(Value::Float(read)) => println!(
                    "{number:?} -> {text} -> {read:?}  bits equal: {}",
                    read.to_bits() == number.to_bits()
                ),
                Ok(other) => println!("{number:?} -> {text} -> not a float: {other:?}"),
                Err(error) => println!("{number:?} -> {text} -> from_str failed: {error}"),
            },
            Err(error) => println!("{number:?} -> to_string failed: {error}"),
        }
    }
    println!();
    failures
}

/// A number text, shortened when the writer spelled it out in full.
fn shown(text: &str) -> String {
    if text.len() <= 40 {
        return text.to_owned();
    }
    format!(
        "{}...{} ({} characters)",
        &text[..20],
        &text[text.len() - 8..],
        text.len()
    )
}

/// The two spellings of a byte sequence this version offers, and whether each reads back.
fn byte_encodings() -> Vec<String> {
    println!("== how a Vec<u8> is spelled ==");
    let mut failures = Vec::new();
    let value = Value::Bytes(vec![0, 1, 127, 255]);

    let array = facet_json::to_string_pretty(&value).expect("the default spelling serializes");
    println!("default (BytesFormat::Array):\n{array}");
    match facet_json::from_str::<Value>(&array) {
        Ok(read) if read == value => println!("default reads back: identical"),
        Ok(read) => {
            println!("default reads back: {read:?}");
            failures.push("Value::Bytes does not read back".to_owned());
        }
        Err(error) => {
            println!("default reads back: from_str failed: {error}");
            failures.push(format!("Value::Bytes does not read back: {error}"));
        }
    }

    let options = facet_json::SerializeOptions::default().bytes_as_hex();
    let hex =
        facet_json::to_string_with_options(&value, &options).expect("the hex spelling writes");
    println!("opaque (BytesFormat::Hex), written with to_string_with_options:\n{hex}");
    match facet_json::from_str::<Value>(&hex) {
        Ok(read) if read == value => println!("hex reads back with the default reader: identical"),
        Ok(read) => println!("hex reads back with the default reader: {read:?}"),
        Err(error) => println!("hex reads back with the default reader: from_str failed: {error}"),
    }
    println!();
    failures
}

/// What a packed run costs as json, against the same data as a tree.
fn packed_cost() -> Vec<String> {
    println!("== what a packed run costs on the wire ==");
    let mut failures = Vec::new();
    let layout = PackedLayout::new([("normal".to_owned(), 3), ("position".to_owned(), 3)]);
    let points = 20_000;
    let mut builder = PackedBuilder::new(layout.clone());
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
    let list = builder.finish().expect("twenty thousand whole points");
    let value = Value::Packed(list.clone());
    println!(
        "a run of {points} points: stride {} bytes, {} bytes of run",
        list.layout().stride_bytes(),
        list.bytes().len()
    );

    let started = Instant::now();
    let compact = facet_json::to_string(&value).expect("a packed run serializes");
    let compact_time = started.elapsed();
    println!(
        "to_string:        {} bytes, x{:.2} the run, in {compact_time:.2?}",
        compact.len(),
        compact.len() as f64 / list.bytes().len() as f64
    );

    let started = Instant::now();
    let pretty = facet_json::to_string_pretty(&value).expect("a packed run pretty prints");
    let pretty_time = started.elapsed();
    println!(
        "to_string_pretty: {} bytes, x{:.2} the run, in {pretty_time:.2?}",
        pretty.len(),
        pretty.len() as f64 / list.bytes().len() as f64
    );

    let started = Instant::now();
    match facet_json::from_str::<Value>(&compact) {
        Ok(read) if read == value => println!(
            "from_str:         {} bytes in {:.2?}, identical",
            compact.len(),
            started.elapsed()
        ),
        Ok(read) => {
            println!("from_str:         NOT identical: {read:?}");
            failures.push("a packed run does not round trip".to_owned());
        }
        Err(error) => {
            println!("from_str:         failed: {error}");
            failures.push(format!("a packed run does not read back: {error}"));
        }
    }

    let started = Instant::now();
    let expanded = value.expanded();
    let expanded_text = facet_json::to_string(&expanded).expect("the tree serializes");
    println!(
        "the same data as an expanded tree ({}): {} bytes in {:.2?}, x{:.2} the run",
        expanded.kind_name(),
        expanded_text.len(),
        started.elapsed(),
        expanded_text.len() as f64 / list.bytes().len() as f64
    );
    println!(
        "so a hundred thousand points is {:.1} MB of run and about {:.1} MB of json",
        (100_000.0 * 24.0) / (1024.0 * 1024.0),
        (100_000.0 * 24.0 * compact.len() as f64 / list.bytes().len() as f64) / (1024.0 * 1024.0)
    );
    println!();
    failures
}

/// The identifiers `register_standard_scene_schema` registers, constants and list together.
fn schema_identifiers() -> Vec<String> {
    println!("== the schema identifiers a scene client must use ==");
    println!("STANDARD_SCENE_SCHEMA = {:?}", scene::STANDARD_SCENE_SCHEMA);
    println!("MESH_SCHEMA           = {:?}", scene::standard::MESH_SCHEMA);
    println!(
        "IMAGE_SCHEMA          = {:?}",
        scene::standard::IMAGE_SCHEMA
    );
    println!("GRAPHICS_SCHEMA       = {:?}", graphics::GRAPHICS_SCHEMA);
    println!("INSTANCE_SCHEMA       = {:?}", graphics::INSTANCE_SCHEMA);

    let mut registry = Registry::new();
    register_standard_scene_schema(&mut registry);
    let listed: Vec<String> = registry
        .schema_identifiers()
        .into_iter()
        .map(SchemaIdentifier::into_string)
        .collect();
    println!("registry.schema_identifiers() after the call: {listed:?}");
    println!();

    let mut failures = Vec::new();
    let expected = [
        scene::STANDARD_SCENE_SCHEMA,
        scene::standard::MESH_SCHEMA,
        scene::standard::IMAGE_SCHEMA,
    ];
    for name in expected {
        if !listed.iter().any(|listed| listed == name) {
            failures.push(format!(
                "register_standard_scene_schema did not register {name:?}"
            ));
        }
    }
    failures
}

/// The envelopes a client posts, exercised against a device on a real socket.
async fn submission() -> Vec<String> {
    println!("== submitting a document ==");
    let mut failures = Vec::new();

    let graphics_identifier = DefinitionIdentifier::new("probe.graphics");
    let instance_identifier = DefinitionIdentifier::new("probe.instance");
    let mut device = Device::new(
        Box::new(ReferenceScene::new(
            graphics_identifier.clone(),
            instance_identifier.clone(),
        )),
        Box::new(NoopRenderer),
    );
    device.install_graphics_schemas();
    register_standard_scene_schema(device.registry_mut());
    // A schema that accepts anything, so the probe is about the envelope and not about the scene
    // language. The route validates against whatever schema the body names.
    device.registry_mut().put_schema(
        SchemaIdentifier::new("probe.any"),
        Schema::new(SchemaKind::Any),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback listener");
    let address = listener.local_addr().expect("a bound address");
    tokio::spawn(async move {
        let _ = axum::serve(listener, whiting_device::router(device)).await;
    });

    // The route's body, built by the route's own json writer.
    let document = Value::structure([
        ("name".to_owned(), Value::String("probe".to_owned())),
        ("points".to_owned(), Value::Packed(small_packed())),
        (
            "tags".to_owned(),
            Value::Dict(FieldMap::from([("added".to_owned(), Value::Int(1))])),
        ),
    ]);
    let body = facet_json::to_string_pretty(&PutDefinitionBody {
        schema: "probe.any".to_owned(),
        value: document.clone(),
    })
    .expect("the body serializes");

    println!("PUT /definition/probe.document");
    println!("Content-Type: application/json");
    println!("{body}");
    let response = request(address, "PUT", "/definition/probe.document", &body).await;
    let (head, echoed) = split_response(&response);
    println!("response head:\n{head}");
    println!("response body:\n{echoed}");
    if !head.starts_with("HTTP/1.1 200") {
        failures.push("a document its schema accepts was refused".to_owned());
    } else {
        match facet_json::from_str::<DefinitionReportBody>(echoed) {
            Ok(report) if report.value == document => {
                println!(
                    "the route stored and echoed the document back identical (schema {:?}, version {})",
                    report.schema, report.version
                );
            }
            Ok(report) => {
                println!("the route echoed a different document: {:?}", report.value);
                failures.push("the stored document is not the one submitted".to_owned());
            }
            Err(error) => {
                println!("the report does not read back: {error}");
                failures.push(format!("the definition report does not read back: {error}"));
            }
        }
    }

    // The same envelope against the schema a scene actually names, with a document that is not a
    // scene: a body that parses is answered with what is wrong in the document, which is what
    // tells a client the envelope was right and the document was not.
    let not_a_scene = facet_json::to_string_pretty(&PutDefinitionBody {
        schema: scene::STANDARD_SCENE_SCHEMA.to_owned(),
        value: Value::structure([("name".to_owned(), Value::String("probe".to_owned()))]),
    })
    .expect("the body serializes");
    println!();
    println!("PUT /definition/probe.scene");
    println!("{not_a_scene}");
    let response = request(address, "PUT", "/definition/probe.scene", &not_a_scene).await;
    let (head, response_body) = split_response(&response);
    println!("response head:\n{head}");
    println!("response body:\n{response_body}");

    failures.extend(snapshot_envelope(address).await);
    failures.extend(restore_route(address).await);
    println!();
    failures
}

/// The snapshot envelope, printed, and then a real one asked of the device.
async fn snapshot_envelope(address: SocketAddr) -> Vec<String> {
    println!();
    println!("== the snapshot envelope ==");
    let mut failures = Vec::new();

    // The envelope, printed from the type the restore route reads. The device writes a
    // snapshot of everything it holds, which is far too much to print, so this is one of each.
    let document = Snapshot {
        schemas: vec![NamedSchema {
            name: "probe.any".to_owned(),
            schema: Schema::new(SchemaKind::Any),
        }],
        definitions: vec![NamedDefinition {
            name: "probe.document".to_owned(),
            schema: "probe.any".to_owned(),
            version: 1,
            value: Value::structure([
                ("points".to_owned(), Value::Packed(small_packed())),
                ("count".to_owned(), Value::Int(2)),
            ]),
        }],
    };
    let text = facet_json::to_string_pretty(&document).expect("a snapshot serializes");
    println!("a Snapshot, pretty printed:\n{text}");
    match facet_json::from_str::<Snapshot>(&text) {
        Ok(read) if read == document => println!("a Snapshot reads back: identical"),
        Ok(read) => {
            println!("a Snapshot reads back different: {read:?}");
            failures.push("a Snapshot does not round trip".to_owned());
        }
        Err(error) => {
            println!("a Snapshot does not read back: {error}");
            failures.push(format!("a Snapshot does not read back: {error}"));
        }
    }

    // And the real thing, written by the device for the restore that follows.
    let request_body = snapshot_path_body();
    println!();
    println!("POST /snapshot with {request_body}");
    let response = request(address, "POST", "/snapshot", &request_body).await;
    let (head, body) = split_response(&response);
    println!("response head:\n{head}");
    println!("response body:\n{body}");
    if !head.starts_with("HTTP/1.1 200") {
        failures.push("the device refused to write a snapshot".to_owned());
        return failures;
    }

    let text = std::fs::read_to_string(snapshot_path()).expect("the device wrote the snapshot");
    println!(
        "the device wrote {} bytes of json, nested {} levels deep",
        text.len(),
        max_nesting(&text)
    );

    let stack = snapshot_stack_bytes();
    println!("reading it back on a thread with a {stack} byte stack");
    let parsed = std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || facet_json::from_str::<Snapshot>(&text))
        .expect("the reading thread starts")
        .join()
        .expect("the reading thread does not panic");
    match parsed {
        Ok(snapshot) => {
            let schemas: Vec<&str> = snapshot.schemas.iter().map(|s| s.name.as_str()).collect();
            let definitions: Vec<&str> = snapshot
                .definitions
                .iter()
                .map(|d| d.name.as_str())
                .collect();
            println!(
                "it holds {} schemas {schemas:?}, {} definitions {definitions:?}, and it parses",
                snapshot.schemas.len(),
                snapshot.definitions.len()
            );
        }
        Err(error) => {
            println!("the file the device wrote does not read back: {error}");
            failures.push(format!(
                "the snapshot the device wrote does not read back: {error}"
            ));
        }
    }
    failures
}

/// The stack the device's own snapshot is read back on.
///
/// The default is large because the reader needs more than a main thread has; see the module
/// documentation. `FACET_PROBE_STACK` is how the size the reader actually needs is measured.
fn snapshot_stack_bytes() -> usize {
    std::env::var("FACET_PROBE_STACK")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(64 * 1024 * 1024)
}

/// Returns how deeply the brackets of `text` nest, which is the depth a reader has to descend.
fn max_nesting(text: &str) -> usize {
    let mut depth = 0usize;
    let mut deepest = 0usize;
    for character in text.chars() {
        match character {
            '{' | '[' => {
                depth += 1;
                deepest = deepest.max(depth);
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}

/// Whether a `Snapshot` file is a viable way in, and how its envelope differs.
async fn restore_route(address: SocketAddr) -> Vec<String> {
    println!();
    println!("== the restore route, exercised ==");
    let mut failures = Vec::new();

    let request_body = snapshot_path_body();
    println!("POST /restore with {request_body}");
    let response = request(address, "POST", "/restore", &request_body).await;
    let (head, body) = split_response(&response);
    println!("response head:\n{head}");
    println!("response body:\n{body}");
    if !head.starts_with("HTTP/1.1 200") {
        failures.push("the device refused to restore the snapshot it wrote".to_owned());
    }

    let response = request(address, "GET", "/definitions", "").await;
    let (_, body) = split_response(&response);
    println!("GET /definitions after the restore:\n{body}");
    let response = request(address, "GET", "/schemas", "").await;
    let (_, body) = split_response(&response);
    println!("GET /schemas after the restore:\n{body}");

    std::fs::remove_file(snapshot_path()).expect("the probe removes the file it asked for");
    failures
}

/// The body of a snapshot or restore request, which names a file rather than carrying one.
fn snapshot_path_body() -> String {
    facet_json::to_string(&SnapshotRequestBody {
        path: snapshot_path().to_string_lossy().into_owned(),
    })
    .expect("the request serializes")
}

/// Where the probe asks the device to write a snapshot, inside the workspace target directory.
fn snapshot_path() -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target");
    std::fs::create_dir_all(&directory).expect("the target directory is writable");
    directory.join("facet-value-probe-snapshot.json")
}

/// Sends one request over a fresh connection and returns the whole response as text.
///
/// The client is blocking, on the blocking pool, because this workspace enables tokio without
/// its `io-util` feature and a probe should not widen a dependency to make its own point. The
/// request asks the server to close and stops at the length the response declares, with a read
/// timeout as the backstop, so a response that never ends is reported as the text that arrived
/// rather than waited on.
async fn request(address: SocketAddr, method: &str, path: &str, body: &str) -> String {
    let method = method.to_owned();
    let path = path.to_owned();
    let body = body.to_owned();
    tokio::task::spawn_blocking(move || send(address, &method, &path, &body))
        .await
        .expect("the client task does not panic")
}

/// One blocking request and response over `std::net`.
fn send(address: SocketAddr, method: &str, path: &str, body: &str) -> String {
    use std::io::{Read, Write};

    let mut stream = std::net::TcpStream::connect(address).expect("the probe server is listening");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout can be set");
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .expect("a write timeout can be set");

    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .expect("the request head is written");
    stream
        .write_all(body.as_bytes())
        .expect("the request body is written");
    stream.flush().expect("the request is flushed");

    let mut response = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                response.extend_from_slice(&chunk[..read]);
                if response_is_whole(&response) {
                    break;
                }
            }
            // A timeout or a reset ends the response as told: what arrived is what is reported.
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&response).into_owned()
}

/// Returns whether `response` holds a whole http response, headers and declared body.
fn response_is_whole(response: &[u8]) -> bool {
    let Some(head_end) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&response[..head_end]).to_lowercase();
    match head
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .and_then(|value| value.trim().parse::<usize>().ok())
    {
        Some(length) => response.len() >= head_end + 4 + length,
        None => false,
    }
}

/// Splits a response into its head and its body.
fn split_response(response: &str) -> (&str, &str) {
    match response.find("\r\n\r\n") {
        Some(at) => (&response[..at], &response[at + 4..]),
        None => (response, ""),
    }
}
