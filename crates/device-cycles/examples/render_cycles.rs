//! Renders a scene document with Cycles, which is what the frame that ships is made of.
//!
//! # Why this exists separately from the rasterising harness
//!
//! `whiting-device-rasterize`'s `render` harness drives a gpu through compiled instructions. This
//! one hands the scene itself to a path tracer in a subprocess. They answer in the same frame
//! type, so the two can be compared pixel for pixel, and they are separate programs because they
//! answer different questions: one asks what the pipeline draws, the other asks what the scene
//! looks like when nothing is approximated.
//!
//! # Usage
//!
//! ```text
//! render_cycles --document <path> <out.png> [samples] [device]
//! ```
//!
//! The scene is a document, and the document is written by the gallery's producer: `cargo run -p
//! whiting-gallery --example scene_document -- <name> <path>`. With no samples it takes a hundred
//! on the cpu.

use std::path::PathBuf;
use std::time::Instant;

use definition::Value;
use whiting_device_cycles::{CyclesOptions, render_scene};

/// The stack a scene document is read on.
///
/// # Invariants
///
/// **It is the reader's size, and the reader's size is measured rather than guessed.**
/// `facet_json::from_str` descends as it parses, and a document of a few hundred megabytes needs
/// more than the megabyte a windows main thread is given — `crates/device/examples/facet_value_probe.rs`
/// measures that and says so.
const DOCUMENT_STACK_BYTES: usize = 64 * 1024 * 1024;

fn main() {
    if let Err(failure) = run() {
        eprintln!("error: {failure}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut document: Option<PathBuf> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--document" => {
                index += 1;
                let asked = arguments
                    .get(index)
                    .ok_or_else(|| "--document needs the path of a scene document".to_owned())?;
                document = Some(PathBuf::from(asked));
            }
            other => rest.push(other.to_owned()),
        }
        index += 1;
    }
    let Some(document) = document else {
        return Err(usage());
    };
    let Some(image) = rest.first() else {
        return Err(usage());
    };
    let samples = rest
        .get(1)
        .map(|value| value.parse::<u32>())
        .transpose()
        .map_err(|_| "the sample count is not a number".to_owned())?
        .unwrap_or(100);
    let device = rest.get(2).cloned().unwrap_or_else(|| "CPU".to_owned());

    let scene = read_document(&document)?;
    let image = PathBuf::from(image);

    // `for_paths` derives the document's name from the image's, so it is handed the image and not a
    // path it has already renamed — passing an already-suffixed path here appends the suffix twice.
    let mut options = CyclesOptions::for_paths(&image, &image);
    // The document's point schemas carry fields Cycles' loader has no socket for. Degrade drops
    // them and says so, which is what makes a path-traced render possible at all;
    // Reject would refuse every scene in it.
    options.lossy = scene::cycles_export::LossyMode::Degrade;
    options.samples = samples;
    options.device = device.clone();

    let started = Instant::now();
    let rendered = render_scene(&scene, &options).map_err(|error| error.to_string())?;
    let total = started.elapsed().as_secs_f64();

    if !rendered.report.is_empty() {
        // On the standard error rather than beside the timing line: the render succeeded and the
        // image is on disk, and a note on the standard output is too easy to read past.
        eprintln!(
            "warning: the scene could not be carried in full: {}",
            rendered.report
        );
    }

    // Read back out of the written file rather than off the request, so the number on this line is
    // the size of the png on disk and not the size something intended. A scene's frame is the only
    // source of that size — `Film` has no resolution socket — so a caller comparing a path-traced
    // frame with a rasterised one needs it stated rather than assumed.
    let written = rendered
        .frame
        .images
        .first()
        .map(|image| format!("{}x{}", image.width, image.height))
        .unwrap_or_else(|| "no image".to_owned());

    println!(
        "{}: {samples} samples on {device}, {} written, {} s in the process, {} s in total, \
         {} dropped",
        document.display(),
        written,
        rendered.seconds,
        total,
        rendered.report.len(),
    );
    Ok(())
}

/// Reads a scene document, on a thread with a larger stack than a windows main thread has.
fn read_document(path: &PathBuf) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("reading {} failed: {error}", path.display()))?;
    std::thread::Builder::new()
        .stack_size(DOCUMENT_STACK_BYTES)
        .spawn(move || facet_json::from_str::<Value>(&text))
        .map_err(|error| format!("the reading thread does not start: {error}"))?
        .join()
        .map_err(|_| "the reading thread panicked".to_owned())?
        .map_err(|error| format!("the document does not read as a scene value: {error}"))
}

fn usage() -> String {
    "usage: render_cycles --document <path> <out.png> [samples] [device]".to_owned()
}
