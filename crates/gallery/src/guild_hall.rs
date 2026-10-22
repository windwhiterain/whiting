//! A **会馆**: the guild hall of a 西南 market town — a gate house with a **stage over its own entrance**, two galleries round a paved court, and the hall of the
//! patron god at the back — the twenty-eighth type.
//!
//! # What this scene is for
//!
//! A 场镇 on a trade road is full of men from elsewhere, and each group of them built one of these: 湖广会馆, 江西会馆, 万寿宫. A 会馆 is **a home for people who
//! have none** — it is where a stranger sleeps, hears his own dialect, settles a debt and buries a corpse — and its plan says so: **the stage is over the gate**,
//! because the entrance is also the theatre, and the god's hall is at the back, furthest from the street, because the god is the last thing you come to.
//!
//! # How it is built
//!
//! - **the gate house**: a mass across the street's front with a door cut in its middle bay — the only way in;
//! - **the stage over the gate**: a slab at the gate's top standing on four posts, because the audience stands in the court and looks **back** at the entrance;
//! - **two galleries** down the court's sides, on posts, which is where a man waits out a rain;
//! - **the hall** at the back on its own plinth, with a door and two windows, which is the god's room and the guild's meeting room at once.
//!
//! # What it does not draw
//!
//! - **the guild**: a 会馆's business is a society and a ledger, and this crate draws neither;
//! - **the god**: a 会馆's hall holds an image, and an image is a carved solid of a kind this catalogue has no vocabulary for.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The court: its half width, its depth and the paving's thickness.
const COURT_HALF: f32 = 6.40;
const COURT_DEEP: f32 = 7.60;
const PAVING: f32 = 0.10;
/// The gate house across the front: its length, its depth, its height and the door's jamb and head.
const GATE_LONG: f32 = 15.6;
const GATE_DEEP: f32 = 3.60;
const GATE_HIGH: f32 = 4.35;
const DOOR_JAMB: f32 = 1.90;
const DOOR_HEAD: f32 = 0.55;
/// The stage over the gate: how high its floor stands, its depth, and its posts.
const STAGE_FLOOR: f32 = 4.45;
const STAGE_DEEP: f32 = 3.20;
const STAGE_POST: f32 = 0.22;
const STAGE_HIGH: f32 = 2.85;
const STAGE_EAVE: f32 = 0.55;
/// The two galleries down the court: their depth, their deck's height and their posts.
const GALLERY: f32 = 2.10;
const DECK: f32 = 0.42;
const GALLERY_POST: f32 = 0.20;
const GALLERY_HIGH: f32 = 2.75;
/// The hall at the back: its length, its depth, its plinth and its height.
const HALL_LONG: f32 = 13.4;
const HALL_DEEP: f32 = 5.60;
const HALL_PLINTH: f32 = 0.72;
const HALL_HIGH: f32 = 4.10;
const HALL_JAMB: f32 = 0.90;
const HALL_HEAD: f32 = 0.70;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.60;

/// Builds the scene: from the street, through the gate and down the court — the order a stranger arrives in.
pub fn guild_hall() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-9.2, 7.4, -20.6],
        [0.2, 3.0, 3.0],
        FIELD_OF_VIEW,
        "guild_hall",
    )
}

/// Returns the document: the court, the gate house, the stage over it, the two galleries and the hall.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The court**: paved, because a 会馆's court is where the audience stands and where the guild meets when it cannot fit in the hall.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-COURT_HALF, -COURT_DEEP],
            [COURT_HALF, -COURT_DEEP],
            [COURT_HALF, COURT_DEEP],
            [-COURT_HALF, COURT_DEEP],
        ]),
        0.0,
        PAVING,
    ));

    // **The gate house** across the front, its door in the middle bay.
    let gate_z = -COURT_DEEP - GATE_DEEP * 0.5;
    let mut gate = Mass::new(
        ops::wound(vec![
            [-GATE_LONG * 0.5, gate_z - GATE_DEEP * 0.5],
            [GATE_LONG * 0.5, gate_z - GATE_DEEP * 0.5],
            [GATE_LONG * 0.5, gate_z + GATE_DEEP * 0.5],
            [-GATE_LONG * 0.5, gate_z + GATE_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        GATE_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the narrow side is the gate's depth.
    gate.wall = GATE_DEEP * 0.20;
    gate.slab = 0.18;
    let gate_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(gate);
    // **The street door** in the gate's `-z` face, which is the side the road is on.
    let street_face = building::FaceRef::new(gate_id, 0);
    if ops::split_face(
        &mut skeleton,
        street_face,
        &building::Rhythm::Even { count: 3 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        assert!(
            ops::add_opening(
                &mut skeleton,
                building::Opening::new(
                    street_face,
                    0,
                    1,
                    building::Inset::new(DOOR_JAMB, DOOR_JAMB, 0.0, DOOR_HEAD),
                ),
            )
            .is_some(),
            "the gate`s door is a hole its own wall accepts"
        );
    }

    // **The stage over the gate**: a floor at the gate's top on four posts, because the audience stands in the court and looks back at the entrance.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [-COURT_HALF, gate_z - STAGE_DEEP * 0.5],
            [COURT_HALF, gate_z - STAGE_DEEP * 0.5],
            [COURT_HALF, gate_z + STAGE_DEEP * 0.5],
            [-COURT_HALF, gate_z + STAGE_DEEP * 0.5],
        ]),
        STAGE_FLOOR,
        STAGE_FLOOR + 0.16,
    ));
    for x in [-COURT_HALF + STAGE_POST, COURT_HALF - STAGE_POST] {
        for z in [
            gate_z - STAGE_DEEP * 0.5 + STAGE_POST,
            gate_z + STAGE_DEEP * 0.5 - STAGE_POST,
        ] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [x, STAGE_FLOOR + 0.16, z],
                },
                building::Anchor::Free {
                    at: [x, STAGE_FLOOR + STAGE_HIGH, z],
                },
                [1.0, 0.0],
                STAGE_POST,
                STAGE_POST,
            ));
        }
    }
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [
                -COURT_HALF - STAGE_EAVE,
                gate_z - STAGE_DEEP * 0.5 - STAGE_EAVE,
            ],
            [
                COURT_HALF + STAGE_EAVE,
                gate_z - STAGE_DEEP * 0.5 - STAGE_EAVE,
            ],
            [
                COURT_HALF + STAGE_EAVE,
                gate_z + STAGE_DEEP * 0.5 + STAGE_EAVE,
            ],
            [
                -COURT_HALF - STAGE_EAVE,
                gate_z + STAGE_DEEP * 0.5 + STAGE_EAVE,
            ],
        ]),
        STAGE_FLOOR + STAGE_HIGH,
        STAGE_FLOOR + STAGE_HIGH + 0.20,
    ));

    // **The two galleries** down the court's sides: a deck on posts, because a man waits out a rain on one.
    for side in [-1.0_f32, 1.0] {
        let inner = side * COURT_HALF;
        let outer = side * (COURT_HALF + GALLERY);
        skeleton.slabs.push(building::Slab::floor(
            ops::wound(vec![
                [inner.min(outer), -COURT_DEEP + 0.60],
                [inner.max(outer), -COURT_DEEP + 0.60],
                [inner.max(outer), COURT_DEEP - 0.60],
                [inner.min(outer), COURT_DEEP - 0.60],
            ]),
            PAVING + DECK,
            PAVING + DECK + 0.14,
        ));
        for post in 0..4 {
            let z = -COURT_DEEP + 1.60 + post as f32 * 3.60;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [outer, PAVING, z],
                },
                building::Anchor::Free {
                    at: [outer, PAVING + GALLERY_HIGH, z],
                },
                [1.0, 0.0],
                GALLERY_POST,
                GALLERY_POST,
            ));
        }
    }

    // **The hall** at the back, on its own plinth, with a door and two windows.
    let hall_z = COURT_DEEP + HALL_DEEP * 0.5;
    let mut plinth = Mass::new(
        ops::wound(vec![
            [-HALL_LONG * 0.5 - 0.40, hall_z - HALL_DEEP * 0.5 - 0.40],
            [HALL_LONG * 0.5 + 0.40, hall_z - HALL_DEEP * 0.5 - 0.40],
            [HALL_LONG * 0.5 + 0.40, hall_z + HALL_DEEP * 0.5 + 0.40],
            [-HALL_LONG * 0.5 - 0.40, hall_z + HALL_DEEP * 0.5 + 0.40],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        PAVING + HALL_PLINTH,
    );
    plinth.wall = HALL_DEEP * 0.14;
    plinth.slab = 0.16;
    skeleton.masses.push(plinth);
    let mut hall = Mass::new(
        ops::wound(vec![
            [-HALL_LONG * 0.5, hall_z - HALL_DEEP * 0.5],
            [HALL_LONG * 0.5, hall_z - HALL_DEEP * 0.5],
            [HALL_LONG * 0.5, hall_z + HALL_DEEP * 0.5],
            [-HALL_LONG * 0.5, hall_z + HALL_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING + HALL_PLINTH))
        .collect(),
        PAVING + HALL_PLINTH + HALL_HIGH,
    );
    hall.wall = HALL_DEEP * 0.16;
    hall.slab = 0.18;
    // **The hall stands on its plinth and is attached to it**: two masses whose plans nest are one mass standing on the other, which is `Join::Storey`
    // (rounds 169, 207, 229 and 239) and not two masses side by side; without the attachment the checker answers `UnattachedOverlap`.
    hall.attachment = Some(building::Attachment {
        host: building::MassId(skeleton.masses.len() - 1),
        join: building::Join::Storey,
    });
    let hall_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(hall);
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [-HALL_LONG * 0.5 - 0.70, hall_z - HALL_DEEP * 0.5 - 0.70],
            [HALL_LONG * 0.5 + 0.70, hall_z - HALL_DEEP * 0.5 - 0.70],
            [HALL_LONG * 0.5 + 0.70, hall_z + HALL_DEEP * 0.5 + 0.70],
            [-HALL_LONG * 0.5 - 0.70, hall_z + HALL_DEEP * 0.5 + 0.70],
        ]),
        PAVING + HALL_PLINTH + HALL_HIGH,
        PAVING + HALL_PLINTH + HALL_HIGH + 0.18,
    ));
    // **The hall's three openings** in the face that looks down the court, which is the face the guild faces.
    let court_face = building::FaceRef::new(hall_id, 0);
    if ops::split_face(
        &mut skeleton,
        court_face,
        &building::Rhythm::Even { count: 3 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        for column in 0..3_usize {
            let sill = if column == 1 { 0.0 } else { 1.10 };
            assert!(
                ops::add_opening(
                    &mut skeleton,
                    building::Opening::new(
                        court_face,
                        0,
                        column,
                        building::Inset::new(HALL_JAMB, HALL_JAMB, sill, HALL_HEAD),
                    ),
                )
                .is_some(),
                "the hall`s middle bay is its door and its two ends are windows"
            );
        }
    }

    Building::new(skeleton)
}
