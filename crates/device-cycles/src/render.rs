//! Rendering one scene document with Cycles, and reading the result back.
//!
//! # Why the frame is decoded here
//!
//! Cycles writes a png file and this crate reads it, so that a caller can hold a Cycles frame and
//! a rasterised frame and compare them. The rasterised path returns texels because a gpu read-back
//! has texels; a path tracer's only output is a file, so decoding is what makes the two answers the
//! same type rather than two things a caller has to reconcile.
//!
//! The two are not the same pixels and this does not pretend otherwise. Cycles renders scene-linear
//! and writes them through whatever colour management this build has, which with no OCIO
//! configuration configured is none. See [`CyclesOptions::exposure`].

use std::path::{Path, PathBuf};

use definition::Value;
use device::render::{RenderedFrame, RenderedImage};
use graphics::PRESENT_OUTPUT;
use scene::cycles_export::{EmitOptions, LossyMode, LossyReport, emit_to_file, render_size};

use crate::runner::{CyclesError, Resolution, locate_binary, run};

/// The name every frame from this backend reports.
pub const NAME: &str = "cycles";

/// Everything needed to render one scene with Cycles.
#[derive(Clone, Debug)]
pub struct CyclesOptions {
    /// Where the binary is. `None` searches the environment and the usual places.
    pub binary: Option<PathBuf>,
    /// Which device to ask for: `CPU`, `CUDA`, `OPTIX`, `HIP`, `ONEAPI`, or `NONE`.
    ///
    /// This is passed straight through, so a name this build does not know becomes a Cycles
    /// error rather than a silent fallback — which is the right way round, since a render that
    /// quietly ran somewhere other than where it was asked to is worse than one that failed.
    pub device: String,
    /// How many samples to take.
    pub samples: u32,
    /// The film's exposure, as a linear multiplier.
    pub exposure: f32,
    /// What to do about scene data the format cannot carry.
    pub lossy: LossyMode,
    /// Where the intermediate document is written.
    ///
    /// Kept, rather than deleted, because it is the only artefact that explains what the renderer
    /// was actually asked to draw, and a surprising picture is usually a surprising document.
    pub scene_path: PathBuf,
    /// Where the image is written.
    pub image_path: PathBuf,
}

impl CyclesOptions {
    /// Options that render `image` from `scene`, on the cpu, at a hundred samples.
    ///
    /// # Why the device defaults to the cpu
    ///
    /// It is the only one of them measured on this build. The cuda path has never been rendered,
    /// because the one machine with a gpu has it through a driver this was not built against. A
    /// default that has not been run is a default nobody has looked at.
    pub fn for_paths(scene: impl AsRef<Path>, image: impl AsRef<Path>) -> Self {
        let scene = scene.as_ref().to_path_buf();
        Self {
            binary: None,
            device: "CPU".to_owned(),
            samples: 100,
            exposure: 1.0,
            lossy: LossyMode::default(),
            image_path: image.as_ref().to_path_buf(),
            scene_path: scene.with_extension("cycles.xml"),
        }
    }
}

/// What one render produced.
#[derive(Clone, Debug)]
pub struct CyclesFrame {
    /// The frame, in the same shape the rasterising path answers in.
    pub frame: RenderedFrame,
    /// The scene document that was rendered, and what had to be left out of it.
    pub report: LossyReport,
    /// The document Cycles was given.
    pub scene_path: PathBuf,
    /// The image it wrote.
    pub image_path: PathBuf,
    /// How long the process took.
    pub seconds: f64,
}

/// Renders one scene document with Cycles.
///
/// # Errors
///
/// Fails rather than returning a picture whenever anything about the run was wrong: the scene
/// could not be written, the binary was not there, the process failed, **or the process reported
/// an error and carried on**. That last one is the reason this function exists in the form it
/// does — see the module documentation of [`crate::runner`].
///
/// # Why the lossiness is carried rather than decided here
///
/// [`CyclesOptions::lossy`] is the emitter's own decision and this does not second-guess it. A
/// caller who asked to be told about dropped fields gets them in [`CyclesFrame::report`] even
/// when the render succeeds, because a knowingly approximate frame is a legitimate result and a
/// silent one is not.
pub fn render_scene(scene: &Value, options: &CyclesOptions) -> Result<CyclesFrame, CyclesError> {
    let binary = locate_binary(options.binary.as_deref())?;

    let emit_options = EmitOptions {
        lossy: options.lossy,
        exposure: options.exposure,
        ..EmitOptions::default()
    };
    let report = emit_to_file(scene, &options.scene_path, &emit_options)
        .map_err(|error| CyclesError::Emit(error.to_string()))?;

    // The size is read through the emitter's own camera reader, so the document and the buffer
    // cannot come from two readings of the same camera that have drifted apart.
    let (width, height) =
        render_size(scene).map_err(|error| CyclesError::Emit(error.to_string()))?;

    let outcome = run(
        &binary,
        &options.scene_path,
        &options.image_path,
        &options.device,
        options.samples,
        Resolution { width, height },
    )?;

    let frame = read_back(&options.image_path)?;
    Ok(CyclesFrame {
        frame,
        report,
        scene_path: options.scene_path.clone(),
        image_path: options.image_path.clone(),
        seconds: outcome.seconds,
    })
}

/// Reads the written png back as the frame type the rest of the workspace uses.
///
/// # What the pixels are
///
/// Whatever Cycles put in the file, decoded without touching it: an eight bit rgba image, in
/// whatever space the renderer's own output stage produced. This does not apply a transfer curve
/// of its own, because doing so would make a comparison against the rasterised path a comparison
/// of two colour pipelines rather than of two renderers.
fn read_back(path: &Path) -> Result<RenderedFrame, CyclesError> {
    let bytes = std::fs::read(path).map_err(|error| CyclesError::Decode(error.to_string()))?;
    let image = decode_rgba8(&bytes)?;
    Ok(RenderedFrame {
        renderer: NAME,
        images: vec![RenderedImage {
            name: PRESENT_OUTPUT.to_owned(),
            format: "rgba8unorm".to_owned(),
            width: image.width,
            height: image.height,
            bytes: image.pixels,
        }],
    })
}

/// One decoded image.
struct Decoded {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// Decodes an eight bit png into rgba, whatever colour type it is stored in.
///
/// # Why this exists rather than a decode on the frame type
///
/// The frame type holds bytes a gpu produced, so its reader is written in terms of texel formats
/// and would have to grow a case for a file container. The container is this renderer's problem
/// alone — the rasterising path never sees a png — so it is answered here and the frame type is
/// left describing what it already described.
///
/// A file that is not eight bits per channel is refused rather than rescaled: Cycles' own output
/// is, and a file that is not would mean the renderer changed, at which point the conversion
/// belongs in a decision somebody makes rather than in a scaling nobody asked for.
fn decode_rgba8(bytes: &[u8]) -> Result<Decoded, CyclesError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|error| CyclesError::Decode(error.to_string()))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| CyclesError::Decode(error.to_string()))?;

    let (width, height) = (info.width, info.height);
    let pixels = match (info.color_type, info.bit_depth) {
        (png::ColorType::Rgba, png::BitDepth::Eight) => buffer[..info.buffer_size()].to_vec(),
        _ => {
            return Err(CyclesError::Decode(format!(
                "the image is {:?} at {:?} per channel, and this reads only rgba at eight",
                info.color_type, info.bit_depth
            )));
        }
    };
    Ok(Decoded {
        width,
        height,
        pixels,
    })
}
