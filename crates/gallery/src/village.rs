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

use building::math::Vec2;
use building::site;
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
/// How far south the lane runs from the square's own edge, in metres of plan.
const LANE_END: f32 = 15.0;

/// How steeply the ground falls away outside the square, in metres of drop per metre, and how far it is
/// allowed to fall.
///
/// **The square is a terrace cut into a hillside**: its own plan is level, and the ground outside it falls away
/// on all four sides. That is what a village square is in this country — the one piece of flat ground a slope
/// can be made to give — and it is also what lets the four types stand at four different heights without any of
/// them being moved by hand.
const SLOPE: f64 = 0.12;
const DROP_LIMIT: f64 = 2.4;

/// Returns the ground's own elevation at a plan point.
///
/// # Invariants
///
/// **The square is the bottom of a lane that climbs east**, so the ground rises past the square's east edge at
/// the lane's own grade and falls away on the other three sides. The two halves meet at `x = SQUARE[2]`, where
/// both are `0.0`, which is what makes one function serve the lane's seating and the village's: the street is
/// authored with `0.055 * x` from its own origin, and this is the same line read from the square's edge.
fn grade(x: f64, z: f64) -> f64 {
    let climbing = LANE_SLOPE * (x - SQUARE[2] as f64).max(0.0);
    let beyond = [
        (SQUARE[0] as f64 - x).max(0.0),
        (z - SQUARE[3] as f64).max(0.0),
        (SQUARE[1] as f64 - z).max(0.0),
    ];
    let out = beyond.iter().copied().fold(0.0, f64::max);
    let falling = -(out * SLOPE).min(DROP_LIMIT);
    if climbing > 0.0 { climbing } else { falling }
}

/// The **lane**'s own grade, which is the street scene's own number: a straight climb east of `0.055` a metre.
/// The village reads it from the square's edge, so the lane's first plot stands exactly where the square ends.
const LANE_SLOPE: f64 = 0.055;

/// How far the terrace's kerb stands above the square's own level.
const KERB: f32 = 0.35;

/// Where the lane is joined on: the offset its own document is moved by, so its first frontage is the square's
/// east side.
const LANE_AT: [f32; 2] = [8.0, 0.0];
/// The fill: `PAD_SPACING` between posts, `PAD_SECTION` square, `PAD_EMBED` into the ground at the foot.
///
/// **A house pad on a slope is a cut and a fill**, and the fill is a forest of short posts rather than a wall:
/// the same construction the 吊脚楼 makes of its whole floor, at the scale of one corner of a house.
const PAD_SPACING: f32 = 1.6;
const PAD_SECTION: f32 = 0.16;
const PAD_EMBED: f32 = 0.03;

/// Where each building stands, as the offset its own document is moved by.
const TOWER_AT: [f32; 2] = [0.0, -13.0];
const EARTHEN_WEST: [f32; 2] = [-13.0, 0.0];
const EARTHEN_EAST: [f32; 2] = [-13.0, -13.0];
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
    // **The ground is the terrace's own grade**, not a level plane: the same function the houses were seated
    // on, so a house cannot float and the hill cannot disagree with the building standing on it.
    let ground = ground::mesh(&building, GROUND_DIVISIONS, GROUND_EXTENT, grade);

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
    let ground = |at: Vec2| grade(at[0] as f64, at[1] as f64) as f32;
    // **The tower stands on the terrace and the houses stand on the hill.** A seat moves every mass's nodes to
    // the ground under them, which is exactly what a house needs and exactly what a tower of stacked storeys
    // must not have: seating the tower would flatten its seven floors onto one elevation. So the tower is
    // translated and merged as it is, and the four houses are seated on their own ground before they are merged
    // — which is the same `site::seat` every single-building scene calls, read once per building.
    for (mut part, at, seated) in [
        (crate::drum_tower::document().skeleton, TOWER_AT, false),
        (crate::earthen::document().skeleton, EARTHEN_WEST, true),
        (crate::earthen::document().skeleton, EARTHEN_EAST, true),
        // **The lane is the street scene, joined on.** Its document seats its own five plots on its own grade,
        // and that grade is the one this village's `grade` continues past the square, so the join is a
        // continuation rather than two hills meeting: the street's `0.055 * x` from its own origin is this
        // document's `0.055 * (x - 8)` from the square's edge.
        (crate::street::document().skeleton, LANE_AT, false),
        (crate::log_house::document().skeleton, LOG_WEST, true),
        (crate::log_house::document().skeleton, LOG_EAST, true),
    ] {
        ops::translate(&mut part, at);
        if seated {
            let report = site::seat(&mut part, ground, 0.0);
            assert!(
                report.len() == part.masses.len(),
                "every mass of a house is seated on the ground this grade reads"
            );
            // **A house on a slope is cut in on one side and filled on the other**, and the fill is what stands
            // the house up: `site::posts` puts a post under the outline wherever the ground has fallen away
            // from the pad, which is the same op the 吊脚楼 uses for its whole floor. Seating the house on the
            // highest ground under it — which is what `seat` does — leaves the downhill side in the air, and a
            // house standing on nothing is a house drawn halfway.
            //
            // **The posts are placed before the merge**, so their anchors resolve against the house's own mass
            // and not against a village that has five of them.
            let stilt = site::posts(&mut part, ground, PAD_SPACING, PAD_SECTION, PAD_EMBED);
            assert!(
                stilt.refused == 0,
                "the grade answers under every post position of every house"
            );
        }
        assert!(
            ops::merge(&mut skeleton, part),
            "a building of the village places no parts, so the merge always happens"
        );
    }
    // **The terrace is held up by a wall**, one run to a side, standing on the ground at the foot of each drop:
    // a plateau cut into a hill is a cut face, and the village's own edge is what a person on the square sees
    // when they look over it. The wall is the site's, not a mass's — it is not a building.
    for (from, to) in [
        (
            [SQUARE[0] - 0.3, SQUARE[1] - 0.3],
            [SQUARE[2] + 0.3, SQUARE[1] - 0.3],
        ),
        (
            [SQUARE[2] + 0.3, SQUARE[1] - 0.3],
            [SQUARE[2] + 0.3, SQUARE[3] + 0.3],
        ),
        (
            [SQUARE[2] + 0.3, SQUARE[3] + 0.3],
            [SQUARE[0] - 0.3, SQUARE[3] + 0.3],
        ),
        (
            [SQUARE[0] - 0.3, SQUARE[3] + 0.3],
            [SQUARE[0] - 0.3, SQUARE[1] - 0.3],
        ),
    ] {
        let foot = grade(from[0] as f64, from[1] as f64) as f32;
        skeleton.walls.push(building::wall::Wall::new(
            building::PlanNode::new([from[0], from[1]], foot),
            building::PlanNode::new([to[0], to[1]], foot),
            0.45,
            // **The wall stands a kerb above the terrace and not level with it**: its top is an absolute
            // elevation, and a top of `0.0` sat below the ground on the east side once the lane began to climb
            // there -- `WallTooLow { top: 0.0, highest: 0.0165 }`, measured.
            KERB,
        ));
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
        // **The lane steps down from the square.** The square opens south between the two log houses, and the
        // way out of it is a stone stair: one plate a step, each landing on the grade at its own run. A ramp is a
        // surface this crate cannot cut, and a stair is what a village on a hillside lays down at the end of its
        // square.
        let lane = 1.6;
        let step = 1.1;
        let mut z = SQUARE[3];
        while z < LANE_END {
            let level = grade(0.0, z as f64) as f32;
            skeleton.slabs.push(building::Slab::plate(
                vec![[-lane, z], [lane, z], [lane, z + step], [-lane, z + step]],
                level - 0.16,
                level + 0.10,
            ));
            z += step;
        }
    }
    Building::new(skeleton)
}
