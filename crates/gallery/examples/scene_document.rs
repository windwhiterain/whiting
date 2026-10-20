//! Writes a named gallery scene out as the scene document a renderer reads.
//!
//! ```text
//! cargo run -p whiting-gallery --example scene_document -- <scene-name> <path> [--census] [--bounds]
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
//! # The two forms
//!
//! **The extension of the path asks for the form.** A path ending in `.whiting` is written as
//! the binary form of [`definition::binary`], and every other path is written as the json that
//! `facet_json` writes, which is what this example has always written. The choice is the
//! caller's, and the extension is how the caller makes it.
//!
//! **The reader does not go by the extension.** A renderer chooses its reader by the magic at
//! the head of the file, so a document that was renamed is still read as what it is, and a file
//! that was truncated is refused by name rather than read as something else. See
//! `crates/device-rasterize/examples/render.rs`.
//!
//! # The document
//!
//! The file holds the `Value` of [`Gallery::build`] and nothing else — no envelope, no name,
//! no version of the scene. The envelope the device's rpc uses is its own, and a file that
//! carried a second spelling of it would be a document two readers had to disagree about. The
//! binary form carries a magic and the version of the *format*, which is the head of every
//! file of that format rather than an envelope around the document.
//!
//! # The census and the bounds
//!
//! `--census` and `--bounds` print what the gallery's own reports print, taken over the
//! document that was just written rather than over a second build of the same scene: the two
//! are the same document, and building a scene twice is the expensive half of the run done
//! twice. They are printed from the document itself, so they say the same thing whichever
//! form the document was written in.
//!
//! # The stack
//!
//! The document is written on a thread with a larger stack than a windows main thread has.
//! `facet_json` descends as it walks the tree, and the reader of the same document needs more
//! than the megabyte a main thread is given — `crates/device/examples/facet_value_probe.rs`
//! measures that and says so. The writer walks the same tree, so it is given the same room
//! rather than a guess about how deep this particular scene is.

use std::path::{Path, PathBuf};

use definition::{Value, binary};
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

/// The extension of a path that asks for the binary form.
///
/// # Invariants
///
/// **It is the caller's choice and nothing else.** The reader of a document chooses by the
/// magic at the head of the file, so this decides only what is written.
const BINARY_EXTENSION: &str = "whiting";

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
    let document = write_on_a_large_stack(&scene, wants_binary(&request.path))?;

    std::fs::write(&request.path, document.bytes())
        .map_err(|error| format!("writing {} failed: {error}", request.path.display()))?;
    println!(
        "wrote {} ({} bytes, the {} scene document of `{}`)",
        request.path.display(),
        document.bytes().len(),
        document.form(),
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

/// The document as it is written, in the form the path asked for.
enum Written {
    /// The json `facet_json` writes.
    Json(String),
    /// The bytes of the binary form.
    Binary(Vec<u8>),
}

impl Written {
    /// Returns the bytes of the document, whichever form it is in.
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Json(text) => text.as_bytes(),
            Self::Binary(bytes) => bytes,
        }
    }

    /// Returns the name of the form, for the line that says what was written.
    fn form(&self) -> &'static str {
        match self {
            Self::Json(_) => "json",
            Self::Binary(_) => "binary",
        }
    }
}

/// Returns whether the path asks for the binary form.
fn wants_binary(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(BINARY_EXTENSION))
}

/// Encodes the scene as the document the path asked for, on a thread with room to walk it.
fn write_on_a_large_stack(scene: &Value, binary_form: bool) -> Result<Written, String> {
    let owned = scene.clone();
    std::thread::Builder::new()
        .stack_size(WRITE_STACK_BYTES)
        .spawn(move || {
            if binary_form {
                Ok(Written::Binary(binary::encode(&owned)))
            } else {
                facet_json::to_string(&owned)
                    .map(Written::Json)
                    .map_err(|error| format!("the scene document does not encode: {error}"))
            }
        })
        .map_err(|error| format!("the writing thread does not start: {error}"))?
        .join()
        .map_err(|_| "the writing thread panicked".to_owned())?
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
    "usage: scene_document <scene-name> <path> [--census] [--bounds]  (a path ending in \
     `.whiting` is written as the binary form)"
        .to_owned()
}
