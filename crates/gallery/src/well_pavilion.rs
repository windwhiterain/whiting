//! A **井亭**: the covered well of a 西南 village — a stone mouth, four posts and a cap, a beam with a windlass, and the trough beside it — the twenty-ninth
//! type.
//!
//! # What this scene is for
//!
//! A village's well is the one place where **every household comes to the same spot every day**, and in the south-west it is roofed: the water is not deep and
//! the rain is heavy, so the mouth is kept covered and the drawing is done under a square cap on four posts. It is **the smallest public building there is**
//! — smaller than the 土地庙, smaller than the gate — and the only one whose plan is determined by **a hole in the ground**.
//!
//! # How it is built
//!
//! - **the mouth**: a stone mass with its top below the paving, because a well's stonework is a **kerb** and the water is inside it;
//! - **four posts and a cap**, which is the smallest roof this crate can make that is still a building;
//! - **the beam across two posts and the windlass on it**, because a covered well is a well you draw from under cover;
//! - **the trough** beside the mouth, which is where the water goes first.
//!
//! # What it does not draw
//!
//! - **the water**: this crate draws solids and the well's content is a hole;
//! - **the bucket**: a bucket is a vessel and a vessel is a shell, which is a mass with an inside — and the well is already one.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The mouth: the stone kerb's side, its height above the paving and how thick the kerb is.
const MOUTH: f32 = 1.40;
const MOUTH_HIGH: f32 = 0.62;
const KERB: f32 = 0.26;
/// The four posts and the cap: where they stand from the mouth's centre, their section, their height, and the cap's oversail.
const POST: f32 = 0.16;
const POST_AT: f32 = 1.42;
const POST_HIGH: f32 = 2.55;
const EAVE: f32 = 0.42;
/// The beam across the two western posts, and the windlass on it.
const BEAM: f32 = 0.18;
const BEAM_HIGH: f32 = 1.85;
const DRUM: f32 = 0.22;
/// The trough beside the mouth: its plan and its height.
const TROUGH_AT: [f32; 2] = [1.86, 0.0];
const TROUGH_LONG: f32 = 1.90;
const TROUGH_WIDE: f32 = 0.72;
const TROUGH_HIGH: f32 = 0.58;
/// The paving under the whole thing, so the well stands on a floor and not on the field.
const PAVING_HALF: f32 = 3.20;
const PAVING: f32 = 0.07;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.62;

/// Builds the scene: from the lane's side, at the height of a woman carrying two buckets.
pub fn well_pavilion() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-5.6, 2.2, 6.4],
        [0.4, 1.5, -0.2],
        FIELD_OF_VIEW,
        "well_pavilion",
    )
}

/// Returns the document: the paving, the kerb, four posts and their cap, the beam with its windlass, and the trough.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The floor** the well stands on: a village well is paved round, because it is where people stand.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-PAVING_HALF, -PAVING_HALF],
            [PAVING_HALF, -PAVING_HALF],
            [PAVING_HALF, PAVING_HALF],
            [-PAVING_HALF, PAVING_HALF],
        ]),
        0.0,
        PAVING,
    ));

    // **The mouth**: the stone kerb, with its head at hip height — the water is inside it and this crate draws the stone.
    let mut mouth = Mass::new(
        ops::wound(vec![
            [-MOUTH * 0.5, -MOUTH * 0.5],
            [MOUTH * 0.5, -MOUTH * 0.5],
            [MOUTH * 0.5, MOUTH * 0.5],
            [-MOUTH * 0.5, MOUTH * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING))
        .collect(),
        PAVING + MOUTH_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the mouth is `1.40` square, so the kerb is under `0.70`.
    mouth.wall = KERB;
    mouth.slab = 0.10;
    skeleton.masses.push(mouth);

    // **Four posts and a cap**: the smallest roof this crate can make that is still a building.
    for x in [-POST_AT, POST_AT] {
        for z in [-POST_AT, POST_AT] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, PAVING, z] },
                building::Anchor::Free {
                    at: [x, PAVING + POST_HIGH, z],
                },
                [1.0, 0.0],
                POST,
                POST,
            ));
        }
    }
    let cap = POST_AT + EAVE;
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![[-cap, -cap], [cap, -cap], [cap, cap], [-cap, cap]]),
        PAVING + POST_HIGH,
        PAVING + POST_HIGH + 0.16,
    ));

    // **The beam and the windlass**: a covered well is a well you draw from under cover, and the beam is what the rope runs over.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-POST_AT, PAVING + BEAM_HIGH, 0.0],
        },
        building::Anchor::Free {
            at: [POST_AT, PAVING + BEAM_HIGH, 0.0],
        },
        [0.0, 1.0],
        BEAM,
        BEAM,
    ));
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-DRUM, PAVING + BEAM_HIGH + BEAM * 0.5, 0.0],
        },
        building::Anchor::Free {
            at: [DRUM, PAVING + BEAM_HIGH + BEAM * 0.5, 0.0],
        },
        [0.0, 1.0],
        DRUM,
        DRUM,
    ));

    // **The trough** beside the mouth, which is where the water goes first.
    let mut trough = Mass::new(
        ops::wound(vec![
            [
                TROUGH_AT[0] - TROUGH_LONG * 0.5,
                TROUGH_AT[1] - TROUGH_WIDE * 0.5,
            ],
            [
                TROUGH_AT[0] + TROUGH_LONG * 0.5,
                TROUGH_AT[1] - TROUGH_WIDE * 0.5,
            ],
            [
                TROUGH_AT[0] + TROUGH_LONG * 0.5,
                TROUGH_AT[1] + TROUGH_WIDE * 0.5,
            ],
            [
                TROUGH_AT[0] - TROUGH_LONG * 0.5,
                TROUGH_AT[1] + TROUGH_WIDE * 0.5,
            ],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, PAVING))
        .collect(),
        PAVING + TROUGH_HIGH,
    );
    // **The ring's offset stays under half the narrow side**: the trough is `0.72` wide.
    trough.wall = TROUGH_WIDE * 0.22;
    trough.slab = 0.10;
    skeleton.masses.push(trough);

    Building::new(skeleton)
}
