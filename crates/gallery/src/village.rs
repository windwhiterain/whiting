//! A **village square**: a drum tower at its head, two earth houses to the sides, two log houses behind, and the
//! paving all four face onto.
//!
//! # What this scene is for
//!
//! Every other scene in this gallery is **one type**. This one is a *place*, and the place is the next scale the
//! repository had never drawn: the six types are compositions of masses, roofs, slabs and members, and a village
//! is a composition of **types**. What makes that possible is two operations, both new here —
//! `ops::translate`, which moves a document's plans, and `ops::merge`, which appends one document's skeleton to
//! another and re-bases every index it names.
//!
//! # The arrangement, and why it is this one
//!
//! - **The tower is at the head of the square**, north of it, facing south down the open ground: a 侗寨's 鼓楼 is
//!   the centre of the village and its square is the room in front of it;
//! - **the two 土掌房 stand east and west**, their doors on the square, because the flat-roofed house belongs to
//!   the same highland economy the tower does and its 晒场 faces the light;
//! - **the two 木楞房 stand south**, the village's own edge, where the log houses of a Naxi or Pumi village
//!   actually are;
//! - **the square is paved**, with one grating at its middle: the piece of ground the four types share.
//!
//! # What it does not draw
//!
//! - **no ground of its own** beyond the harness's own mesh: the village sits on level earth, and a terraced
//!   village on a slope would be a scene of its own;
//! - **no people, no trees, no well**: the square is the subject;
//! - **no lanes**: the square is bounded by the four buildings and open at its two ends, and the streets that
//!   run out of it belong to the settlement scene that does not exist yet.

use building::{Building, BuildingSkeleton, derive, ops};
use definition::Value;
use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::kit::{
    camera, environment_light, flux_for_irradiance, frame, image, instance, instances,
    light_with_radius, mesh, mesh_material, post_processes, screen_material, surface,
};
use document::shaders::{Projection, post_shader};
use scene::standard::Stage;

use crate::ground;

/// The square: sixteen metres by twelve, and the grating at its middle.
const SQUARE: [f32; 4] = [-8.0, -6.0, 8.0, 6.0];
const GRATE: f32 = 0.7;

/// Where each building stands, as the offset its own document is moved by.
const TOWER_AT: [f32; 2] = [0.0, -13.0];
const EARTHEN_WEST: [f32; 2] = [-13.0, 0.0];
const EARTHEN_EAST: [f32; 2] = [13.0, 0.0];
const LOG_WEST: [f32; 2] = [-10.5, 12.0];
const LOG_EAST: [f32; 2] = [10.5, 12.0];

/// The sun, low from the south-west, so the tower's seven skirts are modelled against each other and the log
/// walls are raked.
const SUN: [f64; 3] = [-26.0, 26.0, -14.0];
const SUN_IRRADIANCE: f64 = 3.1;
const SHADOW_EXTENT: f64 = 60.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.62;
const GROUND_DIVISIONS: usize = 160;
const GROUND_EXTENT: f64 = 90.0;

/// The air over the village.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.19, 0.35, 0.62])
        .horizon([0.84, 0.80, 0.71])
        .ground([0.35, 0.31, 0.24])
        .density(0.0011)
        .scale_height(200.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the south, across the square, at the tower.
pub fn village_square() -> Value {
    scene(
        [0.0, 7.5, 30.0],
        [0.0, 7.0, -12.0],
        FIELD_OF_VIEW,
        "village_square",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the village expands: every type it is made of expands on its own");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "the village's own document must be clean, and it is not: {violations:?}"
    );

    let table = building::identity::Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    let roofs = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Roof(_, _) | building::Element::Roll(_, _)
        )
    });
    let flats = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            )
        )
    });
    let timbers = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
    });

    let plaster = || surface([0.74, 0.70, 0.60], 0.0, 0.88, [0.0; 3]);
    let earth = || surface([0.54, 0.43, 0.30], 0.0, 0.92, [0.0; 3]);
    let tiles = || surface([0.28, 0.28, 0.29], 0.0, 0.58, [0.0; 3]);
    let paving = || surface([0.50, 0.48, 0.43], 0.0, 0.86, [0.0; 3]);
    let timber = || surface([0.29, 0.19, 0.11], 0.0, 0.78, [0.0; 3]);
    let ground_earth = || surface([0.33, 0.29, 0.22], 0.0, 0.96, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("wall", plaster as fn() -> Value),
        ("earthen", earth),
        ("roof", tiles),
        ("paving", paving),
        ("timber", timber),
    ] {
        materials.push(mesh_material(
            &format!("{label}-cast"),
            "authored_shadowing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{label}-depth"),
            "authored_depthing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{label}-shade"),
            "authored_lighting",
            body(),
        ));
    }
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(
            &format!("ground-{stage}"),
            shader,
            ground_earth(),
        ));
    }

    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("roofs", "roof"),
            ("flats", "paving"),
            ("timber", "timber"),
            ("ground", "ground"),
        ]
        .into_iter()
        .map(|(label, material)| {
            instance(
                format!("{label}-{stage}"),
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
    let ground = ground::mesh(&building, GROUND_DIVISIONS, GROUND_EXTENT, |_, _| 0.0);

    Value::structure([
        ("name".to_owned(), Value::String(name.to_owned())),
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            Value::List(vec![camera(eye, target, field_of_view)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "sun",
                    SUN,
                    [1.0, 0.93, 0.80],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    170.0,
                ),
                environment_light("air", "air", [1.0; 3], 1.05),
            ]),
        ),
        (
            "images".to_owned(),
            Value::List(vec![image(
                "air",
                256,
                128,
                atmosphere().environment_pixels(256, 128),
                Projection::Latlong,
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List({
                let helpers = atmosphere().wgsl();
                let mut shaders = stage_set("authored", "", helpers.as_str(), &point, &extras);
                shaders.extend(stage_set(
                    "plain",
                    "",
                    helpers.as_str(),
                    &plain.point_schema(),
                    &plain,
                ));
                shaders.push(post_shader("", &helpers, Some(Projection::Latlong)));
                shaders
            }),
        ),
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("flats", "Triangles", point.clone(), flats),
                mesh("timber", "Triangles", point.clone(), timbers),
                mesh("ground", "Triangles", plain.point_schema(), ground),
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

/// Returns the document: five buildings and the paving they face onto.
///
/// # Invariants
///
/// **Each building is the scene that already draws it**, moved and merged rather than rebuilt: a village that
/// re-derived a tower would be a second implementation of the tower, and the two would drift the first time one
/// of them was edited. The document is the composition of five documents, which is what `merge` and `translate`
/// exist for.
///
/// **A merge appends where the document was authored**, so each one is translated first — and translated by its
/// own offset, which is the only thing this function says about where anything stands.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    for (mut part, at) in [
        (crate::drum_tower::document().skeleton, TOWER_AT),
        (crate::earthen::document().skeleton, EARTHEN_WEST),
        (crate::earthen::document().skeleton, EARTHEN_EAST),
        (crate::log_house::document().skeleton, LOG_WEST),
        (crate::log_house::document().skeleton, LOG_EAST),
    ] {
        ops::translate(&mut part, at);
        assert!(
            ops::merge(&mut skeleton, part),
            "a building of the village places no parts, so the merge always happens"
        );
    }
    // **The square is paved, with one grating at its middle**: the piece of ground the four types share, and
    // the hole the village's water leaves by.
    if let Some(paving) = derive::court_paving(
        [
            [SQUARE[0], SQUARE[1]],
            [SQUARE[2], SQUARE[1]],
            [SQUARE[2], SQUARE[3]],
            [SQUARE[0], SQUARE[3]],
        ],
        0.02,
        0.12,
        GRATE,
        0.40,
    ) {
        skeleton.slabs.push(paving);
    }
    Building::new(skeleton)
}
