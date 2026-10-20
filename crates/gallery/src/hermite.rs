//! A Hermite surface with one crease in it, drawn from the model rather than sampled off it.
//!
//! # What this scene is for
//!
//! [`nurbs`] shows the per-patch Bernstein model: continuity there is a property a pass has to
//! restore after every edit, and the scene exists to show what that costs. This one shows the other
//! side. Continuity here is **structural** — the corner jets two faces share on a soft edge are the
//! same numbers, so the two patches are the same surface — and the scene exists to show the two
//! things that follow from that.
//!
//! - **The crease is a tag, not a shape.** The ridge runs down `x = 0` because the edges along it
//!   are [`Join::Hard`] and the two sides were twisted apart. Nothing about the geometry forces a
//!   fold there; removing the tag and re-twisting removes the fold, and both are ordinary writes.
//! - **The rest is smooth because it has nothing to be repaired by.** There is no enforcement pass
//!   in this model, so the sheet's seams are exactly C1 rather than measurably smooth, and no edit
//!   above can leave a control point out of step with its neighbour.
//!
//! # The sampling is the scene's geometry, not a preview of it
//!
//! A scene document holds points and nothing else, so the surface reaches one by being sampled into
//! them — three points a triangle, which is the shape every other mesh in this gallery is written
//! in. Positions are the patch's own evaluation and normals are the patch's own cross product, both
//! read per face: **a hard edge means two faces genuinely disagree about the normal**, and sharing
//! one across the crease would round the fold into a ridge. That disagreement is the point of the
//! scene, so it is written twice rather than reconciled once.

use definition::{PackedList, Value};
use patch::hermite_surface::{HermiteSurface, Join};
use patch::topology::{EdgeKey, canonical};
use patch::{FaceId, Point3, VertexId};
use scene::standard::Stage;

use document::geometry::Shaded;
use document::kit::{
    camera, instance, instances, light_with_radius, mesh, mesh_material, post_processes,
    screen_material, shaded_point_schema, surface,
};
use document::shaders::{mesh_shaders, post_shader};

/// Patches across the sheet.
const COLUMNS: usize = 13;

/// Patches down it.
const ROWS: usize = 9;

/// Half the sheet's width, in world units.
const EXTENT: f64 = 3.0;

/// The column the crease runs down, counted from zero.
///
/// Thirteen columns puts a grid line at `x = 0` exactly, so the fold is **along** a shared edge
/// rather than through the middle of a patch. A crease that cut a patch in half would not be a
/// crease in this model's sense, which is a property of an edge.
const CREASE_COLUMN: usize = 6;

/// How far the two sides of the crease are twisted apart, in world units.
///
/// **This number is the crease.** The positions carry a gentle dome and nothing else; the fold comes
/// entirely from the twist difference across a hard edge, because the twist is the one quantity that
/// reaches the cross-boundary derivative and reaches no boundary curve. Set it to zero and the tag
/// is still there and the sheet is still smooth across it.
///
/// The scale follows from the cell. The cross-boundary derivative carries `duv / step`, so the fold
/// grows with the twist and shrinks with the cell. Measured on this sheet: `4.0` gives `4.953e-1`
/// rad and `9.0` gives `1.034e0`, against `2.980e-8` for the soft seams beside it — a separation of
/// `3.5e+07`. So the crease is the sharpest thing on the sheet by seven orders of magnitude, which
/// is the claim the scene is making and the number that makes it checkable rather than asserted.
const CREASE_TWIST: f64 = 9.0;

/// How tall the dome is, in world units.
const DOME: f64 = 1.1;

/// How far the sheet's corners are knocked about, in world units.
///
/// Small, and deliberately so. A knock is the only thing that makes a soft seam fold at all, and
/// what this scene has to show is that a soft seam does **not** fold: the seams here stay at the
/// `acos` floor no matter how far the corners move, which is the property the old model's `4.771e-2`
/// rad was an accepted cost of.
const KNOCK: f64 = 0.05;

/// Samples a side each patch is drawn at.
const SIDE: usize = 7;

/// Builds the scene.
pub fn hermite() -> Value {
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
        ("name".to_owned(), Value::String("hermite".to_owned())),
        (
            "cameras".to_owned(),
            // High enough to look across the sheet rather than along it. A crease seen edge on is a
            // line and not a fold, and the whole of what this scene has to show is the fold.
            //
            // **The eye stands out along its own direction, two and a half sheet-widths out**
            // (2026-10-08): the number here was the unit direction itself standing where a
            // position belongs, seven tenths of a unit from the origin of a sheet six units
            // wide, so the frame was drawn from inside the dome and showed one flat facet of it
            // edge to edge. The direction is kept and the distance is the sheet's: at six degrees
            // of arc a second further in or out either clips the corners or drowns the fold.
            Value::List(vec![camera(
                [2.703, 7.052, 2.703],
                [0.0, 0.6, 0.0],
                std::f64::consts::FRAC_PI_3,
            )]),
        ),
        (
            "lights".to_owned(),
            // Low and to one side, so the two sides of the crease are lit differently. A light
            // straight overhead lights a fold symmetrically, and a symmetric fold reads as a dip.
            //
            // **The power is the inverse-square law, not a taste** (2026-10-08): a point light's
            // power is a flux, and what reaches the sheet is it spread over the sphere at the
            // light's own distance — `4 pi d^2` with `d^2 = 110` here — so the `3.2` this scene
            // carried from before the photometry handoff arrived as two thousandths of an
            // irradiance and drew the sheet black. Four units of irradiance at the sheet is what
            // the other scenes' directional lights give their rock, and four times that sphere
            // is the flux that delivers it.
            Value::List(vec![light_with_radius(
                "key",
                [-7.0, 6.0, 5.0],
                [1.0, 0.95, 0.88],
                document::kit::flux_for_irradiance(4.0, 110.0f64.sqrt()),
                14.0,
                30.0,
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List(
                mesh_shaders(SURFACE, "")
                    .into_iter()
                    .chain(std::iter::once(post_shader(GRADE, "", None)))
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
pub const POINT_COUNT: usize = TRIANGLE_COUNT * 3;

/// The surface this scene draws.
///
/// Public so that a probe can measure the model rather than the document it compiles to: what the
/// scene is here to show is what the surface does, and a measurement of the document would say
/// nothing about that.
pub fn sheet() -> HermiteSurface {
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

    let mut surface =
        HermiteSurface::build(positions, quads).expect("a grid mesh is representable");
    crease(&mut surface);
    surface
}

/// The height of the sheet's authored shape at `(x, z)`.
///
/// A dome and **nothing else**. The fold is not here: it is made by the tag and the twist, which is
/// the point this scene is making. Putting a crease in the geometry as well would leave the sheet
/// with a fold that no tag accounts for, and a reader could not tell which of the two made it.
fn height(x: f64, z: f64) -> f64 {
    DOME * (1.0 - (x / EXTENT).powi(2)) * (1.0 - (z / EXTENT).powi(2))
}

/// Tags the edges along the fold hard and twists the two sides apart.
///
/// **Two writes, and the fold appears.** Tagging the edges is what stops the twist being carried
/// across them; twisting each side is what makes the normals disagree. Both are ordinary mutations
/// on the surface and neither needs a repair afterwards, which is the difference from the model in
/// [`nurbs`]: there, the caller writes a control point and a later pass makes the tags true, so the
/// surface is briefly wrong and the repair is invisible.
fn crease(surface: &mut HermiteSurface) {
    for edge in fold_edges() {
        surface.set_join(edge, Join::Hard);
    }
    // Opposite twists at the two corners of the crease on each face, so the fold runs the whole way
    // down rather than pinching at one end of it.
    for row in 0..ROWS - 1 {
        let left = face_at(row, CREASE_COLUMN - 1);
        let right = face_at(row, CREASE_COLUMN);
        surface.set_twist(left, 1, Point3::new(0.0, 0.0, CREASE_TWIST));
        surface.set_twist(right, 3, Point3::new(0.0, 0.0, -CREASE_TWIST));
    }
}

/// The edges along the fold, named by the patch column they sit between.
///
/// Named by column rather than as index pairs, so the crease is a line in the **authoring** and not
/// a set of numbers that has to be kept in step with the grid by hand.
pub fn fold_edges() -> Vec<EdgeKey> {
    (0..ROWS - 1)
        .map(|row| {
            let top = (row * COLUMNS + CREASE_COLUMN) as VertexId;
            let bottom = ((row + 1) * COLUMNS + CREASE_COLUMN) as VertexId;
            canonical([top, bottom])
        })
        .collect()
}

/// The face at one cell of the grid.
fn face_at(row: usize, column: usize) -> FaceId {
    (row * (COLUMNS - 1) + column) as FaceId
}

/// Samples the surface into the points a scene document holds: three a triangle.
///
/// Two triangles per cell, in the winding the crate's own tessellation uses. Normals are read from
/// the **face**, never from the sample and never shared between the two sides of a crease: a hard
/// edge means the two faces' normals genuinely differ, and one normal written to both would round
/// the fold into a ridge — which is the exact failure [`nurbs`]'s module documentation warns about.
fn drawn(surface: &HermiteSurface) -> PackedList {
    let last = SIDE - 1;
    let mut points = Shaded::with_capacity(POINT_COUNT);

    for face in 0..surface.face_count() as FaceId {
        let patch = surface.patch(face);
        let sample = |i: usize, j: usize| {
            let (s, t) = (i as f64 / last as f64, j as f64 / last as f64);
            let position = patch.evaluate(s, t);
            let normal = patch.normal(s, t);
            let length = normal.length();
            // A zero normal is a patch with no area there and has no normal to report; up is the
            // only defensible guess and the sample's own position says which way that is.
            let unit = if length > 0.0 {
                normal / length
            } else {
                Point3::new(0.0, 1.0, 0.0)
            };
            (
                [position.x, position.y, position.z],
                [unit.x, unit.y, unit.z],
                [s, t],
            )
        };

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
