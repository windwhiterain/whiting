//! A **傣族寨子**: a Dai hamlet of Xishuangbanna — a row of 竹楼 along a lane, each on its piles with its veranda and its
//! stair turned to the lane, and the well the row draws from.
//!
//! # What this scene is for
//!
//! The gallery's second settlement, and the one the tenth type belongs in. The first village is a mountain village of
//! walled houses round a square; a Dai hamlet is neither: it stands on flat ground in the monsoon valleys, it has no
//! square, and its houses are **one type repeated** rather than several types held together. What makes it a settlement
//! at all is the **lane** and the **well** — the two things a row of houses shares.
//!
//! # The two facts it is composed of
//!
//! - **One building, three times.** `bamboo_house::document()` is a whole house — floor, decks, piles, stair, roof — and
//!   the hamlet translates three copies of it and seats each on the ground under it. Nothing is re-authored.
//! - **The well is a mass.** A hamlet's well-head is a ring of stone and it has an inside, so it is drawn as a small
//!   mass with a thick wall rather than as four members: the crate has no ring primitive and a well with no opening is a
//!   block.
//!
//! # What it does not draw
//!
//! - **no vegetation**: the palms and the bamboo clumps a Dai hamlet lives among are the vegetation arc's work;
//! - **the houses do turn** (the two outer ones are a quarter turn, so every veranda faces the well — `ops::rotate` has been in the crate since the villages
//!   were built, and the bullet that said "a rotation in the skeleton, which this crate does not have" was written before it and never removed);
//! - **the 篱笆 and its gate are drawn** since round 219: a low hedge with a gap and a leaf in it, which is what turns three houses round a well into a
//!   **compound**.

use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, site};
use definition::Value;

/// The lane the houses stand along, and how far apart their plots are.
const LANE_Z: f32 = 0.0;
/// The plots, and how far each house is turned: the middle house faces the lane, the two outer ones a quarter turn
/// towards the well.
const HOUSES: [(f32, f32); 3] = [
    (-13.0, -core::f32::consts::FRAC_PI_2),
    (0.0, 0.0),
    (13.0, core::f32::consts::FRAC_PI_2),
];

/// The well: a stone ring two metres across, standing in the open ground in front of the row.
const WELL_AT: [f32; 2] = [0.0, 9.5];
const WELL_HALF: f32 = 1.0;
const WELL_WALL: f32 = 0.28;
const WELL_HIGH: f32 = 0.95;

/// The 篱笆 in front of the two side houses and the gate in the gap: where the hedge starts and ends across the hamlet, how far in front of the well it
/// runs, its depth and height, and its gate's post and its own height.
const HEDGE_FROM: f32 = 1.70;
const HEDGE_TO: f32 = 16.40;
const HEDGE_OFF: f32 = -1.40;
const HEDGE_DEEP: f32 = 0.52;
const HEDGE_HIGH: f32 = 0.95;
const GATE_POST: f32 = 0.11;
const HEDGE_GATE: f32 = 1.35;

/// The wat`s place in the hamlet: behind the row, so its roof rises over the houses` own.
const WAT_AT: [f32; 2] = [4.0, -21.0];

const SUN: [f64; 3] = [-22.0, 26.0, 18.0];
const SUN_IRRADIANCE: f64 = 3.2;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the hamlet: three 竹楼 along the lane, the well in front of them, and the bamboo clumps a Dai hamlet lives in.
pub fn hamlet() -> Value {
    let building = document();
    // **The clumps stand where a hamlet's do**: behind the row, along the lane's ends, and one by the well. The species is
    // grown once and placed at every seat, so five clumps cost one plant.
    let mut clumps = crate::settlement_plants::bamboo_clumps(
        &[
            [-17.5, 4.5],
            [-6.5, 7.5],
            [6.0, 8.0],
            [17.0, 5.0],
            [11.5, -7.0],
            [-14.0, -7.5],
        ],
        &|at| grade(at[0] as f64, at[1] as f64) as f32,
    );
    // **Three trees between the houses**, which is what a Dai hamlet keeps: a mango or a jackfruit by a stair with its
    // crown over the veranda. `derive::tree` puts them on a four-metre bole, and the two species merge into one extra.
    clumps.merge(crate::settlement_plants::trees(
        &[[-20.0, -3.0], [2.5, -10.5], [21.0, -2.0]],
        &|at: [f32; 2]| grade(at[0] as f64, at[1] as f64) as f32,
    ));
    crate::village::scene_of(
        &building,
        &clumps,
        &grade,
        [-8.0, 6.2, 26.0],
        [0.0, 3.2, 1.0],
        FIELD_OF_VIEW,
        "dai_hamlet",
    )
}

/// Returns the ground: the lane's own fall, which is what the houses are seated on.
fn grade(x: f64, z: f64) -> f64 {
    // **The lane falls along `x` and is level across it**, which is what a lane on a valley floor does: 4 % one way and
    // nothing the other, so the three plots step by a third of a metre each and their verandas stay level with each
    // other's.
    let _ = z;
    0.018 * (x + 12.0) - 0.02
}

/// The sun's own irradiance needs a distance, and the hamlet states it at its own scale.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Returns the document: three houses and the well.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    for (at, turn) in HOUSES {
        let mut part = crate::bamboo_house::document().skeleton;
        // **A house turns about its own plan centre and then takes its plot.** The two outer houses are turned a quarter
        // turn, so all three verandas face the well rather than the lane: a hamlet is a group round the water, and a row
        // that all faces one way is a street.
        if turn.abs() > 1.0e-3 {
            building::ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        building::ops::translate(&mut part, [at, LANE_Z]);
        // **Each house is seated on the ground under it**, and `site::posts` fills the gap the seat opens under its
        // deck: the same pair of calls the single-house scene makes, read once per house.
        let seated = site::seat(
            &mut part,
            &|at: building::math::Vec2| grade(at[0] as f64, at[1] as f64) as f32,
            0.0,
        );
        assert_eq!(seated.len(), 1, "a 竹楼 is one room");
        let stilt = site::posts(
            &mut part,
            &|at: building::math::Vec2| grade(at[0] as f64, at[1] as f64) as f32,
            1.6,
            0.17,
            0.06,
        );
        assert_eq!(stilt.refused, 0, "the ground answers under the veranda");
        assert!(
            building::ops::merge(&mut skeleton, part),
            "a house of the hamlet places no parts, so the merge always happens"
        );
    }

    // **The well-head: a mass with a wall thick enough to be a ring.** Its plan is two metres across and its wall is
    // `WELL_WALL`, so what the expansion draws is a stone curb with a hole one and a half metres wide in plan — which is
    // what a well looks like from the lane.
    let level = grade(WELL_AT[0] as f64, WELL_AT[1] as f64) as f32;
    let (across, along) = (WELL_AT[0], WELL_AT[1]);
    let mut well = Mass::new(
        [
            PlanNode::new([across - WELL_HALF, along - WELL_HALF], level),
            PlanNode::new([across + WELL_HALF, along - WELL_HALF], level),
            PlanNode::new([across + WELL_HALF, along + WELL_HALF], level),
            PlanNode::new([across - WELL_HALF, along + WELL_HALF], level),
        ]
        .to_vec(),
        level + WELL_HIGH,
    );
    well.wall = WELL_WALL;
    well.slab = 0.18;
    let well_id = MassId(skeleton.masses.len());
    skeleton.masses.push(well);
    let _ = well_id;

    // **The village's wat.** A Dai hamlet is not a row of houses: it is a row of houses **and a temple**, and the temple stands
    // behind the row so that its roof rises over the houses' — which is what a Dai village looks like from its lane. The whole
    // `dai_temple::document()` is translated into place and merged, nothing re-authored, the same answer the hamlet gave for its
    // three houses.
    let mut wat = crate::dai_temple::document().skeleton;
    building::ops::translate(&mut wat, WAT_AT);
    assert!(
        building::ops::merge(&mut skeleton, wat),
        "the wat places no parts, so the merge always happens"
    );

    // **The 篱笆 and its gate.** This file said a Dai house's garden is *"a hedge and a gate, and this catalogue has neither"* — the hedge is a low mass and
    // the gate is two posts and a leaf, so **it has both**. A low bamboo hedge now runs in front of the two side houses, stopping either side of the way
    // in, and the gap has a gate standing in it: the two things that turn a row of verandas looking at a well into a **compound**.
    for side in [-1.0_f32, 1.0] {
        let inner = side * HEDGE_FROM;
        let outer = side * HEDGE_TO;
        let mut hedge = Mass::new(
            building::ops::wound(vec![
                [inner.min(outer), WELL_AT[1] + HEDGE_OFF],
                [inner.max(outer), WELL_AT[1] + HEDGE_OFF],
                [inner.max(outer), WELL_AT[1] + HEDGE_OFF + HEDGE_DEEP],
                [inner.min(outer), WELL_AT[1] + HEDGE_OFF + HEDGE_DEEP],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, 0.0))
            .collect(),
            HEDGE_HIGH,
        );
        // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
        hedge.wall = HEDGE_DEEP * 0.30;
        hedge.slab = 0.12;
        skeleton.masses.push(hedge);
    }
    // **The gate in the gap**: two posts and a leaf swung open, in the same vocabulary as every other gate here (`derive::leaf`).
    for side in [-1.0_f32, 1.0] {
        let x = side * HEDGE_FROM;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, 0.0, WELL_AT[1] + HEDGE_OFF + HEDGE_DEEP * 0.5],
            },
            building::Anchor::Free {
                at: [x, HEDGE_GATE, WELL_AT[1] + HEDGE_OFF + HEDGE_DEEP * 0.5],
            },
            [1.0, 0.0],
            GATE_POST,
            GATE_POST,
        ));
    }
    for side in [-1.0_f32, 1.0] {
        if let Some(leaf) = building::derive::leaf(
            [
                side * HEDGE_FROM,
                0.06,
                WELL_AT[1] + HEDGE_OFF + HEDGE_DEEP * 0.5,
            ],
            HEDGE_FROM - 0.10,
            HEDGE_GATE - 0.20,
            0.06,
            [-side, 0.0],
        ) {
            skeleton.members.push(leaf);
        }
    }

    Building::new(skeleton)
}

/// Returns the scene's own report of the sun, which `scene_of` states through the village's constants.
pub fn sun() -> ([f64; 3], f64, f64) {
    (SUN, SUN_IRRADIANCE, distance_to_origin(SUN))
}
