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

/// The steps between the benches: how many flights, where they start and how far apart, how wide each is, and how many treads a flight has.
const FLIGHTS: usize = 2;
const FLIGHT_FROM: f32 = -21.00;
const FLIGHT_GAP: f32 = 37.60;
const FLIGHT_HALF: f32 = 1.35;
const TREADS: usize = 4;

/// The 牲畜栏 beside the lower houses: where it stands, its plan, how many rails, the rail's section and height, a post's section and the gate's
/// half width.
const PEN_AT: [f32; 2] = [-20.40, -7.60];
const PEN: [f32; 2] = [3.80, 3.10];
const PEN_RAILS: usize = 3;
const PEN_RAIL: f32 = 0.11;
const PEN_HIGH: f32 = 1.05;
const PEN_POST: f32 = 0.15;
const PEN_GATE: f32 = 0.80;
/// The 柴棚 beside it: where it stands, its plan, the posts' height and how far its roof stands out.
const SHED_AT: [f32; 2] = [-20.40, -10.90];
const SHED: [f32; 2] = [2.70, 2.10];
const SHED_HIGH: f32 = 1.95;
const SHED_EAVE: f32 = 0.28;
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

/// The **close plate**: the retaining face, its flight of steps and a corn rack, at the distance a person walks past them.
///
/// # Why a second camera and not a second document
///
/// Round 185 put four treads on a `1.40 m` rise into a village composed for five houses, and said the frame could not show them. The document is
/// the same one; round 172 made this move for the 土掌房 village and it is the cheaper of the two answers every time.
pub fn lisu_steps() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-7.0, 2.6, -2.2],
        [-21.0, 1.0, 2.4],
        FIELD_OF_VIEW * 1.30,
        "lisu_steps",
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

    // **The steps up the retaining face.** Round 184 raised the upper bench by `RISE` and left no way onto it, which is the same gap round 175
    // found on the longhouse: a level with no route is a level nobody uses. Two flights of four treads each, one at either end of the lane.
    for flight in 0..FLIGHTS {
        let x = FLIGHT_FROM + FLIGHT_GAP * flight as f32;
        for tread in 0..TREADS {
            let along = tread as f32 / TREADS as f32;
            let z = UPPER_EDGE + RETAIN * along;
            let rise = RISE * along;
            skeleton.slabs.push(building::Slab::paving(
                ops::wound(vec![
                    [x - FLIGHT_HALF, z],
                    [x + FLIGHT_HALF, z],
                    [x + FLIGHT_HALF, z + RETAIN / TREADS as f32],
                    [x - FLIGHT_HALF, z + RETAIN / TREADS as f32],
                ]),
                rise,
                rise + PAVING,
            ));
        }
    }

    // **The 牲畜栏 and the 柴棚.** A 木楞房 household keeps its animals in a rail pen beside the house and its split wood under a small shed, and
    // both are **posts and rails** rather than walls: this is the 侗寨's 猪栏 in another country, and neither one is a building.
    let [pen_x, pen_z] = PEN_AT;
    for (x0, x1, z0, z1) in [
        (
            pen_x - PEN[0] * 0.5,
            pen_x - PEN[0] * 0.5 + PEN_RAIL,
            pen_z - PEN[1] * 0.5,
            pen_z + PEN[1] * 0.5,
        ),
        (
            pen_x + PEN[0] * 0.5 - PEN_RAIL,
            pen_x + PEN[0] * 0.5,
            pen_z - PEN[1] * 0.5,
            pen_z + PEN[1] * 0.5,
        ),
        (
            pen_x - PEN[0] * 0.5,
            pen_x - PEN_GATE,
            pen_z + PEN[1] * 0.5 - PEN_RAIL,
            pen_z + PEN[1] * 0.5,
        ),
        (
            pen_x + PEN_GATE,
            pen_x + PEN[0] * 0.5,
            pen_z + PEN[1] * 0.5 - PEN_RAIL,
            pen_z + PEN[1] * 0.5,
        ),
    ] {
        // **A rail is a member with both ends at the pen's own height**, so its `across` is the run turned a quarter (rounds 134, 141, 144).
        let along_x = (z1 - z0).abs() < 1.1e-3;
        for rail in 0..PEN_RAILS {
            let up = PAVING + PEN_HIGH * (rail as f32 + 1.0) / (PEN_RAILS + 1) as f32;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x0, up, z0] },
                building::Anchor::Free { at: [x1, up, z1] },
                if along_x { [0.0, 1.0] } else { [1.0, 0.0] },
                PEN_RAIL,
                PEN_RAIL,
            ));
        }
    }
    // **The posts at the pen's corners**, which is what makes the rails a fence and not four floating lines.
    for x in [pen_x - PEN[0] * 0.5, pen_x + PEN[0] * 0.5] {
        for z in [pen_z - PEN[1] * 0.5, pen_z + PEN[1] * 0.5] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, PAVING, z] },
                building::Anchor::Free {
                    at: [x, PAVING + PEN_HIGH + PEN_RAIL, z],
                },
                [1.0, 0.0],
                PEN_POST,
                PEN_POST,
            ));
        }
    }

    // **The 柴棚: four posts, a head frame and a slab roof**, open on every side — a shed for wood and not a room for people.
    let [shed_x, shed_z] = SHED_AT;
    for x in [shed_x - SHED[0] * 0.5, shed_x + SHED[0] * 0.5] {
        for z in [shed_z - SHED[1] * 0.5, shed_z + SHED[1] * 0.5] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, PAVING, z] },
                building::Anchor::Free {
                    at: [x, PAVING + SHED_HIGH, z],
                },
                [1.0, 0.0],
                PEN_POST,
                PEN_POST,
            ));
        }
    }
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [
                shed_x - SHED[0] * 0.5 - SHED_EAVE,
                shed_z - SHED[1] * 0.5 - SHED_EAVE,
            ],
            [
                shed_x + SHED[0] * 0.5 + SHED_EAVE,
                shed_z - SHED[1] * 0.5 - SHED_EAVE,
            ],
            [
                shed_x + SHED[0] * 0.5 + SHED_EAVE,
                shed_z + SHED[1] * 0.5 + SHED_EAVE,
            ],
            [
                shed_x - SHED[0] * 0.5 - SHED_EAVE,
                shed_z + SHED[1] * 0.5 + SHED_EAVE,
            ],
        ]),
        PAVING + SHED_HIGH,
        PAVING + SHED_HIGH + 0.13,
    ));

    Building::new(skeleton)
}
