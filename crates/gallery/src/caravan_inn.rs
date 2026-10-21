//! A **马店**: the caravan inn of a 茶马古道 town — an inn, a stable, a trough, the posts a mule is tied to, and one gate onto the road — the twenty-third
//! type.
//!
//! # What this scene is for
//!
//! Every 西南 road town on the 茶马古道 has one building the others are not: an inn that takes **mules as well as men**. Its yard is not a garden and not a
//! temple court — it is a **working yard**, and everything in it is addressed to an animal: the trough along one wall, the hitching posts along another,
//! the stable's open front at the back, and the gate wide enough for a loaded animal. It is the first type here whose user is **travelling**.
//!
//! # How it is built
//!
//! - **the inn**: a body of a single storey with a gallery across its front, because the rooms are upstairs in a real one and this crate has no interior;
//! - **the stable**: an open front of posts with a low wall behind them, so the yard and the stalls are one space;
//! - **the trough and the posts**: a stone trough along the stable's side and four posts along the opposite wall — a member each, and both are what the
//!   yard is for;
//! - **the gate**: a stone frame in the street wall, wide enough that a loaded mule walks through it standing up.
//!
//! # What it does not draw
//!
//! - **the upstairs**: the rooms over the gallery are the interior this crate does not have, and the gallery is drawn as a floor and a rail because that
//!   is what can be seen from the yard;
//! - **the animals**: a mule is not a solid this catalogue draws, and a yard with a trough and four posts is a yard a mule is expected in.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The yard: its half width and its depth, and the paving's thickness.
const YARD_HALF: f32 = 7.60;
const YARD_DEEP: f32 = 8.20;
const PAVING: f32 = 0.09;
/// The inn across the yard's head: its plan and its height, and its gallery's depth.
const INN_LONG: f32 = 14.4;
const INN_DEEP: f32 = 5.20;
const INN_HIGH: f32 = 4.60;
const GALLERY: f32 = 1.65;
const GALLERY_DECK: f32 = 0.14;
const RAIL: f32 = 0.92;
const RAIL_SECTION: f32 = 0.09;
const POST: f32 = 0.17;
/// The upper floor's own height, which is the rooms over the gallery.
const UPPER_HIGH: f32 = 2.90;
/// The stable down one side: its own long side, its low wall's height and its roof's oversail.
const STABLE_LONG: f32 = 7.20;
const STABLE_DEEP: f32 = 3.30;
const STABLE_WALL: f32 = 1.30;
const STABLE_HIGH: f32 = 2.95;
const STABLE_EAVE: f32 = 0.45;
/// The trough along the stable's front, and the four hitching posts opposite.
const TROUGH_LONG: f32 = 4.40;
const TROUGH_HIGH: f32 = 0.62;
const TROUGH_WIDE: f32 = 0.55;
const HITCH_POSTS: usize = 4;
const HITCH_HIGH: f32 = 1.15;
/// The gate in the street wall: its two posts, their height, the lintel's depth, and how wide the opening is.
const GATE_WIDE: f32 = 3.10;
const GATE_POST: f32 = 0.28;
const GATE_HIGH: f32 = 3.20;
const LINTEL: f32 = 0.34;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the scene: from the road, through the gate, with the stable at the frame's left and the inn at its head.
pub fn caravan_inn() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-5.2, 4.0, 25.6],
        [0.0, 2.4, -4.2],
        FIELD_OF_VIEW,
        "caravan_inn",
    )
}

/// Returns the document: the yard's paving, the inn, the stable, the trough, the posts and the gate.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The yard**: paved, because a yard with mules in it is a floor and not a garden.
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

    // **The inn** across the yard's head, with its gallery across the front.
    let inn_at = -YARD_DEEP - INN_DEEP * 0.5 + 0.35;
    let mut inn = Mass::new(
        ops::wound(vec![
            [-INN_LONG * 0.5, inn_at - INN_DEEP * 0.5],
            [INN_LONG * 0.5, inn_at - INN_DEEP * 0.5],
            [INN_LONG * 0.5, inn_at + INN_DEEP * 0.5],
            [-INN_LONG * 0.5, inn_at + INN_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING))
        .collect(),
        PAVING + INN_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the narrow side is the inn's depth.
    inn.wall = INN_DEEP * 0.20;
    inn.slab = 0.18;
    skeleton.masses.push(inn);
    let cap_x = INN_LONG * 0.5 + 0.55;
    let cap_z = INN_DEEP * 0.5 + 0.55;
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [-cap_x, inn_at - cap_z],
            [cap_x, inn_at - cap_z],
            [cap_x, inn_at + cap_z],
            [-cap_x, inn_at + cap_z],
        ]),
        PAVING + INN_HIGH,
        PAVING + INN_HIGH + 0.17,
    ));
    // **The gallery**: a floor and a rail across the inn's front, which is the part of the upstairs a man in the yard can see.
    let front = inn_at + INN_DEEP * 0.5;
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [-INN_LONG * 0.5, front],
            [INN_LONG * 0.5, front],
            [INN_LONG * 0.5, front + GALLERY],
            [-INN_LONG * 0.5, front + GALLERY],
        ]),
        PAVING + 2.55,
        PAVING + 2.55 + GALLERY_DECK,
    ));
    let rail_z = front + GALLERY - RAIL_SECTION * 0.5;
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-INN_LONG * 0.5, PAVING + 2.55 + GALLERY_DECK + RAIL, rail_z],
        },
        building::Anchor::Free {
            at: [INN_LONG * 0.5, PAVING + 2.55 + GALLERY_DECK + RAIL, rail_z],
        },
        [0.0, 1.0],
        RAIL_SECTION,
        RAIL_SECTION,
    ));
    // **The gallery's posts**, one under each end and one in the middle.
    for post in -1..=1 {
        let x = INN_LONG * 0.5 * post as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, PAVING, rail_z],
            },
            building::Anchor::Free {
                at: [x, PAVING + 2.55, rail_z],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }

    // **The upstairs, which is what round 228 said was missing.** The inn's rooms stand **behind the gallery and over the ground floor**, as an attached mass
    // with `Join::Storey` — the answer the 土掌房 gave in round 169 and the 风水塔 re-paid in round 207 — set back by the gallery's own depth so the gallery
    // is a **floor with a roof over it** rather than a shelf on a wall. Its cap is a second roof at the upper height, which is what makes the building
    // read as two storeys from the yard.
    let upper_at = inn_at + GALLERY * 0.5;
    let upper_deep = INN_DEEP - GALLERY;
    let mut upper = Mass::new(
        ops::wound(vec![
            [-INN_LONG * 0.5, upper_at - upper_deep * 0.5],
            [INN_LONG * 0.5, upper_at - upper_deep * 0.5],
            [INN_LONG * 0.5, upper_at + upper_deep * 0.5],
            [-INN_LONG * 0.5, upper_at + upper_deep * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING + INN_HIGH))
        .collect(),
        PAVING + INN_HIGH + UPPER_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the narrow side is the upper floor's own depth.
    upper.wall = upper_deep * 0.20;
    upper.slab = 0.16;
    upper.attachment = Some(building::Attachment {
        host: building::MassId(0),
        join: building::Join::Storey,
    });
    skeleton.masses.push(upper);
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [-cap_x, upper_at - upper_deep * 0.5 - 0.55],
            [cap_x, upper_at - upper_deep * 0.5 - 0.55],
            [cap_x, upper_at + upper_deep * 0.5 + 0.55],
            [-cap_x, upper_at + upper_deep * 0.5 + 0.55],
        ]),
        PAVING + INN_HIGH + UPPER_HIGH,
        PAVING + INN_HIGH + UPPER_HIGH + 0.17,
    ));

    // **The stable** down the yard's west side: an open front of posts and a low wall behind them.
    let stable_x = -YARD_HALF + STABLE_DEEP * 0.5;
    let mut wall = Mass::new(
        ops::wound(vec![
            [stable_x - STABLE_DEEP * 0.5, -STABLE_LONG * 0.5],
            [stable_x - STABLE_DEEP * 0.5, STABLE_LONG * 0.5],
            [stable_x - STABLE_DEEP * 0.5 + 0.40, STABLE_LONG * 0.5],
            [stable_x - STABLE_DEEP * 0.5 + 0.40, -STABLE_LONG * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING))
        .collect(),
        PAVING + STABLE_WALL,
    );
    // **The ring's offset stays under half the narrow side**: this wall is `0.40 m` thick, so its offset is under `0.20`.
    wall.wall = 0.10;
    wall.slab = 0.12;
    skeleton.masses.push(wall);
    // The stable's open front: five posts along its edge, and its roof over them.
    for stall in 0..5 {
        let z = -STABLE_LONG * 0.5 + STABLE_LONG * stall as f32 / 4.0;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [stable_x + STABLE_DEEP * 0.5, PAVING, z],
            },
            building::Anchor::Free {
                at: [stable_x + STABLE_DEEP * 0.5, PAVING + STABLE_HIGH, z],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [
                stable_x - STABLE_DEEP * 0.5 - STABLE_EAVE,
                -STABLE_LONG * 0.5 - STABLE_EAVE,
            ],
            [
                stable_x + STABLE_DEEP * 0.5 + STABLE_EAVE,
                -STABLE_LONG * 0.5 - STABLE_EAVE,
            ],
            [
                stable_x + STABLE_DEEP * 0.5 + STABLE_EAVE,
                STABLE_LONG * 0.5 + STABLE_EAVE,
            ],
            [
                stable_x - STABLE_DEEP * 0.5 - STABLE_EAVE,
                STABLE_LONG * 0.5 + STABLE_EAVE,
            ],
        ]),
        PAVING + STABLE_HIGH,
        PAVING + STABLE_HIGH + 0.15,
    ));

    // **The trough** along the stable's front, where the beasts drink.
    let mut trough = Mass::new(
        ops::wound(vec![
            [stable_x + STABLE_DEEP * 0.5 + 0.35, -TROUGH_LONG * 0.5],
            [
                stable_x + STABLE_DEEP * 0.5 + 0.35 + TROUGH_WIDE,
                -TROUGH_LONG * 0.5,
            ],
            [
                stable_x + STABLE_DEEP * 0.5 + 0.35 + TROUGH_WIDE,
                TROUGH_LONG * 0.5,
            ],
            [stable_x + STABLE_DEEP * 0.5 + 0.35, TROUGH_LONG * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING))
        .collect(),
        PAVING + TROUGH_HIGH,
    );
    // **The ring's offset stays under half the narrow side**: `0.55 m` wide, so its offset is under `0.27`.
    trough.wall = 0.16;
    trough.slab = 0.10;
    skeleton.masses.push(trough);

    // **The hitching posts** along the yard's east side, where a mule is tied while its load comes off.
    for post in 0..HITCH_POSTS {
        let z = -YARD_HALF * 0.55 + 2.85 * post as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [YARD_HALF - 1.15, PAVING, z],
            },
            building::Anchor::Free {
                at: [YARD_HALF - 1.15, PAVING + HITCH_HIGH, z],
            },
            [1.0, 0.0],
            POST * 0.75,
            POST * 0.75,
        ));
    }

    // **The gate** in the street wall: two stone posts and a lintel, wide enough for a loaded mule.
    for side in [-1.0_f32, 1.0] {
        let x = side * GATE_WIDE * 0.5;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, 0.0, YARD_DEEP + GATE_POST * 0.5],
            },
            building::Anchor::Free {
                at: [x, PAVING + GATE_HIGH, YARD_DEEP + GATE_POST * 0.5],
            },
            [1.0, 0.0],
            GATE_POST,
            GATE_POST,
        ));
    }
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [
                -GATE_WIDE * 0.5 - GATE_POST * 0.5,
                PAVING + GATE_HIGH,
                YARD_DEEP + GATE_POST * 0.5,
            ],
        },
        building::Anchor::Free {
            at: [
                GATE_WIDE * 0.5 + GATE_POST * 0.5,
                PAVING + GATE_HIGH,
                YARD_DEEP + GATE_POST * 0.5,
            ],
        },
        [0.0, 1.0],
        LINTEL,
        LINTEL,
    ));

    // **The two leaves**, swung open against the yard's walls — which is how a 马店's gate stands all day: **an inn's gate is not shut against the road, it is
    // shut against the night**, and the trade comes in through it. Same `derive::leaf` the Dai hamlet's gate uses.
    for side in [-1.0_f32, 1.0] {
        if let Some(leaf) = building::derive::leaf(
            [side * GATE_WIDE * 0.5, 0.06, YARD_DEEP + GATE_POST * 0.5],
            GATE_WIDE * 0.5 - 0.06,
            PAVING + GATE_HIGH - 0.18,
            0.07,
            [-side, 0.0],
        ) {
            skeleton.members.push(leaf);
        }
    }

    Building::new(skeleton)
}
