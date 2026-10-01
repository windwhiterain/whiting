//! Procedurally built scenes, as documents.
//!
//! A gallery scene is an ordinary standard scene document, built in rust rather than typed
//! out by hand. That is not a second kind of scene: it is the same `Value` the registry
//! validates and the same [`StandardScene`](crate::StandardScene) compiler consumes,
//! assembled by the builders in [`kit`] from the geometry in [`geometry`].
//!
//! Two things make a scene worth building in code rather than writing out:
//!
//! - **Repetition.** A dune field is a hundred thousand triangles, which is a loop rather
//!   than a hundred thousand blocks of text, and a loop is reviewable where the text is
//!   not.
//! - **Shaders travel with the scene.** A shader carries its own source, so a scene names
//!   a shader from [`shaders`] and the document it produces is still self contained: it
//!   renders wherever it is sent.
//!
//! # What a scene can reach
//!
//! Everything the standard pipeline has, and that is a deliberate limit rather than a
//! shortage. A scene says what is in it and where it is drawn from; it does not say how
//! light behaves, because that is a choice the shader makes. So a scene cannot add a light
//! type, but it can add any surface behaviour a WGSL fragment stage can express, and the
//! custom data of a material is carried per instance, so two objects drawn by one pass
//! differ only in the row they are drawn from.

use definition::Value;

pub mod geometry;
pub mod kit;
pub mod shaders;

mod dunes;
mod monoliths;
mod terraces;

pub use dunes::dunes;
pub use monoliths::monoliths;
pub use terraces::terraces;

/// Builds named scenes out of the parts in this module.
pub struct Gallery;

impl Gallery {
    /// Returns the names of every scene the gallery holds.
    pub fn names() -> &'static [&'static str] {
        &["dunes", "monoliths", "studio", "terraces"]
    }

    /// Builds the named scene.
    ///
    /// An unknown name lists the ones that exist rather than panicking, because the caller
    /// is usually a command line and a list is more use than a backtrace.
    pub fn build(name: &str) -> Value {
        match name {
            "dunes" => dunes(),
            "monoliths" => monoliths(),
            "studio" => crate::standard::example_standard_scene(),
            "terraces" => terraces(),
            other => unimplemented!(
                "the gallery holds {}, and `{other}` is not one of them",
                Self::names().join(", ")
            ),
        }
    }
}

/// The shader labels a gallery scene uses, so a scene names one rather than a string.
///
/// # Invariants
///
/// A label is written once here and used everywhere: the shader is declared under it, the
/// material names it, and the emitter names the instance entry after it. Three spellings
/// of one name would be three chances to disagree, and a material naming a shader that is
/// not declared is a compile error rather than a pass that draws nothing.
pub const LABELS: Labels = Labels {
    shadowing: "shadowing",
    depthing: "depthing",
    lighting: "lighting",
    post: "post",
};

/// The labels a gallery scene's shaders are declared under.
#[derive(Debug, Clone, Copy)]
pub struct Labels {
    /// The shader that renders a light's depth map.
    pub shadowing: &'static str,
    /// The shader that renders the depth of the scene.
    pub depthing: &'static str,
    /// The shader that shades the scene.
    pub lighting: &'static str,
    /// The shader that draws over the frame once the scene is shaded.
    pub post: &'static str,
}
