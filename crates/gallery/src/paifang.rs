//! A **牌坊**: the memorial gateway of a 西南 market town — four stone posts, three bays, a lintel over each and a roof stepped bay by bay — the twenty-fifth
//! type.
//!
//! # What this scene is for
//!
//! A 牌坊 is the one structure here that **is only a face**: it has no inside, no floor and no door that shuts, and every part of it exists to be read from the
//! road. A village raises one for a degree, a chaste widow or a merchant's gift, and the stone says which — so the type is **a wall turned inside out**, and
//! its whole content is the elevation the traveller sees.
//!
//! # How it is built
//!
//! - **four posts**, the two middle ones taller, each standing on a **夹杆石** — the stone drum that holds a free-standing post up;
//! - **three bays**: a wide centre and two narrow sides, each bay spanned by **two lintels** with a small gap between them, which is what a 牌坊's beam really
//!   is — a stack and not a single beam;
//! - **three roofs**, one per bay, the middle one **a step higher** than its neighbours, which is what gives the type its silhouette;
//! - **the plaque**: a blank slab in the middle bay between the lintels, where the inscription goes — blank because a text is not a solid.
//!
//! # What it does not draw
//!
//! - **the inscription**: a 牌坊's meaning is written on it and this crate draws no letters; the plaque is the stone the letters would be cut in;
//! - **the carvings**: the dragons and the clouds are relief and a relief is a surface, not a solid.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The bays: the centre's half width, a side bay's width, and where the four posts stand.
const CENTRE: f32 = 2.30;
const SIDE: f32 = 1.55;
/// The posts: their section, the middle pair's height and the outer pair's.
const POST: f32 = 0.46;
const POST_HIGH: f32 = 5.40;
const SIDE_HIGH: f32 = 4.35;
/// The 夹杆石: how wide and deep a drum is, and how high.
const DRUM: f32 = 0.92;
const DRUM_HIGH: f32 = 1.05;
/// The lintels: how many per bay, their depth and their thickness, and the gap between the pair.
const LINTELS: usize = 2;
const LINTEL: f32 = 0.34;
const LINTEL_THICK: f32 = 0.30;
const LINTEL_GAP: f32 = 0.62;
/// The roofs: how far each oversails its own bay, their thickness, and how far the middle one stands above its neighbours.
const EAVE: f32 = 0.42;
const ROOF: f32 = 0.34;
const STEP_UP: f32 = 0.55;
/// The plaque in the middle bay: its half width, its height and where it hangs.
const PLAQUE: f32 = 1.10;
const PLAQUE_HIGH: f32 = 0.95;
/// The plaque's own thickness either way from the bay's face.
const PLAQUE_WIDE: f32 = 0.22;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.60;

/// Builds the scene: from the road, square on, because a 牌坊 is a face and is read from in front of it.
pub fn paifang() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [0.0, 3.9, 15.6],
        [0.0, 3.2, 0.0],
        FIELD_OF_VIEW,
        "paifang",
    )
}

/// Returns the document: four posts on their drums, three bays of lintels and three roofs.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The four posts**, each standing on its **夹杆石** — the stone drum a free-standing post is held by.
    let middle = CENTRE + POST * 0.5;
    let outer = CENTRE + SIDE + POST * 0.5;
    for x in [-outer, -middle, middle, outer] {
        let high = if x.abs() > middle + 1.0e-3 {
            SIDE_HIGH
        } else {
            POST_HIGH
        };
        skeleton.members.push(building::Member::new(
            building::Anchor::Free { at: [x, 0.0, 0.0] },
            building::Anchor::Free { at: [x, high, 0.0] },
            [1.0, 0.0],
            POST,
            POST,
        ));
        // **The drum**: a short thick stone at the post's foot, buried deeper on the road's side.
        let mut drum = Mass::new(
            ops::wound(vec![
                [x - DRUM * 0.5, -DRUM * 0.5],
                [x + DRUM * 0.5, -DRUM * 0.5],
                [x + DRUM * 0.5, DRUM * 0.5],
                [x - DRUM * 0.5, DRUM * 0.5],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, 0.0))
            .collect(),
            DRUM_HIGH,
        );
        // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
        drum.wall = DRUM * 0.22;
        drum.slab = 0.14;
        skeleton.masses.push(drum);
    }

    // **The three bays**: a wide centre and two sides, each with its pair of lintels and its own roof.
    for (from, to, high, roof_high) in [
        (
            -middle + POST * 0.5,
            middle - POST * 0.5,
            POST_HIGH,
            POST_HIGH + STEP_UP,
        ),
        (
            -outer + POST * 0.5,
            -middle - POST * 0.5,
            SIDE_HIGH,
            SIDE_HIGH + STEP_UP,
        ),
        (
            middle + POST * 0.5,
            outer - POST * 0.5,
            SIDE_HIGH,
            SIDE_HIGH + STEP_UP,
        ),
    ] {
        for lintel in 0..LINTELS {
            let up = high - LINTEL_GAP * 0.5 - LINTEL_THICK * (lintel as f32 + 0.5);
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [from - POST * 0.5, up, 0.0],
                },
                building::Anchor::Free {
                    at: [to + POST * 0.5, up, 0.0],
                },
                [0.0, 1.0],
                LINTEL,
                LINTEL_THICK,
            ));
        }
        // **The roof** over the bay, standing on the lintels' own top.
        // **The roof sits on the top lintel and not above the posts**: a 牌坊`s roof is a roof between the posts, and a plate floating above them is what
        // the first draft drew (measured: the three roofs read as three lines in the sky).
        let sits = high - LINTEL_GAP * 0.5 - LINTEL_THICK * LINTELS as f32;
        skeleton.slabs.push(building::Slab::roof(
            ops::wound(vec![
                [from - EAVE, -LINTEL * 0.5 - EAVE],
                [to + EAVE, -LINTEL * 0.5 - EAVE],
                [to + EAVE, LINTEL * 0.5 + EAVE],
                [from - EAVE, LINTEL * 0.5 + EAVE],
            ]),
            sits,
            sits + ROOF + if roof_high > high { STEP_UP } else { 0.0 },
        ));
    }

    // **The plaque**: the blank stone in the middle bay, where the inscription is cut. Blank because a text is not a solid.
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [-PLAQUE, -PLAQUE_WIDE],
            [PLAQUE, -PLAQUE_WIDE],
            [PLAQUE, PLAQUE_WIDE],
            [-PLAQUE, PLAQUE_WIDE],
        ]),
        POST_HIGH - LINTEL_GAP * 0.5 - LINTEL_THICK * LINTELS as f32 - PLAQUE_HIGH,
        POST_HIGH - LINTEL_GAP * 0.5 - LINTEL_THICK * LINTELS as f32,
    ));

    Building::new(skeleton)
}
