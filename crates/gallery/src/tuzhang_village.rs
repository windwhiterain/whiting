//! A **彝族土掌房聚落**: a village of flat-roofed rammed-earth houses **stacked up a slope**, each roof the terrace of the house behind it.
//!
//! # What this scene is for
//!
//! The gallery's **ninth settlement** and the first from **滇南**. A 土掌房 village is the one西南 form where the **roofs are the streets**:
//! the houses step up a hillside and each flat roof is the yard of the one above, so the settlement is read as a **stair of roofs** and not as
//! rows of buildings.
//!
//! # What it composes
//!
//! Five `earthen::document()`s — the type's own house — on three terraces, the higher terrace set back and up, plus the flights of flags that
//! join them. The terraces are the composition's spine: **the ground steps, and the houses stand on the steps.**
//!
//! # What it does not draw
//!
//! - **the terrain**: the terraces here are exact slabs, not a landform. A village on real ground is the landscape arc's work, and this scene
//!   composes the *arrangement* the landform would carry;
//! - **the 火塘 smoke**: every house has one, and smoke is not this crate's material.

use building::{Building, BuildingSkeleton, ops};
use definition::Value;

/// The terraces: how far apart in `z`, how high each stands, and how far back each is set.
const STEPS: usize = 3;
const STEP_DEPTH: f32 = 15.0;
const STEP_RISE: f32 = 1.65;
const STEP_SETBACK: f32 = 1.10;
/// How wide the terrace runs.
const TERRACE_HALF: f32 = 26.0;
/// The houses on each terrace: the first stands at `-x`, the second at `+x`, and where they stand across the terrace.
const HOUSES: [(usize, f32, f32); 5] = [
    // (terrace, x, turn)
    (0, -9.5, 0.0),
    (0, 9.5, 0.0),
    (1, -9.0, 0.0),
    (2, -8.5, 0.0),
    (2, 9.0, 0.0),
];
/// The flights of flags up the terraces: how wide, how many, and the joint between them.
/// The 院墙 in front of each house: its half width, how deep the yard runs, the walls depth and height, and the gap left for the gate.
const YARD_HALF: f32 = 5.20;
const YARD_DEEP: f32 = 1.55;
const YARD_THICK: f32 = 0.38;
const YARD_HIGH: f32 = 1.95;
const YARD_GATE: f32 = 1.30;
/// Where the houses own front edge stands from its centre, which is what the yard starts from.
const HOUSE_FRONT: f32 = 5.15;
const STAIR_HALF: f32 = 1.60;
const STAIR_FLAGS: usize = 7;
const JOINT: f32 = 0.06;
const PAVING: f32 = 0.07;
/// The 水窖 on the lowest terrace: where the pair stands, the rim's plan and the curb's height above the paving.
const CISTERN_AT: [f32; 2] = [-4.60, -3.40];
const CISTERN: f32 = 1.55;
const CISTERN_HIGH: f32 = 0.46;
/// The 柴垛 in a yard: its length, how deep it lies and how high it is stacked.
const STACK: f32 = 1.90;
const STACK_DEEP: f32 = 1.05;
const STACK_HIGH: f32 = 1.45;
/// The terrace's own retaining face, how high it stands above the step it holds, and how thick.
const RETAIN_HIGH: f32 = 2.10;
const RETAIN_THICK: f32 = 0.55;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the scene: from below and in front, so the roofs read as a stair and the retaining faces as its risers.
pub fn tuzhang_village() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-30.0, 17.0, 46.0],
        [0.0, 4.0, -14.0],
        FIELD_OF_VIEW,
        "tuzhang_village",
    )
}

/// Returns the document: three terraces, their retaining faces, five houses and the flights between them.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The terraces and their retaining faces.** Each terrace is a paving slab at its own level and a wall along its front — the wall is
    // what a stepped village is *made of*, and without it the houses stand on shelves that float.
    for step in 0..STEPS {
        let level = STEP_RISE * step as f32;
        let front = -STEP_DEPTH * step as f32 - STEP_SETBACK * step as f32;
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
        let mut face = building::Mass::new(
            ops::wound(vec![
                [-TERRACE_HALF, front],
                [TERRACE_HALF, front],
                [TERRACE_HALF, front + RETAIN_THICK],
                [-TERRACE_HALF, front + RETAIN_THICK],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, 0.0))
            .collect(),
            level + RETAIN_HIGH,
        );
        // **The ring's offset stays under half the wall's depth** (round 144): the face is drawn as a solid wall and not as a ring.
        face.wall = RETAIN_THICK * 0.34;
        face.slab = 0.20;
        skeleton.masses.push(face);
    }

    // **The five houses**, each on its terrace's own level and turned to face down the slope.
    for (step, across, turn) in HOUSES {
        let level = STEP_RISE * step as f32 + PAVING;
        let front = -STEP_DEPTH * step as f32 - STEP_SETBACK * step as f32;
        let mut part = crate::earthen::document().skeleton;
        if turn.abs() > 1.0e-3 {
            ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        ops::translate(&mut part, [across, front - STEP_DEPTH * 0.5]);
        ops::raise(&mut part, level);
        assert!(
            ops::merge(&mut skeleton, part),
            "a house of the village places no parts, so the merge always happens"
        );
    }

    // **The 院墙: every house's own yard.** A 土掌房 household's yard is a low earth wall in front of the house with a gap for the gate, and
    // without it five houses stand in a field. Each yard is two side lengths and a front split by the gap; they stop short of the flights of
    // flags and of the retaining face they stand on.
    for (step, across, _) in HOUSES {
        let level = STEP_RISE * step as f32 + PAVING;
        let front = -STEP_DEPTH * step as f32 - STEP_SETBACK * step as f32;
        let house_front = front - STEP_DEPTH * 0.5 + HOUSE_FRONT;
        for (x0, x1, z0, z1) in [
            (
                across - YARD_HALF,
                across - YARD_HALF + YARD_THICK,
                house_front,
                house_front + YARD_DEEP,
            ),
            (
                across + YARD_HALF - YARD_THICK,
                across + YARD_HALF,
                house_front,
                house_front + YARD_DEEP,
            ),
            (
                across - YARD_HALF + YARD_THICK,
                across - YARD_GATE,
                house_front + YARD_DEEP - YARD_THICK,
                house_front + YARD_DEEP,
            ),
            (
                across + YARD_GATE,
                across + YARD_HALF - YARD_THICK,
                house_front + YARD_DEEP - YARD_THICK,
                house_front + YARD_DEEP,
            ),
        ] {
            let mut wall = building::Mass::new(
                ops::wound(vec![[x0, z0], [x1, z0], [x1, z1], [x0, z1]])
                    .into_iter()
                    .map(|at| building::PlanNode::new(at, level))
                    .collect(),
                level + YARD_HIGH,
            );
            // **The ring's offset stays under half the wall's depth** (round 144).
            wall.wall = YARD_THICK * 0.34;
            wall.slab = 0.14;
            skeleton.masses.push(wall);
        }
    }

    // **The 水窖: the cistern that makes the hill habitable.** A 彝族 village on a dry ridge lives on the rain it catches, so its one
    // irreplaceable work is a **stone rimmed cistern** sunk into the terrace, standing open to the sky with its stone curb above the paving.
    for (cistern_x, cistern_z, level) in [
        (CISTERN_AT[0], CISTERN_AT[1], 0.0_f32),
        (CISTERN_AT[0] + 1.60, CISTERN_AT[1] - 1.10, 0.0),
    ] {
        let mut rim = building::Mass::new(
            ops::wound(vec![
                [cistern_x - CISTERN * 0.5, cistern_z - CISTERN * 0.5],
                [cistern_x + CISTERN * 0.5, cistern_z - CISTERN * 0.5],
                [cistern_x + CISTERN * 0.5, cistern_z + CISTERN * 0.5],
                [cistern_x - CISTERN * 0.5, cistern_z + CISTERN * 0.5],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, level + PAVING))
            .collect(),
            level + PAVING + CISTERN_HIGH,
        );
        rim.wall = CISTERN * 0.22;
        rim.slab = 0.12;
        skeleton.masses.push(rim);
    }

    // **The 柴垛: the woodpile in the yard.** Every 土掌房 yard keeps one — a stack of split wood under a stone cap — and it is the object
    // that says a yard is lived in rather than swept.
    for (across, _, step) in [
        (HOUSES[0].1, 0.0_f32, HOUSES[0].0),
        (HOUSES[1].1, 0.0, HOUSES[1].0),
    ] {
        let level = STEP_RISE * step as f32 + PAVING;
        let front = -STEP_DEPTH * step as f32 - STEP_SETBACK * step as f32;
        let house_front = front - STEP_DEPTH * 0.5 + HOUSE_FRONT;
        let stack_x = across - YARD_HALF + 1.40;
        let stack_z = house_front + STACK_DEEP * 0.5 + 0.10;
        let mut stack = building::Mass::new(
            ops::wound(vec![
                [stack_x - STACK * 0.5, stack_z - STACK_DEEP * 0.5],
                [stack_x + STACK * 0.5, stack_z - STACK_DEEP * 0.5],
                [stack_x + STACK * 0.5, stack_z + STACK_DEEP * 0.5],
                [stack_x - STACK * 0.5, stack_z + STACK_DEEP * 0.5],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, level))
            .collect(),
            level + STACK_HIGH,
        );
        // **The ring's offset is a fraction of the NARROW side**, not of the length: 'STACK * 0.30' is '0.57' against a half-depth of
        // '0.525' and folds the inner loop (rounds 74 and 144, and this is the twelfth time the rule has been paid for).
        stack.wall = STACK_DEEP * 0.30;
        stack.slab = 0.10;
        skeleton.masses.push(stack);
    }

    // **The flights of flags** between the terraces, at the village's own crossing point: seven flags to a flight, up the retaining face.
    for step in 0..STEPS - 1 {
        let low = STEP_RISE * step as f32 + PAVING;
        let front = -STEP_DEPTH * (step + 1) as f32 - STEP_SETBACK * (step + 1) as f32;
        let run = (STEP_RISE + RETAIN_THICK) / STAIR_FLAGS as f32;
        for flag in 0..STAIR_FLAGS {
            let from = front + RETAIN_THICK + run * flag as f32 + JOINT * 0.5;
            let to = front + RETAIN_THICK + run * (flag + 1) as f32 - JOINT * 0.5;
            let rise = low + (STEP_RISE + PAVING) * (flag as f32 + 0.5) / STAIR_FLAGS as f32;
            skeleton.slabs.push(building::Slab::paving(
                ops::wound(vec![
                    [-STAIR_HALF, from],
                    [STAIR_HALF, from],
                    [STAIR_HALF, to],
                    [-STAIR_HALF, to],
                ]),
                rise,
                rise + PAVING,
            ));
        }
    }

    Building::new(skeleton)
}
