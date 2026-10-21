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

use building::{Building, BuildingSkeleton, Covering, Mass, PlanNode, derive, ops};
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

/// The 禾晾: how many racks, where the first stands, how far apart, the racks own length, its height and a posts section.
const RACKS: usize = 3;
const RACK_X: f32 = -11.20;
const RACK_FROM: f32 = -11.40;
const RACK_GAP: f32 = 5.40;
const RACK_LONG: f32 = 3.60;
const RACK_HIGH: f32 = 2.90;
const RACK_POST: f32 = 0.14;
/// How many sheaves hang from each rack, how wide one is and how far off the paving its foot hangs.
const RACK_SHEAVES: usize = 4;
const SHEAF_WIDE: f32 = 0.58;
const SHEAF_FOOT: f32 = 0.55;

/// The 寨门 over the lane: where it stands, its depth, how much it reaches past the lane either side, its wall and height, and its
/// roofs pitch.
const GATE_AT: f32 = LANE_TO - 3.20;
const GATE_DEEP: f32 = 2.40;
const GATE_WING: f32 = 1.60;
const GATE_WALL: f32 = 0.22;
const GATE_HIGH: f32 = 3.05;
const GATE_ROOF: f32 = 0.72;
const GATE_ROOF_RISE: f32 = 1.65;
/// The tiles the gate and the tower's neighbours wear.
const TILES: Covering = Covering {
    period: 0.19,
    fill: 0.72,
    height: 0.045,
};

/// The 水塘: where it lies in the square, its plan, the rims thickness and its height.
const POND_AT: [f32; 2] = [-15.80, -22.40];
const POND: [f32; 2] = [3.60, 2.60];
const POND_WALL: f32 = 0.30;
const POND_HIGH: f32 = 0.48;

/// The 石板小径: where it runs, half its width, and how many flags it is laid in.
const PATH_X: f32 = -9.40;
const PATH_HALF: f32 = 0.75;
const PATH_FLAGS: usize = 9;

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

    // **The 寨门: the village's own gate, where the lane leaves it.** A 侗寨 has a gate house over its lane — a small box with a
    // doorway and a roof, and a bench under the roof where the old men sit and watch who comes in. It is the same construction the
    // 本主庙's 山门 used: a mass with an opening cut in one face and a gable roof over it.
    // **Two stone wings and a lintel over them.** The doorway is the **gap between two masses** and not an opening cut in one: a wall
    // with a hole needs a grid, a bay and a mitre, and three attempts at that answered `OpeningOutsideWall`; two masses with a lintel
    // is the pattern rounds 89, 114 and 125 all landed on, and the gap it leaves is the door.
    let mut lintel = Mass::new(
        ops::wound(vec![
            [GATE_AT - GATE_DEEP * 0.5, LANE_Z - LANE_HALF - GATE_WING],
            [GATE_AT + GATE_DEEP * 0.5, LANE_Z - LANE_HALF - GATE_WING],
            [GATE_AT + GATE_DEEP * 0.5, LANE_Z + LANE_HALF + GATE_WING],
            [GATE_AT - GATE_DEEP * 0.5, LANE_Z + LANE_HALF + GATE_WING],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, GATE_HIGH))
        .collect(),
        GATE_HIGH + 0.55,
    );
    lintel.wall = 0.26;
    lintel.slab = 0.20;
    let gate_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(lintel);
    for across in [-1.0_f32, 1.0] {
        let mut wing = Mass::new(
            ops::wound(vec![
                [GATE_AT - GATE_DEEP * 0.5, LANE_Z + across * LANE_HALF],
                [GATE_AT + GATE_DEEP * 0.5, LANE_Z + across * LANE_HALF],
                [
                    GATE_AT + GATE_DEEP * 0.5,
                    LANE_Z + across * (LANE_HALF + GATE_WING),
                ],
                [
                    GATE_AT - GATE_DEEP * 0.5,
                    LANE_Z + across * (LANE_HALF + GATE_WING),
                ],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            GATE_HIGH,
        );
        wing.wall = GATE_WALL;
        wing.slab = 0.18;
        wing.attachment = Some(building::Attachment {
            host: gate_id,
            join: building::Join::Under,
        });
        skeleton.masses.push(wing);
    }
    // **Its roof**: a lean-to over the lintel, shedding into the village.
    let plan = [
        [
            GATE_AT - GATE_DEEP * 0.5 - 0.55,
            LANE_Z + LANE_HALF + GATE_WING + 0.45,
        ],
        [
            GATE_AT + GATE_DEEP * 0.5 + 0.55,
            LANE_Z + LANE_HALF + GATE_WING + 0.45,
        ],
        [
            GATE_AT + GATE_DEEP * 0.5 + 0.55,
            LANE_Z - LANE_HALF - GATE_WING - 0.45,
        ],
        [
            GATE_AT - GATE_DEEP * 0.5 - 0.55,
            LANE_Z - LANE_HALF - GATE_WING - 0.45,
        ],
    ];
    if let Some(roof) = derive::lean_to(
        gate_id,
        plan,
        GATE_HIGH + 0.55,
        GATE_HIGH + 0.55 + GATE_ROOF_RISE,
        0.14,
    ) {
        skeleton.roofs.push(roof.with_covering(TILES));
    }
    // **The bench**: a plank between the gate's walls, on the village side, where the old men sit.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [GATE_AT - GATE_DEEP * 0.5 - 0.42, 0.46, LANE_Z - LANE_HALF],
        },
        building::Anchor::Free {
            at: [GATE_AT - GATE_DEEP * 0.5 - 0.42, 0.46, LANE_Z + LANE_HALF],
        },
        [1.0, 0.0],
        0.32,
        0.07,
    ));

    // **The 水塘: the village pond, beside the lane where it leaves the square.** It stood in the square first and the plate could
    // not find it: a rim half a metre high is paving at this distance unless it is set against open ground, which is where it is. A drum tower's square has water in it — a pond with fish in
    // it, and the village's fire water — and it is a **mass whose wall is a rim**, the same answer the wat's 放生池 and the two lanes'
    // wells gave: the top of the rim is the water's level.
    let pond = POND_AT;
    let mut basin = Mass::new(
        ops::wound(vec![
            [pond[0] - POND[0] * 0.5, pond[1] - POND[1] * 0.5],
            [pond[0] + POND[0] * 0.5, pond[1] - POND[1] * 0.5],
            [pond[0] + POND[0] * 0.5, pond[1] + POND[1] * 0.5],
            [pond[0] - POND[0] * 0.5, pond[1] + POND[1] * 0.5],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        POND_HIGH,
    );
    basin.wall = POND_WALL;
    basin.slab = 0.18;
    skeleton.masses.push(basin);

    // **The 禾晾: the rice-drying racks at the square's east edge.** This module said they wait "because a frame with nothing on it
    // reads as scaffolding" — so they are drawn **loaded**: two posts and a head beam, and four bundles of rice hanging from the beam,
    // which is what the rack is for and what makes it a rack. The bundles are members too: a sheaf of rice is a panel, and it hangs
    // from the beam to a hand's width above the paving.
    for rack in 0..RACKS {
        let z = RACK_FROM + RACK_GAP * rack as f32;
        for x in [RACK_X, RACK_X + RACK_LONG] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, 0.0, z] },
                building::Anchor::Free {
                    at: [x, RACK_HIGH, z],
                },
                [0.0, 1.0],
                RACK_POST,
                RACK_POST,
            ));
        }
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [RACK_X, RACK_HIGH, z],
            },
            building::Anchor::Free {
                at: [RACK_X + RACK_LONG, RACK_HIGH, z],
            },
            [0.0, 1.0],
            RACK_POST * 0.8,
            RACK_POST * 0.9,
        ));
        // **The sheaves**, hung from the beam's own line and set a hand apart.
        for index in 0..RACK_SHEAVES {
            let x = RACK_X
                + RACK_POST
                + (RACK_LONG - RACK_POST * 2.0) * (index as f32 + 0.5) / RACK_SHEAVES as f32;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [x, RACK_HIGH - 0.06, z],
                },
                building::Anchor::Free {
                    at: [x, SHEAF_FOOT, z],
                },
                [1.0, 0.0],
                SHEAF_WIDE,
                0.11,
            ));
        }
    }

    // **The 石板小径: the flagged path from the lane to the racks.** The lane, the pond, the racks and the square are four pieces of
    // ground that a village walks between, and at the moment they are four pieces of ground a village walks across. The path is laid
    // in the lane's own flags — one run of them, from the lane's north edge up the square's west side to the racks' foot — so the
    // pieces are joined by the same stone.
    let from = LANE_Z + LANE_HALF;
    let to = RACK_FROM + RACK_GAP * (RACKS - 1) as f32;
    let run = (to - from) / PATH_FLAGS as f32;
    for flag in 0..PATH_FLAGS {
        let near = from + run * flag as f32 + JOINT * 0.5;
        let far = from + run * (flag + 1) as f32 - JOINT * 0.5;
        skeleton.slabs.push(building::Slab::paving(
            ops::wound(vec![
                [PATH_X - PATH_HALF, near],
                [PATH_X + PATH_HALF, near],
                [PATH_X + PATH_HALF, far],
                [PATH_X - PATH_HALF, far],
            ]),
            0.0,
            PAVING,
        ));
    }

    Building::new(skeleton)
}
