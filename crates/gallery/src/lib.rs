//! Building standard scene documents in rust, and the scenes built with it.
//!
//! A standard scene is a `Value` a schema validates, and writing one out by hand means writing
//! the same shapes at every call site: a light is four fields, a transform is three, a material
//! names a shader and carries custom data shaped like that shader's `custom_schema`. The builders
//! in [`kit`] write each of those once, so a scene says [`light`](kit::light) and reads as the
//! scene it is rather than as the document it compiles to.
//!
//! Nothing here is a second kind of scene. What these functions build is the same `Value` the
//! registry validates and
//! [`StandardScene`](scene::standard::StandardScene) compiles.
//!
//! # What is here
//!
//! - [`kit`] — one builder per field a scene document has: a light, a mesh, a material, a shader,
//!   an image, a camera, an instance.
//! - [`air`] — the air a scene is drawn through, written once and handed to every stage, so
//!   the sky and the haze a distant surface fades into are the same number.
//! - [`geometry`] — the generators that produce points: a heightfield, a sphere, a cuboid, and the
//!   noise a heightfield is usually given.
//! - [`shaders`] — WGSL the standard stages are written from, and the vertex attribute lists they
//!   read.
//! - [`attributes`] — derives those attribute lists from the schemas a scene declares, so a scene
//!   that changes its point schema gets matching shader input without writing it out.
//! - [`Gallery`] — the named scenes, built out of all of the above.
//!
//! # Why it is its own crate
//!
//! The builders are the part worth reusing and the scenes are what a particular set of them
//! produced. Keeping both here means a new scene is a file beside the others rather than an
//! addition to the compiler, and it means the builders can be depended on without the scenes.
//!
//! # Two things make a scene worth building in code
//!
//! - **Repetition.** A dune field is a hundred thousand triangles, which is a loop rather than a
//!   hundred thousand blocks of text, and a loop is reviewable where the text is not.
//! - **Shaders travel with the scene.** A shader carries its own source, so a scene names one from
//!   [`shaders`] and the document it produces is still self contained: it renders wherever it is
//!   sent.
//!
//! # What a scene can reach
//!
//! Everything the standard pipeline has, and that is a deliberate limit rather than a shortage. A
//! scene says what is in it and where it is drawn from; it does not say how light behaves, because
//! that is a choice the shader makes. So a scene cannot add a light type, but it can add any
//! surface behaviour a WGSL fragment stage can express, and the custom data of a material is
//! carried per instance, so two objects drawn by one pass differ only in the row they are drawn
//! from.

use definition::Value;

pub mod air;
pub mod attributes;
pub mod geometry;
pub mod kit;
pub mod shaders;

mod courtyard;
mod dunes;
mod lanterns;
mod monoliths;
mod spotlight;
mod stilllife;
mod sunlight;
mod terraces;
mod tidewater;
mod tiles;

pub use courtyard::{MOMENTS as COURTYARD_MOMENTS, courtyard};
pub use dunes::dunes;
pub use lanterns::lanterns;
pub use monoliths::monoliths;
pub use spotlight::spotlight;
pub use stilllife::stilllife;
pub use sunlight::sunlight;
pub use terraces::terraces;
pub use tidewater::tidewater;
pub use tiles::tiles;

/// Builds named scenes out of the parts in this module.
pub struct Gallery;

impl Gallery {
    /// Returns the names of every scene the gallery holds.
    pub fn names() -> &'static [&'static str] {
        &[
            "dunes",
            "lanterns",
            "monoliths",
            "spotlight",
            "stilllife",
            "studio",
            "sunlight",
            "terraces",
            "tidewater",
            "tiles",
            "courtyard-dawn",
            "courtyard-cloud",
            "courtyard-rain",
            "courtyard-dusk",
            "courtyard-night",
            "courtyard-snow",
        ]
    }

    /// Reports what a scene holds, in the terms its own document is written in.
    ///
    /// A scene is a document of lists and one dictionary of instances bucketed by pass, so this
    /// is the count of every list and of every bucket. It is the answer to a question a scene
    /// author asks constantly — how many instances did that grove actually place, and how many
    /// passes did they become — and it costs nothing, because building a document is the cheap
    /// half of anything.
    pub fn census(name: &str) -> String {
        let mut report = String::new();
        let scene = match name {
            other if COURTYARD_MOMENTS.contains(&other) => Some(courtyard(other)),
            _ => None,
        };
        let Some(scene) = scene else {
            return format!("`{name}` has no census of its own\n");
        };
        if let Some(fields) = scene.as_field_map() {
            // The three facts that decide whether a frame is bright: what the lights are, what
            // the camera is, and how much air is between the eye and the subject.
            for key in ["lights", "cameras"] {
                if let Some(list) = fields.get(key).and_then(Value::as_list) {
                    report.push_str(&format!("{key}: {}\n", list.len()));
                    for entry in list {
                        report.push_str(&format!("  {entry:?}\n"));
                    }
                }
            }
            if let Some(Value::List(shaders)) = fields.get("shaders") {
                report.push_str(&format!("shaders: {}\n", shaders.len()));
                for entry in shaders {
                    let Some(entry) = entry.as_field_map() else {
                        continue;
                    };
                    if entry.get("label").and_then(Value::as_str) != Some("post") {
                        continue;
                    }
                    if let Some(Value::String(source)) = entry.get("source") {
                        for line in source.lines() {
                            if line.starts_with("const ") && line.contains("f32") {
                                report.push_str(&format!("  {line}\n"));
                            }
                        }
                    }
                }
            }
        }
        report
    }

    /// Reports the box every instance a scene places occupies, and where its labels are.
    ///
    /// # Why this is worth a function
    ///
    /// A scene author places things by arithmetic and then has to know where they ended up. The
    /// two ways to find out are to render a frame and guess from the picture, or to read the
    /// positions back out of the document; the first is a day of guessing and the second is this.
    /// It is the same measurement the render harness makes of a debug channel, taken on the
    /// document instead of on a frame.
    pub fn bounds(name: &str) -> String {
        let scene = match name {
            other if COURTYARD_MOMENTS.contains(&other) => courtyard(other),
            _ => return format!("`{name}` has no bounds of its own\n"),
        };
        let mut report = String::new();
        let Some(instances) = scene
            .as_field_map()
            .and_then(|fields| fields.get("instances"))
            .and_then(Value::as_dict)
        else {
            return report;
        };
        for (pass, held) in instances {
            let Some(list) = held.as_list() else { continue };
            let (mut low, mut high) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
            let mut named: Vec<String> = Vec::new();
            for entry in list {
                let Some(fields) = entry.as_field_map() else {
                    continue;
                };
                let label = fields
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
                    .to_owned();
                let Some(transform) = fields.get("transform").and_then(Value::as_field_map) else {
                    continue;
                };
                let Some(at) = transform.get("translation").and_then(Value::as_list) else {
                    continue;
                };
                let mut point = [0.0; 3];
                for (axis, component) in at.iter().take(3).enumerate() {
                    match component {
                        Value::Float(number) => point[axis] = *number,
                        Value::Int(number) => point[axis] = *number as f64,
                        _ => {}
                    }
                }
                for axis in 0..3 {
                    low[axis] = low[axis].min(point[axis]);
                    high[axis] = high[axis].max(point[axis]);
                }
                if !label.contains(' ') && named.len() < 400 {
                    named.push(label);
                }
            }
            report.push_str(&format!(
                "{pass}: {} placements, x [{:.2}, {:.2}], y [{:.2}, {:.2}], z [{:.2}, {:.2}]\n",
                list.len(),
                low[0],
                high[0],
                low[1],
                high[1],
                low[2],
                high[2],
            ));
        }
        report
    }

    /// Builds the named scene.
    ///
    /// An unknown name lists the ones that exist rather than panicking, because the caller
    /// is usually a command line and a list is more use than a backtrace.
    pub fn build(name: &str) -> Value {        match name {
            "dunes" => dunes(),
            "lanterns" => lanterns(),
            "monoliths" => monoliths(),
            "spotlight" => spotlight(),
            "stilllife" => stilllife(),
            "studio" => scene::standard::example_standard_scene(),
            "sunlight" => sunlight(),
            "terraces" => terraces(),
            "tidewater" => tidewater(),
            "tiles" => tiles(),
            other if COURTYARD_MOMENTS.contains(&other) => courtyard(other),
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
