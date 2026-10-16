//! A house: one authored building, standing on the ground, seen from outside.
//!
//! This is the shell phase's proof scene. The house is not modelled here — it is *derived* by
//! `building::derive` and expanded by `building::expand`, and what this file adds is the three
//! things a frame needs that a building is not: a ground, a sun, and a camera. The two meshes are
//! one solid each (the mass and the roof) because a material belongs to an instance and not to a
//! face, which is why walls and roof are drawn by different materials and are therefore different
//! meshes.
//!
//! What the scene is here to show, in the order the phase's contract names it:
//!
//! - **every solid is closed and outward** — the walls' inner faces and the floor exist, so the
//!   solid is watertight rather than a box of four planes, and the roof has a soffit, so a camera
//!   below the eave meets a face rather than a hole (the failure `gallery::built` records as
//!   "single-sided roofs");
//! - **one authored surface shades as one surface** — the normals come from the building's own
//!   corner normals, so a roof plane is smooth and the bends between planes are hard;
//! - **the roof is seated, not balanced** — its eaves stand `sink` below the wall top, an
//!   intersection the roof's `host` authorizes, which is the state `terraces` hand-builds today.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, RoofStyle, Starter};
use building::{MassId, RoofId, SolidId};

use crate::building_mesh::points_of;
use crate::geometry::heightfield;
use crate::kit::{
    camera, flux_for_irradiance, instance, instances, light_with_radius, mesh, mesh_material,
    post_processes, screen_material, shaded_point_schema, surface,
};
use crate::shaders::{mesh_shaders, post_shader};

/// What the derived house is: a rectangular two-bay cottage with a hipped roof.
///
/// The numbers are the scene's, not the crate's: the starter's defaults are a house of the same
/// order and this one is a little longer and a little steeper, because the proof is a picture.
const STARTER: Starter = Starter {
    width: 7.0,
    depth: 11.0,
    height: 3.6,
    wall: 0.35,
    slab: 0.45,
    overhang: 0.7,
    sink: 0.15,
    rise: 2.1,
    roof_thickness: 0.28,
    roof: RoofStyle::Hip,
    seed: 3,
};

/// Where the sun stands. The house's own middle is the origin, so this is a direction as much as a
/// place: high and to the left, so the near wall is lit and the far one is not. High rather than
/// grazing on purpose — a low sun rakes the wall and the ground at the angle a shadow map is
/// least able to resolve, and the frame this scene exists to produce is not the place to discover
/// that (see the shadow notes in `.agents/proposals`).
const SUN: [f64; 3] = [-22.0, 30.0, 20.0];

/// How far the sun's shadow page has to cover. The house stands inside twelve units and its shadow
/// reaches about twenty, so a tighter page than the scene's own extent is what puts texels where
/// the frame is looking.
const SHADOW_EXTENT: f64 = 14.0;

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// Builds the house scene.
pub fn house() -> Value {
    let building = derive::starter(&STARTER);
    let built = building
        .build()
        .expect("the starter house expands: its outline is a quad and its roof's rows are paired");

    let walls = points_of(&built, SolidId::Mass(MassId(0)));
    let roof = points_of(&built, SolidId::Roof(RoofId(0)));

    // A plaster wall, a tiled roof and dry ground. The three differ in tint and roughness and in
    // nothing else: what a surface *does* is the shader's business and this scene adds no body to
    // it, so the materials are the standard ones with a colour apiece.
    let plaster = || surface([0.74, 0.70, 0.61], 0.0, 0.85, [0.0; 3]);
    let tiles = || surface([0.33, 0.14, 0.10], 0.0, 0.62, [0.0; 3]);
    let ground = || surface([0.23, 0.21, 0.15], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("wall", plaster as fn() -> Value),
        ("roof", tiles as fn() -> Value),
        ("ground", ground as fn() -> Value),
    ] {
        materials.push(mesh_material(&format!("{name}-cast"), "shadowing", body()));
        materials.push(mesh_material(&format!("{name}-depth"), "depthing", body()));
        materials.push(mesh_material(&format!("{name}-shade"), "lighting", body()));
    }

    // Every mesh is authored in world coordinates, so its three instances carry no transform: a
    // house placed by a translation here would be a second copy of where the house already is.
    let placed = |stage: &str| {
        [("walls", "wall"), ("roof", "roof"), ("ground", "ground")]
            .into_iter()
            .map(|(label, material)| {
                instance(
                    &format!("{label}-{stage}"),
                    label,
                    [0.0; 3],
                    [0.0; 3],
                    [1.0; 3],
                    &format!("{material}-{stage}"),
                )
            })
            .collect::<Vec<_>>()
    };

    Value::structure([
        ("name".to_owned(), Value::String("house".to_owned())),
        (
            "cameras".to_owned(),
            // A three-quarter view from the near corner, high enough that the roof reads as a
            // volume rather than as a silhouette: the eave line and the ridge are both visible,
            // which is what says the roof has a soffit and a thickness.
            Value::List(vec![camera(
                [11.5, 4.2, 13.0],
                [0.0, 2.0, 0.0],
                std::f64::consts::FRAC_PI_3 * 0.85,
            )]),
        ),
        (
            "lights".to_owned(),
            // A `power` rather than a brightness, derived from the irradiance wanted at the
            // house's own distance from the light: see `.agents/proposals/physical-light-units.md`.
            Value::List(vec![light_with_radius(
                "sun",
                SUN,
                [1.0, 0.87, 0.70],
                flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                SHADOW_EXTENT,
                160.0,
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List(
                mesh_shaders("", "")
                    .into_iter()
                    .chain(std::iter::once(post_shader(GRADE, SKY, None)))
                    .collect(),
            ),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("walls", "Triangles", shaded_point_schema(), walls),
                mesh("roof", "Triangles", shaded_point_schema(), roof),
                mesh(
                    "ground",
                    "Triangles",
                    shaded_point_schema(),
                    heightfield(140, 420.0, |_, _| 0.0),
                ),
            ]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "instances".to_owned(),
            instances(vec![
                (Stage::Shadowing, placed("cast")),
                (Stage::Depthing, placed("depth")),
                (Stage::Lighting, placed("shade")),
            ]),
        ),
    ])
}

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The sky this scene draws, as a function the grade can call more than once.
///
/// # Invariants
///
/// The law is written once and the sky behind the house and the air in front of it both call it, so
/// the haze a distant wall fades into is the colour of the sky it is fading into rather than three
/// numbers copied out of a branch.
const SKY: &str = r#"
fn sky_at(ray: vec3<f32>) -> vec3<f32> {
    let above = pow(clamp(ray.y, 0.0, 1.0), 0.55);
    let horizon = vec3<f32>(0.88, 0.74, 0.56);
    let zenith = vec3<f32>(0.16, 0.27, 0.50);
    let below = vec3<f32>(0.34, 0.30, 0.24);
    let sun = normalize(vec3<f32>(-22.0, 30.0, 20.0));
    // A glow rather than a disc: the light itself is not in this frame, and what a sky shows of a
    // sun it does not contain is the scattering around it.
    let glow = pow(clamp(dot(normalize(ray), sun), 0.0, 1.0), 8.0) * 0.35;
    let sky = select(mix(horizon, below, clamp(-ray.y * 4.0, 0.0, 1.0)),
                     mix(horizon, zenith, above),
                     ray.y > 0.0);
    return sky + vec3<f32>(1.0, 0.86, 0.62) * glow;
}
"#;

/// What the post stage does to the frame.
///
/// # Invariants
///
/// **The air goes on before the curve and only over the geometry.** The sky is already the far
/// distance; what has to be crossed with air is everything between the eye and it, and the distance
/// is the post stage's own recovery of where the pixel is rather than a number assembled out of a
/// near and a far plane.
const GRADE: &str = r#"
    if depth >= 1.0 {
        color = sky_at(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * 0.006) * 0.85, 0.0, 0.92);
        // The air in front of a surface takes the colour of the sky *behind* it, so the ray's
        // height is held to the horizon rather than allowed to point into the ground.
        color = mix(color, sky_at(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)), air);
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.22), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.45, 0.0, 1.0);
"#;
