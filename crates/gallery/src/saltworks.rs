//! A **盐井与盐灶**: the salt works of a Yunnan brine village — a wellhead with its windlass, a row of pans over their fires, and an open shed over both —
//! the twenty-second type.
//!
//! # What this scene is for
//!
//! 滇南's villages were built on brine: at 黑井 and 盐津 the well is the reason for the settlement, and the works round it are **a well, a boiling house
//! and a store**, because salt is not grown and not quarried — it is **drawn and boiled**, and the two halves of that are two buildings. It is the second
//! type here whose subject is a **process** (the 水碾房 is the first), and it is the only one whose product is not for the household that makes it.
//!
//! # How it is built
//!
//! - **the wellhead**: four posts, a windlass beam across them and a small cap over it, because the brine comes up in a bucket and not in a jar;
//! - **the 灶**: a stone bench with `PANS` shallow pans set in it, which is what a boiling house is — **a fire and a floor**;
//! - **the 卤池**: the brine tank beside the well, a stone box the bucket is emptied into;
//! - **the shed**: posts and a cap over the bench, open on both long sides so the steam leaves.
//!
//! # What it does not draw
//!
//! - **the fire and the steam**: this crate draws solids and the two things that make a boiling house are neither;
//! - **the bucket and its rope**: a rope is a member and the bucket is a solid, and neither is worth a row in a plate whose subject is the works.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The wellhead: its four posts' plan and section, their height, and the windlass beam's height and section.
const WELL_HALF: f32 = 0.95;
const WELL_POST: f32 = 0.15;
const WELL_HIGH: f32 = 3.05;
const BEAM_HIGH: f32 = 2.55;
const BEAM: f32 = 0.18;
/// The windlass: its drum across the beam, the drum's radius and its two cranks' reach.
const DRUM: f32 = 0.24;
const CRANK: f32 = 0.42;
/// The 灶: how many pans, their plan and depth, the bench's own height, and where the first pan stands.
const PANS: usize = 3;
const PAN: f32 = 1.15;
const PAN_DEEP: f32 = 0.18;
const BENCH_HIGH: f32 = 0.85;
const PAN_FROM: f32 = 2.60;
const PAN_GAP: f32 = 1.55;
/// The 卤池: where it stands and its plan.
const TANK_AT: [f32; 2] = [-3.40, 0.0];
const TANK_HALF: f32 = 1.05;
const TANK_HIGH: f32 = 0.92;
/// The shed over the bench: its posts, their height, and its cap's oversail.
const SHED_POST: f32 = 0.16;
const SHED_HIGH: f32 = 3.35;
const SHED_EAVE: f32 = 0.62;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.60;

/// Builds the scene: from the bench's side, with the wellhead at the frame's left and the pans running away from the camera.
pub fn saltworks() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-8.4, 5.0, 11.6],
        [1.4, 2.2, -0.6],
        FIELD_OF_VIEW,
        "saltworks",
    )
}

/// Returns the document: the wellhead, the brine tank, the pan bench and the shed.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The wellhead**: four posts standing a windlass over the shaft.
    for x in [-WELL_HALF, WELL_HALF] {
        for z in [-WELL_HALF, WELL_HALF] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, 0.0, z] },
                building::Anchor::Free {
                    at: [x, WELL_HIGH, z],
                },
                [1.0, 0.0],
                WELL_POST,
                WELL_POST,
            ));
        }
    }
    // **The windlass beam** across the two pairs, and **the drum** on it.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-WELL_HALF, BEAM_HIGH, 0.0],
        },
        building::Anchor::Free {
            at: [WELL_HALF, BEAM_HIGH, 0.0],
        },
        [0.0, 1.0],
        BEAM,
        BEAM,
    ));
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [0.0, BEAM_HIGH - DRUM, 0.0],
        },
        building::Anchor::Free {
            at: [0.0, BEAM_HIGH + DRUM, 0.0],
        },
        [1.0, 0.0],
        DRUM * 2.0,
        DRUM * 2.0,
    ));
    // **The two cranks**, on the drum's two ends, which is how four men turn it.
    for side in [-1.0_f32, 1.0] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [side * (DRUM + 0.10), BEAM_HIGH, 0.0],
            },
            building::Anchor::Free {
                at: [side * (DRUM + 0.10), BEAM_HIGH - CRANK, 0.0],
            },
            [0.0, 1.0],
            BEAM * 0.6,
            BEAM * 0.6,
        ));
    }
    // **A small cap over the windlass**, because the rope is wet and the rain would spoil it.
    let cap = WELL_HALF + 0.42;
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![[-cap, -cap], [cap, -cap], [cap, cap], [-cap, cap]]),
        WELL_HIGH,
        WELL_HIGH + 0.14,
    ));

    // **The 卤池**: the stone tank the bucket is emptied into, on the well's far side.
    let mut tank = Mass::new(
        ops::wound(vec![
            [TANK_AT[0] - TANK_HALF, TANK_AT[1] - TANK_HALF],
            [TANK_AT[0] + TANK_HALF, TANK_AT[1] - TANK_HALF],
            [TANK_AT[0] + TANK_HALF, TANK_AT[1] + TANK_HALF],
            [TANK_AT[0] - TANK_HALF, TANK_AT[1] + TANK_HALF],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        TANK_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
    tank.wall = TANK_HALF * 0.45;
    tank.slab = 0.14;
    skeleton.masses.push(tank);

    // **The 灶**: one stone bench with the pans set in it, which is the boiling house's whole fabric.
    let bench_from = PAN_FROM - PAN_GAP * 0.5;
    let bench_to = PAN_FROM + PAN_GAP * (PANS as f32 - 0.5) + PAN_GAP * 0.5;
    let mut bench = Mass::new(
        ops::wound(vec![
            [bench_from, -PAN * 0.85],
            [bench_to, -PAN * 0.85],
            [bench_to, PAN * 0.85],
            [bench_from, PAN * 0.85],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        BENCH_HIGH,
    );
    bench.wall = PAN * 0.30;
    bench.slab = 0.16;
    skeleton.masses.push(bench);
    // **The pans themselves**: three shallow solids standing in the bench's own top, which is what a pan over a fire is.
    for pan in 0..PANS {
        let x = PAN_FROM + PAN_GAP * pan as f32;
        skeleton.slabs.push(building::Slab::roof(
            ops::wound(vec![
                [x - PAN * 0.5, -PAN * 0.5],
                [x + PAN * 0.5, -PAN * 0.5],
                [x + PAN * 0.5, PAN * 0.5],
                [x - PAN * 0.5, PAN * 0.5],
            ]),
            BENCH_HIGH,
            BENCH_HIGH + PAN_DEEP,
        ));
    }

    // **The shed** over the bench, open on both long sides so the steam leaves.
    for x in [bench_from - 0.30, bench_to + 0.30] {
        for z in [-PAN * 1.15, PAN * 1.15] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, 0.0, z] },
                building::Anchor::Free {
                    at: [x, SHED_HIGH, z],
                },
                [1.0, 0.0],
                SHED_POST,
                SHED_POST,
            ));
        }
    }
    let shed_x = (bench_to - bench_from) * 0.5 + SHED_EAVE;
    let shed_z = PAN * 1.15 + SHED_EAVE;
    let middle = (bench_from + bench_to) * 0.5;
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [middle - shed_x, -shed_z],
            [middle + shed_x, -shed_z],
            [middle + shed_x, shed_z],
            [middle - shed_x, shed_z],
        ]),
        SHED_HIGH,
        SHED_HIGH + 0.18,
    ));

    Building::new(skeleton)
}
