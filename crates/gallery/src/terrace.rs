//! A terrace: three units of one style side by side, sharing their party walls.
//!
//! This is the slice that asked the vocabulary a question before it drew anything. A terrace's party wall is
//! **two leaves whose outer faces meet** — 0.15 m each, so the wall reads as one 0.3 m party wall — and
//! `examples/terrace_probe.rs` measured that the crate already accepts it: two masses flush at a shared face
//! report **zero violations**, and each stays watertight and manifold on its own.
//!
//! # What the document is
//!
//! Three units from `derive::COTTAGE`'s relations at a 6 × 8 footprint, each its own closed solid, each with its
//! own gable roof running along the row. **The wall is thinner than the style's external wall**: a party leaf is
//! 0.15 m because a party wall is a thin wall, and the style's `1/19` relation describes a wall that faces the
//! weather. That is the one number here the style does not decide, and it is the one the terrace's own type
//! decides.
//!
//! # What the frame shows
//!
//! The row, the party walls between the units, three bays of windows a front, and the roofs' independent ridges —
//! which is what a terrace looks like from the street.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, COTTAGE};
use building::grid::{FaceRef, Rhythm};
use building::identity::Identity;
use building::openings::{Inset, Opening};
use building::ops;
use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode};

use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::geometry::heightfield;
use crate::kit::{
    camera, flux_for_irradiance, instance, instances, light_with_radius, mesh, mesh_material,
    post_processes, screen_material, surface,
};
use crate::shaders::post_shader;

/// One party leaf's thickness. A party wall is thin because nothing weathers it, so it is the one number the
/// style's `1/19` external-wall relation does not decide.
const PARTY_LEAF: f32 = 0.15;

/// How many units the row has.
const UNITS: usize = 3;

/// Each unit's plan size: 6 m across, 8 m deep.
const UNIT_WIDTH: f32 = 6.0;
const UNIT_DEPTH: f32 = 8.0;

/// Returns a rectangular outline from `(x0, z0)` to `(x1, z1)` at the given elevation.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32, elevation: f32) -> Vec<PlanNode> {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, elevation))
        .collect()
}

/// Returns the row's document: `UNITS` units of `COTTAGE`'s proportions, flush at their party walls.
///
/// # Invariants
///
/// - **Each unit is a closed solid of its own**, which is what lets the party wall be two leaves: the row is not
///   one building with two internal walls, it is three buildings that touch.
/// - **Every unit's numbers come from the style except the party leaf.** Height, slab, eave, rise and the roof's
///   thickness are `COTTAGE::starter`'s answers; the wall is `PARTY_LEAF` twice.
/// - **Each unit's roof runs along the row**, so the ridges are parallel and the party walls are visible as the
///   gaps between them — a hip roof per unit would have cut the row into three separate cottages.
pub fn document() -> Building {
    let style = COTTAGE.starter(UNIT_WIDTH, UNIT_DEPTH, 1);
    let mut skeleton = BuildingSkeleton::default();
    for unit in 0..UNITS {
        let centre = (unit as f32 - (UNITS as f32 - 1.0) * 0.5) * UNIT_WIDTH;
        let mass = Mass::new(
            rectangle(
                centre - UNIT_WIDTH * 0.5,
                -UNIT_DEPTH * 0.5,
                centre + UNIT_WIDTH * 0.5,
                UNIT_DEPTH * 0.5,
                0.0,
            ),
            style.height,
        )
        .with_wall(PARTY_LEAF)
        .with_slab(style.slab);
        let id = ops::add_mass(&mut skeleton, mass);
        // **Each unit's plate is its own solid**, spanning the unit's footprint from one plate thickness below the
        // floor level up to it, so the wall ring overlaps it and no floor is split at a wall.
        skeleton.slabs.push(
            building::slab::Slab::new(
                rectangle(
                    centre - UNIT_WIDTH * 0.5,
                    -UNIT_DEPTH * 0.5,
                    centre + UNIT_WIDTH * 0.5,
                    UNIT_DEPTH * 0.5,
                    0.0,
                )
                .into_iter()
                .map(|node| node.position)
                .collect(),
                -style.slab,
                0.0,
            )
            .with_host(id),
        );
        // The roof is derived from the mass it covers, so a unit's eave follows its own walls rather than a
        // number the scene keeps beside them.
        if let Some(roof) = derive::gable_roof(
            id,
            &skeleton.masses[id.0],
            style.overhang,
            style.rise,
            style.sink,
            style.roof_thickness,
        ) {
            ops::add_roof(&mut skeleton, roof);
        }
    }
    // Three bays of windows a front, sized by the style's own glazing.
    let bays = COTTAGE.bays(UNIT_WIDTH);
    let (left, right, sill, height) = COTTAGE.glazing(UNIT_WIDTH / bays as f32, style.height);
    for unit in 0..UNITS {
        let front = FaceRef::new(MassId(unit), 2);
        ops::split_face(
            &mut skeleton,
            front,
            &Rhythm::Even { count: bays },
            &Rhythm::Even { count: 1 },
        );
        for column in 0..bays {
            let _ = ops::add_opening(
                &mut skeleton,
                Opening::new(front, 0, column, Inset::new(left, right, sill, height)),
            );
        }
    }
    Building::new(skeleton)
}

/// Builds the terrace scene.
pub fn terrace() -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the row expands: every unit's outline is a quad and its roof's rows are paired");

    // **Every mesh the frame draws carries its elements' ids**: three masses' walls and three roofs, so a pixel
    // of any unit can be asked which unit and which face it is.
    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // One plate a unit, over the unit's own footprint: three solids `document` authors, drawn here rather
    // than left in the table where nothing can see them.
    let floors = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Slab(_, _))
    });
    let roof = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Roof(_, _))
    });

    // Brick, tile and dry ground; the row is long and low, so the camera stands off its middle.
    let brick = || surface([0.56, 0.34, 0.26], 0.0, 0.88, [0.0; 3]);
    let tile = || surface([0.30, 0.20, 0.16], 0.0, 0.70, [0.0; 3]);
    let ground = || surface([0.26, 0.24, 0.17], 0.0, 0.95, [0.0; 3]);
    let floor = || surface([0.32, 0.24, 0.17], 0.0, 0.82, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("brick", brick as fn() -> Value),
        ("tile", tile),
        ("floor", floor),
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
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, ground()));
    }

    let placed = |stage: &str| {
        [
            ("walls", "brick"),
            ("roofs", "tile"),
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

    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();
    let ridge = style_ridge();
    let extent = f64::from(UNIT_WIDTH * UNITS as f32) * 0.9;

    Value::structure([
        ("name".to_owned(), Value::String("terrace".to_owned())),
        (
            "cameras".to_owned(),
            // Down the row from its near corner: the party walls read as the gaps between three roofs.
            Value::List(vec![camera(
                [
                    f64::from(UNIT_WIDTH) * 2.0,
                    ridge * 0.62,
                    f64::from(UNIT_DEPTH) * 1.5,
                ],
                [0.0, ridge * 0.35, 0.0],
                std::f64::consts::FRAC_PI_3 * 0.95,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![light_with_radius(
                "sun",
                SUN,
                [1.0, 0.87, 0.70],
                flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                extent,
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
                mesh("roofs", "Triangles", point.clone(), roof),
                mesh("floors", "Triangles", point, floors),
                mesh(
                    "ground",
                    "Triangles",
                    plain.point_schema(),
                    heightfield(140, 460.0, |_, _| 0.0),
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

/// Returns the ridge's height the frame has to fit: the style's wall top plus its own rise.
fn style_ridge() -> f64 {
    let style = COTTAGE.starter(UNIT_WIDTH, UNIT_DEPTH, 1);
    f64::from(style.height + style.rise)
}

/// Where the sun stands: high and to the left, so the row's near face is lit and the party walls cast into
/// shadow between the units.
const SUN: [f64; 3] = [-30.0, 36.0, 26.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

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
