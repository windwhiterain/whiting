//! Writes a named gallery scene out as the scene document a renderer reads.
//!
//! ```text
//! cargo run -p whiting-gallery --example scene_document -- <scene-name> <path.json> [--census] [--bounds]
//! ```
//!
//! # What this is for
//!
//! **The gallery is a producer of scene documents, and this is the producer.** A renderer draws
//! a document and knows no scene by name; the gallery builds the documents and knows every
//! scene by name. The two meet at a file: this writes one, and the renderer reads it.
//!
//! Nothing here is a second way to build a scene. [`Gallery::build`] is the one builder, and
//! what this adds is the writing of what it built and the reporting of what the document
//! holds.
//!
//! # The document
//!
//! The file is the `Value` of [`Gallery::build`] and nothing else — no envelope, no name, no
//! version. The envelope the device's rpc uses is its own, and a file that carried a second
//! spelling of it would be a document two readers had to disagree about. A caller that wants
//! the rpc's envelope writes it around this value.
//!
//! # The census and the bounds
//!
//! `--census` and `--bounds` print what the gallery's own reports print, taken over the
//! document that was just written rather than over a second build of the same scene: the two
//! are the same document, and building a scene twice is the expensive half of the run done
//! twice.
//!
//! # The stack
//!
//! The document is written on a thread with a larger stack than a windows main thread has.
//! `facet_json` descends as it walks the tree, and the reader of the same document needs more
//! than the megabyte a main thread is given — `crates/device/examples/facet_value_probe.rs`
//! measures that and says so. The writer walks the same tree, so it is given the same room
//! rather than a guess about how deep this particular scene is.

use std::path::PathBuf;

use definition::Value;
use whiting_gallery::Gallery;

/// The stack the document is written on.
///
/// # Invariants
///
/// **It is the reader's size, because it is the same tree.** The reader of a scene document
/// needs more stack than a windows main thread has, and the writer descends the same
/// structure, so a writer on the main thread would be a writer that works for every scene but
/// the deepest one.
const WRITE_STACK_BYTES: usize = 64 * 1024 * 1024;

fn main() {
    if let Err(failure) = run() {
        eprintln!("error: {failure}");
        std::process::exit(1);
    }
}

/// Writes the scene named on the command line into the file named after it.
fn run() -> Result<(), String> {
    let request = parse_arguments()?;
    let scene = Gallery::build(&request.name);
    let text = write_on_a_large_stack(&scene)?;

    std::fs::write(&request.path, &text)
        .map_err(|error| format!("writing {} failed: {error}", request.path.display()))?;
    println!(
        "wrote {} ({} bytes, the scene document of `{}`)",
        request.path.display(),
        text.len(),
        request.name
    );

    if request.census {
        print!("{}", Gallery::census_of(&scene, &request.name));
    }
    if request.bounds {
        print!("{}", Gallery::bounds_of(&scene));
    }
    Ok(())
}

/// Encodes the scene as the json `facet_json` writes, on a thread with room to walk it.
fn write_on_a_large_stack(scene: &Value) -> Result<String, String> {
    let owned = scene.clone();
    std::thread::Builder::new()
        .stack_size(WRITE_STACK_BYTES)
        .spawn(move || facet_json::to_string(&owned))
        .map_err(|error| format!("the writing thread does not start: {error}"))?
        .join()
        .map_err(|_| "the writing thread panicked".to_owned())?
        .map_err(|error| format!("the scene document does not encode: {error}"))
}

/// What a producer was asked for.
struct Request {
    /// The name of the scene to write.
    name: String,
    /// Where the document is written.
    path: PathBuf,
    /// Whether to report what the document holds.
    census: bool,
    /// Whether to report the box the document's placements occupy.
    bounds: bool,
}

/// Reads the command line into what to write and where to put it.
fn parse_arguments() -> Result<Request, String> {
    let mut arguments = std::env::args().skip(1);
    let name = arguments.next().ok_or_else(usage)?;
    let path = arguments.next().ok_or_else(usage)?;
    let mut census = false;
    let mut bounds = false;
    for argument in arguments {
        match argument.as_str() {
            "--census" => census = true,
            "--bounds" => bounds = true,
            other => return Err(format!("`{other}` is not an option: {}", usage())),
        }
    }
    Ok(Request {
        name,
        path: PathBuf::from(path),
        census,
        bounds,
    })
}

/// The one sentence a wrong command line is answered with.
fn usage() -> String {
    "usage: scene_document <scene-name> <path.json> [--census] [--bounds]".to_owned()
}
