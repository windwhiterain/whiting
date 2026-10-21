//! A **盐村**: a brine village of 滇中 — two 土掌房 on either side of a paved yard, the **盐井与盐灶** at its head, and the wall that holds the yard up —
//! the thirteenth settlement.
//!
//! # What this scene is for
//!
//! Round 224 built the salt works and left them standing in a field, which is not where a works stands: at 黑井 and 盐津 the works **is** the village, and
//! the houses are where the people who draw and boil the brine live. This settlement is the works' own home — **a yard with a well at its head and two
//! houses along its sides** — and it is the first village here whose reason for existing is a **product** and not a lane, a square or a temple.
//!
//! # What it composes
//!
//! - **two `earthen::document()`s**, the 滇中 house, one either side of the yard and both facing it, because a salt village is a works with houses round
//!   it and not a row;
//! - **one `saltworks::document()`**, at the yard's north head, turned to face down it;
//! - **the yard's own paving** and the **retaining wall** that makes the yard a shelf, as the 土掌房 village's terraces are held.
//!
//! # What it does not draw
//!
//! - **the brine below**: the shaft and the seam are underground and this crate draws what stands on the ground;
//! - **the salt itself**: the pans are drawn empty, which is the same compromise round 224 recorded about fire and steam.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The two houses: their plots either side of the yard and the turn each takes to face it.
const HOUSES: [(f32, f32, f32); 2] = [
    (-11.4, 1.6, core::f32::consts::FRAC_PI_2),
    (11.4, 1.6, -core::f32::consts::FRAC_PI_2),
];
/// The works at the yard's head, turned to look down it.
const WORKS_AT: [f32; 2] = [0.0, -13.2];
const WORKS_TURN: f32 = -core::f32::consts::FRAC_PI_2;
/// The yard: its half width either way, its paving's thickness.
const YARD_HALF: f32 = 9.0;
const YARD_DEEP: f32 = 12.4;
const PAVING: f32 = 0.08;
/// The wall holding the yard up along its south edge.
const RETAIN_HALF: f32 = 11.0;
const RETAIN_HIGH: f32 = 1.05;
const RETAIN_THICK: f32 = 0.52;
/// The 盐仓 at the yard's south end: where it stands, its plan and height, how far its cap oversails, and its door's jamb and head.
const STORE_AT: f32 = 9.20;
const STORE_LONG: f32 = 7.40;
const STORE_DEEP: f32 = 4.60;
const STORE_HIGH: f32 = 4.10;
const STORE_EAVE: f32 = 0.42;
const STORE_JAMB: f32 = 0.62;
const STORE_HEAD: f32 = 0.55;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the scene: from the south, down the yard, with the works at its head.
pub fn salt_village() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [2.0, 13.0, 31.0],
        [0.0, 2.0, -8.0],
        FIELD_OF_VIEW,
        "salt_village",
    )
}

/// Returns the document: the yard, the two houses, the works and the retaining wall.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The yard**: one slab the whole settlement stands on, because a boiling house needs a floor and not a road.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-YARD_HALF, -YARD_DEEP],
            [YARD_HALF, -YARD_DEEP],
            [YARD_HALF, YARD_DEEP],
            [-YARD_HALF, YARD_DEEP],
        ]),
        0.0,
        PAVING,
    ));

    // **The retaining wall** along the yard's south edge, so the yard is a shelf and not a field.
    let mut retain = Mass::new(
        ops::wound(vec![
            [-RETAIN_HALF, YARD_DEEP],
            [RETAIN_HALF, YARD_DEEP],
            [RETAIN_HALF, YARD_DEEP + RETAIN_THICK],
            [-RETAIN_HALF, YARD_DEEP + RETAIN_THICK],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        RETAIN_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
    retain.wall = RETAIN_THICK * 0.38;
    retain.slab = 0.12;
    skeleton.masses.push(retain);

    // **The two houses**, one either side, both facing the yard.
    for (x, z, turn) in HOUSES {
        let mut part = crate::earthen::document().skeleton;
        if turn.abs() > 1.0e-3 {
            ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        ops::translate(&mut part, [x, z]);
        assert!(
            ops::merge(&mut skeleton, part),
            "a 土掌房 places no parts, so the merge always happens"
        );
    }

    // **The works at the yard's head.** Round 224's own document, turned to look down the yard, with none of its numbers restated.
    let mut works = crate::saltworks::document().skeleton;
    ops::rotate(&mut works, [0.0, 0.0], WORKS_TURN);
    ops::translate(&mut works, WORKS_AT);
    assert!(
        ops::merge(&mut skeleton, works),
        "the works place no parts, so the merge always happens"
    );

    // **The 盐仓 at the yard's south end.** Salt is not boiled and eaten: it is **weighed, stored and taxed**, and a works without a store is a works
    // whose product stands in the open. The store faces the works down the yard's own axis, so the two ends of the yard are the two ends of the process —
    // **the brine at the head and the salt at the foot** — and its door is a real opening, cut in the face that looks up the yard (round 210's answer for
    // the 塔门: `split_face` and an `Opening`).
    let mut store = Mass::new(
        ops::wound(vec![
            [-STORE_LONG * 0.5, STORE_AT - STORE_DEEP * 0.5],
            [STORE_LONG * 0.5, STORE_AT - STORE_DEEP * 0.5],
            [STORE_LONG * 0.5, STORE_AT + STORE_DEEP * 0.5],
            [-STORE_LONG * 0.5, STORE_AT + STORE_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING))
        .collect(),
        PAVING + STORE_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the narrow side is the store's own depth.
    store.wall = STORE_DEEP * 0.22;
    store.slab = 0.18;
    let store_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(store);
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [
                -STORE_LONG * 0.5 - STORE_EAVE,
                STORE_AT - STORE_DEEP * 0.5 - STORE_EAVE,
            ],
            [
                STORE_LONG * 0.5 + STORE_EAVE,
                STORE_AT - STORE_DEEP * 0.5 - STORE_EAVE,
            ],
            [
                STORE_LONG * 0.5 + STORE_EAVE,
                STORE_AT + STORE_DEEP * 0.5 + STORE_EAVE,
            ],
            [
                -STORE_LONG * 0.5 - STORE_EAVE,
                STORE_AT + STORE_DEEP * 0.5 + STORE_EAVE,
            ],
        ]),
        PAVING + STORE_HIGH,
        PAVING + STORE_HIGH + 0.16,
    ));
    // Face 0 is the store's `-z` side, which is the side that looks up the yard at the works.
    let face = building::FaceRef::new(store_id, 0);
    if ops::split_face(
        &mut skeleton,
        face,
        &building::Rhythm::Even { count: 3 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        let _ = ops::add_opening(
            &mut skeleton,
            building::Opening::new(
                face,
                0,
                1,
                building::Inset::new(STORE_JAMB, STORE_JAMB, 0.0, STORE_HEAD),
            ),
        );
    }

    Building::new(skeleton)
}
