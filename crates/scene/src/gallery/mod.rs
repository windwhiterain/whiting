//! Procedurally built scenes, as documents.
//!
//! A gallery scene is an ordinary standard scene document, built in rust rather than
//! written out by hand. That is not a second kind of scene: it is the same `Value` the
//! registry validates and the same [`StandardScene`](crate::StandardScene) compiler
//! consumes, assembled by helpers that a hand written document would otherwise repeat.
//!
//! Two things make a scene worth building in code rather than typing out:
//!
//! - **Repetition.** A field of a thousand trees is a loop, not a thousand blocks of
//!   text, and a loop is reviewable where the text is not.
//! - **Shaders travel with the scene.** A shader carries its own source, so a gallery
//!   scene names a shader from the library in this module and the document it produces is
//!   still self contained: it renders wherever it is sent.

use definition::Value;

mod geometry;
mod kit;
mod shaders;

pub use geometry::MeshBuilder;
pub use kit::{instance, light, material, mesh, shader, triple};
pub use shaders::ShaderLibrary;

/// Builds named scenes out of the parts in this module.
pub struct Gallery;

impl Gallery {
    /// Returns the names of every scene the gallery holds.
    pub fn names() -> &'static [&'static str] {
        &["studio"]
    }

    /// Builds the named scene, where `argument` is whatever the caller supplied after the
    /// name.
    pub fn build(name: &str, _argument: &str) -> Value {
        match name {
            "studio" => crate::standard::example_standard_scene(),
            other => unimplemented!(
                "the gallery holds {}, and `{other}` is not one of them",
                Self::names().join(", ")
            ),
        }
    }
}
