//! A **傈僳木楞房聚落**: five log houses on a bench above the 怒江, with a lane between them — the first settlement from 滇西北.
//!
//! # What this scene is for
//!
//! The gallery's **eleventh settlement** and its first of the **log** family at settlement scale. A 木楞房 village is the simplest arrangement in
//! this whole gallery: square houses of stacked logs, each with its log ends crossing at the corners, set down in a row on whatever bench the
//! mountain has, with no courtyard, no gate and no wall — the houses **are** the village.
//!
//! # What it composes
//!
//! Five `log_house::document()`s in two rows either side of a lane, on one paved bench, plus the fence that keeps the animals off the lane.
//! **The type is unchanged and the round adds no part of its own** except the bench, the lane and the fence.
//!
//! # What it does not draw
//!
//! - **the slope**: a 怒江 village is on a bench cut into a hillside, and this one is flat. Round 169's `ops::raise` could step it, and the
//!   type does not need it yet — the bench is the first thing and the slope is the second;
//! - **the 苞谷架**: the corn racks standing in every 傈僳 yard, which are the 禾晾 of round 140 in another country.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The bench: how far the paved ground runs either side of the lane, and how thick it lies.
const BENCH_HALF: f32 = 22.0;
const BENCH_FROM: f32 = -24.0;
const BENCH_TO: f32 = 20.0;
const PAVING: f32 = 0.05;
/// The village's **slope**: the upper bench's front edge, the rise between the two benches, and how deep the retaining face is.
const UPPER_EDGE: f32 = 1.80;
const RISE: f32 = 1.40;
const RETAIN: f32 = 0.50;
/// The lane: its half width, which is the gap the two rows of houses face each other across.
const LANE_HALF: f32 = 3.60;
/// The five houses: where each stands and which way it is turned. The two rows face across the lane.
const HOUSES: [(f32, f32, f32); 5] = [
    (-13.0, -7.4, core::f32::consts::PI),
    (0.0, -7.4, core::f32::consts::PI),
    (-6.5, 7.4, 0.0),
    (7.0, 7.4, 0.0),
    (-19.0, 7.4, 0.0),
];
/// The fence along the lane's far side: how many posts, how far apart, their section, and the rail's height and depth.
const FENCE_POSTS: usize = 14;
const FENCE_GAP: f32 = 2.60;
const FENCE_POST: f32 = 0.12;
const FENCE_HIGH: f32 = 1.05;

/// The 苞谷架: how many racks, where the first stands, how far apart, how long each runs, its height and its post's section, and how many
/// sheaves hang from it and how long they are.
const RACKS: usize = 3;
const RACK_FROM: f32 = -15.6;
const RACK_GAP: f32 = 12.4;
const RACK_LONG: f32 = 3.40;
const RACK_Z: f32 = -4.30;
const RACK_HIGH: f32 = 2.85;
const RACK_POST: f32 = 0.13;
const RACK_SHEAVES: usize = 4;
const SHEAF_LONG: f32 = 1.20;
const SHEAF_WIDE: f32 = 0.26;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the scene: down the lane, so the two rows' crossed log ends and their low gables read as a street of houses.
pub fn lisu_village() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [28.0, 11.5, 26.0],
        [-1.0, 2.6, 0.0],
        FIELD_OF_VIEW,
        "lisu_village",
    )
}

/// Returns the document: one bench, five log houses, the lane's fence.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The bench.** A 木楞房 village is cut into a slope as one flat shelf, and the shelf is paved because a shelf of earth in the rain is mud.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [BENCH_FROM, -BENCH_HALF],
            [BENCH_TO, -BENCH_HALF],
            [BENCH_TO, UPPER_EDGE],
            [BENCH_FROM, UPPER_EDGE],
        ]),
        0.0,
        PAVING,
    ));
    // **And the second bench, one rise higher.** A 怒江 village is cut into a hillside in **steps**, and the upper row stands on the second —
    // which is what round 182 recorded as owed and round 169's `ops::raise` is for.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [BENCH_FROM, UPPER_EDGE + RETAIN],
            [BENCH_TO, UPPER_EDGE + RETAIN],
            [BENCH_TO, BENCH_HALF],
            [BENCH_FROM, BENCH_HALF],
        ]),
        RISE,
        RISE + PAVING,
    ));
    // **The retaining face between them**, drawn as a wall of the rise's height and no more.
    let mut face = Mass::new(
        ops::wound(vec![
            [BENCH_FROM, UPPER_EDGE],
            [BENCH_TO, UPPER_EDGE],
            [BENCH_TO, UPPER_EDGE + RETAIN],
            [BENCH_FROM, UPPER_EDGE + RETAIN],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        RISE + PAVING,
    );
    // **The ring's offset stays under half the wall's depth** (round 144).
    face.wall = RETAIN * 0.34;
    face.slab = 0.14;
    skeleton.masses.push(face);

    // **The five houses**, each the type's own document, turned so the two rows face across the lane, and each on its own bench's level.
    for (across, along, turn) in HOUSES {
        let mut part = crate::log_house::document().skeleton;
        if turn.abs() > 1.0e-3 {
            ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        ops::translate(&mut part, [across, along]);
        // **Up onto the upper bench**, which is the whole of the slope: one call, and the house arrives with its logs, its roof and its door.
        if along > 0.0 {
            ops::raise(&mut part, RISE);
        }
        assert!(
            ops::merge(&mut skeleton, part),
            "a log house places no parts, so the merge always happens"
        );
    }

    // **The fence along the lane's near side**, which keeps the animals out of the street: posts and one rail, and no wall — a 傈僳 village has
    // fences and not walls.
    for post in 0..FENCE_POSTS {
        let x = BENCH_FROM + 2.0 + FENCE_GAP * post as f32;
        if x > BENCH_TO - 2.0 {
            break;
        }
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, PAVING, -LANE_HALF],
            },
            building::Anchor::Free {
                at: [x, PAVING + FENCE_HIGH, -LANE_HALF],
            },
            [0.0, 1.0],
            FENCE_POST,
            FENCE_POST,
        ));
    }
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [
                BENCH_FROM + 2.0,
                PAVING + FENCE_HIGH - FENCE_POST,
                -LANE_HALF,
            ],
        },
        building::Anchor::Free {
            at: [BENCH_TO - 2.0, PAVING + FENCE_HIGH - FENCE_POST, -LANE_HALF],
        },
        [0.0, 1.0],
        FENCE_POST * 0.7,
        FENCE_POST * 0.8,
    ));

    // **The 苞谷架: the corn racks.** A 傈僳 yard's one standing object is the **rack the maize hangs on** — two posts and a head beam with the
    // husks tied on it in bunches — and it is the 侗寨's 禾晾 in another country (round 140). Three racks along the lane's near side, inside the
    // free band between the lane and the houses.
    for rack in 0..RACKS {
        let x = RACK_FROM + RACK_GAP * rack as f32;
        for side in [-1.0_f32, 1.0] {
            let across = x + side * RACK_LONG * 0.5;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [across, PAVING, RACK_Z],
                },
                building::Anchor::Free {
                    at: [across, PAVING + RACK_HIGH, RACK_Z],
                },
                [1.0, 0.0],
                RACK_POST,
                RACK_POST,
            ));
        }
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x - RACK_LONG * 0.5, PAVING + RACK_HIGH, RACK_Z],
            },
            building::Anchor::Free {
                at: [x + RACK_LONG * 0.5, PAVING + RACK_HIGH, RACK_Z],
            },
            [0.0, 1.0],
            RACK_POST * 0.8,
            RACK_POST * 0.9,
        ));
        // **The sheaves hang from the head beam**, which is what makes a rack read as a rack and not as a gateway.
        for sheaf in 0..RACK_SHEAVES {
            let at = x - RACK_LONG * 0.5 + RACK_LONG * (sheaf as f32 + 0.5) / RACK_SHEAVES as f32;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [at, PAVING + RACK_HIGH - 0.10, RACK_Z],
                },
                building::Anchor::Free {
                    at: [at, PAVING + RACK_HIGH - SHEAF_LONG, RACK_Z],
                },
                [1.0, 0.0],
                SHEAF_WIDE,
                SHEAF_WIDE,
            ));
        }
    }

    Building::new(skeleton)
}
