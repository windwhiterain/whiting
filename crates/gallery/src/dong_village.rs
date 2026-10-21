//! A **侗寨**: a Dong village around its drum tower — the tower in the middle of a paved square, the timber houses round it, and the
//! lane that runs past both.
//!
//! # What this scene is for
//!
//! The gallery's **seventh settlement**, and the first built around a **single vertical** object. Every other settlement here is a row:
//! the 傣寨 is a line of houses on a well, the 屯堡石板街 is two rows facing each other, the 白族巷 is three courts on a lane. A 侗寨 is
//! not a row — it is a **square with a tower on it**, and the houses stand round the square.
//!
//! # What it composes
//!
//! - **`drum_tower::document()` at the origin**, the settlement's one purpose-built object;
//! - **three `chuandou::document()`s round it** — a Dong house is a 穿斗 timber frame, which is the type this gallery already has — each
//!   turned to face the tower;
//! - **the 鼓楼坪: the square**, paved, because the space round a drum tower is a floor and not a road;
//! - **the lane** running past the square's south side, laid in flags.
//!
//! # What it does not draw
//!
//! - **the 风雨桥**: a Dong village's covered bridge is a type of its own and the gallery has one (`rain_bridge`); putting it in this
//!   scene would need a river, which is the landscape arc's work and not this scene's;
//! - **the 禾晾**: the rice-drying racks that stand at a Dong village's edge are open frames, and this crate's members can draw them —
//!   but a frame of five members with nothing on it reads as scaffolding, so they wait for the harvest-arc's own decision.

use building::{Building, BuildingSkeleton, ops};
use definition::Value;

/// The square: its half width either way, and how far the tower's plinth stands inside it.
const SQUARE: f32 = 13.0;
/// The lane: half its width, and how far it runs.
const LANE_HALF: f32 = 3.0;
const LANE_FROM: f32 = -30.0;
const LANE_TO: f32 = 30.0;
/// How many flags the lane is laid in, and the joint between them.
const LANE_FLAGS: usize = 20;
const JOINT: f32 = 0.05;
const PAVING: f32 = 0.07;
/// The square's own paving: a lower, wider slab the whole village stands on.
const SQUARE_PAVING: f32 = 0.05;
/// The lane's south edge is the square's north; this is where the lane's centre line stands.
const LANE_Z: f32 = -SQUARE - LANE_HALF + 1.2;

/// The houses: where each stands and which way it is turned, so its porch faces the tower.
const HOUSES: [(f32, f32, f32); 3] = [
    (-17.5, 4.5, core::f32::consts::FRAC_PI_2),
    (17.5, 4.5, -core::f32::consts::FRAC_PI_2),
    (-2.0, -17.0, 0.0),
];

const SUN: [f64; 3] = [-26.0, 30.0, 20.0];
const SUN_IRRADIANCE: f64 = 3.3;
const SHADOW_EXTENT: f64 = 52.0;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the village: from the square's south-west corner, so the tower rises against the sky and the houses flank it.
pub fn dong_village() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        // **Backed off to take the square in**: a tower in the middle of a village is a picture of the village, not of the tower.
        [-34.0, 13.5, -36.0],
        [0.0, 7.5, 0.0],
        FIELD_OF_VIEW,
        "dong_village",
    )
}

/// Returns the document: the tower, three houses, the square and the lane.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The drum tower at the origin**, which is what the village is arranged around.
    let mut tower = crate::drum_tower::document().skeleton;
    ops::translate(&mut tower, [0.0, 0.0]);
    assert!(
        ops::merge(&mut skeleton, tower),
        "the tower places no parts, so the merge always happens"
    );

    // **Three houses round it**, each turned so its porch faces the square.
    for (across, along, turn) in HOUSES {
        let mut part = crate::chuandou::document().skeleton;
        if turn.abs() > 1.0e-3 {
            ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        ops::translate(&mut part, [across, along]);
        assert!(
            ops::merge(&mut skeleton, part),
            "a house of the village places no parts, so the merge always happens"
        );
    }

    // **The 鼓楼坪: the square's paving.** The space round a drum tower is a floor — it is where the village meets, sings and dances —
    // and not a road, which is why it is paved as one slab and the lane is laid in flags.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-SQUARE, -SQUARE],
            [SQUARE, -SQUARE],
            [SQUARE, SQUARE],
            [-SQUARE, SQUARE],
        ]),
        0.0,
        SQUARE_PAVING,
    ));

    // **The lane, in flags**, running east and west past the square's south side.
    let run = (LANE_TO - LANE_FROM) / LANE_FLAGS as f32;
    for flag in 0..LANE_FLAGS {
        let from = LANE_FROM + run * flag as f32 + JOINT * 0.5;
        let to = LANE_FROM + run * (flag + 1) as f32 - JOINT * 0.5;
        skeleton.slabs.push(building::Slab::paving(
            ops::wound(vec![
                [from, LANE_Z - LANE_HALF],
                [to, LANE_Z - LANE_HALF],
                [to, LANE_Z + LANE_HALF],
                [from, LANE_Z + LANE_HALF],
            ]),
            0.0,
            PAVING,
        ));
    }

    Building::new(skeleton)
}
