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
//! - [`ridges`] — far ranges painted into the sky behind whatever the air has already closed
//!   over, for a horizon that is otherwise a patch of sky a shade off.
//! - [`geometry`] — the generators that produce points: a heightfield, a sphere, a cuboid, and the
//!   noise a heightfield is usually given.
//! - [`shaders`] — WGSL the standard stages are written from, and the vertex attribute lists they
//!   read.
//! - [`attributes`] — derives those attribute lists from the schemas a scene declares, so a scene
//!   that changes its point schema gets matching shader input without writing it out.
//! - [`patchmesh`] — a massif's facets as an editable patch surface. The one place this crate
//!   joins `mountain` to `patch`, which is the only reason it is here rather than in either.
//! - [`hermite`] — a patch surface with a crease in it, authored here rather than converted and
//!   sampled into the triangles a scene document holds. It replaced the `nurbs` scene, which drew the
//!   same picture on the per-patch Bernstein model; keeping both would have been two scenes doing one
//!   job, and the crease is made by one call rather than a tag plus two order-sensitive writes.
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
/// The identity scene: a building drawn so every pixel names its element.
pub mod authored;
/// The identity a building's frame can carry: which element a pixel shows.
pub mod building_identity;
pub mod building_mesh;
pub mod built;
pub mod geometry;
pub mod identity;
pub mod kit;
pub mod land;
pub mod locate;
pub mod patchmesh;
pub mod ridges;
pub mod shaders;

/// The dawn over the rice terraces, held as numbers so a tracer can be handed it as a world.
mod dawn;
mod dunes;
mod foldyard;
/// The shell phase's proof scene: one authored building, derived and expanded, standing in a frame.
///
/// Public before it is listed in [`Gallery::names`] because the scene is the phase's acceptance
/// picture and a session that is iterating on it should be able to call it directly.
pub mod house;
/// The mountain scene, and the massif it stands on.
///
/// Public so that `examples/alpine_probe.rs` can report on the skeleton without compiling the
/// whole document: the massif is this crate's one authored geometry, and the cheap half of the
/// authoring loop is the only way to iterate on it.
pub mod landscape;
mod lanterns;
mod monoliths;
/// The shed scene: the facade — a rhythm, holes cut in it, and parts standing in the bays.
pub mod shed;
mod spotlight;
mod stilllife;
mod sunlight;
mod terraces;
mod tidewater;
mod tiles;
/// The workshop scene: two storeys with a floor between them.
pub mod workshop;
/// The yard scene: a site on a slope, with a courtyard and a gate.
pub mod yard;

pub use dunes::dunes;
pub use foldyard::foldyard;
pub use hermite::hermite;
pub use landscape::landscape;
pub use lanterns::lanterns;
pub use monoliths::monoliths;
/// The Hermite patch surface, and the model it is sampled from.
///
/// Public so that `examples/hermite_probe.rs` can measure the surface rather than the document it
/// compiles to: what the scene is here to show is what the surface does, and a measurement of the
/// document would say nothing about that.
pub mod blockmesh;
pub mod hermite;
pub mod hermite_massif;
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
            "foldyard",
            "hermite",
            "hermite_massif",
            "house",
            "identity",
            "landscape",
            "lanterns",
            "monoliths",
            "shed",
            "spotlight",
            "stilllife",
            "studio",
            "sunlight",
            "terraces",
            "tidewater",
            "tiles",
        ]
    }

    /// Builds the named scene.
    ///
    /// An unknown name lists the ones that exist rather than panicking, because the caller
    /// is usually a command line and a list is more use than a backtrace.
    pub fn build(name: &str) -> Value {
        match name {
            "dunes" => dunes(),
            "foldyard" => foldyard(),
            "hermite" => hermite::hermite(),
            "hermite_massif" => hermite_massif::hermite_massif(),
            "house" => house::house(),
            "identity" => authored::identity(),
            "landscape" => landscape(),
            "lanterns" => lanterns(),
            "monoliths" => monoliths(),
            "shed" => shed::shed(),
            "workshop" => workshop::workshop(),
            "yard" => yard::yard(),
            "spotlight" => spotlight(),
            "stilllife" => stilllife(),
            "studio" => scene::standard::example_standard_scene(),
            "sunlight" => sunlight(),
            "terraces" => terraces(),
            "tidewater" => tidewater(),
            "tiles" => tiles(),
            other => unimplemented!(
                "the gallery holds {}, and `{other}` is not one of them",
                Self::names().join(", ")
            ),
        }
    }

    /// Reports what a named scene holds, in the terms its own document is written in.
    ///
    /// # Why this is worth a function
    ///
    /// A scene is a document of lists and one dictionary of instances bucketed by pass, so this
    /// is the count of every list and of every bucket. It is the answer to a question a scene
    /// author asks constantly — how many instances did that grove actually place, and how many
    /// passes did they become — and it costs no gpu work at all, because building a document is
    /// the cheap half of anything.
    ///
    /// # Invariants
    ///
    /// It reads the document and never the frame, so it is available for a scene that does not
    /// compile and for one that has never been drawn. That is the whole of its use: the two
    /// ways to find out what a scene placed are to render a frame and guess from the picture,
    /// or to read the placements back, and only the second one is a measurement.
    ///
    /// A field the document does not carry is **not** reported as zero. An absent field is a
    /// field the scene said nothing about, and a zero beside it reads as a claim about the
    /// scene rather than about this report. `exposure` is the one number an author reaches for
    /// when a frame is too bright or too dark, so it is printed by name when the document
    /// carries it and omitted when it does not.
    pub fn census(name: &str) -> String {
        let scene = Self::build(name);
        let mut report = format!("{name}\n");
        let Some(fields) = scene.as_field_map() else {
            report.push_str("  the scene is not a field map, so it holds no lists\n");
            return report;
        };
        // Every top level list, in the order the document writes them, so two scenes read side
        // by side line up. Sorted, because the field map is not in schema order and a report
        // whose lines move between runs cannot be diffed against itself.
        let mut lists: Vec<(&String, &Value)> = fields
            .iter()
            .filter(|(_, value)| value.as_list().is_some())
            .collect();
        lists.sort_by(|left, right| left.0.cmp(right.0));
        for (key, value) in lists {
            let Some(entries) = value.as_list() else {
                continue;
            };
            report.push_str(&format!("  {key}: {}\n", entries.len()));
        }
        // The instance buckets, which is the count a scene author is usually after: the number
        // of placements that became a pass.
        if let Some(buckets) = fields
            .get(scene::standard::INSTANCES)
            .and_then(Value::as_dict)
        {
            let mut buckets: Vec<(&String, &Value)> = buckets.iter().collect();
            buckets.sort_by(|left, right| left.0.cmp(right.0));
            for (pass, held) in buckets {
                let count = held.as_list().map_or(0, |list| list.len());
                report.push_str(&format!("  {pass}: {count} placements\n"));
            }
        }
        // The exposure, because it is the frame's brightness and it is a number an author
        // changes by hand. It used to live as a `const … f32` inside the post stage's WGSL,
        // where a census could only have reached it by scraping the source text; it is a
        // declared field now, so it is read the way every other fact here is.
        if let Some(exposure) = fields.get(scene::standard::EXPOSURE) {
            let shown = match exposure {
                Value::Float(number) => format!("{number}"),
                Value::Int(number) => format!("{number}"),
                other => format!("{other:?}"),
            };
            report.push_str(&format!("  exposure: {shown}\n"));
        }
        report
    }

    /// Reports the box every placement a scene makes occupies, bucket by bucket.
    ///
    /// # Why this is worth a function
    ///
    /// A scene author places things by arithmetic and then has to know where they ended up. This
    /// is the same measurement the render harness makes of a debug channel, taken on the
    /// document instead of on a frame.
    ///
    /// # Invariants
    ///
    /// The box is of **translation only**. A placement's rotation and scale are not folded in,
    /// so the box is where a placement's origin stands rather than the whole of its geometry,
    /// and it is a lower bound on what the scene draws. Folding them in would need each
    /// placement's matrix against each mesh's own extent, which is a different measurement and
    /// a more expensive one; an origin box is what tells an author their arithmetic was wrong.
    ///
    /// A placement carrying no transform, or a transform with no translation, is counted in
    /// the bucket and left out of the box, so the two numbers can disagree. They are printed
    /// together because the disagreement is the finding.
    pub fn bounds(name: &str) -> String {
        let scene = Self::build(name);
        let mut report = String::new();
        let Some(buckets) = scene
            .as_field_map()
            .and_then(|fields| fields.get(scene::standard::INSTANCES))
            .and_then(Value::as_dict)
        else {
            return report;
        };
        let mut buckets: Vec<(&String, &Value)> = buckets.iter().collect();
        buckets.sort_by(|left, right| left.0.cmp(right.0));
        for (pass, held) in buckets {
            let Some(list) = held.as_list() else {
                continue;
            };
            let (mut low, mut high) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
            let mut placed = 0usize;
            for entry in list {
                let Some(fields) = entry.as_field_map() else {
                    continue;
                };
                let Some(at) = fields
                    .get(scene::standard::TRANSFORM)
                    .and_then(Value::as_field_map)
                    .and_then(|transform| transform.get(scene::standard::TRANSLATION))
                    .and_then(Value::as_list)
                else {
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
                placed += 1;
            }
            if placed == 0 {
                report.push_str(&format!(
                    "{pass}: {} placements, no translation to measure\n",
                    list.len()
                ));
                continue;
            }
            report.push_str(&format!(
                "{pass}: {} placements, {placed} measured, x [{:.2}, {:.2}], y [{:.2}, {:.2}], z [{:.2}, {:.2}]\n",
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
