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

use building::BuildingSkeleton;
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
const LANE_FROM: f32 = -21.0;
const LANE_TO: f32 = 21.0;

/// The lane`s well: where it stands, its half width, its own wall and its curb`s height.
const WELL_AT: [f32; 2] = [0.0, -13.0];
const WELL_HALF: f32 = 0.72;
const WELL_WALL: f32 = 0.24;
const WELL_HIGH: f32 = 0.85;

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

    // **The lane is paving and not a mass**: it has no wall and nothing stands on it, so it is a `Slab::paving` from one end
    // to the other — the same answer the courts' own floors gave.
    skeleton.slabs.push(building::Slab::paving(
        building::ops::wound(vec![
            [-LANE_HALF, LANE_FROM],
            [LANE_HALF, LANE_FROM],
            [LANE_HALF, LANE_TO],
            [-LANE_HALF, LANE_TO],
        ]),
        0.0,
        0.06,
    ));

    // **The lane's well.** A 白族 lane has one, and it is the lane's institution: the courts have their own water in their
    // yards, and the lane's well is where the lane meets. It is a mass whose wall is thick enough to be a ring — `0.24 m` in a
    // `1.44 m` plan, so the inner loop cannot fold (round 73) — which is the same answer the Dai hamlet's well-head gave.
    let well = WELL_AT;
    let mut ring = building::Mass::new(
        building::ops::wound(vec![
            [well[0] - WELL_HALF, well[1] - WELL_HALF],
            [well[0] + WELL_HALF, well[1] - WELL_HALF],
            [well[0] + WELL_HALF, well[1] + WELL_HALF],
            [well[0] - WELL_HALF, well[1] + WELL_HALF],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        WELL_HIGH,
    );
    ring.wall = WELL_WALL;
    ring.slab = 0.18;
    skeleton.masses.push(ring);

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

    building::Building::new(skeleton)
}
