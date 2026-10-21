//! A **景颇/傈僳干栏长屋**: one house the length of a village street, standing on stilts with its gable roof reaching down over both long sides.
//!
//! # What this scene is for
//!
//! The gallery's **nineteenth type** and the first from **滇西**. The 景颇 and the 傈僳 of the 德宏 hills build the opposite of a courtyard: a
//! **single house twenty metres long**, where a whole extended household lives in one room under one ridge, and the roof is low and wide enough
//! that from outside the house is mostly roof.
//!
//! # How it is built, and what it borrows
//!
//! `derive::starter` builds a gabled body, and this type is that body **lifted onto stilts**: the starter's own skeleton is raised by `LEG`
//! with [`ops::raise`] — round 169's helper, used here for its second purpose — and two rows of posts are dropped under it. **The roof does not
//! need building**: a long gable is a gable with a longer ridge, and the starter already draws the profile, the gable crowns and the covering.
//!
//! # What it does not draw
//!
//! - **the thatch**: the covering is the crate's tile rolls at a coarse period. A thatched ridge is a different material and a different
//!   profile, and it is the one thing this type would be recognised by at a glance;
//! - **the 火塘 and the partition**: a longhouse is divided by low partitions and each household has its own hearth, and this crate has no
//!   interior light to show them (round 121).

use building::derive::{self, RoofStyle, Starter};
use building::ops;
use definition::Value;

/// The house: one long body under one ridge.
const STARTER: Starter = Starter {
    width: 21.0,
    depth: 5.60,
    height: 2.45,
    wall: 0.28,
    slab: 0.26,
    overhang: 1.05,
    sink: 0.12,
    rise: 2.00,
    roof_thickness: 0.24,
    roof: RoofStyle::Gable,
    seed: 9,
};

/// The 干栏: how high the floor stands on its posts, the posts' section, how far apart they are and how far in from the floor's edge.
const LEG: f32 = 1.75;
const POST: f32 = 0.22;
const POST_SPACING: f32 = 2.60;
const POST_INSET: f32 = 0.40;

/// The 独木梯 against the long side: how far along the wall it stands, how wide it is, how far its foot is set out from the floor's edge, a
/// stringer's section and how many rungs it carries.
const LADDER_X: f32 = 6.20;
const LADDER_WIDE: f32 = 0.36;
const LADDER_FOOT: f32 = 1.05;
const LADDER_POST: f32 = 0.09;
const LADDER_RUNGS: usize = 4;

/// The 敞廊 along the long side: its half length, how deep it runs past the wall and how thick its deck is.
const GALLERY_HALF: f32 = 8.20;
const GALLERY: f32 = 0.95;
const GALLERY_DECK: f32 = 0.12;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the scene: from the side and low, because a longhouse is a length and a roof and both are read along the long wall.
pub fn longhouse() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [27.0, 8.5, 24.0],
        [0.0, 4.2, 0.0],
        FIELD_OF_VIEW,
        "longhouse",
    )
}

/// Returns the document: a gabled body raised onto two rows of posts.
pub fn document() -> building::Building {
    let mut skeleton = derive::starter(&STARTER).skeleton;
    // **Up onto the stilts.** The starter builds a body standing on the ground; a 干栏's floor is `LEG` above it, and every part rises
    // together — mass, roof, slab and the gable crowns — which is what `ops::raise` is for.
    building::ops::raise(&mut skeleton, LEG);

    // **The posts.** Two rows under the floor's long edges, as many as the length needs, each from the ground to the floor.
    let half = STARTER.width * 0.5;
    let rows = STARTER.depth * 0.5 - POST_INSET;
    let span = STARTER.width - 2.0 * POST_INSET;
    let bays = (span / POST_SPACING).round().max(1.0) as usize;
    for bay in 0..=bays {
        let x = -half + POST_INSET + span * bay as f32 / bays as f32;
        for z in [-rows, rows] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, 0.0, z] },
                building::Anchor::Free { at: [x, LEG, z] },
                [1.0, 0.0],
                POST,
                POST,
            ));
        }
    }

    // **The 敞廊: the open gallery along the long side.** A 干栏's day happens outside its rooms: the floor runs on past the wall under the same
    // eave, and a row of posts carries its edge. It is **a floor and a row of posts**, and it is what makes the long wall a house instead of a
    // wall — the roof already reaches over it, because a longhouse's eave is `1.05 m` and the gallery is narrower than that.
    let wall = STARTER.depth * 0.5;
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-GALLERY_HALF, wall],
            [GALLERY_HALF, wall],
            [GALLERY_HALF, wall + GALLERY],
            [-GALLERY_HALF, wall + GALLERY],
        ]),
        LEG - GALLERY_DECK,
        LEG,
    ));
    let gallery_bays = (GALLERY_HALF * 2.0 / POST_SPACING).round().max(1.0) as usize;
    for bay in 0..=gallery_bays {
        let x = -GALLERY_HALF + 2.0 * GALLERY_HALF * bay as f32 / gallery_bays as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, 0.0, wall + GALLERY - POST * 0.5],
            },
            building::Anchor::Free {
                at: [x, LEG, wall + GALLERY - POST * 0.5],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }

    // **The 独木梯: the way up to the floor.** A 干栏 has no stair inside it — the household climbs a **notched log** or a plank ladder set against
    // the long side, and takes it up at night. Two stringers and four rungs, from the ground to the floor's edge, leaning on the wall the
    // camera looks at.
    let foot = rows + LADDER_FOOT;
    let head = rows - 0.16;
    for side in [-1.0_f32, 1.0] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [LADDER_X + side * LADDER_WIDE, 0.0, foot],
            },
            building::Anchor::Free {
                at: [LADDER_X + side * LADDER_WIDE, LEG, head],
            },
            [1.0, 0.0],
            LADDER_POST,
            LADDER_POST,
        ));
    }
    for rung in 1..=LADDER_RUNGS {
        let along = rung as f32 / (LADDER_RUNGS + 1) as f32;
        let z = foot + (head - foot) * along;
        let y = LEG * along;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [LADDER_X - LADDER_WIDE, y, z],
            },
            building::Anchor::Free {
                at: [LADDER_X + LADDER_WIDE, y, z],
            },
            [0.0, 1.0],
            LADDER_POST * 0.8,
            LADDER_POST * 1.1,
        ));
    }

    building::Building::new(skeleton)
}
