//! Reports what a scene document costs in each of its two forms.
//!
//! ```text
//! cargo run -p whiting-gallery --example document_binary_probe -- <scene-name> [directory]
//! ```
//!
//! # What it reports
//!
//! 1. how long the scene takes to build, and what the document holds
//! 2. the json form and the binary form: how many bytes, how long to write, how long to
//!    read back, and how long to decode
//! 3. whether the two forms hold the same document, compared by the bytes of the binary
//!    form rather than by a walk of the two trees
//! 4. what the size of the document is made of: the packed runs against everything else
//!
//! # Why the two forms are compared by bytes
//!
//! **The binary form is the yardstick, because it writes everything there is to write.** A
//! float is written as its bits, so two documents that hold the same numbers are the same
//! bytes and two that hold numbers that print alike are not. Comparing the two forms by
//! their binary encodings therefore says exactly what the question is: whether the json
//! form and the binary form hold the same document.
//!
//! # Why an example and not a test
//!
//! The numbers this reports are the record of the two forms, and they are read rather than
//! gated. The round trip is printed rather than asserted, so a reader sees what happened
//! instead of a build that says nothing.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use definition::{Value, binary};
use whiting_gallery::Gallery;

/// The stack the json document is read on.
///
/// # Invariants
///
/// **It is the reader's size, and the reader's size is measured rather than guessed.** The
/// json reader descends as it parses, and the renderer reads its documents on a thread of
/// this size; see `crates/device-rasterize/examples/render.rs`.
const READ_STACK_BYTES: usize = 64 * 1024 * 1024;

fn main() {
    let name = std::env::args().nth(1).unwrap_or_else(|| "hall".to_owned());
    let directory = PathBuf::from(std::env::args().nth(2).unwrap_or_else(|| "out".to_owned()));
    std::fs::create_dir_all(&directory).expect("the output directory is writable");

    println!("== the scene `{name}` ==");
    let started = Instant::now();
    let scene = Gallery::build(&name);
    let built = started.elapsed();
    let composition = Composition::of(&scene);
    println!("built in {built:.2?}");
    println!(
        "the document holds {} values, of which {} are in packed runs of {} bytes",
        composition.values, composition.runs, composition.run_bytes
    );
    println!(
        "and {} of them are byte sequences of {} bytes",
        composition.sequences, composition.sequence_bytes
    );
    println!(
        "the longest list holds {} elements, and every list holds {} together",
        composition.longest_list, composition.elements
    );

    let json = measure(&scene, &directory.join(format!("{name}.json")), |value| {
        facet_json::to_string(value)
            .expect("the scene document encodes")
            .into_bytes()
    });
    let binary = measure(
        &scene,
        &directory.join(format!("{name}.whiting")),
        binary::encode,
    );

    println!();
    println!("== the two forms ==");
    println!(
        "json:   {:>12} bytes, encoded in {:>9.2?}, written in {:>9.2?}, read in {:>9.2?}, parsed in {:>9.2?}",
        json.bytes.len(),
        json.encoded,
        json.written,
        json.read,
        json.parsed
    );
    println!(
        "binary: {:>12} bytes, encoded in {:>9.2?}, written in {:>9.2?}, read in {:>9.2?}, decoded in {:>9.2?}",
        binary.bytes.len(),
        binary.encoded,
        binary.written,
        binary.read,
        binary.parsed
    );
    println!(
        "the binary form is {:.1}% of the json form",
        100.0 * binary.bytes.len() as f64 / json.bytes.len() as f64
    );

    println!();
    println!("== do the two forms hold the same document ==");
    let json_again = binary::encode(&json.value);
    let binary_again = binary::encode(&binary.value);
    if json_again == binary_again {
        println!("the json document and the binary document are the same document");
    } else {
        println!("the json document and the binary document DIFFER");
        println!(
            "  the json form holds {} bytes when written as binary, and the binary form holds {}",
            json_again.len(),
            binary_again.len()
        );
    }
    if binary_again == binary.bytes {
        println!("the binary document reads back and writes the same bytes");
    } else {
        println!("the binary document does NOT read back to the same bytes");
    }
    if json_again == binary.bytes {
        println!("the json document reads back to the same document as the binary one");
    } else {
        println!("the json document does NOT read back to the same document");
    }

    println!();
    println!("== what the size is made of ==");
    let runs = composition.run_bytes;
    let sequences = composition.sequence_bytes;
    println!(
        "the binary form: {} bytes of run, {} bytes of byte sequence, {} bytes of everything else",
        runs,
        sequences,
        binary.bytes.len().saturating_sub(runs + sequences)
    );
    println!(
        "the json form:   {} bytes of base64 run, {} bytes of base64 byte sequence, {} bytes of \
         everything else",
        base64_len(runs),
        base64_len(sequences),
        json.bytes
            .len()
            .saturating_sub(base64_len(runs) + base64_len(sequences))
    );
    println!(
        "so the geometry is {:.1}% of the binary form and {:.1}% of the json form",
        100.0 * runs as f64 / binary.bytes.len() as f64,
        100.0 * base64_len(runs) as f64 / json.bytes.len() as f64
    );
}

/// What one form of the document cost.
struct Measured {
    /// The bytes of the document.
    bytes: Vec<u8>,
    /// The value the document holds once it has been read back.
    value: Value,
    /// How long the encoding took.
    encoded: Duration,
    /// How long the writing took.
    written: Duration,
    /// How long the reading took.
    read: Duration,
    /// How long the decoding took.
    parsed: Duration,
}

/// Encodes a document, writes it, reads it back and decodes it, timing each step.
fn measure(
    scene: &Value,
    path: &std::path::Path,
    encode: impl FnOnce(&Value) -> Vec<u8> + Send,
) -> Measured {
    let started = Instant::now();
    let bytes = on_a_large_stack(|| encode(scene));
    let encoded = started.elapsed();

    let started = Instant::now();
    std::fs::write(path, &bytes).expect("the document is written");
    let written = started.elapsed();

    let started = Instant::now();
    let raw = std::fs::read(path).expect("the document is read");
    let read = started.elapsed();

    let started = Instant::now();
    let value = read_value(raw);
    let parsed = started.elapsed();

    Measured {
        bytes,
        value,
        encoded,
        written,
        read,
        parsed,
    }
}

/// Runs a walk of the document on a thread with room to walk it.
///
/// # Invariants
///
/// **It is the reader's size, and the reader's size is measured rather than guessed.** The
/// json reader descends as it parses, and the renderer reads its documents on a thread of
/// this size; see `crates/device-rasterize/examples/render.rs`. The writer descends the same
/// tree, so it is given the same room.
fn on_a_large_stack<T: Send>(walk: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(READ_STACK_BYTES)
            .spawn_scoped(scope, walk)
            .expect("the thread starts")
            .join()
            .expect("the thread does not panic")
    })
}

/// Reads a document back, on a thread with room to walk it.
///
/// # Invariants
///
/// **The read happens on a thread of its own, because the read is what needs the room.**
/// The bytes are moved into that thread rather than copied, so the document is held once.
fn read_value(bytes: Vec<u8>) -> Value {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(READ_STACK_BYTES)
            .spawn_scoped(scope, move || {
                if binary::has_magic(&bytes) {
                    binary::decode(&bytes).expect("the document reads back")
                } else {
                    let text = String::from_utf8(bytes).expect("the document is utf-8");
                    facet_json::from_str::<Value>(&text).expect("the document reads back")
                }
            })
            .expect("the reading thread starts")
            .join()
            .expect("the reading thread does not panic")
    })
}

/// What a document is made of.
struct Composition {
    /// How many values the tree holds.
    values: usize,
    /// How many packed runs it holds.
    runs: usize,
    /// How many bytes those runs hold.
    run_bytes: usize,
    /// How many byte sequences it holds.
    sequences: usize,
    /// How many bytes those sequences hold.
    sequence_bytes: usize,
    /// How many elements the longest list holds.
    longest_list: usize,
    /// How many elements every list holds together.
    elements: usize,
}

impl Composition {
    /// Walks a document and says what it is made of.
    fn of(value: &Value) -> Self {
        let mut composition = Self {
            values: 0,
            runs: 0,
            run_bytes: 0,
            sequences: 0,
            sequence_bytes: 0,
            longest_list: 0,
            elements: 0,
        };
        composition.walk(value);
        composition
    }

    /// Adds one value and everything under it.
    fn walk(&mut self, value: &Value) {
        self.values += 1;
        match value {
            Value::Bytes(bytes) => {
                self.sequences += 1;
                self.sequence_bytes += bytes.len();
            }
            Value::Packed(list) => {
                self.runs += 1;
                self.run_bytes += list.bytes().len();
            }
            Value::Struct(fields) | Value::Dict(fields) => {
                for value in fields.values() {
                    self.walk(value);
                }
            }
            Value::Enum { fields, .. } => {
                for value in fields.values() {
                    self.walk(value);
                }
            }
            Value::List(values) => {
                self.longest_list = self.longest_list.max(values.len());
                self.elements += values.len();
                for value in values {
                    self.walk(value);
                }
            }
            _ => {}
        }
    }
}

/// Returns the length of the base64 text that carries `bytes`.
fn base64_len(bytes: usize) -> usize {
    bytes.div_ceil(3) * 4
}
