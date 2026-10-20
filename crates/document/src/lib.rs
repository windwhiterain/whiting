//! The standard scene document: the builders that write one, the identity it carries, and the
//! naming it answers.
//!
//! A standard scene is a `Value` a schema validates, and writing one out by hand means writing
//! the same shapes at every call site: a light is four fields, a transform is three, a material
//! names a shader and carries custom data shaped like that shader's `custom_schema`. The builders
//! in [`kit`] write each of those once, so a scene says [`light`](kit::light) and reads as the
//! scene it is rather than as the document it compiles to.
//!
//! # What is here
//!
//! - [`kit`] — one builder per field a scene document has: a light, a mesh, a material, a shader,
//!   an image, a camera, an instance, and the frame the scene is drawn into.
//! - [`shaders`] — WGSL the standard stages are written from, and the vertex attribute lists they
//!   read.
//! - [`attributes`] — derives those attribute lists from the schemas a scene declares, so a scene
//!   that changes its point schema gets matching shader input without writing it out.
//! - [`geometry`] — the generators that produce points: a heightfield, a sphere, a cuboid, and the
//!   noise a heightfield is usually given.
//! - [`identity`] — which authored element of a scene a pixel shows: the table that turns a
//!   number back into a name, and the mesh writers that put the number in the frame.
//! - [`building_identity`] — the same for a building, whose elements are its own.
//! - [`naming`] — the naming a scene answers with: the table of ids and handles, the sentence
//!   about what an id means, and the words a caller puts around a handle.
//!
//! # Why it is its own crate
//!
//! **The document is not the scene that made it.** A renderer draws a document and knows no
//! scene by name; the gallery builds the documents and knows every scene by name. The two meet
//! at a file, and this crate is the half of the meeting that is about the document: how one is
//! written, what an id in it means, and what a caller is told about a pixel.
//!
//! The builders are the part worth reusing and the scenes are what a particular set of them
//! produced, so the two are kept apart: a new scene is a file in the gallery rather than an
//! addition to the compiler, and the builders can be depended on without the scenes.
//!
//! # What a scene can reach
//!
//! Everything the standard pipeline has, and that is a deliberate limit rather than a shortage. A
//! scene says what is in it and where it is drawn from; it does not say how light behaves, because
//! that is a choice the shader makes. So a scene cannot add a light type, but it can add any
//! surface behaviour a WGSL fragment stage can express, and the custom data of a material is
//! carried per instance, so two objects drawn by one pass differ only in the row they are drawn
//! from.

pub mod air;
pub mod attributes;
/// The identity a building's frame can carry: which element a pixel shows.
pub mod building_identity;
pub mod geometry;
/// The identity scene's own half: which authored part of a massif a pixel shows.
pub mod identity;
pub mod kit;
/// The naming a scene answers with, and the words a caller puts around a handle.
pub mod naming;
pub mod shaders;
