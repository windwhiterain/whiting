//! A surface built out of patches, smooth everywhere except along a crease you chose.
//!
//! # What this scene is for
//!
//! Every other surface in the gallery is geometry a generator laid down once: a heightfield, a
//! sphere, a dune field. None of them has an **edge**, and an edge with a tag on it is the whole
//! point of the crate this surface comes from. So this scene authors a patch mesh, marks most of its
//! edges soft and a chosen few hard, raises the patches to a real degree, adds a displacement layer,
//! and then **samples the result into triangles** — which is the only way a scene document can hold
//! a freeform surface.
//!
//! The three things it is here to show, each of which is a feature rather than a picture:
//!
//! - **A crease.** The sheet folds along `x = 0`, and the edges along that fold are hard. The two
//!   sides of the fold carry different normals at the same point, and the scene gives each its own,
//!   so the fold reads as a fold rather than as a rounded ridge.
//! - **Smoothness everywhere else.** Every other interior edge is soft, so the sheet reads as one
//!   surface with a fold in it rather than as a grid of facets. The corners are knocked about a
//!   little, so the seams are not trivially satisfied to begin with. They are not made exactly zero
//!   even so, and should not be reported as: see [`CREASE_SLOPE`] for what this scene asserts
//!   instead, which is the separation between a crease and a seam.
//! - **A hierarchy.** The patches are raised to a real degree and one displacement layer carries a
//!   ripple, so what is drawn is a sum rather than the level the model started as.
//!
//! # The sampling is the scene's geometry, not a preview of it
//!
//! A scene document holds points and nothing else, so a patch surface reaches one by being sampled
//! into them, **three points to a triangle** — which is the shape every other mesh in this gallery is
//! written in, and the reason the indices `tessellate` produces are not what the scene carries.
//!
//! What the scene keeps is the part that mattered. Positions come from [`PatchMesh::tessellate`],
//! which samples every shared boundary **once** and hands both faces the same positions — so the two
//! sides of the crease are written from one set of numbers and the fold cannot open into a crack,
//! even though the two triangles either side of it repeat them.
//!
//! Normals are each face's own, read from the **composed** level: a hard edge means the two faces'
//! normals genuinely differ there, and one normal shared across the crease would round the fold into
//! a ridge. That is what the repetition buys, and it is the only reason this scene wants it.

use definition::{PackedList, Value};
use patch::topology::{EdgeKey, canonical};
use patch::{PatchMesh, Point3, VertexId};
use scene::standard::Stage;

use crate::geometry::Shaded;
use crate::kit::{
    camera, instance, instances, light_with_radius, mesh, mesh_material, post_processes,
    screen_material, shaded_point_schema, surface,
};
use crate::shaders::{mesh_shaders, post_shader};

/// Patches across the sheet.
const COLUMNS: usize = 9;

/// Patches down it.
const ROWS: usize = 9;

/// Half the sheet's width, in world units.
const EXTENT: f64 = 3.0;

/// The column the crease runs down, counted from zero.
///
/// Nine columns puts a grid line at `x = 0` exactly, so the fold is **along** a shared edge rather
/// than through the middle of a patch — a crease that cut a patch in half would not be a crease in
/// the model's own sense, which is a property of an edge.
const CREASE_COLUMN: usize = 4;

/// How steeply the sheet rises away from the fold, in world units per unit of `x`.
///
/// The number is the crease's **tangent-plane fold**, and it is the one measurement this scene has to
/// win: a fold of `slope / EXTENT` on each side puts the two sides' transverse slopes `2 * slope /
/// EXTENT` apart, which is an angle of `atan` of that. At the `0.55` first authored here that is
/// `0.367`, or `0.352` rad -- and the soft edges on this sheet already fold by up to `0.206`, so the
/// crease was not reliably the sharpest thing on it. At `1.10` the fold is `0.646` rad against a soft
/// worst of `0.207`, which is the separation the scene is for. `nurbs_probe` measurement 3 asserts
/// it, so a shape that loses the separation fails rather than merely looking wrong.
const CREASE_SLOPE: f64 = 1.10;

/// How far the sheet's corners are knocked about, in world units.
///
/// Kept small, and deliberately so. A knock is the only thing that makes a soft edge fold at all, and
/// the fold grows with it: measured on this grid at degree 3, a knock of `0.01` puts the soft edges at
/// a median `1.4e-2` rad and `0.055` at `7.7e-2`, while the crease is fixed by [`CREASE_SLOPE`]
/// whatever the knock is. The larger value first authored here bought the crease some competition and
/// nothing else.
const KNOCK: f64 = 0.02;

/// The degree the patches are raised to before anything is tagged.
///
/// Raised while every edge is still hard, because elevation is exact on an all-hard mesh and is not
/// on a mesh with soft edges. Doing it first keeps the sheet the shape it was authored as.
const DEGREE: usize = 3;

/// Samples a side each face is drawn at.
const SIDE: usize = 7;

/// How tall the ripple on the displacement layer stands, in world units.
const RIPPLE: f64 = 0.07;

/// Builds the scene.
pub fn nurbs() -> Value {
    let sheet = sheet();
    let points = drawn(&sheet);
    let point = shaded_point_schema();

    let materials = Value::List(vec![
        mesh_material(
            "sheet-cast",
            "shadowing",
            surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "sheet-depth",
            "depthing",
            surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
        ),
        // Pale enough that the crease is read by the shading rather than by a colour change: a
        // crease that is painted on is a crease in the artwork, not in the surface.
        mesh_material(
            "sheet-shade",
            "lighting",
            surface([0.74, 0.76, 0.80], 0.05, 0.42, [0.0; 3]),
        ),
    ]);

    Value::structure([
        ("name".to_owned(), Value::String("nurbs".to_owned())),
        (
            "cameras".to_owned(),
            // High enough to look across the sheet rather than along it. A crease seen edge on is a
            // line and not a fold, and the whole of what this scene has to show is the fold.
            Value::List(vec![camera(
                [5.4, 3.6, 5.4],
                [0.0, 0.45, 0.0],
                std::f64::consts::FRAC_PI_3,
                0.05,
            )]),
        ),
        (
            "lights".to_owned(),
            // Low and to one side, so the two sides of the crease are lit differently. A light
            // straight overhead lights a fold symmetrically, and a symmetric fold reads as a dip.
            Value::List(vec![light_with_radius(
                "key",
                [-7.0, 6.0, 5.0],
                [1.0, 0.95, 0.88],
                3.2,
                14.0,
                30.0,
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List(
                mesh_shaders(SURFACE, "")
                    .into_iter()
                    .chain(std::iter::once(post_shader(GRADE, "")))
                    .collect(),
            ),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![mesh("sheet", "Triangles", point, points)]),
        ),
        ("mesh_materials".to_owned(), materials),
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
                    vec![instance(
                        "sheet-cast",
                        "sheet",
                        [0.0; 3],
                        [0.0; 3],
                        [1.0; 3],
                        "sheet-cast",
                    )],
                ),
                (
                    Stage::Depthing,
                    vec![instance(
                        "sheet-depth",
                        "sheet",
                        [0.0; 3],
                        [0.0; 3],
                        [1.0; 3],
                        "sheet-depth",
                    )],
                ),
                (
                    Stage::Lighting,
                    vec![instance(
                        "sheet",
                        "sheet",
                        [0.0; 3],
                        [0.0; 3],
                        [1.0; 3],
                        "sheet-shade",
                    )],
                ),
            ]),
        ),
    ])
}

/// How many triangles the sheet is drawn as, which a caller needs before it builds the point list.
pub const TRIANGLE_COUNT: usize = (COLUMNS - 1) * (ROWS - 1) * (SIDE - 1) * (SIDE - 1) * 2;

/// How many points the sheet is drawn with: three a triangle.
///
/// A scene document holds points rather than indices — every other mesh in this gallery is the same
/// — so the crease line is written once per triangle that touches it. The two copies hold the same
/// numbers, because both come from the one shared sample, so the fold is continuous; what differs
/// between them is the normal.
pub const POINT_COUNT: usize = TRIANGLE_COUNT * 3;

/// The authored patch surface this scene draws.
///
/// Public because `examples/nurbs_probe.rs` measures it directly: the point of the scene is what the
/// model does, and measuring the document it compiles to would say nothing about that.
pub fn sheet() -> PatchMesh {
    let step = 2.0 * EXTENT / (COLUMNS - 1) as f64;
    // The grid runs `v` down `-z`, which is what puts a patch's `S_u` cross `S_v` **up**. With `v`
    // running up `+z` instead, every patch's own normal points into the ground and the sheet is lit
    // from underneath — a scene that builds and looks like nothing.
    let at = |i: usize, j: usize| {
        let (x, z) = (-EXTENT + i as f64 * step, EXTENT - j as f64 * step);
        Point3::new(x, height(x, z), z)
    };

    let mut positions = Vec::with_capacity(COLUMNS * ROWS);
    for j in 0..ROWS {
        for i in 0..COLUMNS {
            let mut corner = at(i, j);
            // A knock that is a function of the corner rather than of a generator's counter, so the
            // same sheet is built the same way every time and two runs are comparable.
            let wobble = KNOCK
                * ((i as f64 * 2.399 + j as f64 * 1.117).sin() * 0.6
                    + (i as f64 * 0.731 - j as f64 * 2.113).cos() * 0.4);
            corner.y += wobble;
            positions.push(corner);
        }
    }

    let mut quads = Vec::with_capacity((COLUMNS - 1) * (ROWS - 1));
    for j in 0..ROWS - 1 {
        for i in 0..COLUMNS - 1 {
            let near = (j * COLUMNS + i) as VertexId;
            let along_u = near + 1;
            let along_v = near + COLUMNS as VertexId;
            quads.push([near, along_u, along_v + 1, along_v]);
        }
    }

    let mut mesh = PatchMesh::build(positions, quads);
    // While every edge is still hard, so the sheet stays the shape it was authored as. Elevation on a
    // mesh with soft edges moves the surface, which is measured and recorded rather than avoided
    // here, but a demo that has moved its own geometry before it starts is harder to read.
    for _ in 1..DEGREE {
        mesh.elevate();
    }

    let crease = crease();
    for edge in interior_edges(&mesh) {
        if !crease.contains(&edge) {
            mesh.set_soft(edge);
        }
    }
    ripple(&mut mesh);
    mesh
}

/// The height of the sheet's authored shape at `(x, z)`.
///
/// A dome with a **fold** along `x = 0`: `x.abs()` has a corner there, and a corner in the shape is
/// a crease in the surface, which is the whole reason the edges along it are left hard. The dome is
/// what the soft edges have to agree about.
fn height(x: f64, z: f64) -> f64 {
    let dome = (1.0 - (x / EXTENT).powi(2)) * (1.0 - (z / EXTENT).powi(2));
    let fold = CREASE_SLOPE * (x.abs() / EXTENT).min(1.3);
    0.85 * dome + fold
}

/// Every edge with two faces on it, which is every edge that can carry a continuity tag.
fn interior_edges(mesh: &PatchMesh) -> Vec<EdgeKey> {
    mesh.topology()
        .edge_faces
        .iter()
        .filter(|(_, faces)| faces.len() == 2)
        .map(|(edge, _)| *edge)
        .collect()
}

/// The edges along the fold, which are the ones left hard.
///
/// Named by which patch column they sit between, so the crease is a line in the **authoring** and
/// not a set of index pairs that has to be kept in step with the grid by hand. Public so a probe
/// can ask which edges are the crease without re-deriving it and risking a second answer.
pub fn crease() -> Vec<EdgeKey> {
    (0..ROWS - 1)
        .map(|j| {
            let top = (j * COLUMNS + CREASE_COLUMN) as VertexId;
            let bottom = ((j + 1) * COLUMNS + CREASE_COLUMN) as VertexId;
            canonical([top, bottom])
        })
        .collect()
}

/// Writes one displacement layer, so what is drawn is a sum rather than the level it started as.
///
/// The wave runs **once across the whole sheet**, not once per patch. Each patch has its own
/// `(u, v)` over `[0, 1]`, so a wave written as `sin(2 pi u)` is a full cycle inside every patch and
/// its control row alternates `0, +0.87, -0.87, 0` — a net that zigzags hard enough for the patch to
/// fold through itself and point its own normal at the ground. One period over the sheet is a net
/// whose rows move together, which is what a displacement layer is for.
///
/// Even in `u` on purpose: a wave that is **odd** about the crease would add a second crease on top
/// of the one this scene is about, and the crease measurement would then be reading two folds.
fn ripple(mesh: &mut PatchMesh) {
    let layer = mesh.add_offset();
    let offsets = mesh.offsets_mut();
    let level = &mut offsets[layer];
    for face in 0..level.len() {
        let (degree_u, degree_v) = level.layouts()[face].degrees();
        // Which cell of the sheet this patch is, so the wave has somewhere to be across the whole.
        let cell = face % (COLUMNS - 1);
        let row = face / (COLUMNS - 1);
        for i in 0..=degree_u {
            let across = (cell as f64 + i as f64 / degree_u.max(1) as f64) / (COLUMNS - 1) as f64;
            for j in 0..=degree_v {
                let down = (row as f64 + j as f64 / degree_v.max(1) as f64) / (ROWS - 1) as f64;
                let wave = RIPPLE
                    * (across * std::f64::consts::TAU).cos()
                    * (down * std::f64::consts::TAU).cos();
                level.set_point(face, i, j, Point3::new(0.0, wave, 0.0));
            }
        }
    }
    // Enforcement runs over every layer, so the ripple's own interior rows are derived rather than
    // written, and the composed surface is as continuous as the tags say it is.
    mesh.enforce();
}

/// Samples the surface into the points a scene document holds: three a triangle.
///
/// The cell split and the winding are `Tessellation::triangles`' own, so this is the same primitive
/// that crate hands a caller who wants indices — written out flat, because a scene document has no
/// place to put an index.
///
/// Normals are each face's own and read from the **composed** level. A normal is not a displacement
/// and cannot be summed across layers, so it is asked of the surface rather than of any one term of
/// it, and it is asked of the face rather than of the sample, because a hard edge means two faces
/// disagree about it.
fn drawn(sheet: &PatchMesh) -> PackedList {
    let tessellation = sheet.tessellate(SIDE);
    let shared = tessellation.points();
    let composed = sheet.composed();
    let last = SIDE - 1;
    let mut points = Shaded::with_capacity(POINT_COUNT);

    for (number, face_grid) in tessellation.faces().iter().enumerate() {
        let net = composed
            .face(number)
            .expect("a face below the tessellation's count resolves");

        // One sample's position and normal, taken from the shared sample and from this face's own
        // patch. `normal` is a cross product and so carries an area, which a vertex shader would
        // shade by; the zero case is a patch with no area there and has no normal to report.
        let sample = |i: usize, j: usize| {
            let index = face_grid
                .at(i, j)
                .expect("a face's own grid reaches its own samples");
            let position = shared[index];
            let normal = net.normal(i as f64 / last as f64, j as f64 / last as f64);
            let length = normal.length();
            let unit = if length > 0.0 {
                normal / length
            } else {
                Point3::new(0.0, 1.0, 0.0)
            };
            (
                [position.x, position.y, position.z],
                [unit.x, unit.y, unit.z],
                [i as f64 / last as f64, j as f64 / last as f64],
            )
        };

        // The same two triangles per cell, in the same order, that `Tessellation::triangles` builds.
        for i in 0..last {
            for j in 0..last {
                let corner = [
                    sample(i, j),
                    sample(i + 1, j),
                    sample(i + 1, j + 1),
                    sample(i, j + 1),
                ];
                for index in [0, 1, 2] {
                    let (position, normal, texture) = corner[index];
                    points.push(position, normal, texture);
                }
                for index in [0, 2, 3] {
                    let (position, normal, texture) = corner[index];
                    points.push(position, normal, texture);
                }
            }
        }
    }
    points.finish()
}

/// What the lighting stage does to a surface after the light is applied.
const SURFACE: &str = r#"
    // A shallow sheen towards the horizon, so the two sides of the crease separate by something
    // other than their tint. It is read off the normal rather than the position, so it follows the
    // surface rather than the shape it was sampled from.
    let view = normalize(uniforms.projection.position - in.world_position);
    let fresnel = pow(1.0 - clamp(dot(normal, view), 0.0, 1.0), 4.0);
    color = color + vec3<f32>(0.05, 0.06, 0.08) * fresnel;
"#;

/// What the post stage does to the frame.
const GRADE: &str = r#"
    if depth >= 1.0 {
        let ray = ray_through(in.uv);
        let above = pow(clamp(ray.y, 0.0, 1.0), 0.5);
        let horizon = vec3<f32>(0.58, 0.62, 0.70);
        let zenith = vec3<f32>(0.10, 0.13, 0.20);
        let sky = mix(horizon, zenith, above);
        color = sky;
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
"#;
