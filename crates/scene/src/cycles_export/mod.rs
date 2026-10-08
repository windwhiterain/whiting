//! Emitting a `StandardScene` as a Cycles Standalone XML scene.
//!
//! # What this is for
//!
//! The rasterised path is for iterating while authoring. This module is for the frame that
//! ships: a scene description plus one subprocess produces an image with a path tracer's
//! light transport, which the rasteriser cannot do at any sample count.
//!
//! # The integration boundary
//!
//! Cycles cannot be linked. It is a program that reads a file. So the whole integration
//! is [`emit`] producing one XML file and a caller running one command, and nothing here
//! spawns a process or reads back an image. Keeping the process out of this module means
//! the emitter stays a pure function of its input and can be tested without a renderer.
//!
//! # Pinning
//!
//! The target format has no version field and no schema. What it accepts is decided by
//! `src/app/cycles_xml.cpp` at the commit the binary was built from, which this emitter
//! was written against and measured on:
//!
//! ```text
//! blender/cycles @ a456b761034dda42c32eef9f4aae0fa5a5c9f604  (5.3.0)
//! ```
//!
//! Bumping that pin means re-reading that file, because the loader changes with it. See
//! `.agents/investigate/cycles-standalone-scene-format.md` for what was established about
//! the format and which parts of it are inferred rather than measured.
//!
//! # What the format cannot express
//!
//! A `StandardScene` is a renderer-independent description, and it is deliberately wider
//! than any one renderer's file format. `SchemaKind::Any` in a `custom_schema`, and a
//! `point_schema` whose field names the author chose, are both open-ended. Cycles' XML
//! loader hardcodes exactly five vertex attributes and has no way to introduce another;
//! its own source says so:
//!
//! ```text
//! /* we don't yet support arbitrary attributes, for now add vertex
//!  * coordinates as generated coordinates if requested */
//! ```
//!
//! So a scene can be perfectly valid and still have nothing to export. What happens then
//! is [`LossyMode`], and it is the one genuinely opinionated decision in this module.

mod attribute_map;
mod error;
mod geometry;
mod light;
mod lossy;
mod material;
mod read;
mod scene;
mod settings;
mod transform;
mod writer;

pub use error::{EmitError, EmitResult, UnsupportedField};
pub use lossy::LossyReport;
pub use scene::render_size;

use std::path::Path;

/// How to write a scene that uses something the target format cannot express.
///
/// # Why this is a parameter and not a rule
///
/// A scene document names the renderer's capabilities nowhere, and it should not: a
/// description that only one backend can express is not a description of the scene. An
/// author writing `point_schema.ao` is describing geometry, and whether Cycles can carry
/// that is a fact about Cycles.
///
/// Both answers are defensible and they suit different authors, so the caller chooses:
///
/// - [`LossyMode::Reject`] is the default because a silently missing attribute produces a
///   plausible image. Nothing in the output says the field was dropped, the surface simply
///   shades as though the field were zero, and the picture looks like an authoring mistake
///   rather than an export one. Failing at compile time puts the author in front of the
///   actual cause while they still remember writing the field.
/// - [`LossyMode::Degrade`] is for an author who knows the backend has a narrower set and
///   wants the frame anyway. The dropped fields come back in a [`LossyReport`] so the
///   caller can log them; the image is knowingly approximate.
///
/// [`LossyMode::Reject`] exists as a distinct mode rather than as the absence of a flag so
/// that "I did not choose" and "I chose strict" cannot be confused when a default changes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LossyMode {
    /// Refuse to emit when the scene uses something the format cannot carry.
    #[default]
    Reject,
    /// Emit what can be carried and report the rest.
    Degrade,
}

/// Everything needed to write one Cycles scene file.
///
/// # Why a struct and not arguments
///
/// The three settings are the choices a caller makes once per render, and they belong
/// together. Passing them separately would make the fourth future option a breaking change
/// to every call site, which is exactly the pressure that produces a parameter object with
/// fifteen unrelated fields.
#[derive(Clone, Debug)]
pub struct EmitOptions {
    /// What to do about data the format cannot express. Defaults to rejecting it.
    pub lossy: LossyMode,
    /// The exposure written to the film's `exposure` socket.
    ///
    /// # Units
    ///
    /// A linear multiplier, on both sides. This was measured rather than assumed, twice and
    /// independently. A world of constant `strength=1.0, color=0.5` rendered through exposures of
    /// `0.5`, `1.0` and `2.0` produced 8-bit png values of 64, 128 and 255, which is `0.25`,
    /// `0.5` and `1.0` linear. A stop count would have produced 149 for the first.
    ///
    /// The same run also settles what the *output* is: with no OCIO configuration this build
    /// writes **scene-linear radiance straight into the file**, with no view transform. A world
    /// of `color=0.1, strength=1.0` came back as 26, which is `0.1 x 255 = 25.5` and not the
    /// 89 an sRGB encode would give. A caller comparing a Cycles frame with a rasterised one is
    /// therefore comparing radiance against a tone-mapped image unless it grades this one too.
    /// Our own `scene_schema::EXPOSURE` documents the same quantity as a multiplier, so the
    /// value passes through unconverted.
    pub exposure: f32,
    /// The integrator's `seed`, so a comparison render is reproducible.
    pub seed: i32,
    /// Written to the integrator's `use_denoise`.
    ///
    /// Leave this off for any frame that is going to be compared against the rasterised
    /// path. Denoising is a nonlinear filter over neighbouring pixels, so it dominates an
    /// RMSE comparison when it is on one side and off the other.
    pub denoise: bool,
    /// The `max_bounce` written to the integrator.
    pub max_bounce: i32,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            lossy: LossyMode::default(),
            exposure: 1.0,
            seed: 0,
            denoise: false,
            max_bounce: 8,
        }
    }
}

/// Emits a scene document as a Cycles Standalone XML document at `path`.
///
/// This is the one entry point a caller normally wants.
///
/// # Why the input is a document and not a compiled scene
///
/// The input is a `StandardScene` *document*, not the output of running the rasterised
/// [`crate::standard::StandardScene`] compiler over it. The compiler produces passes,
/// instance entries and buffer payloads for our own pipeline; a second renderer wants none
/// of those and would have to be fed the scene itself. Taking the document means the
/// emitter is independent of whether the rasterised pipeline ran at all, and it is the
/// same document an author wrote rather than something reconstructed from compiled state.
///
/// # Errors
///
/// Returns [`EmitError`] when the scene is not one Cycles can draw: a field the format has
/// no room for, a light type without a mapping, or geometry that is not triangles. In
/// [`LossyMode::Reject`] that includes fields that could have been dropped.
pub fn emit_to_file(
    scene: &definition::Value,
    path: &Path,
    options: &EmitOptions,
) -> EmitResult<LossyReport> {
    let (document, report) = emit_document(scene, options)?;
    std::fs::write(path, &document).map_err(|source| EmitError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(report)
}

/// Emits a scene and returns the document together with what had to be left out.
///
/// The report is returned rather than accumulated in hidden state, so a caller that
/// renders many scenes cannot be handed a previous scene's losses.
pub fn emit_document(
    scene: &definition::Value,
    options: &EmitOptions,
) -> EmitResult<(String, LossyReport)> {
    let mut report = LossyReport::default();
    let document = scene::write(scene, options, &mut report)?;
    Ok((document, report))
}

/// Emits a scene as a Cycles Standalone XML document and returns it as a string.
///
/// The losses are discarded. Use [`emit_document`] when they matter.
pub fn emit_to_string(scene: &definition::Value, options: &EmitOptions) -> EmitResult<String> {
    emit_document(scene, options).map(|(document, _)| document)
}
