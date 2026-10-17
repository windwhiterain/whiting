//! A tube, a blade and two envelopes in one frame: every one of them a `shape::sweep`.
//!
//! # What this scene is for
//!
//! `shape::sweep` measures itself in [`sweep_probe`](../../shape/examples/sweep_probe.rs): the
//! frustum volume, the closed solid, the `(u, v)` of every vertex, the tip that collapses to a
//! point. **Those are numbers about a mesh, and a mesh can satisfy all of them and still look like
//! nothing** — inside out, edge on, lit so flat that it reads as a silhouette, or wound the wrong
//! way round so that every cap is a hole. This is the scene that shows the primitive reaching a
//! frame, and it is here rather than in an example because a gallery scene is the tree's one
//! picture format: this one is drawn by [`render`], `--locate`d by a person pointing at a pixel, and
//! swept by every suite that holds that every named scene draws at all.
//!
//! The four shapes are the table in `shape::sweep`'s own module, drawn side by side:
//!
//! | in the frame | spine | section | scale |
//! | --- | --- | --- | --- |
//! | the tube | straight, vertical, length 1 | [`shape::section::Section::circle`] | a three knot taper |
//! | the blade | a quarter arc: level, then vertical | [`shape::section::Section::flat`] | width, zero at both ends |
//! | the envelope, twice | a bent trunk | a circle, then a three-lobed outline | the crown's reach |
//!
//! The two envelopes are the same spine, the same reach law, the same ring count and the same call,
//! and **the only thing that differs is the section**. That is the claim the sweep module leads
//! with — *refining an envelope is a change of section and not of type* — and it is a claim about a
//! call site, so it belongs in a picture rather than in a table.
//!
//! # The `(u, v)` the loft returns is drawn, not printed
//!
//! There is no `u`/`v` debug channel in this tree: `scene::DebugChannel` is `Position`, `Depth` and
//! `Grade`, and none of them is a surface's own parameter. **What the gallery does offer is this
//! crate's own [`textured`] material** — the same one [`tiles`] draws its floor with — sampling a
//! generated image by a point's `texture` field, and the loft hands back a `(u, v)` per vertex, so
//! the tube's are written straight into that field and sampled by a checker written here. No
//! channel, no feature and no shader: the material is the one `tiles` already renders with, and the
//! image is a picture a loop can make, which is the same reason [`tile_pixels`] exists.
//!
//! # What is not built here
//!
//! No normals: [`shape::loft::Loft`] carries none, because a consumer's shading chooses its own, so
//! the ones in [`points_of`] are accumulated from the triangles the loft wrote. No windings of their
//! own either: every triangle comes out of the loft in the order it was wound, and the frame is
//! evidence of that order rather than a correction to it.
//!
//! [`render`]: ../../../device-rasterize/examples/render.rs
//! [`textured`]: crate::shaders::textured

use definition::{PackedList, Schema, Value};
use scene::standard::Stage;
use shape::curve::{Curve, Jet};
use shape::loft::Loft;
use shape::math::Vec2;
use shape::scalar::Scalar;
use shape::section::Section;
use shape::sweep::Sweep;

use crate::attributes::gallery_vertex_inputs;
use crate::geometry::{Shaded, cross, normalized, plane_facing, subtract};
use crate::kit::{
    IMAGE_FIELD, camera_placed, flux_for_irradiance, gallery_custom_schema_with_image,
    gallery_instance_schema, image, instance, instances, light, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, shaded_point_schema, shader, surface,
};
use crate::shaders::{
    Projection, depthing, lighting_with_normal, post_shader, shadowing, textured,
};

/// How many rings a sweep is sampled at, and how many points a ring has.
///
/// The same pair for every shape in the frame, so **the two envelopes cannot be told apart by a
/// resolution**: they are one call each with one set of numbers, and only the section differs.
const STATIONS: usize = 28;
const AROUND: usize = 32;

/// The length of an accumulated normal below which it is thrown away.
///
/// A vertex whose every incident triangle has no area — the ring at a station the scale law has
/// collapsed to a point — accumulates nothing, and a normal of zero length is a normal the shader
/// would read as `NaN` on the one vertex where it is asked about. The floor is far above the
/// rounding of a sum of a few thousand face normals and far below any real one, so it catches the
/// genuinely degenerate case and nothing else.
const NORMAL_FLOOR: f64 = 1.0e-12;

/// The checker the tube is painted with, in texels a side.
///
/// # Why the counts differ
///
/// `u` runs along the spine and `v` around the section, and the two are read by the eye as
/// different things: a band across the tube is a **station**, and a band around it is a **slot**.
/// Eight of the first over the length and sixteen of the second around it makes the two bands
/// visibly different widths on screen, so a reader can tell which is which without being told.
const CHECKER_SIZE: i64 = 128;
const CHECKER_ACROSS_U: i64 = 8;
const CHECKER_AROUND_V: i64 = 16;

/// How many cells the pattern runs before it repeats.
///
/// **Three, and the number is a property of the ring rather than of taste**: the last of sixteen
/// cells and the first are read side by side at the seam, and only a cycle that divides their
/// difference keeps them the same shade. See [`checker_pixels`].
const CHECKER_CYCLE: i64 = 3;

/// What the post stage does to the frame, which is [`tiles`]' own grade.
const GRADE: &str = r#"
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
"#;

/// The camera, which is placed rather than fitted.
///
/// **The azimuth is thirty one degrees and the elevation seventeen**, and both are the whole of the
/// framing: dead on, all four are silhouettes; past sixty, the tube stops being round and the
/// envelope stops being an envelope. Thirty one reads the tube as a tube, keeps the blade's broad
/// face towards the lens rather than edge on, and separates the four in the plan instead of
/// stacking them.
const EYE: [f64; 3] = [1.62, 1.34, 2.72];
const TARGET: [f64; 3] = [0.22, 0.42, 0.0];
const FIELD_OF_VIEW: f64 = 0.70;
const NEAR: f64 = 0.05;
const FAR: f64 = 12.0;

/// The frame, which is **not square**: four shapes side by side in a square are four shapes in a
/// square with a band of empty sky above them and nothing to say about it.
const WIDTH: i64 = 1280;
const HEIGHT: i64 = 720;

/// The floor they stand on, and how far it reaches.
const FLOOR_EXTENT: f64 = 3.2;
const FLOOR_TINT: [f64; 3] = [0.50, 0.49, 0.47];

/// How far each shape stands above the floor.
///
/// Two thousandths of a unit, which is under a pixel at this camera and is the whole of the reason
/// the floor is not z-fighting every shape's bottom cap: the shapes are built on `y = 0` and the
/// floor is at `y = 0`, so a small lift is the difference between a solid standing on a floor and a
/// floor speckled with the seams of four coincident surfaces.
const STANDING: f64 = 0.002;

/// The key, which is the light that makes the four read as solids rather than as outlines.
///
/// It stands **above and a little to the lens's right**, so every shape has a lit side and a
/// shadowed side at once and every shadow falls across the floor to the left of its caster — a light
/// straight overhead gives all four a lit top and a dark side of the same value, which is a
/// silhouette with a highlight on it. The reach is the distance from the lamp to the far corner of
/// the floor and past the shapes, so nothing in frame falls out of the cluster grid at its boundary.
const KEY_POSITION: [f64; 3] = [1.5, 2.3, 1.9];
const KEY_COLOUR: [f64; 3] = [1.0, 0.95, 0.87];
const KEY_IRRADIANCE: f64 = 2.6;
const KEY_REACH: f64 = 5.0;
const KEY_EXTENT: f64 = 3.2;

/// The fill, which names no reach and so is **global**: it is a second light on the dark side and
/// it is culled by nothing, so no shadow of it falls on the floor and no cluster boundary steps
/// across the frame where it stops.
const FILL_POSITION: [f64; 3] = [-2.1, 1.1, 1.5];
const FILL_COLOUR: [f64; 3] = [0.42, 0.52, 0.74];

/// Where each shape stands in the plan.
///
/// Four slots rather than three, because **the two envelopes are drawn beside each other rather
/// than one on top of the other**: they are the same spine, the same reach law and the same call, and
/// the whole of the claim is that swapping the section is all that separates them. Side by side, a
/// reader compares two silhouettes; stacked, they compare one silhouette and an occlusion.
///
/// **The blade is first and is given a whole unit of its own, because its midrib is a quarter arc of
/// radius one and it sweeps that whole unit sideways as it rises.** The first run of this scene put
/// it second and drew half of it behind the coarse envelope: a shape that occupies a unit of plan is
/// a unit of plan whichever slot it is in, and the slot is the only thing that decides whether it is
/// seen or hidden. Nothing overlaps: the scene's claim is that they are four **different shapes**,
/// and two solids sharing a pixel is a frame where that claim cannot be checked by looking.
const BLADE_AT: [f64; 3] = [-1.20, STANDING, 0.0];
const TUBE_AT: [f64; 3] = [0.06, STANDING, 0.02];
const COARSE_AT: [f64; 3] = [0.70, STANDING, 0.04];
const REFINED_AT: [f64; 3] = [1.30, STANDING, -0.04];

/// The midrib's initial normal, which is the only thing that decides **which way the blade is thin**.
///
/// # Why this number and not a round one
///
/// The frame's `binormal` is `tangent × normal`, and the midrib leaves its base along `+x`, so a
/// normal of `(0, a, b)` gives a binormal of `(0, -b, a)` — it can only ever point somewhere in the
/// `y`/`z` plane. The lens stands at `(1.62, 1.34, 2.72)` and looks down and to the left, so the
/// direction that shows a plate its broad face here is up and towards that eye. Writing `(0, 1, 0)`
/// would give a binormal of `(0, 0, 1)`, straight at the lens and a flat lit card; this tilts it
/// fifteen degrees back, which is what leaves the blade a shade of its own width in shadow down one
/// side and makes it read as a solid rather than as a cut-out. The first run of this scene wrote
/// `(0, 0, 1)` and drew the blade lying on the floor like a table top, because a binormal of `-y` is
/// a lens whose thickness stands up.
const BLADE_NORMAL: [f32; 3] = [0.0, 0.95, -0.31];

/// The tube's tint, which is near white because the checker is multiplied onto it and a coloured
/// tint would turn the pattern into one hue.
const TUBE_TINT: [f64; 3] = [0.96, 0.94, 0.90];
const BLADE_TINT: [f64; 3] = [0.62, 0.74, 0.42];
const COARSE_TINT: [f64; 3] = [0.86, 0.44, 0.30];
const REFINED_TINT: [f64; 3] = [0.30, 0.56, 0.88];

/// One shape in the frame: where it stands, what draws it, and the loft it was sampled into.
struct Shape {
    /// Names the mesh, its material and its three placements, so it is one word in a report.
    label: &'static str,
    /// Where its origin stands.
    at: [f64; 3],
    /// The material its lighting pass draws it with.
    material: &'static str,
    /// What the sweep sampled into.
    loft: Loft,
}

/// Builds the scene: one camera, two lights, a floor and the four shapes, each in its own material.
pub fn sweep_picture() -> Value {
    let shapes = [
        tube(),
        blade(),
        envelope("coarse", COARSE_AT, Section::circle()),
        envelope(
            "refined",
            REFINED_AT,
            Section::outline(&lobes()).expect("three lobes of twenty four points is a loop"),
        ),
    ];

    let point = shaded_point_schema();
    // **The attribute list is derived from the gallery's own custom schema, not from the one the
    // shaders below declare.** `image` is a marker rather than a value the row carries — the
    // compiler reads it out of the material and drops it before the bytes are packed — so a vertex
    // struct derived from the schema *with* the image field would put every field after it one
    // location too high, and the frame would refuse to bind an attribute of one component where the
    // shader read a matrix column. This is the half of [`tiles`] that is easy to copy wrong.
    let inputs = gallery_vertex_inputs(&point);
    // Every shader in the frame declares the custom schema **with** the image field, so one
    // instance row is one layout in every pass and the tube's material may name its checker. A
    // scene whose lighting shader added a field the shadowing stage's did not would be a scene
    // whose shadow pass reads the wrong offsets.
    let custom = gallery_custom_schema_with_image();

    let shaders = vec![
        shader(
            Stage::Shadowing,
            "shadowing",
            &shadowing(&inputs),
            scene::standard::shadow_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        shader(
            Stage::Depthing,
            "depthing",
            &depthing(&inputs),
            scene::standard::depth_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        // **Two lighting shaders, and that is the whole of the `(u, v)` channel.** The plain one
        // draws the three shapes whose colour is the point; the textured one draws the tube and
        // multiplies the checker by its tint. Both are declared against the same point schema and
        // the same custom schema, so either can draw either mesh, and which one draws a shape is a
        // property of the material it names.
        lighting_shader("lighting", &inputs, &point, &custom),
        shader(
            Stage::Lighting,
            "parameterised",
            &textured(&inputs, ""),
            scene::standard::light_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        post_shader(GRADE, "", None),
    ];

    let materials = vec![
        mesh_material("caster", "shadowing", surface([1.0; 3], 0.0, 1.0, [0.0; 3])),
        mesh_material(
            "occluder",
            "depthing",
            surface([1.0; 3], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material("floor", "lighting", surface(FLOOR_TINT, 0.0, 1.0, [0.0; 3])),
        // Only the tube's row names the image: a shadow map and a depth buffer are the pipeline's
        // own textures, and the other three surfaces are the colours the frame is about.
        mesh_material(
            "tube",
            "parameterised",
            surface_naming(TUBE_TINT, "checker"),
        ),
        mesh_material("blade", "lighting", surface(BLADE_TINT, 0.0, 1.0, [0.0; 3])),
        mesh_material(
            "coarse",
            "lighting",
            surface(COARSE_TINT, 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "refined",
            "lighting",
            surface(REFINED_TINT, 0.0, 1.0, [0.0; 3]),
        ),
    ];

    let mut meshes = vec![mesh(
        "floor",
        "Triangles",
        point.clone(),
        plane_facing(FLOOR_EXTENT, 0.0, true),
    )];
    for shape in &shapes {
        meshes.push(mesh(
            shape.label,
            "Triangles",
            point.clone(),
            points_of(&shape.loft),
        ));
    }

    let placed = |stage: &str, label: &str, at: [f64; 3], material: &str| -> Value {
        instance(
            format!("{stage}-{label}"),
            label,
            at,
            [0.0; 3],
            [1.0; 3],
            material,
        )
    };
    let standing: Vec<(&str, [f64; 3])> = std::iter::once(("floor", [0.0; 3]))
        .chain(shapes.iter().map(|shape| (shape.label, shape.at)))
        .collect();

    Value::structure([
        ("name".to_owned(), Value::String("sweep".to_owned())),
        (
            "frame".to_owned(),
            Value::structure([
                ("width".to_owned(), Value::Int(WIDTH)),
                ("height".to_owned(), Value::Int(HEIGHT)),
            ]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed(EYE, TARGET, FIELD_OF_VIEW, NEAR, FAR)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "key",
                    KEY_POSITION,
                    KEY_COLOUR,
                    flux_for_irradiance(KEY_IRRADIANCE, KEY_REACH),
                    KEY_EXTENT,
                    KEY_REACH,
                ),
                light(
                    "fill",
                    FILL_POSITION,
                    FILL_COLOUR,
                    flux_for_irradiance(0.55, 3.0),
                    KEY_EXTENT,
                ),
            ]),
        ),
        ("shaders".to_owned(), Value::List(shaders)),
        (
            "images".to_owned(),
            Value::List(vec![image(
                "checker",
                CHECKER_SIZE,
                CHECKER_SIZE,
                checker_pixels(),
                Projection::Latlong,
            )]),
        ),
        ("meshes".to_owned(), Value::List(meshes)),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "instances".to_owned(),
            instances(vec![
                (
                    Stage::Shadowing,
                    standing
                        .iter()
                        .map(|(label, at)| placed("shadowing", label, *at, "caster"))
                        .collect(),
                ),
                (
                    Stage::Depthing,
                    standing
                        .iter()
                        .map(|(label, at)| placed("depthing", label, *at, "occluder"))
                        .collect(),
                ),
                (
                    Stage::Lighting,
                    standing
                        .iter()
                        .map(|(label, at)| {
                            let material = if *label == "floor" {
                                "floor"
                            } else {
                                shapes
                                    .iter()
                                    .find(|shape| shape.label == *label)
                                    .map_or("blade", |shape| shape.material)
                            };
                            placed("lighting", label, *at, material)
                        })
                        .collect(),
                ),
            ]),
        ),
    ])
}

/// The plain lighting stage, declared against this scene's own custom schema.
///
/// `crate::shaders` has a builder that carries its own schema and one that takes the attribute
/// list, and **this scene needs the second**: the schema a shader *declares* is the one with the
/// image field, and the attribute list a shader *reads* is the one without it. A builder that
/// derived one from the other would put them one location apart and the frame would refuse to draw.
fn lighting_shader(label: &str, inputs: &str, point: &Schema, custom: &Schema) -> Value {
    shader(
        Stage::Lighting,
        label,
        &lighting_with_normal(inputs, "", "", "", Projection::default()),
        scene::standard::light_pass_schema(),
        point.clone(),
        gallery_instance_schema(),
        custom.clone(),
    )
}

/// The custom data of a row that names the image its shader samples.
///
/// **The field goes into the row and not into the material**, because a material's `custom_data` is
/// a value the shader's own `custom_schema` is checked against and the material around it is a
/// fixed three fields: `label`, `shader`, `custom_data`. This is the row [`tiles`] writes, unchanged.
fn surface_naming(tint: [f64; 3], image: &str) -> Value {
    let mut row = surface(tint, 0.0, 1.0, [0.0; 3]);
    if let Value::Struct(fields) = &mut row {
        fields.insert(IMAGE_FIELD.to_owned(), Value::String(image.to_owned()));
    }
    row
}

/// The tube: a straight spine, a circular section, and a three knot taper.
fn tube() -> Shape {
    let mut sweep = Sweep::new(straight(1.0), Section::circle());
    sweep.scale =
        Scalar::new(&[(0.0, 0.10), (0.5, 0.075), (1.0, 0.02)]).expect("three knots are a law");
    Shape {
        label: "tube",
        at: TUBE_AT,
        material: "tube",
        loft: sweep.sample(STATIONS, AROUND),
    }
}

/// The blade: a midrib that starts level and ends vertical, and a lens that narrows to a point.
///
/// # The tip is a ring collapsed to a point, and the surface is still closed
///
/// The width law is zero at `t = 0` and zero at `t = 1` and widest at `t = 0.45`, so **the ring at
/// either end is thirty two coincident vertices** — one place, written thirty two times — and the
/// fan that closes each end is thirty degenerate triangles rather than thirty real ones. Nothing is
/// added for it: the cap reuses the end ring's own vertices, so the loft is still `rings × around`
/// vertices and `2 (rings - 1) around + 2 (around - 2)` triangles. The surface has no boundary
/// because **no vertex is left out of a triangle**: the degenerate ones are inside their neighbours,
/// and a solid whose surface passes through the same point thirty two times is still a solid. That
/// is what makes a leaf's tip a number in a law rather than a case in the sweep.
fn blade() -> Shape {
    let mut sweep = Sweep::new(midrib(), Section::flat(1.0, 0.06));
    sweep.scale =
        Scalar::new(&[(0.0, 0.0), (0.45, 0.09), (1.0, 0.0)]).expect("three knots are a law");
    Shape {
        label: "blade",
        at: BLADE_AT,
        material: "blade",
        loft: sweep.sample(STATIONS, AROUND),
    }
}

/// The envelope: a bent trunk and a reach that grows toward the crown, with the section as the only
/// thing that differs between the two of them.
fn envelope(label: &'static str, at: [f64; 3], section: Section) -> Shape {
    let mut sweep = Sweep::new(trunk(), section);
    sweep.scale =
        Scalar::new(&[(0.0, 0.055), (0.45, 0.10), (1.0, 0.20)]).expect("three knots are a law");
    Shape {
        label,
        at,
        material: label,
        loft: sweep.sample(STATIONS, AROUND),
    }
}

/// A straight spine of unit length along `+y`: the one sweep whose volume has a closed form.
fn straight(length: f32) -> Curve {
    Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.0, length, 0.0]),
        Jet::new([0.0, length, 0.0], [0.0, length, 0.0]),
    ])
    .expect("the straight spine's knots are distinct")
}

/// A quarter arc of radius one: a leaf's midrib, which starts level and ends vertical.
///
/// # Why the initial normal is what it is
///
/// A flat section is an ellipse of a half width along the frame's `normal` and a half thickness
/// along its `binormal`, so **the blade's thin axis is the binormal and `set_initial_normal` is what
/// chooses it.** With the normal written as `+z` the binormal is `-y` — the lens lies flat, like a
/// table top, and the first run of this scene drew exactly that: a green sliver on the floor. The
/// normal here is chosen so the binormal comes out **up and towards the camera**, which stands the
/// blade on its broad face where the frame can read its width. A sweep has no "which way is up" of
/// its own, so this is a statement the spine makes and the frame inherits.
fn midrib() -> Curve {
    let steps = 4;
    let step = std::f32::consts::FRAC_PI_2 / steps as f32;
    let jets = (0..=steps)
        .map(|index| {
            let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * step;
            let (sin, cos) = angle.sin_cos();
            Jet::new([cos, 1.0 + sin, 0.0], [-sin * step, cos * step, 0.0])
        })
        .collect();
    let mut spine = Curve::new(jets).expect("the midrib's knots are distinct");
    spine.set_initial_normal(BLADE_NORMAL);
    spine
}

/// The trunk an envelope is swept along: three knots, and a lean rather than a bend.
///
/// The lean is **0.28 of a unit sideways over a unit of height**, which is about sixteen degrees —
/// gentle enough that the trunk reads as a trunk and not as a wire, and enough that the two
/// envelopes show their spine curving rather than standing up like the tube beside them.
fn trunk() -> Curve {
    let mut spine = Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.03, 1.05, 0.02]),
        Jet::new([0.03, 0.40, 0.02], [0.17, 0.88, 0.06]),
        Jet::new([0.17, 0.86, 0.06], [0.13, 0.52, 0.02]),
    ])
    .expect("the trunk's knots are distinct");
    spine.set_initial_normal([1.0, 0.0, 0.0]);
    spine
}

/// The envelope's refinement: a three lobed loop of twenty four points.
///
/// The radius is `1 + 0.25 cos(3θ)`, so the loop's widest lobe is a quarter over the coarse
/// envelope's radius and its waist is a quarter under it. That is deliberate: **a refinement that
/// only added detail would be hard to see**, and the three lobes are what tell the two apart at the
/// distance this camera stands at.
fn lobes() -> Vec<Vec2> {
    (0..24)
        .map(|index| {
            let angle = index as f32 / 24.0 * std::f32::consts::TAU;
            let radius = 1.0 + 0.25 * (3.0 * angle).cos();
            [radius * angle.cos(), radius * angle.sin()]
        })
        .collect()
}

/// The loft written as the triangle list a scene document draws: one shaded point per corner.
///
/// # Invariants
///
/// - **Every triangle is emitted in the order the loft wound it**, and every corner carries the
///   `(u, v)` the loft gave that vertex. Nothing here reorders, re-winds or re-samples: the frame
///   is the evidence of what `Sweep::sample` wrote, and a normal computed from the same triangles
///   is the only normal this scene knows how to be right about.
/// - **A normal is accumulated per loft vertex and normalised once**, not per triangle, so a tube
///   shades as a tube however many rings it is given. A facet normal per corner would make a
///   thirty two sided cylinder read as a prism, which is the one thing the section's own arc-length
///   sampling exists to avoid.
/// - **A vertex that accumulated nothing falls back to the direction away from the shape's own
///   centre.** That is the ring at a station the scale law collapsed to nothing, where every
///   incident triangle has no area; it is outward for every solid here and it is better than the
///   zero-length vector a shader would read as `NaN`.
fn points_of(loft: &Loft) -> PackedList {
    let positions = loft.positions();
    let mut accumulated = vec![[0.0f64; 3]; positions.len()];
    for triangle in loft.triangles() {
        let [a, b, c] = *triangle;
        let [first, second, third] = [a, b, c].map(|index| {
            let point = positions[index as usize];
            [
                f64::from(point[0]),
                f64::from(point[1]),
                f64::from(point[2]),
            ]
        });
        let face = cross(subtract(second, first), subtract(third, first));
        for index in [a, b, c] {
            let slot = &mut accumulated[index as usize];
            for axis in 0..3 {
                slot[axis] += face[axis];
            }
        }
    }
    let centre = {
        let mut sum = [0.0f64; 3];
        for point in positions {
            for axis in 0..3 {
                sum[axis] += f64::from(point[axis]);
            }
        }
        let count = positions.len().max(1) as f64;
        [sum[0] / count, sum[1] / count, sum[2] / count]
    };

    let mut points = Shaded::with_capacity(loft.triangles().len() * 3);
    for triangle in loft.triangles() {
        for &index in triangle {
            let position = positions[index as usize];
            let parameter = loft.uvs()[index as usize];
            let mut normal = accumulated[index as usize];
            let outward = [
                f64::from(position[0]) - centre[0],
                f64::from(position[1]) - centre[1],
                f64::from(position[2]) - centre[2],
            ];
            let squared = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
            if squared <= NORMAL_FLOOR * NORMAL_FLOOR {
                normal = outward;
            }
            points.push(
                [
                    f64::from(position[0]),
                    f64::from(position[1]),
                    f64::from(position[2]),
                ],
                normalized(normal),
                [f64::from(parameter[0]), f64::from(parameter[1])],
            );
        }
    }
    points.finish()
}

/// The checker the tube is painted with: `CHECKER_ACROSS_U` stations by `CHECKER_AROUND_V` slots,
/// four bytes a texel, top row first.
///
/// # Invariants
///
/// **The pattern closes in `v`, and it closes because its cycle is three cells.** `v = slot / around`
/// never reaches one, so the texels at the top of the image are read by the points nearest the seam
/// and by nothing else; a two phase checker puts sixteen cells round a ring, and a ring of sixteen
/// is not a whole number of two phases, so the last cell and the first carry opposite shades and
/// draw a dark line down a tube that has no seam. Three phases divide sixteen's wrap exactly —
/// `15 ≡ 0 (mod 3)` — so the last cell and the first agree and the join is two cells of one shade
/// rather than a step between two. `u` is **not** closed and is not meant to be: it runs from the
/// base to the crown, and its two ends being different shades is the difference worth showing.
///
/// The lit cell is the lighter of the two, because the `textured` shader multiplies the albedo by
/// `(0.25 + 1.5 · pattern)` and a pattern centred on the dark end draws a dark solid.
fn checker_pixels() -> Vec<u8> {
    let size = CHECKER_SIZE;
    let cell_u = size / CHECKER_ACROSS_U;
    let cell_v = size / CHECKER_AROUND_V;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for row in 0..size {
        for column in 0..size {
            let (across_u, around_v) = (column / cell_u, row / cell_v);
            let lit = (across_u + around_v) % CHECKER_CYCLE < CHECKER_CYCLE - 1;
            let shade = if lit { 235 } else { 105 };
            pixels.extend_from_slice(&[shade, shade, shade, 255]);
        }
    }
    pixels
}
