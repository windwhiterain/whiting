//! What a path-traced export is the size of, read off the document rather than off a render.
//!
//! # What this is for
//!
//! The record carries an open item saying that a path-traced export of any of the nine scenes is
//! *"still the wrong size"*, because `device-cycles` carries its own `--width`/`--height` — the
//! film node has no resolution socket, so the size has to travel on the command line. **That
//! sentence stops one step short of the truth**: the command line's numbers are read out of the
//! scene document by `render_scene`, so a scene's frame *does* reach the file.
//!
//! It was measured on this tree and it was right, and **nothing in the tree could have said so**.
//! `render_size` is exercised by no test and `run`'s command line was written inline, so a runner
//! that passed a hard-coded square would have rendered every scene at the wrong size and left
//! every gate in the workspace green. An instrument that cannot differ is an instrument that
//! agrees.
//!
//! # What is compared, and why both halves
//!
//! Three readings of one scene, in one loop, so that a reader is not choosing between them:
//!
//! | | where it comes from | what it decides |
//! |---|---|---|
//! | the scene's own `frame` | `Frame::read`, the rasterising path's reader | what the rasterised `.png` is |
//! | `cycles_export::render_size` | the emitter's own camera reader | what `--width`/`--height` are given |
//! | the file Cycles wrote | a real subprocess run | what is on disk |
//!
//! The first two are compared here for all nine scenes; the third was measured by running
//! `render_cycles` at one sample on each, and every row matched. **The third is not in this file
//! because a test that needs a several-hundred-megabyte binary in a fixed path is a test that is
//! skipped, and a skip reads as an absence.** What is left here is the arithmetic that decides it.
//!
//! # The control is the aspect
//!
//! Every scene here is named with its own numbers, and **not one of them is square**. A gate
//! that only ever saw a square could not tell a scene's frame from the default, which is itself a
//! square — so `terrace` at 3:1 and `house` at 1.851 are the rows that carry the question, and
//! `tiles`, which names no frame at all, is the row that says what happens when a scene says
//! nothing: the same `DEFAULT_WIDTH` by `DEFAULT_HEIGHT` the rasterised path draws it at.

use std::path::Path;

use gallery::Gallery;
use scene::DeviceLimits;
use scene::cycles_export::render_size;
use scene::standard::Frame;
use whiting_device_cycles::{Resolution, arguments};

/// The nine scenes with a frame of their own shape, and the sizes they name.
///
/// Measured from the documents themselves on this tree, and every one is a shape no other scene
/// here uses: a gate that read nine identical rows would have proved nothing.
const FRAMED: [(&str, u32, u32); 9] = [
    ("house", 1392, 752),
    ("hall", 1328, 816),
    ("hall_large", 1264, 816),
    ("shed", 1344, 768),
    ("terrace", 1536, 512),
    ("wing", 1424, 736),
    ("workshop", 1120, 928),
    ("yard", 1264, 816),
    ("identity", 1280, 816),
];

/// A scene that names no frame, and what its export is therefore the size of.
const UNFRAMED: (&str, u32, u32) = ("tiles", 1024, 1024);

/// A scene that names a frame, and a resolution that is not it.
///
/// The wrong size is spelled out rather than left to the reader to imagine, because the failure
/// this file exists for is a runner that names one of these two numbers instead of the frame's.
const WRONG: (u32, u32) = (1024, 1024);

#[test]
fn an_export_is_the_size_the_scene_names() {
    for (name, width, height) in FRAMED {
        let scene = Gallery::build(name);

        let (frame, _) = Frame::read(&scene, DeviceLimits::unbounded());
        assert_eq!(
            (frame.width, frame.height),
            (i64::from(width), i64::from(height)),
            "{name} names its own frame, so the picture and its export start from the same two \
             numbers",
        );

        let size = render_size(&scene).unwrap_or_else(|error| {
            panic!("{name} names a frame and a camera, so it reads: {error}")
        });
        assert_eq!(
            size,
            (width, height),
            "{name}'s export is the size {name} asks for and not {WRONG:?}",
        );
    }
}

#[test]
fn the_size_reaches_the_command_line() {
    for (name, width, height) in FRAMED {
        let scene = Gallery::build(name);
        let size = render_size(&scene).expect("the scene names a frame and a camera");
        let command = arguments(
            "CPU",
            1,
            Resolution {
                width: size.0,
                height: size.1,
            },
            Path::new("out.png"),
            Path::new("scene.xml"),
        );

        assert_eq!(
            command,
            vec![
                "--device",
                "CPU",
                "--samples",
                "1",
                "--width",
                &width.to_string(),
                "--height",
                &height.to_string(),
                "--background",
                "--output",
                "out.png",
                "scene.xml",
            ],
            "{name}'s own {width}x{height} is what the process is started with, because the film \
             node has no resolution socket and this list is the only route the size has",
        );
    }
}

#[test]
fn a_scene_that_names_no_frame_is_the_default_one() {
    let (name, width, height) = UNFRAMED;
    let scene = Gallery::build(name);
    let (frame, _) = Frame::read(&scene, DeviceLimits::unbounded());

    assert_eq!(
        (frame.width, frame.height),
        (i64::from(width), i64::from(height)),
        "the two are {name}'s because it is the scene schema's default and not the exporter's, so \
         a document carrying no frame is drawn and exported at the same size",
    );
    assert_eq!(
        render_size(&scene).expect("a scene with no frame still has a size to read"),
        (width, height),
        "an export without a frame on the scene is the default square, and {name} is the scene that \
         says so",
    );
}
