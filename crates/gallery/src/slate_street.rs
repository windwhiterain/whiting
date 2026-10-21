//! A **屯堡石板街**: a street of slate houses — two rows facing each other across a flagged lane, and its own well.
//!
//! # What this scene is for
//!
//! The gallery's **sixth settlement**, and the first that is a **street** and not a compound. The 屯堡 village is a garrison's
//! yard with a wall and a 朝门; a 屯堡 *street* is what a growing one becomes: houses on both sides of a flagged lane, each
//! turning its 院坝 onto it, and the lane itself the village's front room.
//!
//! # What it composes
//!
//! - **six `tunpu::document()`s in two rows**, the north row turned a half so both rows face the lane. A 石板房's yard is at
//!   `+z`, so the south row takes no turn and the north row does — one fact about which way a yard looks, exactly as the 白族
//!   lane's gates are placed by one fact about which way a gate looks;
//! - **the lane, laid in flags** across its whole length, because a 屯堡's street is the same stone as its houses;
//! - **a wall at each end** with the lane's own width left open, so the street has two mouths and no third.
//!
//! # What it does not draw
//!
//! - **the gate towers**: a 屯堡 street's 碉楼 stands at the village's corner and not on its street, and the compound scene has
//!   it;
//! - **the culverts** the channel would empty into: a slab drains to its lowest edge and this crate has no word for a mouth.

use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, ops};
use definition::Value;

/// The plots: three to a row, and how far each row stands off the lane's centre line.
const PLOTS: [f32; 3] = [-11.0, 0.0, 11.0];
const ROW: f32 = 9.40;

/// The lane: half its width, and how far it runs past the last house.
const LANE_HALF: f32 = 3.10;
const LANE_FROM: f32 = -26.0;
const LANE_TO: f32 = 26.0;
/// How many flags it is laid in, the joint between them, and how thick a flag is.
const LANE_FLAGS: usize = 18;
const JOINT: f32 = 0.05;
const FLAG: f32 = 0.07;
/// The channel down the lane`s middle: half its width, and how deep it stands below the walked strips.
const CHANNEL: f32 = 0.34;
const GUTTER: f32 = 0.015;

/// The walls that close the street's two ends: their height and thickness, and the gap the lane leaves open.
const END_HALF: f32 = 0.45;
const END_HIGH: f32 = 2.30;
const END_THICK: f32 = 0.36;
const MOUTH: f32 = 3.30;

const SUN: [f64; 3] = [-26.0, 30.0, 18.0];
const SUN_IRRADIANCE: f64 = 3.3;
const SHADOW_EXTENT: f64 = 52.0;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the street: from its south mouth, looking north between the two rows.
pub fn slate_street() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        // **Inside the street, under the flags`, looking north between the two rows**: from outside its mouth the end wall is the
        // whole foreground, and what the scene is a picture of is the lane between the houses.
        [-22.0, 3.0, 0.6],
        [6.0, 2.0, -0.6],
        FIELD_OF_VIEW,
        "slate_street",
    )
}

/// Returns the document: six houses, the flags between them, and the two end walls.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    // **Two rows facing each other.** A 石板房's yard is at `+z`, so the south row is placed as it is built and the north row is
    // turned a half — the same answer the 白族 lane gave, where the turn is what puts a gate on the lane.
    for (along, turn) in [(-ROW, 0.0_f32), (ROW, core::f32::consts::PI)] {
        for across in PLOTS {
            let mut part = crate::tunpu::document().skeleton;
            if turn.abs() > 1.0e-3 {
                ops::rotate(&mut part, [0.0, 0.0], turn);
            }
            ops::translate(&mut part, [across, along]);
            assert!(
                ops::merge(&mut skeleton, part),
                "a house of the street places no parts, so the merge always happens"
            );
        }
    }

    // **The lane, laid in flags with a channel down its middle.** One slab from mouth to mouth reads as concrete; the joint is
    // where a mason's day ended, and the **channel** is where the water goes. Each flag is three bands: two walked strips at
    // `FLAG` and the gutter between them at `GUTTER`, a hand's depth lower — which is how a stone street drains without a pipe.
    let run = (LANE_TO - LANE_FROM) / LANE_FLAGS as f32;
    for flag in 0..LANE_FLAGS {
        let from = LANE_FROM + run * flag as f32 + JOINT * 0.5;
        let to = LANE_FROM + run * (flag + 1) as f32 - JOINT * 0.5;
        for (near, far, top) in [
            (-LANE_HALF, -CHANNEL, FLAG),
            (-CHANNEL, CHANNEL, GUTTER),
            (CHANNEL, LANE_HALF, FLAG),
        ] {
            skeleton.slabs.push(building::Slab::paving(
                ops::wound(vec![[from, near], [to, near], [to, far], [from, far]]),
                0.0,
                top,
            ));
        }
    }

    // **A wall at each end, with the lane's width left open.** A street with two mouths and no third is a street a garrison
    // could close, which is what a 屯堡 is for; the gap is the lane's own width plus a hand's breadth either side.
    for x in [LANE_FROM + END_THICK, LANE_TO - END_THICK] {
        let lengths = [
            (-LANE_HALF - 9.0, -MOUTH * 0.5),
            (MOUTH * 0.5, LANE_HALF + 9.0),
        ];
        for (from, to) in lengths {
            let mut wall = Mass::new(
                ops::wound(vec![
                    [x - END_HALF, from],
                    [x + END_HALF, from],
                    [x + END_HALF, to],
                    [x - END_HALF, to],
                ])
                .into_iter()
                .map(|at| PlanNode::new(at, 0.0))
                .collect(),
                END_HIGH,
            );
            wall.wall = END_THICK;
            wall.slab = 0.22;
            skeleton.masses.push(wall);
        }
    }
    let _ = MassId(0);

    Building::new(skeleton)
}
