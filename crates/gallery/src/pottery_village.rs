//! A **陶村**: a clay village of 川滇 — the **龙窑** climbing the slope, the **拉坯棚** where the pots are thrown, the **柴垛** that feeds the firing, and the
//! drying floor between them — the fourteenth settlement.
//!
//! # What this scene is for
//!
//! Round 230 built the kiln and left it standing in a field. A 龙窑 is **never alone**: the shed where the pots are thrown stands at its foot, the woodpile
//! stands beside its doors, and the ground between them is a **drying floor** — flat, swept and the only paving in the village, because a wet pot cannot
//! stand on grass. **The works are the village**, as they are at 黑井 for salt and at 高坡 for clay, and the four parts of the scene are the four steps of
//! the process.
//!
//! # What it composes
//!
//! - **one `dragon_kiln::document()`**, its doors at the yard's low end, climbing away from them;
//! - **the 拉坯棚**: posts, a cap and a low bench, which is a wheel's shed and not a house;
//! - **the 柴垛**: a long low mass of stacked wood beside the doors, which is the one thing a kiln must have within reach;
//! - **the drying floor**: paving from the shed's door to the kiln's foot.
//!
//! # What it does not draw
//!
//! - **the pots**: a pot is a solid of revolution and this crate draws solids of plan — the same limit that leaves the salt pans empty and the kiln cold;
//! - **the clay pit**: the clay comes from a cut beyond the village, which is the ground's work and not the building's.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The kiln: where its foot stands, and the turn that puts its doors towards the yard.
const KILN_AT: [f32; 2] = [0.0, -9.40];
/// The yard between the shed and the kiln: its half width and its depth.
const YARD_HALF: f32 = 7.20;
const YARD_FROM: f32 = -9.40;
const YARD_TO: f32 = 4.60;
const PAVING: f32 = 0.07;
/// The 拉坯棚: where it stands, its plan, its posts and their height, and its cap's oversail.
const SHED_AT: [f32; 2] = [0.0, 7.20];
const SHED_LONG: f32 = 8.40;
const SHED_DEEP: f32 = 4.20;
const SHED_HIGH: f32 = 2.75;
const SHED_POST: f32 = 0.16;
const SHED_EAVE: f32 = 0.50;
/// The bench in the shed, where the wheels stand.
const BENCH_LONG: f32 = 6.80;
const BENCH_DEEP: f32 = 1.10;
const BENCH_HIGH: f32 = 0.72;
/// The 柴垛 beside the kiln's doors: its plan and its height.
const WOOD_AT: [f32; 2] = [-6.60, -4.20];
const WOOD_LONG: f32 = 3.10;
const WOOD_DEEP: f32 = 1.80;
const WOOD_HIGH: f32 = 1.35;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the scene: from behind the shed, down the drying floor, with the kiln climbing away at the far end.
pub fn pottery_village() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-13.6, 9.4, 25.0],
        [0.0, 2.4, -7.0],
        FIELD_OF_VIEW,
        "pottery_village",
    )
}

/// Returns the document: the drying floor, the shed, the woodpile and the kiln.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The drying floor**: the only paving in the village, from the shed's door to the kiln's foot.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-YARD_HALF, YARD_FROM],
            [YARD_HALF, YARD_FROM],
            [YARD_HALF, YARD_TO],
            [-YARD_HALF, YARD_TO],
        ]),
        0.0,
        PAVING,
    ));

    // **The 拉坯棚**: four posts, a cap and the bench the wheels stand on.
    for x in [
        SHED_AT[0] - SHED_LONG * 0.5 + SHED_POST,
        SHED_AT[0] + SHED_LONG * 0.5 - SHED_POST,
    ] {
        for z in [
            SHED_AT[1] - SHED_DEEP * 0.5 + SHED_POST,
            SHED_AT[1] + SHED_DEEP * 0.5 - SHED_POST,
        ] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, PAVING, z] },
                building::Anchor::Free {
                    at: [x, PAVING + SHED_HIGH, z],
                },
                [1.0, 0.0],
                SHED_POST,
                SHED_POST,
            ));
        }
    }
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [
                SHED_AT[0] - SHED_LONG * 0.5 - SHED_EAVE,
                SHED_AT[1] - SHED_DEEP * 0.5 - SHED_EAVE,
            ],
            [
                SHED_AT[0] + SHED_LONG * 0.5 + SHED_EAVE,
                SHED_AT[1] - SHED_DEEP * 0.5 - SHED_EAVE,
            ],
            [
                SHED_AT[0] + SHED_LONG * 0.5 + SHED_EAVE,
                SHED_AT[1] + SHED_DEEP * 0.5 + SHED_EAVE,
            ],
            [
                SHED_AT[0] - SHED_LONG * 0.5 - SHED_EAVE,
                SHED_AT[1] + SHED_DEEP * 0.5 + SHED_EAVE,
            ],
        ]),
        PAVING + SHED_HIGH,
        PAVING + SHED_HIGH + 0.14,
    ));
    let mut bench = Mass::new(
        ops::wound(vec![
            [SHED_AT[0] - BENCH_LONG * 0.5, SHED_AT[1] - BENCH_DEEP * 0.5],
            [SHED_AT[0] + BENCH_LONG * 0.5, SHED_AT[1] - BENCH_DEEP * 0.5],
            [SHED_AT[0] + BENCH_LONG * 0.5, SHED_AT[1] + BENCH_DEEP * 0.5],
            [SHED_AT[0] - BENCH_LONG * 0.5, SHED_AT[1] + BENCH_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING))
        .collect(),
        PAVING + BENCH_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the narrow side is the bench's depth.
    bench.wall = BENCH_DEEP * 0.26;
    bench.slab = 0.12;
    skeleton.masses.push(bench);

    // **The 柴垛**: stacked wood beside the kiln's doors, which is what a firing is fed from.
    let mut wood = Mass::new(
        ops::wound(vec![
            [WOOD_AT[0] - WOOD_LONG * 0.5, WOOD_AT[1] - WOOD_DEEP * 0.5],
            [WOOD_AT[0] + WOOD_LONG * 0.5, WOOD_AT[1] - WOOD_DEEP * 0.5],
            [WOOD_AT[0] + WOOD_LONG * 0.5, WOOD_AT[1] + WOOD_DEEP * 0.5],
            [WOOD_AT[0] - WOOD_LONG * 0.5, WOOD_AT[1] + WOOD_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        WOOD_HIGH,
    );
    wood.wall = WOOD_DEEP * 0.30;
    wood.slab = 0.14;
    skeleton.masses.push(wood);

    // **The kiln** at the yard's low end, its doors towards the shed. Round 230's own document, translated, with none of its numbers restated.
    let mut kiln = crate::dragon_kiln::document().skeleton;
    ops::translate(&mut kiln, KILN_AT);
    assert!(
        ops::merge(&mut skeleton, kiln),
        "the kiln places no parts, so the merge always happens"
    );

    Building::new(skeleton)
}
