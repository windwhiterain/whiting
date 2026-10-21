//! A **景颇寨**: three longhouses round a lane, with a gate at the lane's mouth — the first settlement of 滇西.
//!
//! # What this scene is for
//!
//! The gallery's **tenth settlement**. A 景颇 village is not a courtyard and not a street of houses: it is **three or four longhouses** set down
//! on a ridge with a lane between them, and the lane is the whole public space the village has.
//!
//! # What it composes
//!
//! Three `longhouse::document()`s — each already carrying its posts, its 敞廊 and its ladder — on either side of a paved lane, plus the gate
//! this project has built seven times: two lengths of wall either side of a gap, a head across it, two leaves and a lean-to over it. **The
//! settlement adds no type**: the lane, the walls and the gate are the same arrangement the 侗寨 and the 羌寨 use.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The lane: how wide it runs, how far it goes, and where the houses stand either side of it.
const LANE_HALF: f32 = 4.20;
const LANE_FROM: f32 = -26.0;
const LANE_TO: f32 = 20.0;
const PAVING: f32 = 0.06;
/// The houses: where each stands and how it is turned. The lane runs along `x`, so the two flanking houses face across it.
const HOUSES: [(f32, f32, f32); 3] = [
    (-6.0, -10.6, core::f32::consts::PI),
    (-6.0, 10.6, 0.0),
    (-24.0, 0.0, core::f32::consts::FRAC_PI_2),
];
/// The gate at the lane's mouth: how wide the gap, the wall's depth and height, and how deep the gate's head lies.
const GATE_AT: f32 = LANE_TO - 2.20;
const GATE_CLEAR: f32 = 3.60;
const WALL_THICK: f32 = 0.44;
const WALL_HIGH: f32 = 2.10;
const HEAD_HIGH: f32 = 0.46;
/// How far the village's wall runs beyond the lane on each side.
const WALL_SPAN: f32 = 15.0;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the scene: from the lane's mouth, so the gate frames the two long roofs and the third behind them.
pub fn jingpo_village() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [37.0, 13.0, 31.0],
        [-2.0, 4.6, 0.0],
        FIELD_OF_VIEW,
        "jingpo_village",
    )
}

/// Returns the document: three longhouses, the lane's paving, the gate's walls, its head and its leaves.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The three longhouses**, each the type's own document, turned so the two flanking houses face across the lane.
    for (across, along, turn) in HOUSES {
        let mut part = crate::longhouse::document().skeleton;
        if turn.abs() > 1.0e-3 {
            ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        ops::translate(&mut part, [across, along]);
        assert!(
            ops::merge(&mut skeleton, part),
            "a longhouse places no parts, so the merge always happens"
        );
    }

    // **The lane**, which is the village's only public ground.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [LANE_FROM, -LANE_HALF],
            [LANE_TO, -LANE_HALF],
            [LANE_TO, LANE_HALF],
            [LANE_FROM, LANE_HALF],
        ]),
        0.0,
        PAVING,
    ));

    // **The gate's walls**: two lengths either side of the lane's mouth, meeting the lane's own edge and running out to the village's wall.
    for side in [-1.0_f32, 1.0] {
        let inner = side * LANE_HALF;
        let outer = side * (LANE_HALF + WALL_SPAN);
        let mut wall = Mass::new(
            ops::wound(vec![
                [GATE_AT, inner],
                [GATE_AT, outer],
                [GATE_AT + WALL_THICK, outer],
                [GATE_AT + WALL_THICK, inner],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, 0.0))
            .collect(),
            WALL_HIGH,
        );
        // **The ring's offset stays under half the wall's depth** (round 144).
        wall.wall = WALL_THICK * 0.34;
        wall.slab = 0.16;
        skeleton.masses.push(wall);
    }

    // **The head across the lane**, spanning exactly the gap and resting on the two lengths' own heads (round 157).
    let mut head = Mass::new(
        ops::wound(vec![
            [GATE_AT, -LANE_HALF],
            [GATE_AT, LANE_HALF],
            [GATE_AT + WALL_THICK, LANE_HALF],
            [GATE_AT + WALL_THICK, -LANE_HALF],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, WALL_HIGH))
        .collect(),
        WALL_HIGH + HEAD_HIGH,
    );
    head.wall = WALL_THICK * 0.32;
    head.slab = 0.14;
    skeleton.masses.push(head);
    for side in [-1.0_f32, 1.0] {
        if let Some(leaf) = building::derive::leaf(
            [GATE_AT + WALL_THICK * 0.5, 0.10, side * (LANE_HALF - 0.06)],
            LANE_HALF - 0.12,
            WALL_HIGH - 0.24,
            0.07,
            [0.0, side],
        ) {
            skeleton.members.push(leaf);
        }
    }

    Building::new(skeleton)
}
