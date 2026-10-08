//! Running Cycles Standalone as an offline renderer.
//!
//! # What this is for
//!
//! The rasterising path is for iterating while authoring. This is for the frame that ships: a
//! scene document plus one subprocess produces an image with a path tracer's light transport,
//! which the rasteriser cannot do at any sample count.
//!
//! # Why it is not a `Renderer`
//!
//! [`device::render::Renderer`] is about *instructions*: it is handed compiled graphics and
//! instance documents. A path tracer wants the opposite — the scene itself — because those
//! compiled documents are exactly the passes, instance entries and buffer payloads a ray tracer
//! has no use for. Implementing the trait would mean either feeding Cycles the wrong thing or
//! teaching the compiler to keep a copy of what it was given, and both would make the scene
//! language depend on which renderer is going to read it.
//!
//! So the entry point here is [`render_scene`], which takes the scene document and answers in
//! the same [`device::render::RenderedFrame`] the rasterising path answers in. A caller can hold
//! both and compare them; neither pretends to be the other.
//!
//! # The boundary
//!
//! Cycles cannot be linked. It is a program that reads a file. So the whole integration is the
//! emitter in [`scene::cycles_export`] writing one XML document and [`runner`] starting one
//! process, and nothing here reaches into the emitter's internals.
//!
//! # The failure this crate exists to prevent
//!
//! **Cycles reports an error, renders anyway, and exits zero.** A scene with
//! `light_type="pointt"` logs `ERROR: Unknown value`, goes on to produce a complete image, and
//! returns success; a missing include does the same. A scheduler that trusts the exit code
//! reports a successful render of a scene that is missing a light.
//!
//! So [`runner`] reads the child's stderr and treats `ERROR` in it as the failure signal it
//! actually is. That is the whole reason this is a crate rather than a two-line `Command::new`.

mod render;
mod runner;

pub use render::{CyclesFrame, CyclesOptions, render_scene};
pub use runner::{CyclesError, Resolution, RunOutcome, locate_binary};
