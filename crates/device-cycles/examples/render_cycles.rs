//! Renders a gallery scene with Cycles, which is what the frame that ships is made of.
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
//! render_cycles <scene> <out.png> [samples] [device]
//! ```
//!
//! `<scene>` is a gallery name. With no samples it takes a hundred on the cpu.

use std::time::Instant;

use gallery::Gallery;
use whiting_device_cycles::{CyclesOptions, render_scene};

fn main() {
    if let Err(failure) = run() {
        eprintln!("error: {failure}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    let Some(name) = arguments.next() else {
        return Err(usage());
    };
    let Some(image) = arguments.next() else {
        return Err(usage());
    };
    let samples = arguments
        .next()
        .map(|value| value.parse::<u32>())
        .transpose()
        .map_err(|_| "the sample count is not a number".to_owned())?
        .unwrap_or(100);
    let device = arguments.next().unwrap_or_else(|| "CPU".to_owned());

    let scene = Gallery::build(&name);
    let image = std::path::PathBuf::from(image);

    // `for_paths` derives the document's name from the image's, so it is handed the image and not a
    // path it has already renamed — passing an already-suffixed path here appends the suffix twice.
    let mut options = CyclesOptions::for_paths(&image, &image);
    // The gallery's point schemas carry fields Cycles' loader has no socket for. Degrade drops
    // them and says so, which is what makes a path-traced gallery render possible at all;
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

    println!(
        "{name}: {samples} samples on {device}, {} s in the process, {} s in total, {} dropped",
        rendered.seconds,
        total,
        rendered.report.len(),
    );
    Ok(())
}

fn usage() -> String {
    "usage: render_cycles <scene> <out.png> [samples] [device]".to_owned()
}
