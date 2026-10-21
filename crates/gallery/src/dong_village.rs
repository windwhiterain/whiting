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
/// The 风雨桥 beyond the lane's east mouth: where it stands, so its span continues the lane past the square.
const BRIDGE_AT: [f32; 2] = [46.4, LANE_Z];

/// The 水碾房 below the bridge, on the stream's own line: its race runs in the stream's direction, which is the lane's.
const MILL_AT: [f32; 2] = [40.0, 12.0];

/// The 牌坊 on the square's own axis, where the lane enters it: its plane across z, its place in x.
const ARCH_AT: [f32; 2] = [-14.0, LANE_Z];

/// The 戏台 on the square's west side, turned to face the tower across it: where it stands.
const STAGE_AT: [f32; 2] = [0.0, 26.4];

/// The 禾仓群 at the village's edge: how many stores, where the first stands, how far apart, each one's plan, how high its floor is posted and how tall
/// the store is, a post's section and how far its cap stands out.
const STORES: usize = 3;
const STORE_FROM: f32 = -7.40;
const STORE_GAP: f32 = 7.40;
const STORE_Z: f32 = 17.20;
const STORE_LONG: f32 = 2.40;
const STORE_DEEP: f32 = 2.00;
const STORE_UP: f32 = 0.95;
const STORE_HIGH: f32 = 1.70;
const STORE_POST: f32 = 0.15;
const STORE_EAVE: f32 = 0.34;

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
const GATE_ROOF_RISE: f32 = 1.65;
/// The tiles the gate and the tower's neighbours wear.
/// The 寨墙 the gate sits in: how far it runs east, how thick it is and how high.
const WALL_RUN: f32 = 16.00;
const WALL_THICK: f32 = 0.42;
const WALL_HIGH_VILLAGE: f32 = 2.30;

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

/// The 禾仓: where it stands, its plan, how far its floor is off the ground, its height, a post and the rat-guard, and its roof.
const GRANARY_AT: [f32; 2] = [-11.60, -19.80];
const GRANARY: [f32; 2] = [2.70, 2.20];
const GRANARY_UP: f32 = 1.05;
const GRANARY_HIGH: f32 = 2.30;
const GRANARY_POST: f32 = 0.16;
const GUARD: f32 = 0.62;
const GRANARY_EAVE: f32 = 0.55;
const GRANARY_RISE: f32 = 1.15;

/// The 猪栏: where it stands, its plan, its height, how many bays its rail has and a rails section.
const PEN_AT: [f32; 2] = [-16.40, -19.40];
const PEN: [f32; 2] = [2.90, 2.10];
const PEN_HIGH: f32 = 1.45;
const PEN_BAYS: usize = 4;
const PEN_RAIL: f32 = 0.10;

/// The 水井: where it stands, the rims half width, its wall and its height, and the 井台 under it.
const WELL_AT: [f32; 2] = [-19.40, -18.60];
const WELL_HALF: f32 = 0.72;
const WELL_WALL: f32 = 0.24;
const WELL_HIGH: f32 = 0.85;
const WELL_PLINTH: f32 = 0.24;

/// The 旗杆石 in front of the tower: how far either side of the axis, how far down the square, its plan and its foot.
const FLAG_STONE_AT: f32 = 2.30;
const FLAG_STONE_Z: f32 = -5.60;
const FLAG_STONE: f32 = 0.56;
const FLAG_STONE_FOOT: f32 = 0.88;
/// The poles the stones hold: how high they stand and their section.
const FLAG_POLE_HIGH: f32 = 5.60;
const FLAG_POLE: f32 = 0.13;

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

/// The **close plate**: the 鼓楼坪 at the tower's foot, where the tower's own door, its flag stones and their poles, the 禾晾 racks and the lane's
/// gate stand within twenty metres of one another.
///
/// # Why a second camera and not a second document
///
/// The 侗寨 is the gallery's **largest scene** — eight and a half thousand rows — and until now it had **one plate**, taken from thirty-four metres
/// out and fifteen up, where nothing smaller than a house is a shape. Round 172 made this move for the 土掌房 village; this is the fourth, and it is
/// the one the biggest scene needed most.
pub fn dong_square() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [17.0, 6.5, 17.0],
        [-1.5, 6.0, -1.5],
        FIELD_OF_VIEW * 0.92,
        "dong_square",
    )
}

/// The **water plate**: the mill, its race and the village behind it, from three metres off the ground.
///
/// # Why a third camera and not a third document
///
/// Rounds 213 and 222 brought the bridge and the mill into this village, and **both are invisible in its two plates** — one is a wide view where nothing
/// smaller than a house is a shape, and the other looks at the square, which is on the far side of the lane. **A 侗 village's water works are its oldest
/// public work and its quietest**, and this is the plate that shows them: the wheel standing in its race with the lantern tower over the roofs behind.
pub fn dong_water() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [31.4, 3.6, 22.2],
        [40.4, 2.4, 11.6],
        FIELD_OF_VIEW * 0.74,
        "dong_water",
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
    // **The 寨墙: the wall the gate is set into.** A gate that stands alone is an arch; a gate is the one place a village wall is
    // broken, and the wall has to run out of it on both sides. Each length starts exactly at its wing's face — an edge shared and no
    // area, the rule the 屯堡's walls paid for in round 113 — and runs east to the edge of the scene.
    for across in [-1.0_f32, 1.0] {
        let centre = LANE_Z + across * (LANE_HALF + GATE_WING * 0.5);
        let mut wall = Mass::new(
            ops::wound(vec![
                [GATE_AT + GATE_DEEP * 0.5, centre - WALL_THICK * 0.5],
                [
                    GATE_AT + GATE_DEEP * 0.5 + WALL_RUN,
                    centre - WALL_THICK * 0.5,
                ],
                [
                    GATE_AT + GATE_DEEP * 0.5 + WALL_RUN,
                    centre + WALL_THICK * 0.5,
                ],
                [GATE_AT + GATE_DEEP * 0.5, centre + WALL_THICK * 0.5],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            WALL_HIGH_VILLAGE,
        );
        // **The ring's offset stays under half the plan's narrowest span.** `WALL_THICK` is the wall's *depth*; passing it as the
        // offset folds the inner loop — `OffsetEdgeFolded` and `OffsetInverted { area: -6.37 }` — which is the rule round 74 wrote down.
        wall.wall = WALL_THICK * 0.38;
        wall.slab = 0.16;
        skeleton.masses.push(wall);
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

    // **The 禾仓: the granary.** A Dong village keeps its rice in a small box on **stilts** — off the ground, so the damp and the rats
    // are both under it — and the stilts carry a **rat-guard**: a flat plate at the post's head that a rat cannot pass. The box is a
    // mass and the four posts are members, which is the same division the 竹楼 makes between its frame and its room.
    let [bin_x, bin_z] = GRANARY_AT;
    for across in [-1.0_f32, 1.0] {
        for along in [-1.0_f32, 1.0] {
            let x = bin_x + across * (GRANARY[0] * 0.5 - GRANARY_POST * 0.5);
            let z = bin_z + along * (GRANARY[1] * 0.5 - GRANARY_POST * 0.5);
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, 0.0, z] },
                building::Anchor::Free {
                    at: [x, GRANARY_UP, z],
                },
                [1.0, 0.0],
                GRANARY_POST,
                GRANARY_POST,
            ));
            // **The rat-guard**, a plate a hand wide at the post's head.
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [x, GRANARY_UP - 0.12, z],
                },
                building::Anchor::Free {
                    at: [x, GRANARY_UP - 0.04, z],
                },
                [1.0, 0.0],
                GUARD,
                GUARD,
            ));
        }
    }
    let mut bin = Mass::new(
        ops::wound(vec![
            [bin_x - GRANARY[0] * 0.5, bin_z - GRANARY[1] * 0.5],
            [bin_x + GRANARY[0] * 0.5, bin_z - GRANARY[1] * 0.5],
            [bin_x + GRANARY[0] * 0.5, bin_z + GRANARY[1] * 0.5],
            [bin_x - GRANARY[0] * 0.5, bin_z + GRANARY[1] * 0.5],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, GRANARY_UP))
        .collect(),
        GRANARY_UP + GRANARY_HIGH,
    );
    bin.wall = GRANARY[0].min(GRANARY[1]) * 0.16;
    bin.slab = 0.16;
    let bin_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(bin);
    if let Some(roof) = derive::gable_roof_by(
        bin_id,
        &skeleton.masses[bin_id.0],
        GRANARY_EAVE,
        GRANARY_EAVE,
        GRANARY_RISE,
        0.06,
        0.13,
    ) {
        skeleton.roofs.push(roof.with_covering(TILES));
    }

    // **The 猪栏: the pig pen at the house's foot.** A 干栏 house keeps its animals under it, and the pen is the smallest building the
    // village makes: three low walls, a rail across the open side, and a lean-to over it. It is the second object at the granary's own
    // scale — between a house and a fence — and a village with only houses and ground has no such scale at all.
    let [pen_x, pen_z] = PEN_AT;
    let mut pen = Mass::new(
        ops::wound(vec![
            [pen_x - PEN[0] * 0.5, pen_z - PEN[1] * 0.5],
            [pen_x + PEN[0] * 0.5, pen_z - PEN[1] * 0.5],
            [pen_x + PEN[0] * 0.5, pen_z + PEN[1] * 0.5],
            [pen_x - PEN[0] * 0.5, pen_z + PEN[1] * 0.5],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        PEN_HIGH,
    );
    pen.wall = PEN[1] * 0.22;
    pen.slab = 0.14;
    let pen_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(pen);
    // **The lean-to** off the pen's high side, and **the rail** across its open one.
    let eave = [
        [pen_x - PEN[0] * 0.5 - 0.35, pen_z - PEN[1] * 0.5 - 0.30],
        [pen_x + PEN[0] * 0.5 + 0.35, pen_z - PEN[1] * 0.5 - 0.30],
        [pen_x + PEN[0] * 0.5 + 0.35, pen_z + PEN[1] * 0.5 + 0.30],
        [pen_x - PEN[0] * 0.5 - 0.35, pen_z + PEN[1] * 0.5 + 0.30],
    ];
    if let Some(roof) = derive::lean_to(pen_id, eave, PEN_HIGH - 0.10, PEN_HIGH + 0.42, 0.11) {
        skeleton.roofs.push(roof.with_covering(TILES));
    }
    let rail_z = pen_z + PEN[1] * 0.5 + 0.06;
    for index in 0..=PEN_BAYS {
        let x = pen_x - PEN[0] * 0.5 + PEN[0] * index as f32 / PEN_BAYS as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, 0.0, rail_z],
            },
            building::Anchor::Free {
                at: [x, PEN_HIGH * 0.72, rail_z],
            },
            [1.0, 0.0],
            PEN_RAIL,
            PEN_RAIL,
        ));
    }
    for high in [PEN_HIGH * 0.62, PEN_HIGH * 0.30] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [pen_x - PEN[0] * 0.5, high, rail_z],
            },
            building::Anchor::Free {
                at: [pen_x + PEN[0] * 0.5, high, rail_z],
            },
            [0.0, 1.0],
            PEN_RAIL * 0.7,
            PEN_RAIL * 0.9,
        ));
    }

    // **The 水井: the village well, the third in this project.** The 石板街's and the 白族巷's wells established the two parts — a rim a
    // mass whose wall is thick enough to be a ring, a 井台 under it, and a **windlass** of two posts, a beam and a drum above it — and
    // this is the same well in a third scene. **A vocabulary that works twice should be used a third time unchanged**, or it was never a
    // vocabulary.
    let [well_x, well_z] = WELL_AT;
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [well_x - WELL_HALF - 0.55, well_z - WELL_HALF - 0.55],
            [well_x + WELL_HALF + 0.55, well_z - WELL_HALF - 0.55],
            [well_x + WELL_HALF + 0.55, well_z + WELL_HALF + 0.55],
            [well_x - WELL_HALF - 0.55, well_z + WELL_HALF + 0.55],
        ]),
        0.0,
        WELL_PLINTH,
    ));
    let mut ring = Mass::new(
        ops::wound(vec![
            [well_x - WELL_HALF, well_z - WELL_HALF],
            [well_x + WELL_HALF, well_z - WELL_HALF],
            [well_x + WELL_HALF, well_z + WELL_HALF],
            [well_x - WELL_HALF, well_z + WELL_HALF],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, WELL_PLINTH))
        .collect(),
        WELL_PLINTH + WELL_HIGH,
    );
    ring.wall = WELL_WALL;
    ring.slab = 0.18;
    skeleton.masses.push(ring);
    // **The 辘轳**: two posts either side of the rim and a beam across them.
    for across in [-1.0_f32, 1.0] {
        let x = well_x + across * (WELL_HALF + 0.18);
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, WELL_PLINTH, well_z],
            },
            building::Anchor::Free {
                at: [x, WELL_PLINTH + WELL_HIGH + 0.95, well_z],
            },
            [1.0, 0.0],
            0.13,
            0.13,
        ));
    }
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [
                well_x - WELL_HALF - 0.18,
                WELL_PLINTH + WELL_HIGH + 0.95,
                well_z,
            ],
        },
        building::Anchor::Free {
            at: [
                well_x + WELL_HALF + 0.18,
                WELL_PLINTH + WELL_HIGH + 0.95,
                well_z,
            ],
        },
        [0.0, 1.0],
        0.11,
        0.15,
    ));

    // **The 旗杆石: the two stones in front of the tower.** Round 105 gave the 本主庙 a pair — a stone with a foot and a slot, where a
    // banner pole is stepped — and a 侗寨's 鼓楼坪 has the same pair for the same purpose: the village's own banners go up beside the
    // tower on the days it dances. **The same object, its second scene**, drawn identically: a mass `FLAG_STONE` square with a foot.
    for across in [-1.0_f32, 1.0] {
        let stone_x = across * FLAG_STONE_AT;
        let mut stone = Mass::new(
            ops::wound(vec![
                [stone_x - FLAG_STONE * 0.5, FLAG_STONE_Z - FLAG_STONE * 0.5],
                [stone_x + FLAG_STONE * 0.5, FLAG_STONE_Z - FLAG_STONE * 0.5],
                [stone_x + FLAG_STONE * 0.5, FLAG_STONE_Z + FLAG_STONE * 0.5],
                [stone_x - FLAG_STONE * 0.5, FLAG_STONE_Z + FLAG_STONE * 0.5],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            FLAG_STONE_FOOT,
        );
        stone.wall = FLAG_STONE * 0.34;
        stone.slab = 0.12;
        skeleton.masses.push(stone);
        // **The pole the stone is for.** A stone with a slot and no pole is a bollard: the pair only reads as 旗杆石 with the poles in
        // them, and at this distance the poles are what the plate can see at all.
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [stone_x, FLAG_STONE_FOOT, FLAG_STONE_Z],
            },
            building::Anchor::Free {
                at: [stone_x, FLAG_POLE_HIGH, FLAG_STONE_Z],
            },
            [1.0, 0.0],
            FLAG_POLE,
            FLAG_POLE,
        ));
    }

    // **The 禾仓群: the rice stores at the village's edge.** A 侗 village keeps its grain **out of the houses and off the ground**: a row of small stores
    // on posts, each with its own stone cap, standing together where the village meets its fields — a village within a village, and the one thing this
    // scene has lacked since round 145. Four posts, a boxed store and a cap, three times.
    for store in 0..STORES {
        let x = STORE_FROM + STORE_GAP * store as f32;
        for side in [-1.0_f32, 1.0] {
            for along in [-1.0_f32, 1.0] {
                skeleton.members.push(building::Member::new(
                    building::Anchor::Free {
                        at: [
                            x + side * STORE_LONG * 0.36,
                            PAVING,
                            STORE_Z + along * STORE_DEEP * 0.36,
                        ],
                    },
                    building::Anchor::Free {
                        at: [
                            x + side * STORE_LONG * 0.36,
                            PAVING + STORE_UP,
                            STORE_Z + along * STORE_DEEP * 0.36,
                        ],
                    },
                    [1.0, 0.0],
                    STORE_POST,
                    STORE_POST,
                ));
            }
        }
        let mut body = Mass::new(
            ops::wound(vec![
                [x - STORE_LONG * 0.5, STORE_Z + STORE_DEEP * 0.5],
                [x + STORE_LONG * 0.5, STORE_Z + STORE_DEEP * 0.5],
                [x + STORE_LONG * 0.5, STORE_Z - STORE_DEEP * 0.5],
                [x - STORE_LONG * 0.5, STORE_Z - STORE_DEEP * 0.5],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, PAVING + STORE_UP))
            .collect(),
            PAVING + STORE_UP + STORE_HIGH,
        );
        // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
        body.wall = STORE_DEEP * 0.26;
        body.slab = 0.14;
        skeleton.masses.push(body);
        let cap = STORE_HIGH;
        let _ = cap;
        skeleton.slabs.push(building::Slab::roof(
            ops::wound(vec![
                [
                    x - STORE_LONG * 0.5 - STORE_EAVE,
                    STORE_Z + STORE_DEEP * 0.5 + STORE_EAVE,
                ],
                [
                    x + STORE_LONG * 0.5 + STORE_EAVE,
                    STORE_Z + STORE_DEEP * 0.5 + STORE_EAVE,
                ],
                [
                    x + STORE_LONG * 0.5 + STORE_EAVE,
                    STORE_Z - STORE_DEEP * 0.5 - STORE_EAVE,
                ],
                [
                    x - STORE_LONG * 0.5 - STORE_EAVE,
                    STORE_Z - STORE_DEEP * 0.5 - STORE_EAVE,
                ],
            ]),
            PAVING + STORE_UP + STORE_HIGH,
            PAVING + STORE_UP + STORE_HIGH + 0.12,
        ));
    }

    // **The 风雨桥 at the lane's east mouth.** A 侗 village's bridge is not a monument at the edge of a village: it **is** the road in — its gallery is
    // the last covered stretch of the lane before the houses, and its pavilion is where a traveller waits out the rain. The bridge's own document,
    // translated so its span continues the lane beyond the square, and **not one of its numbers restated**.
    let mut bridge = crate::rain_bridge::document().skeleton;
    ops::translate(&mut bridge, BRIDGE_AT);
    assert!(
        ops::merge(&mut skeleton, bridge),
        "the bridge places no parts, so the merge always happens"
    );

    // **The 水碾房 beyond the bridge, on the same water.** Round 221 built the mill and left it standing in a field; a 侗 village's water works are a
    // **group** — the bridge carries the lane over the stream and the mill takes the stream's fall below it, so the two stand on one line and the mill's
    // race runs in the stream's own direction. Its own document, translated, with none of its numbers restated.
    let mut mill = crate::watermill::document().skeleton;
    ops::translate(&mut mill, MILL_AT);
    assert!(
        ops::merge(&mut skeleton, mill),
        "the mill places no parts, so the merge always happens"
    );

    // **The 牌坊 where the lane enters the square.** A 侗 village`s memorial arch stands **on the way in**, and its square is the one place in this gallery wide
    // enough for one: the lane is three metres of flags but the square is twenty-six across, and four posts on their stone drums need the room. Round 233`s
    // own document, turned a quarter to face down the lane and translated, with none of its numbers restated.
    let mut arch = crate::paifang::document().skeleton;
    ops::rotate(&mut arch, [0.0, 0.0], core::f32::consts::FRAC_PI_2);
    ops::translate(&mut arch, ARCH_AT);
    assert!(
        ops::merge(&mut skeleton, arch),
        "the arch places no parts, so the merge always happens"
    );

    // **The 戏台 on the square`s west side, facing the tower.** 侗 opera is played on a stage at the drum tower`s own square — the tower is where the village
    // gathers and the stage is what it gathers to watch — so the stage stands **outside the paving and looks across it**, turned a quarter. Round 235`s own
    // document, translated and turned, with none of its numbers restated.
    let mut stage = crate::opera_stage::document().skeleton;
    ops::translate(&mut stage, STAGE_AT);
    assert!(
        ops::merge(&mut skeleton, stage),
        "the stage places no parts, so the merge always happens"
    );

    Building::new(skeleton)
}
