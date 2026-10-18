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

use building::Building;
use building::derive::{self, RoofStyle, Starter};
use building::identity::Identity;

use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, surface,
};
use crate::shaders::post_shader;

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

/// The image this scene is drawn into, in pixels.
///
/// # Why this frame is this shape
///
/// A house seen from its near corner is **wider than it is tall in the picture**, and the measured
/// number says by how much: the silhouette this camera draws stands `x 10..=915` by `y 286..=775`
/// of the pipeline's 1024 square, which is **905 by 489 pixels, an aspect of 1.851**. A square frame
/// therefore spends about half its width on sky and ground beside a building that is not square, and
/// no lens undoes that: the shape is the ceiling.
///
/// So the frame takes the silhouette's own proportions — **1.851 to one**, here 1392 by 752 — and
/// the lens is then worked out against it. The picture is 1,046,784 pixels against the square's
/// 1,048,576, so the frame is also very slightly the cheaper of the two.
///
/// **One number here is the measured one and the other is the decision.** The silhouette's
/// proportions were read out of a render (`x 10..=915` by `y 286..=775` of a 1024 square); the 1392
/// by 752 is those proportions rounded to a whole number of pixels without growing the picture past
/// the square it replaces.
const FRAME_WIDTH: i64 = 1392;
const FRAME_HEIGHT: i64 = 752;

/// The full **vertical** angle this house is drawn through, in radians.
///
/// # The arithmetic, so a later reader does not have to re-derive it
///
/// The square this scene replaced held `FRAC_PI_3 * 0.85` = **0.89012 rad** down the frame, whose
/// half-angle tangent is **0.476976**, and the silhouette above took 489 of that frame's 1024 rows.
/// The building must now take **89 % of 752 rows**, so the half-angle's tangent is scaled by the
/// silhouette's own share of the old frame over the share the new one asks for:
///
/// ```text
/// tan(new / 2) = 0.476976 * 489 / (0.89 * 1024) = 0.256906
/// new = 2 * atan(0.256906) = 0.50254 rad = FRAC_PI_3 * 0.4799
/// ```
///
/// **The two frame heights cancel, which is the whole of a trap worth naming.** A reader who scales
/// the lens by the magnification alone — 489 px becoming 89 % of 752, a factor of 1.369 — arrives at
/// a lens 4 % shorter than this one and a picture that does not quite fill the frame it was composed
/// for. Pixels per metre go as `height / tan(angle / 2)`, so it is the **ratio** of the two that is
/// held and not the factor.
///
/// **89 % and not 90 %**, because the hip's fascia stands outside the skin it belongs to: at 90 % the
/// silhouette measured `x 0..=1257` of `0..=1391`, and the left column of the frame carried **22
/// pixels of it**. The roof's own bounding edge is the last thing to arrive at a silhouette's side,
/// and a frame that clips a fascia clips a building.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.4799;

/// How far the sun's shadow page has to cover. The house stands inside twelve units and its shadow
/// reaches about twenty, so a tighter page than the scene's own extent is what puts texels where
/// the frame is looking.
const SHADOW_EXTENT: f64 = 14.0;

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// Builds the house scene.
pub fn house() -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the starter house expands: its outline is a quad and its roof's rows are paired");

    // **Every mesh the frame draws carries its elements' ids**, so a pixel of this house can be asked what it
    // is: the walls name their edges, the hip names its skins and its fascia.
    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // The starter's own foundation plate: a mass is a wall ring **and** a plate, so this house has always
    // authored one and this is the mesh that draws it.
    let floors = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Slab(_, _))
    });
    let roof = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Roof(_, _))
    });

    // A plaster wall, a tiled roof and dry ground. The three differ in tint and roughness and in
    // nothing else: what a surface *does* is the shader's business and this scene adds no body to
    // it, so the materials are the standard ones with a colour apiece.
    let plaster = || surface([0.74, 0.70, 0.61], 0.0, 0.85, [0.0; 3]);
    let tiles = || surface([0.33, 0.14, 0.10], 0.0, 0.62, [0.0; 3]);
    let ground = || surface([0.23, 0.21, 0.15], 0.0, 0.95, [0.0; 3]);
    let floor = || surface([0.34, 0.26, 0.18], 0.0, 0.80, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("wall", plaster as fn() -> Value),
        ("roof", tiles as fn() -> Value),
        ("floor", floor as fn() -> Value),
    ] {
        materials.push(mesh_material(
            &format!("{name}-cast"),
            "authored_shadowing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{name}-depth"),
            "authored_depthing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{name}-shade"),
            "authored_lighting",
            body(),
        ));
    }
    // The ground is not an element of the house, and it still writes the output — with the reserved zero.
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, ground()));
    }

    // Every mesh is authored in world coordinates, so its three instances carry no transform: a
    // house placed by a translation here would be a second copy of where the house already is.
    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("roof", "roof"),
            ("floors", "floor"),
            ("ground", "ground"),
        ]
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

    // **Two point schemas**: the walls and the hip carry `authored_id`; the ground carries none and writes
    // the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();

    Value::structure([
        ("name".to_owned(), Value::String("house".to_owned())),
        // The one field here that is about the picture rather than what is in it: a cottage seen from
        // its near corner is nearly twice as wide as it is tall, and the frame takes that shape. A
        // scene that names no frame is drawn into the pipeline's own square, so this is a decision.
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            // A three-quarter view from the near corner, high enough that the roof reads as a
            // volume rather than as a silhouette: the eave line and the ridge are both visible,
            // which is what says the roof has a soffit and a thickness.
            //
            // **The eye did not move and the target moved 0.43 m, for a measured reason.** A house
            // seven wide and eleven deep seen from its near corner does not project onto a frame
            // centred on its plan centre: the far corner pulls the silhouette's own centre
            // **0.0963 of a half-width to the left**, which is 67 px of a 1392 frame and which the
            // square was large enough to hide. `0.0963 * 17.4954 * 0.253028 = 0.4265 m` along the
            // camera's own right axis — `[0.749014, 0, -0.662587]` — brings the silhouette to the
            // middle, and the middle is what lets a lens be 4 % shorter without clipping it.
            //
            // **The lens is the other free variable**, and it is free because the frame is now the
            // silhouette's own shape rather than the pipeline's square.
            Value::List(vec![camera(
                [11.5, 4.2, 13.0],
                [-0.3195, 2.0, 0.2826],
                FIELD_OF_VIEW,
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
            Value::List({
                let mut shaders = stage_set("authored", "", "", &point, &extras);
                shaders.extend(stage_set("plain", "", "", &plain.point_schema(), &plain));
                shaders.push(post_shader(GRADE, SKY, None));
                shaders
            }),
        ),
        // The one image this scene writes beside the frame: which element each pixel shows.
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("roof", "Triangles", point.clone(), roof),
                mesh("floors", "Triangles", point, floors),
                mesh(
                    "ground",
                    "Triangles",
                    plain.point_schema(),
                    crate::ground::mesh(&building, 140, 420.0, |_, _| 0.0),
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

/// Returns the house's document alone, for a caller that wants to name what a frame shows.
pub fn document() -> Building {
    derive::starter(&STARTER)
}
