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
const SIDE: usize = 4;

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
                    [-0.45, -0.78, 0.44],
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
            // A single lighting material: the massif is one surface drawn once. The tint is rock
            // rather than the pale sheet the crease scene uses, because here the subject is the
            // shading's continuity and a colour change would read as a crease that is not there.
            Value::List(vec![mesh_material(
                "massif-shade",
                "lighting",
                surface([0.66, 0.60, 0.50], 0.0, 0.72, [0.0; 3]),
            )]),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![mesh(
                "massif",
                "Triangles",
                crate::kit::shaded_point_schema(),
                points,
            )]),
        ),
        (
            "instances".to_owned(),
            instances(vec![(
                Stage::Lighting,
                vec![instance(
                    "massif",
                    "massif",
                    [0.0; 3],
                    [0.0; 3],
                    [1.0; 3],
                    "massif-shade",
                )],
            )]),
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
                // Two triangles per cell, wound so the surface looks up.
                for (dr, dc) in [(0usize, 0usize), (0, 1), (1, 0), (1, 0), (0, 1), (1, 1)] {
                    let s = (column + dc) as f64 / span;
                    let t = (row + dr) as f64 / span;
                    let at = patch.evaluate(s, t);
                    let normal = patch.normal(s, t);
                    // **In the schema's own field order** — normal, position, texture, as
                    // `landscape::eroded` writes them. `PackedBuilder` enforces the layout rather
                    // than accepting fields in any order, which is why a mistake here is a build
                    // failure and not a scrambled vertex.
                    points
                        .field("normal", &[normal.x, normal.y, normal.z])
                        .and_then(|points| points.field("position", &[at.x, at.y, at.z]))
                        .and_then(|points| points.field("texture", &[0.0, 0.0]))
                        .expect("a generator writes whole elements of its own layout");
                }
            }
        }
    }
    points
        .finish()
        .expect("every generator writes whole elements of its own layout")
}
