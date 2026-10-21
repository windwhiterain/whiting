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
const STAIR_HALF: f32 = 1.60;
const STAIR_FLAGS: usize = 7;
const JOINT: f32 = 0.06;
const PAVING: f32 = 0.07;
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
