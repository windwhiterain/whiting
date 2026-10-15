//! The massif drawn on its **Hermite surface**, so it can be looked at rather than measured.
//!
//! # What this scene is for
//!
//! Every number this branch produced came from a probe reading a surface. This draws the same
//! massif through `blockmesh::from_belts` and lets it be rendered, which is the check no amount of
//! measuring substitutes for.
//!
//! **It is the `landscape` scene's massif and the `landscape` scene's eye**, so the two pictures
//! differ in exactly one thing: the `landscape` one draws the massif's own triangles, and this one
//! draws the Hermite surface those triangles were sampled into. The camera is `landscape::eye_at`
//! rather than a second eye written here, because an eye written twice is a camera nobody rendered
//! — the same reason `landscape` exposes its own.
//!
//! # What it is meant to show
//!
//! The Hermite surface is **C1 inside every belt**, so a flank reads as one smooth rock face
//! rather than as a fan of facets, and it is **creased along every valley**, because that is where
//! two belts meet and `from_belts` tags their shared edge hard. The picture is therefore smooth
//! across a flank and sharp down every valley — which is what a massif is supposed to look like,
//! and is not what a triangle soup looks like.

use definition::Value;
use mountain::FacetMesh;
use scene::standard::Stage;

use crate::blockmesh::{self, BeltSurfaces};
use crate::kit::{
    camera_placed_up, directional, environment_light, image, instance, instances, mesh,
    mesh_material, post_processes, screen_material, shaded_point_schema, surface,
};
use crate::landscape;
use crate::shaders::{mesh_shaders, post_shader};

/// The massif's grid resolution per belt.
const SIDE: usize = 6;

/// The angle the frame takes in.
const FIELD_OF_VIEW: f64 = 0.72;

/// How far the frame reaches, as a multiple of the massif's own height.
const PLAIN: f64 = 1450.0;

/// What the rock's surface stage does.
///
/// **Plainly lit, with one deliberate term: a shallow sheen read off the normal.** It is what makes
/// a smooth flank and a faceted one separable by eye — a facet soup breaks the sheen into flat
/// plates and a C1 surface keeps it continuous, and that difference is the picture's whole subject.
/// Without it both read as the same tan lump under a sun.
const SURFACE: &str = r#"
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

/// A large quad standing behind the massif, square to the view.
///
/// **Placed by the eye rather than by the massif's own axes**, because it has to fill the frame
/// from where the camera is rather than merely be behind the subject in plan. It is drawn with an
/// emissive material so that a hole in the massif shows this colour instead of the film, which is
/// what makes a hole distinguishable from a dark triangle at all.
fn backdrop(eye: &[f64; 3], target: &[f64; 3], radius: f64) -> definition::PackedList {
    let forward = normalize([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    // Far enough that the massif is wholly in front of it, wide enough to fill the frame.
    let distance = radius * 4.0;
    let centre = [
        target[0] + forward[0] * distance,
        target[1] + forward[1] * distance,
        target[2] + forward[2] * distance,
    ];
    // Any two directions square to `forward` will do; world up and the view's own right are the
    // two a reader would pick, and they are never parallel because the eye looks near-horizontally.
    let right = normalize(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    let half = radius * 6.0;
    let corner = |sx: f64, sy: f64| {
        [
            centre[0] + right[0] * half * sx + up[0] * half * sy,
            centre[1] + right[1] * half * sx + up[1] * half * sy,
            centre[2] + right[2] * half * sx + up[2] * half * sy,
        ]
    };
    let vertices = [
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ];
    let mut points = definition::PackedBuilder::new(
        graphics::layout::packed_layout(&shaded_point_schema())
            .expect("the shaded point schema is a struct of components"),
    );
    // Two triangles, wound to face back along the view.
    for at in [0usize, 1, 2, 2, 1, 3] {
        let position = vertices[at];
        // Facing the eye, which is the only way an emissive quad shows anything.
        let normal = [-forward[0], -forward[1], -forward[2]];
        points
            .field("normal", &normal)
            .and_then(|points| points.field("position", &position))
            .and_then(|points| points.field("texture", &[0.0, 0.0]))
            .expect("a generator writes whole elements of its own layout");
    }
    points
        .finish()
        .expect("every generator writes whole elements of its own layout")
}

/// A sun over the camera's shoulder: up and behind, aimed down at the subject.
///
/// **Over the shoulder rather than beside the eye**, because a sun level with the eye throws a
/// slope at the silhouette into shadow no matter how well it faces the light — and a slope at the
/// silhouette is where a fold shows most, so it would put the artifacts and the ordinary shading
/// in the same place and make them impossible to tell apart. From above and behind, the only dark
/// slopes left are ones facing away, which is what a real massif looks like.
fn sun_from_behind(eye: &[f64; 3], target: &[f64; 3]) -> [f64; 3] {
    let back = normalize([eye[0] - target[0], 0.0, eye[2] - target[2]]);
    // Two thirds of the way up and one third of the way back: high enough to light the flanks, low
    // enough that the peaks still cast onto the valleys behind them.
    //
    // **Negated, because `directional` takes the direction the light travels** and not the
    // direction it comes from. The first version of this returned it the other way round and
    // rendered the whole massif black, which looked like a catastrophic lighting bug and was a
    // sign.
    normalize([-back[0] * 0.6, -0.75, -back[2] * 0.6])
}

fn normalize(vector: [f64; 3]) -> [f64; 3] {
    let length = (vector[0].powi(2) + vector[1].powi(2) + vector[2].powi(2)).sqrt();
    if length <= f64::MIN_POSITIVE {
        return [0.0, 0.0, 1.0];
    }
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// Builds the scene.
pub fn hermite_massif() -> Value {
    let authored = landscape::massif();
    let built: FacetMesh = authored.build().expect("the authored massif expands");
    let (low, high) = built.bounds();
    let radius = (((high[0] - low[0]).powi(2) + (high[2] - low[2]).powi(2)) as f64).sqrt() * 0.5;
    let summit = high[1] as f64;

    let belts = blockmesh::from_belts(&built, SIDE)
        .expect("the massif's belts are one representable surface");
    let points = tessellate(&belts);

    // The eye and the aim are the `landscape` scene's, read back through its own accessors so this
    // scene cannot drift from the picture it is meant to be compared with.
    let eye = landscape::eye_at(radius);
    let target = aim_at(&built, radius, summit);

    Value::structure([
        (
            "name".to_owned(),
            Value::String("hermite_massif".to_owned()),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed_up(
                eye,
                target,
                [0.0, 1.0, 0.0],
                FIELD_OF_VIEW,
                (radius * 0.05).max(0.5),
                PLAIN * 1.2,
            )]),
        ),
        (
            // The same sky the `landscape` scene draws, so the two pictures differ only in the rock.
            "images".to_owned(),
            Value::List(vec![image(
                "air",
                256,
                128,
                crate::air::Atmosphere::default().environment_pixels(256, 128),
                crate::shaders::Projection::Latlong,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                directional(
                    "sun",
                    // **The sun is placed from the eye, not written down.** Every round since the
                    // backdrop has assumed the whole of the dark in this picture was an artifact,
                    // and none of them separated that from a slope that genuinely faces away from
                    // the light -- which is what most of a massif looks like from any one side. This
                    // direction is the one over the camera's shoulder, so a backlit slope has to be
                    // the exception rather than the rule, and whatever is still dark after it is
                    // dark for a reason worth chasing.
                    sun_from_behind(&eye, &target),
                    [1.0, 0.95, 0.86],
                    4.6,
                    radius * 1.4,
                ),
                environment_light("air", "air", [1.0; 3], 0.55),
            ]),
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
            "mesh_materials".to_owned(),
            Value::List(vec![
                mesh_material(
                    "massif-cast",
                    "shadowing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                mesh_material(
                    "massif-depth",
                    "depthing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                // A single lighting material: the massif is one surface drawn once. The tint is rock
                // rather than the pale sheet the crease scene uses, because here the subject is the
                // shading's continuity and a colour change would read as a crease that is not there.
                mesh_material(
                    "massif-shade",
                    "lighting",
                    surface([0.66, 0.60, 0.50], 0.0, 0.72, [0.0; 3]),
                ),
                // **The backdrop is emissive and it is loud on purpose.** This scene renders against
                // Cycles' own film, which is black, so a hole in the massif and a triangle shaded
                // almost black are the same colour in the picture and no amount of looking tells
                // them apart -- two hypotheses were disproved by exactly that. A hole shows what is
                // behind the massif, so what is behind the massif is made bright: a hole then reads
                // as a hole.
                mesh_material(
                    "backdrop-shade",
                    "lighting",
                    surface([0.0, 0.0, 0.0], 0.0, 1.0, [0.9, 0.25, 0.6]),
                ),
            ]),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("massif", "Triangles", shaded_point_schema(), points),
                mesh(
                    "backdrop",
                    "Triangles",
                    shaded_point_schema(),
                    backdrop(&eye, &target, radius),
                ),
            ]),
        ),
        (
            "instances".to_owned(),
            instances(vec![
                (
                    Stage::Shadowing,
                    vec![instance(
                        "massif-cast",
                        "massif",
                        [0.0; 3],
                        [0.0; 3],
                        [1.0; 3],
                        "massif-cast",
                    )],
                ),
                (
                    Stage::Depthing,
                    vec![instance(
                        "massif-depth",
                        "massif",
                        [0.0; 3],
                        [0.0; 3],
                        [1.0; 3],
                        "massif-depth",
                    )],
                ),
                (
                    Stage::Lighting,
                    vec![
                        instance(
                            "massif",
                            "massif",
                            [0.0; 3],
                            [0.0; 3],
                            [1.0; 3],
                            "massif-shade",
                        ),
                        instance(
                            "backdrop",
                            "backdrop",
                            [0.0; 3],
                            [0.0; 3],
                            [1.0; 3],
                            "backdrop-shade",
                        ),
                    ],
                ),
            ]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
    ])
}

/// Where the eye looks: the massif's own centre of mass in plan, raised to a third of its height.
///
/// **Read off the built massif** rather than placed at the origin, because a frame aimed at the
/// origin is only centred when the author happened to write a massif that straddles it — which is
/// the same mistake the blank `hermite` scene made and the reason its render came out empty.
fn aim_at(built: &FacetMesh, _radius: f64, summit: f64) -> [f64; 3] {
    let (low, high) = built.bounds();
    let floor = f64::from(low[1]);
    [
        (f64::from(low[0]) + f64::from(high[0])) * 0.5,
        floor + (f64::from(summit) - floor) * 0.35,
        (f64::from(low[2]) + f64::from(high[2])) * 0.5,
    ]
}

/// How many samples a patch is cut into, per side.
///
/// **This number is the difference between the picture showing the claim and hiding it.** A patch
/// cut into its own four corners is two flat triangles, and a bicubic surface drawn that way is
/// visibly faceted — which is the picture the massif's own triangles give, so the render would
/// show the opposite of what it is for. Cutting each patch into a `SAMPLES × SAMPLES` grid is what
/// puts the curvature in the mesh, and the normals are the patch's own, so the shading is C1
/// whether or not the sampling is fine.
const SAMPLES: usize = 4;

/// Samples the Hermite surface into triangles.
///
/// **Every vertex is the Hermite patch's own evaluation** — that is the whole claim, and it is what
/// distinguishes this mesh from the massif's own triangles that it is otherwise identical to.
///
/// Written through `PackedBuilder` and the schema's own layout rather than into a flat buffer,
/// because that is the only way the field order stays the schema's rule and not a second copy of it.
fn tessellate(belts: &BeltSurfaces) -> definition::PackedList {
    let surface = belts.surface();
    let schema = shaded_point_schema();
    let layout = graphics::layout::packed_layout(&schema)
        .expect("the shaded point schema is a struct of components");
    let mut points = definition::PackedBuilder::new(layout);
    let span = (SAMPLES - 1) as f64;

    for face in 0..surface.face_count() as u32 {
        let patch = surface.patch(face);
        for row in 0..SAMPLES - 1 {
            for column in 0..SAMPLES - 1 {
                let at =
                    |dr: usize, dc: usize| ((column + dc) as f64 / span, (row + dr) as f64 / span);
                let near = at(0, 0);
                let along_u = at(0, 1);
                let along_v = at(1, 0);
                let far = at(1, 1);

                // **Which way this cell actually faces**, from its own corners.
                //
                // The patch's normal is `tangent_u x tangent_v` in the patch's own frame, and that
                // frame is not the same handedness everywhere: a valley's two belts wind along
                // their shared edge in opposite directions, so on one side the product points down
                // while the rock is up. Measured, 2554 of the massif's 29484 drawn triangles were
                // shaded by a normal pointing the opposite way — by pi, the whole turn — and a
                // path tracer draws those black.
                //
                // So a vertex's analytic normal is **turned to agree with the geometry it is drawn
                // on**. That keeps the crease, because the two sides of a seam still differ from
                // each other; what it removes is the one-sided flip that is not a crease at all.
                let near_p = patch.evaluate(near.0, near.1);
                let along_u_p = patch.evaluate(along_u.0, along_u.1);
                let along_v_p = patch.evaluate(along_v.0, along_v.1);
                let facing = (along_u_p - near_p).cross(along_v_p - near_p);
                let turned = |uv: (f64, f64)| {
                    let normal = patch.normal(uv.0, uv.1);
                    let length = normal.length();
                    if length <= 1.0e-12 {
                        return facing;
                    }
                    let unit = normal * (1.0 / length);
                    if unit.dot(facing) < 0.0 {
                        unit * -1.0
                    } else {
                        unit
                    }
                };

                // **The diagonal is chosen per cell, not fixed for the grid.**
                //
                // A quad whose four corners are not coplanar can be split so that its two triangles
                // face **opposite ways** — and then the path tracer draws the one facing away as
                // nothing, which reads as a hole. Measured: 3 of the massif's 1638 cells split along
                // the `(near, far)` diagonal have triangles a whole `pi` apart, at faces 686, 1163
                // and 1397, and an emissive backdrop behind the massif showed every dark patch
                // stayed dark rather than showing the backdrop, so they are not gaps in the mesh.
                //
                // They are in cells where the covered region **narrows**: belts 4/12 and 5/12 run
                // `u` to `0.9429` and `0.9199` at the top against `1.0000` at the bottom, and the
                // fold sits on that top row. A taper is a trapezoid and a trapezoid does not fold,
                // which is what said the problem was the split rather than the surface.
                //
                // So each cell is split by whichever diagonal leaves its two triangles agreeing,
                // which costs one cross product per cell and is the difference between a rock and a
                // rock with holes in it.
                let corners = [
                    patch.evaluate(near.0, near.1),
                    patch.evaluate(along_u.0, along_u.1),
                    patch.evaluate(far.0, far.1),
                    patch.evaluate(along_v.0, along_v.1),
                ];
                let agree = |a: usize, b: usize, c: usize, d: usize| {
                    let first = (corners[b] - corners[a]).cross(corners[c] - corners[a]);
                    let second = (corners[c] - corners[a]).cross(corners[d] - corners[a]);
                    let (one, two) = (first.length(), second.length());
                    if one < 1.0e-12 || two < 1.0e-12 {
                        return false;
                    }
                    (first * (1.0 / one)).dot(second * (1.0 / two)) > 0.0
                };
                let along_far = if agree(0, 1, 2, 3) {
                    [(0usize, 1usize, 3usize), (1usize, 2usize, 3usize)]
                } else {
                    [(0, 1, 2), (0, 2, 3)]
                };
                for (first, second, third) in along_far {
                    for corner in [first, second, third] {
                        let uv = [near, along_u, far, along_v][corner];
                        let position = corners[corner];
                        let normal = turned(uv);
                        points
                            .field("normal", &[normal.x, normal.y, normal.z])
                            .and_then(|points| {
                                points.field("position", &[position.x, position.y, position.z])
                            })
                            .and_then(|points| points.field("texture", &[0.0, 0.0]))
                            .expect("a generator writes whole elements of its own layout");
                    }
                }
            }
        }
    }
    points
        .finish()
        .expect("every generator writes whole elements of its own layout")
}
