//! A **白族巷**: three 三坊一照壁 courts on a lane that runs north and south, their gates all facing it.
//!
//! # What this scene is for
//!
//! The gallery's **fifth settlement**. The mountain village is walled houses round a square, the Dai hamlet is a row on a
//! valley floor, the plateau village is four towers on one shelf, the 屯堡 is a garrison's compound — a **白族 village** is
//! neither a row nor a compound: it is a *lane* with a courtyard house behind each gate, and the gates are what the lane is
//! for. In Dali a village's street is called a 巷 and its houses turn their one formal face onto it.
//!
//! # What it composes
//!
//! - **three `bai_court::document()`s**, one on each plot: two west of the lane facing east, one east of it facing west, so
//!   every gate opens onto the same strip of paving;
//! - **the lane itself**, as the paving the courts' own floors already are — a slab from one end to the other;
//! - **planting** along the lane from the bridge the settlements use, because a 白族 lane has its trees in it.
//!
//! # What it does not draw
//!
//! - **the lane's other end**: a village street is longer than three plots, and three is this scene's frame;
//! - **the village's 本主庙**, which is the settlement's public building and not a house.

use building::{BuildingSkeleton, ops};
use definition::Value;

/// The plots: two west of the lane, one east, and how far each is turned. **The gate of a court faces `+x`**, so the west
/// plots take no turn and the east plot is turned a half, which is what puts all three gates on the lane.
const PLOTS: [(f32, f32, f32); 3] = [
    (-14.0, -10.0, 0.0),
    (-14.0, 10.0, 0.0),
    (14.0, 0.0, core::f32::consts::PI),
];

/// The lane: how wide, and how far it runs past the plots.
const LANE_HALF: f32 = 3.20;
/// The 文笔塔 past the lane's north end: where it stands, measured from the lane's own origin.
const TOWER_AT: [f32; 2] = [8.0, 26.0];

const LANE_FROM: f32 = -21.0;
const LANE_TO: f32 = 21.0;
/// How many flags the lane is laid in, the joint between them, and how thick a flag is.
const LANE_SLABS: usize = 14;
const JOINT: f32 = 0.045;
const LANE_PAVING: f32 = 0.06;

/// The lane`s well: where it stands, its half width, its own wall and its curb`s height.
const WELL_AT: [f32; 2] = [0.0, -13.0];
const WELL_HALF: f32 = 0.72;
const WELL_WALL: f32 = 0.24;
const WELL_HIGH: f32 = 0.85;
/// The 井台 the curb stands on: its height, and how deep the step in front of it is.
const WELL_PLINTH: f32 = 0.24;
const WELL_STEP: f32 = 0.34;
/// The 辘轳 over the well: how high its posts stand above the curb, and the section of a post.
const WINDLASS_HIGH: f32 = 1.55;
const WINDLASS: f32 = 0.11;

/// The 巷门 where the lane leaves the village: the gate`s big plan over the lane, the two piers under it, and its roof.
///
/// **The spanning mass is the host and the piers are `Under` it.** `Join::Under` on a mass says: its top meets the host's
/// foot and its plan stands inside the host's — the mass *carries* the host. That is the only relation in this crate that a
/// mass over a gap can have, and it is why a gateway is a lintel with two piers under it and not two piers with a beam
/// between them.
const GATE_AT: f32 = -17.0;
const GATE_HALF: f32 = 4.40;
const GATE_DEEP: f32 = 0.55;
const GATE_FOOT: f32 = 3.20;
const GATE_RISE: f32 = 0.75;
const PIER_HALF: f32 = 0.50;
const GATE_WALL: f32 = 0.30;
const GATE_EAVE: f32 = 0.55;
/// The gate`s leaves and its board: how thick a leaf is, and how wide the 题额 is.
const BOARD: f32 = 0.08;
const BOARD_HALF: f32 = 0.95;

/// The 石敢当 at the lanes mouth: where it stands, its width, its thickness, its wall, its foot and its height.
const STELE_AT: [f32; 2] = [5.30, -18.30];
const STELE_HALF: f32 = 0.30;
const STELE_DEEP: f32 = 0.22;
const STELE_WALL: f32 = 0.08;
const STELE_FOOT: f32 = 0.14;
const STELE_HIGH: f32 = 1.12;

/// The 抱鼓石 at the gates piers: its plan and its height.
const DRUM: f32 = 0.46;
const DRUM_HIGH: f32 = 0.52;
/// **The drums stand DRUM_Z, clear of the lintels plan.** The validator compares plans and not volumes, so a stone under the
/// lintel is a stone *inside* it — measured as UnattachedOverlap { mass: 22, other: 25 } and { 22, 26 } while they stood on the
/// gates line. Set out in front, the same pair overlaps nothing and is where the plate sees it.
const DRUM_Z: f32 = -16.20;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the lane: from its south end, looking north between the gates.
pub fn bai_lane() -> Value {
    let plants = crate::settlement_plants::bamboo_clumps(
        &[
            [-6.0, -2.0],
            [-6.0, 6.0],
            [6.0, 8.0],
            [6.0, -6.0],
            [-6.5, 14.0],
        ],
        &|_: [f32; 2]| 0.0,
    );
    crate::village::scene_of(
        &document(),
        &plants,
        &|_, _| 0.0,
        [1.5, 4.2, -26.0],
        [0.0, 2.6, 2.0],
        FIELD_OF_VIEW,
        "bai_lane",
    )
}

/// The **close plate**: a court's gate from the lane, where its 题额, its four 门簪 and its two 抱鼓石 are the frame.
///
/// # Why a second camera and not a second document
///
/// Round 132's lane plate is a view down twenty-six metres of lane, and at that distance the 门簪 (round 136) and the 抱鼓石 (round 137) are three
/// pixels each — they were built to be looked at from the lane's own width. This is round 190's, 194's and 195's move for the fourth time.
pub fn bai_gate() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [10.2, 2.7, -23.4],
        [3.2, 2.0, -16.2],
        FIELD_OF_VIEW * 1.15,
        "bai_gate",
    )
}

/// Returns the document: three courts, and the lane's paving between them.
pub fn document() -> building::Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    for (across, along, turn) in PLOTS {
        let mut part = crate::bai_court::document().skeleton;
        if turn.abs() > 1.0e-3 {
            building::ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        building::ops::translate(&mut part, [across, along]);
        assert!(
            building::ops::merge(&mut skeleton, part),
            "a court of the lane places no parts, so the merge always happens"
        );
    }

    // **The lane is paving and not a mass**: it has no wall and nothing stands on it, so it is `Slab::paving` — the same answer
    // the courts' own floors gave. It is laid in **`LANE_SLABS` flags with a joint between them**, because a stone lane is a run
    // of flags and not one plate: one slab from end to end reads as concrete, and the joint is where the water goes.
    let run = (LANE_TO - LANE_FROM) / LANE_SLABS as f32;
    for slab in 0..LANE_SLABS {
        let from = LANE_FROM + run * slab as f32 + JOINT * 0.5;
        let to = LANE_FROM + run * (slab + 1) as f32 - JOINT * 0.5;
        skeleton.slabs.push(building::Slab::paving(
            building::ops::wound(vec![
                [-LANE_HALF, from],
                [LANE_HALF, from],
                [LANE_HALF, to],
                [-LANE_HALF, to],
            ]),
            0.0,
            LANE_PAVING,
        ));
    }

    // **The lane's well.** A 白族 lane has one, and it is the lane's institution: the courts have their own water in their
    // yards, and the lane's well is where the lane meets. It is a mass whose wall is thick enough to be a ring — `0.24 m` in a
    // `1.44 m` plan, so the inner loop cannot fold (round 73) — which is the same answer the Dai hamlet's well-head gave.
    let well = WELL_AT;
    // **The 井台: the platform the curb stands on.** Round 130 gave the street's well one; this is the same answer in the same
    // vocabulary — a floor slab a hand's height up, a step in front of it, and the curb and the windlass standing on its top. Two
    // wells in two scenes and **one way of standing a well on the ground**: what a plate reads as the same thing is the same thing.
    let apron = [
        [well[0] - WELL_HALF - 0.55, well[1] - WELL_HALF - 0.55],
        [well[0] + WELL_HALF + 0.55, well[1] - WELL_HALF - 0.55],
        [well[0] + WELL_HALF + 0.55, well[1] + WELL_HALF + 0.55],
        [well[0] - WELL_HALF - 0.55, well[1] + WELL_HALF + 0.55],
    ];
    skeleton.slabs.push(building::Slab::floor(
        building::ops::wound(apron.to_vec()),
        0.0,
        WELL_PLINTH,
    ));
    // **One step in front of it**, on the side the lane comes from.
    skeleton.slabs.push(building::Slab::floor(
        building::ops::wound(vec![
            [apron[0][0], apron[0][1] - WELL_STEP],
            [apron[1][0], apron[1][1] - WELL_STEP],
            [apron[1][0], apron[1][1]],
            [apron[0][0], apron[0][1]],
        ]),
        0.0,
        WELL_PLINTH * 0.5,
    ));
    let mut ring = building::Mass::new(
        building::ops::wound(vec![
            [well[0] - WELL_HALF, well[1] - WELL_HALF],
            [well[0] + WELL_HALF, well[1] - WELL_HALF],
            [well[0] + WELL_HALF, well[1] + WELL_HALF],
            [well[0] - WELL_HALF, well[1] + WELL_HALF],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, WELL_PLINTH))
        .collect(),
        WELL_PLINTH + WELL_HIGH,
    );
    ring.wall = WELL_WALL;
    ring.slab = 0.18;
    skeleton.masses.push(ring);

    // **The well's 辘轳: the windlass over it.** A lane's well is not a hole with a curb — it is a curb with a **frame** over it,
    // two posts and a beam, and a drum between them that the rope winds on. The four members are what makes the well an object
    // a village uses and not a drain.
    let [well_x, well_z] = WELL_AT;
    for across in [-1.0_f32, 1.0] {
        let x = well_x + across * (WELL_HALF + 0.24);
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, WELL_HIGH - 0.10, well_z],
            },
            building::Anchor::Free {
                at: [x, WELL_HIGH + WINDLASS_HIGH, well_z],
            },
            [1.0, 0.0],
            WINDLASS,
            WINDLASS,
        ));
    }
    // **The beam across the two posts**, and the **drum** hanging under it: the drum is the member the rope winds on, so it is
    // thinner than the beam and it turns — which is a thing the frame can only say by drawing it.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [well_x - WELL_HALF - 0.34, WELL_HIGH + WINDLASS_HIGH, well_z],
        },
        building::Anchor::Free {
            at: [well_x + WELL_HALF + 0.34, WELL_HIGH + WINDLASS_HIGH, well_z],
        },
        [0.0, 1.0],
        WINDLASS * 1.4,
        WINDLASS * 0.8,
    ));
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [
                well_x - WELL_HALF - 0.02,
                WELL_HIGH + WINDLASS_HIGH - 0.22,
                well_z,
            ],
        },
        building::Anchor::Free {
            at: [
                well_x + WELL_HALF + 0.02,
                WELL_HIGH + WINDLASS_HIGH - 0.22,
                well_z,
            ],
        },
        [0.0, 1.0],
        0.20,
        0.20,
    ));

    // **The 巷门: where the lane leaves the village.** The lintel is one mass over the lane and the two piers are **under** it
    // — `Join::Under` read the other way round from a storey: the mass states that its top meets the host's foot and that its
    // plan stands inside the host's, so the *mass* carries the *host*. That is the only relation this crate has for a thing
    // over a gap, and it is why a gateway is a lintel with two piers beneath it and not two piers with a beam between them.
    // The lane passes between the piers, which stand outside its paving so a cart is not stopped by them.
    let mut lintel = building::Mass::new(
        building::ops::wound(vec![
            [-GATE_HALF, GATE_AT - GATE_DEEP],
            [GATE_HALF, GATE_AT - GATE_DEEP],
            [GATE_HALF, GATE_AT + GATE_DEEP],
            [-GATE_HALF, GATE_AT + GATE_DEEP],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, GATE_FOOT))
        .collect(),
        GATE_FOOT + GATE_RISE,
    );
    lintel.wall = GATE_WALL;
    lintel.slab = 0.20;
    let lintel_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(lintel);
    for across in [-1.0_f32, 1.0] {
        let x = across * (LANE_HALF + PIER_HALF + 0.15);
        let mut pier = building::Mass::new(
            building::ops::wound(vec![
                [x - PIER_HALF, GATE_AT - GATE_DEEP],
                [x + PIER_HALF, GATE_AT - GATE_DEEP],
                [x + PIER_HALF, GATE_AT + GATE_DEEP],
                [x - PIER_HALF, GATE_AT + GATE_DEEP],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, 0.0))
            .collect(),
            GATE_FOOT,
        );
        pier.wall = 0.20;
        pier.slab = 0.18;
        pier.attachment = Some(building::Attachment {
            host: lintel_id,
            join: building::Join::Under,
        });
        skeleton.masses.push(pier);
    }
    // **The 抱鼓石 at the piers' feet, in the doorway.** Round 104 gave the 屯堡 朝门 this pair; a 白族 gate has the same two stones,
    // for the same two reasons — a leaf swings back against one, and the jamb is held upright by one. They stand **inside the
    // opening** and against the pier's inner face, so neither overlaps the stone: a drum that reached past the jamb would be a mass
    // inside a mass.
    let inner = LANE_HALF + PIER_HALF + 0.15 - PIER_HALF;
    for across in [-1.0_f32, 1.0] {
        let centre = across * (inner - DRUM * 0.5 - 0.22);
        let mut drum = building::Mass::new(
            [
                building::PlanNode::new([centre - DRUM * 0.5, DRUM_Z - DRUM * 0.5], 0.0),
                building::PlanNode::new([centre + DRUM * 0.5, DRUM_Z - DRUM * 0.5], 0.0),
                building::PlanNode::new([centre + DRUM * 0.5, DRUM_Z + DRUM * 0.5], 0.0),
                building::PlanNode::new([centre - DRUM * 0.5, DRUM_Z + DRUM * 0.5], 0.0),
            ]
            .to_vec(),
            DRUM_HIGH,
        );
        drum.wall = DRUM * 0.28;
        drum.slab = 0.14;
        skeleton.masses.push(drum);
    }

    // **The gate's roof: a lean-to over the lintel**, shedding into the lane and hosted by the lintel itself.
    let plan = [
        [-GATE_HALF - GATE_EAVE, GATE_AT + GATE_DEEP + GATE_EAVE],
        [GATE_HALF + GATE_EAVE, GATE_AT + GATE_DEEP + GATE_EAVE],
        [GATE_HALF + GATE_EAVE, GATE_AT - GATE_DEEP],
        [-GATE_HALF - GATE_EAVE, GATE_AT - GATE_DEEP],
    ];
    if let Some(roof) = building::derive::lean_to(
        lintel_id,
        plan,
        GATE_FOOT + GATE_RISE - 0.10,
        GATE_FOOT + GATE_RISE + 0.55,
        0.14,
    ) {
        skeleton.roofs.push(roof);
    }

    // **The gate's own two leaves, standing open against the piers.** A 巷门 is a gate and not an arch: it closes, and when
    // it is open each leaf lies back against its pier, hanging from the lintel's underside to a hand's width above the paving.
    // **A leaf is a panel and not a rod** — round 90's finding, and `derive::leaf` is that finding in one place, so the 屯堡's
    // 朝门 and this gate hang the same panel.
    let leaf = LANE_HALF - 0.10;
    for across in [-1.0_f32, 1.0] {
        let x = across * (LANE_HALF - BOARD * 0.5);
        if let Some(panel) = building::derive::leaf(
            [x, 0.08, GATE_AT - GATE_DEEP + 0.06],
            leaf,
            GATE_FOOT - 0.16,
            0.06,
            [0.0, 1.0],
        ) {
            skeleton.members.push(panel);
        }
    }

    // **The 题额: the board over the gate.** A 巷门 wears the village's name over its opening, and that board is the one piece
    // of a gate that is written rather than built. It is a member on the lintel's outer face, wider than a door leaf and a
    // hand's height, standing `0.06 m` proud of the stone.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-BOARD_HALF, GATE_FOOT + 0.26, GATE_AT - GATE_DEEP - 0.03],
        },
        building::Anchor::Free {
            at: [BOARD_HALF, GATE_FOOT + 0.26, GATE_AT - GATE_DEEP - 0.03],
        },
        [0.0, 1.0],
        0.12,
        0.52,
    ));

    // **The 门簪: the four studs that pin the 题额 to the lintel.** They are the gate's one purely ornamental joint — a head at each
    // end of the board that says the board is fixed and not hung — and they are members because a stud is a short square stick and
    // nothing more. Two either side of the board's middle, a hand apart.
    for across in [-1.0_f32, 1.0] {
        for offset in [0.42_f32, 1.06] {
            let x = across * offset;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [x, GATE_FOOT + 0.50, GATE_AT - GATE_DEEP - 0.05],
                },
                building::Anchor::Free {
                    at: [x, GATE_FOOT + 0.70, GATE_AT - GATE_DEEP - 0.05],
                },
                [1.0, 0.0],
                0.13,
                0.13,
            ));
        }
    }

    // **The 石敢当: the stone at the lane's mouth.** Where a lane meets a wider way, a 白族 village sets a small stone stele at the
    // corner — 石敢当, "the stone that dares" — and it is the one object in the street that is *written* and stands alone. It is a
    // mass: a stele has two faces, two edges, a foot and a top, and it is not a post.
    let mut stele = building::Mass::new(
        building::ops::wound(vec![
            [STELE_AT[0] - STELE_HALF, STELE_AT[1] - STELE_DEEP * 0.5],
            [STELE_AT[0] + STELE_HALF, STELE_AT[1] - STELE_DEEP * 0.5],
            [STELE_AT[0] + STELE_HALF, STELE_AT[1] + STELE_DEEP * 0.5],
            [STELE_AT[0] - STELE_HALF, STELE_AT[1] + STELE_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, STELE_FOOT))
        .collect(),
        STELE_FOOT + STELE_HIGH,
    );
    stele.wall = STELE_WALL;
    stele.slab = 0.16;
    skeleton.masses.push(stele);
    // **Its foot**: a slab the stele stands in, so the stone is set into the ground and not balanced on it.
    skeleton.slabs.push(building::Slab::floor(
        building::ops::wound(vec![
            [
                STELE_AT[0] - STELE_HALF - 0.18,
                STELE_AT[1] - STELE_DEEP * 0.5 - 0.18,
            ],
            [
                STELE_AT[0] + STELE_HALF + 0.18,
                STELE_AT[1] - STELE_DEEP * 0.5 - 0.18,
            ],
            [
                STELE_AT[0] + STELE_HALF + 0.18,
                STELE_AT[1] + STELE_DEEP * 0.5 + 0.18,
            ],
            [
                STELE_AT[0] - STELE_HALF - 0.18,
                STELE_AT[1] + STELE_DEEP * 0.5 + 0.18,
            ],
        ]),
        0.0,
        STELE_FOOT,
    ));

    // **The 文笔塔 at the lane's north end.** A 白族 village raises its tower where the lane leaves it, and this one stands past the last gate rather than
    // beside it: the tower's own document, translated, with none of its numbers restated. **The same type in a second village** — round 208 put one at
    // the 屯堡's east mouth, and the two are the same construction read against two different streets.
    let mut tower = crate::fengshui_tower::document().skeleton;
    ops::translate(&mut tower, TOWER_AT);
    assert!(
        ops::merge(&mut skeleton, tower),
        "the tower places no parts, so the merge always happens"
    );

    building::Building::new(skeleton)
}
