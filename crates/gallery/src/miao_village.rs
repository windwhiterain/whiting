//! A **苗寨**: six 吊脚楼 on three terraces up a hillside, with the flights between them — the twelfth settlement.
//!
//! # What this scene is for
//!
//! The gallery has had the Miao stilt house since Arc 1 (`stilt`, 2 167 rows) and has never put it in a village. A 苗寨 is **the clearest case in the
//! whole gallery of a settlement whose form is its ground**: the houses do not stand on a slope, they **hang off** it — each is seated on a bank and
//! its downhill side is a wall of posts — so a village of them is a **stair of houses**, each one's floor at the height of the terrace behind it.
//!
//! # What it composes
//!
//! Six `stilt::document()`s — each already seated on its own bank by the type's own `site::seat` — on three terraces raised by [`ops::raise`], plus
//! each terrace's paving and retaining face and the flights between them.
//!
//! # What it does not draw
//!
//! - **the bank under each house**: the type's `sink` and its posts are built for a 35 % slope, and here every terrace is a flat shelf. The houses'
//!   posts still show on their downhill side, because the type draws them, but the ground they were measured against is a shelf and not a bank;
//! - **the 芦笙场**: a Miao village's dancing ground, which is a court and wants its own round.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The terraces: how many, how deep each is, the rise between them and how far back each is set.
const STEPS: usize = 3;
const STEP_DEPTH: f32 = 15.0;
const STEP_RISE: f32 = 1.70;
/// How wide the terraces run, and the paving's thickness.
const TERRACE_HALF: f32 = 22.0;
const PAVING: f32 = 0.06;
/// The six houses: which terrace, where across it, and which way it is turned.
const HOUSES: [(usize, f32, f32); 6] = [
    (0, -7.4, 0.0),
    (0, 7.4, 0.0),
    (1, -7.0, 0.0),
    (1, 7.0, 0.0),
    (2, -6.6, 0.0),
    (2, 6.6, 0.0),
];
/// The retaining face between the terraces: its depth and how high it stands above the step it holds.
const RETAIN: f32 = 0.55;
const RETAIN_HIGH: f32 = 2.20;
/// The flights: how wide, how many treads.
const FLIGHT_HALF: f32 = 1.45;

/// The 芦笙场: its half width and how deep it runs out from the lowest terrace, the 花杆's section and height, how many cross-arms it carries and
/// how far each drops down the pole, an arm's reach, and the streamers' length and width.
const GROUND_HALF: f32 = 10.0;
const GROUND_DEEP: f32 = 13.0;
const POLE: f32 = 0.17;
const POLE_HIGH: f32 = 8.60;
const POLE_ARMS: usize = 2;
const POLE_ARM_DROP: f32 = 1.30;
const POLE_ARM: f32 = 1.15;
const STREAMER: f32 = 0.72;
const STREAMER_WIDE: f32 = 0.14;
const TREADS: usize = 5;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the scene: from below and in front, so the houses read as a stair and each terrace's face as its riser.
pub fn miao_village() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-26.0, 14.0, 40.0],
        [0.0, 4.5, -12.0],
        FIELD_OF_VIEW,
        "miao_village",
    )
}

/// Returns the document: three terraces, their faces, six stilt houses and the flights.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The terraces and their retaining faces.** Each shelf is a paving slab at its own level and a wall along its front; the wall is what a
    // stepped village is made of and without it the houses stand on shelves that float.
    for step in 0..STEPS {
        let level = STEP_RISE * step as f32;
        let front = -STEP_DEPTH * step as f32;
        skeleton.slabs.push(building::Slab::paving(
            ops::wound(vec![
                [-TERRACE_HALF, front - STEP_DEPTH],
                [TERRACE_HALF, front - STEP_DEPTH],
                [TERRACE_HALF, front],
                [-TERRACE_HALF, front],
            ]),
            level,
            level + PAVING,
        ));
        let mut face = Mass::new(
            ops::wound(vec![
                [-TERRACE_HALF, front],
                [TERRACE_HALF, front],
                [TERRACE_HALF, front + RETAIN],
                [-TERRACE_HALF, front + RETAIN],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, 0.0))
            .collect(),
            level + RETAIN_HIGH,
        );
        // **The ring's offset stays under half the wall's depth** (round 144).
        face.wall = RETAIN * 0.34;
        face.slab = 0.18;
        skeleton.masses.push(face);
    }

    // **The six houses**, each the type's own document raised onto its terrace. The type seats its house on a 35 % bank, so its posts and its
    // `sink` are already drawn; the raise is the only thing the village adds to it.
    for (step, across, turn) in HOUSES {
        let level = STEP_RISE * step as f32 + PAVING;
        let front = -STEP_DEPTH * step as f32;
        let mut part = crate::stilt::document().skeleton;
        if turn.abs() > 1.0e-3 {
            ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        ops::translate(&mut part, [across, front - STEP_DEPTH * 0.5]);
        ops::raise(&mut part, level);
        assert!(
            ops::merge(&mut skeleton, part),
            "a stilt house places no parts, so the merge always happens"
        );
    }

    // **The flights between the terraces**, at the village's crossing point: five treads up each face.
    for step in 0..STEPS - 1 {
        let level = STEP_RISE * step as f32;
        let front = -STEP_DEPTH * (step + 1) as f32;
        for tread in 0..TREADS {
            let along = tread as f32 / TREADS as f32;
            let z = front + RETAIN * along;
            let rise = level + STEP_RISE * along;
            skeleton.slabs.push(building::Slab::paving(
                ops::wound(vec![
                    [-FLIGHT_HALF, z],
                    [FLIGHT_HALF, z],
                    [FLIGHT_HALF, z + RETAIN / TREADS as f32],
                    [-FLIGHT_HALF, z + RETAIN / TREADS as f32],
                ]),
                rise,
                rise + PAVING,
            ));
        }
    }

    // **The 芦笙场: the dancing ground.** A 苗寨's one public work is the **paved court in front of the lowest terrace**, with the **花杆** standing in
    // its middle — a stripped fir pole with streamers and cross-arms, round which the 芦笙 dancers turn at the year's festivals. The court is the
    // village's only flat ground that is not a shelf, and it is why a 苗寨 is built as a stair: the dance needs a level and the houses need a slope.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-GROUND_HALF, 0.0],
            [GROUND_HALF, 0.0],
            [GROUND_HALF, GROUND_DEEP],
            [-GROUND_HALF, GROUND_DEEP],
        ]),
        0.0,
        PAVING,
    ));
    // **The 花杆**, with its cross-arms and the streamers hanging from them.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [0.0, PAVING, GROUND_DEEP * 0.5],
        },
        building::Anchor::Free {
            at: [0.0, PAVING + POLE_HIGH, GROUND_DEEP * 0.5],
        },
        [1.0, 0.0],
        POLE,
        POLE,
    ));
    for arm in 0..POLE_ARMS {
        let up = PAVING + POLE_HIGH - POLE_ARM_DROP * (arm as f32 + 1.0);
        for side in [-1.0_f32, 1.0] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [0.0, up, GROUND_DEEP * 0.5],
                },
                building::Anchor::Free {
                    at: [side * POLE_ARM, up, GROUND_DEEP * 0.5],
                },
                [0.0, 1.0],
                POLE * 0.55,
                POLE * 0.6,
            ));
            // **A streamer hangs from each arm's end**, which is what a 花杆 is for and what makes it read as a festival pole rather than a mast.
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [side * POLE_ARM, up, GROUND_DEEP * 0.5],
                },
                building::Anchor::Free {
                    at: [side * POLE_ARM, up - STREAMER, GROUND_DEEP * 0.5],
                },
                [1.0, 0.0],
                STREAMER_WIDE,
                STREAMER_WIDE,
            ));
        }
    }

    Building::new(skeleton)
}
