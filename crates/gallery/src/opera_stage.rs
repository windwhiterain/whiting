//! An **戏台**: the free-standing opera stage of a 西南 market town — a plinth a man's height off the ground, a back wall, three open sides under a deep eave, and
//! the 扮戏房 behind it — the twenty-sixth type.
//!
//! # What this scene is for
//!
//! A 戏台 is the one building here that **exists for one hour of one day**: the village raises it for the god's birthday or the market's opening, and for the
//! rest of the year it is a roofed platform with nothing on it. Its whole form is **a stage and a roof**, and its whole function is **to be seen from in
//! front** — which is why it is a plinth with **three open sides** and a **deep eave**, and why its back wall is the one solid face it has.
//!
//! # How it is built
//!
//! - **the 台基**: a plinth `STAGE_HIGH` off the ground, because a crowd stands and the stage must be over its heads;
//! - **the stage floor**: a slab on the plinth, its front edge the line the audience walks up to;
//! - **the back wall and the 扮戏房**: a solid at the back, and the small room behind it where the players dress;
//! - **the posts and the roof**: four posts at the corners, two more at the front's middle, and **one deep eave** over all of them — a stage's roof is its
//!   acoustic and its proscenium at once.
//!
//! # What it does not draw
//!
//! - **the audience**: this crate draws buildings; a stage seen from its own front is empty by definition on a day that is not the feast;
//! - **the 藻井**: the painted caisson over a good stage is a ceiling, and this crate has no interior for a ceiling to be under.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The 台基: its plan and its height above the ground.
const STAGE_LONG: f32 = 8.60;
const STAGE_DEEP: f32 = 6.20;
const STAGE_HIGH: f32 = 1.42;
/// The stage floor: its thickness.
const FLOOR: f32 = 0.16;
/// The back wall and the 扮戏房 behind it: the wall's thickness and height, and the room's own depth.
const BACK: f32 = 0.52;
const BACK_HIGH: f32 = 3.85;
const DRESS_DEEP: f32 = 2.60;
const DRESS_HIGH: f32 = 3.10;
/// The posts at the stage's corners and its front's middle, and their height above the floor.
const POST: f32 = 0.24;
const POST_HIGH: f32 = 3.35;
/// The roof: how far it oversails and its thickness.
const EAVE: f32 = 0.86;
const ROOF: f32 = 0.22;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the scene: from the audience's own place, a little below the stage and off its centre.
pub fn opera_stage() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [4.2, 2.7, 18.2],
        [-0.5, 2.6, -1.4],
        FIELD_OF_VIEW * 1.05,
        "opera_stage",
    )
}

/// Returns the document: the plinth, the stage floor, the back wall, the dressing room, the posts and the roof.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The 台基**: a plinth a man's height off the ground, which is what a stage is before it is anything else. **Its plan begins at the back wall's front
    // face**, because masses may not overlap in plan (round 137) and the wall and the dressing room both stand behind it.
    let back_z = -STAGE_DEEP * 0.5 + BACK * 0.5;
    let mut plinth = Mass::new(
        ops::wound(vec![
            [-STAGE_LONG * 0.5, back_z + BACK * 0.5],
            [STAGE_LONG * 0.5, back_z + BACK * 0.5],
            [STAGE_LONG * 0.5, STAGE_DEEP * 0.5],
            [-STAGE_LONG * 0.5, STAGE_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        STAGE_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the narrow side is the stage's depth.
    plinth.wall = STAGE_DEEP * 0.18;
    plinth.slab = 0.20;
    skeleton.masses.push(plinth);

    // **The stage floor**, on the plinth and inset a little from its edge, which is the line the audience walks up to.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [-STAGE_LONG * 0.5 + 0.30, -STAGE_DEEP * 0.5 + 0.30],
            [STAGE_LONG * 0.5 - 0.30, -STAGE_DEEP * 0.5 + 0.30],
            [STAGE_LONG * 0.5 - 0.30, STAGE_DEEP * 0.5 - 0.30],
            [-STAGE_LONG * 0.5 + 0.30, STAGE_DEEP * 0.5 - 0.30],
        ]),
        STAGE_HIGH,
        STAGE_HIGH + FLOOR,
    ));

    // **The back wall**: the one solid face a stage has, and the one the audience never sees from in front.
    let mut back = Mass::new(
        ops::wound(vec![
            [-STAGE_LONG * 0.5, back_z - BACK * 0.5],
            [STAGE_LONG * 0.5, back_z - BACK * 0.5],
            [STAGE_LONG * 0.5, back_z + BACK * 0.5],
            [-STAGE_LONG * 0.5, back_z + BACK * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, STAGE_HIGH))
        .collect(),
        STAGE_HIGH + BACK_HIGH,
    );
    // **The ring's offset stays under half the narrow side**: this wall is `0.52 m` thick, so its offset is under `0.26`.
    back.wall = 0.18;
    back.slab = 0.14;
    skeleton.masses.push(back);

    // **The 扮戏房** behind the wall, where the players dress: a low room with its own cap, standing on the ground behind the plinth.
    // **A hair's clearance from the wall**: two masses that touch are two masses that overlap as far as the checker is concerned.
    let dress_z = back_z - BACK * 0.5 - DRESS_DEEP * 0.5 - 0.03;
    let mut dress = Mass::new(
        ops::wound(vec![
            [-STAGE_LONG * 0.5 + 0.60, dress_z - DRESS_DEEP * 0.5],
            [STAGE_LONG * 0.5 - 0.60, dress_z - DRESS_DEEP * 0.5],
            [STAGE_LONG * 0.5 - 0.60, dress_z + DRESS_DEEP * 0.5],
            [-STAGE_LONG * 0.5 + 0.60, dress_z + DRESS_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        DRESS_HIGH,
    );
    dress.wall = DRESS_DEEP * 0.22;
    dress.slab = 0.16;
    skeleton.masses.push(dress);
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [-STAGE_LONG * 0.5 + 0.20, dress_z - DRESS_DEEP * 0.5 - 0.35],
            [STAGE_LONG * 0.5 - 0.20, dress_z - DRESS_DEEP * 0.5 - 0.35],
            [STAGE_LONG * 0.5 - 0.20, dress_z + DRESS_DEEP * 0.5 + 0.35],
            [-STAGE_LONG * 0.5 + 0.20, dress_z + DRESS_DEEP * 0.5 + 0.35],
        ]),
        DRESS_HIGH,
        DRESS_HIGH + 0.16,
    ));

    // **The posts**: four at the corners and two at the front's middle, which is what an open stage's roof stands on.
    let front_z = STAGE_DEEP * 0.5 - 0.34;
    for x in [
        -STAGE_LONG * 0.5 + 0.42,
        -STAGE_LONG / 6.0,
        STAGE_LONG / 6.0,
        STAGE_LONG * 0.5 - 0.42,
    ] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, STAGE_HIGH, front_z],
            },
            building::Anchor::Free {
                at: [x, STAGE_HIGH + POST_HIGH, front_z],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }
    for x in [-STAGE_LONG * 0.5 + 0.42, STAGE_LONG * 0.5 - 0.42] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, STAGE_HIGH, back_z],
            },
            building::Anchor::Free {
                at: [x, STAGE_HIGH + POST_HIGH, back_z],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }

    // **The roof**: one deep eave over the stage and the dressing room both, which is a stage's acoustic and its proscenium at once.
    let cap_x = STAGE_LONG * 0.5 + EAVE;
    let cap_z = (front_z - (dress_z - DRESS_DEEP * 0.5)) * 0.5 + EAVE;
    let cap_middle = (front_z + dress_z - DRESS_DEEP * 0.5) * 0.5;
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [-cap_x, cap_middle - cap_z],
            [cap_x, cap_middle - cap_z],
            [cap_x, cap_middle + cap_z],
            [-cap_x, cap_middle + cap_z],
        ]),
        STAGE_HIGH + POST_HIGH,
        STAGE_HIGH + POST_HIGH + ROOF,
    ));

    Building::new(skeleton)
}
