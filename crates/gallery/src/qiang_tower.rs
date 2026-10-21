//! A **羌族碉楼**: the stone watchtower of the Qiang country — a square tower that batters inward as it rises, eight courses of
//! coursed stone to a small crown, with its door in the air.
//!
//! # What this scene is for
//!
//! The gallery's **seventeenth type**, and the first from the **northwest** of the region: 羌区 is the 岷江's upper valley, where the
//! houses are stone and the towers are the thing the country is known by. Every other type here is timber or earth; this one is
//! **coursed stone with a batter**, and the batter is the whole form.
//!
//! # The construction
//!
//! `derive::stepped_courses` lays a stack of masses that share an edge with the one below and no volume, which is exactly what a
//! battered tower is: each course a square a little smaller than the one under it, carried by it through [`building::Join::Storey`].
//! The **door is high** — a 碉楼's entrance is reached by a plank taken up behind the entrant, which is the reason the type exists at
//! all — so it is cut in the second course and nothing leads to it.
//!
//! # What it does not draw
//!
//! - **the 碉房 houses** round it: they are the `tibetan` type's own building and putting one here would make this a settlement. The
//!   tower is drawn alone, on its ground, which is how a plate reads its height;
//! - **the battlements**: a 碉楼's crown has a wall with gaps in it, and this crate's `crown_gables` is a roof's own edge. The crown
//!   here is the top course's wall, and its gaps are the one thing the type still owes.

use building::{Building, BuildingSkeleton, Covering, derive, ops};
use definition::Value;

/// The base: a square, `BASE` a side, standing on a plinth two steps up out of the ground.
const BASE: f32 = 6.20;
const PLINTH: f32 = 0.55;
/// Eight courses, each `COURSE` high and set in `BATTER` further than the one below — the profile a stone tower keeps its plumb by.
const COURSES: usize = 8;
const COURSE: f32 = 2.55;
const BATTER: f32 = 0.30;
/// The wall's thickness, which grows as the courses shrink: this is the fraction of the course's width the ring is offset by.
const WALL: f32 = 0.19;
/// The door: how high its sill is above the ground (a 碉楼's door is not reached from the ground), and the course it is cut in.
const DOOR_SILL: f32 = 4.10;
const DOOR_COURSE: usize = 1;

/// The 木梁头: how many to a face, their section, how far they stand proud and how far below a course's top they sit.
const BEAMS: usize = 3;
const BEAM: f32 = 0.15;
const BEAM_PROUD: f32 = 0.24;
const BEAM_DROP: f32 = 0.42;

/// The 垛口 on the crown: how many merlons to a face, what fraction of a bay each fills, and its thickness and height.
const MERLONS: usize = 3;
const MERLON_FILL: f32 = 0.58;
const MERLON_DEEP: f32 = 0.34;
const MERLON_HIGH: f32 = 0.82;

/// The 碉房 beside the tower: where it stands in the towers own coordinates.
const HOUSE_AT: [f32; 2] = [9.80, 2.60];

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.5;

/// Builds the scene: from the ground's own level, close enough that the batter reads.
pub fn qiang_tower() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [27.0, 15.0, 33.0],
        [0.0, 11.5, 0.0],
        FIELD_OF_VIEW,
        "qiang_tower",
    )
}

/// Returns the document: a plinth, eight courses of coursed stone and a door in the air.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = BASE * 0.5;

    // **The plinth**, which is where the tower's batter starts from and what keeps its foot out of the ground.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [-half - 0.45, -half - 0.45],
            [half + 0.45, -half - 0.45],
            [half + 0.45, half + 0.45],
            [-half - 0.45, half + 0.45],
        ]),
        0.0,
        PLINTH,
    ));

    // **The eight courses.** `stepped_courses` wants the base's plan, the height of one course, the stack's own top, how many courses
    // and how far each is set in; it carries each course on the one below and answers their ids in order.
    let base = [[-half, -half], [half, -half], [half, half], [-half, half]];
    let ids = derive::stepped_courses(
        &mut skeleton,
        base,
        PLINTH,
        PLINTH + COURSE * COURSES as f32,
        COURSES,
        BATTER,
        WALL,
        0.22,
    );

    // **The door in the air**, cut in the course the sill falls in: a 碉楼 is entered by a plank that is pulled up behind you, which is
    // why it has a door four metres up and no stair.
    if let Some(id) = ids.get(DOOR_COURSE) {
        // **The door faces the plate**: `edge 1` is the plan`s `+x` side, which is the side the camera stands on — round 97 and
        // round 136 each measured what happens when a face is chosen by the building rather than by the view.
        let face = building::FaceRef::new(*id, 1);
        if ops::split_face(
            &mut skeleton,
            face,
            &building::Rhythm::Even { count: 3 },
            &building::Rhythm::Even { count: 1 },
        )
        .is_some()
        {
            let sill = DOOR_SILL - (PLINTH + COURSE * DOOR_COURSE as f32);
            let _ = ops::add_opening(
                &mut skeleton,
                building::Opening::new(
                    face,
                    0,
                    1,
                    building::Inset::new(0.80, 0.80, sill.max(0.0), 0.75),
                ),
            );
        }
    }

    // **The 木梁头: the beam ends that stand proud of every storey's floor.** A 碉楼's floors are timber and their beams run through the
    // stone, so each storey's face carries a row of **square beam ends** — the tower's one ornament, and the one thing that says a 碉楼
    // has floors in it and is not a solid obelisk. Three to a face on the two faces the plate looks at, at every course's own top.
    for course in 0..COURSES {
        let level = PLINTH + COURSE * (course + 1) as f32 - BEAM_DROP;
        let span = half - BATTER * course as f32;
        for index in 0..BEAMS {
            let at = -span + 2.0 * span * (index as f32 + 0.5) / BEAMS as f32;
            // The `+x` face: the member runs out of it, so its `across` is `z`.
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [span - 0.02, level, at],
                },
                building::Anchor::Free {
                    at: [span + BEAM_PROUD, level, at],
                },
                [0.0, 1.0],
                BEAM,
                BEAM,
            ));
            // The `+z` face: the member runs out of that one, so its `across` is `x`.
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [at, level, span - 0.02],
                },
                building::Anchor::Free {
                    at: [at, level, span + BEAM_PROUD],
                },
                [1.0, 0.0],
                BEAM,
                BEAM,
            ));
        }
    }

    // **The 垛口: the battlements.** Round 150 recorded this as the type's debt — a 碉楼's crown is **a wall with gaps in it**, and a tower
    // that ends in a plain course is a chimney. The merlons are members: both anchors at the course's own top, the section carrying the
    // height, which is how this crate draws a parapet. Three to a face on the two faces the plate looks at, with the gaps between them.
    let crown = half - BATTER * (COURSES - 1) as f32;
    let top = PLINTH + COURSE * COURSES as f32;
    for index in 0..MERLONS {
        let from = -crown + 2.0 * crown * index as f32 / MERLONS as f32;
        let to = from + 2.0 * crown * (index as f32 + MERLON_FILL) / MERLONS as f32;
        // The `+x` face: the parapet runs along `z` and is `MERLON_DEEP` thick in `x`.
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [crown - MERLON_DEEP * 0.5, top, from],
            },
            building::Anchor::Free {
                at: [crown - MERLON_DEEP * 0.5, top, to],
            },
            [1.0, 0.0],
            MERLON_DEEP,
            MERLON_HIGH,
        ));
        // The `+z` face: the same parapet, turned.
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [from, top, crown - MERLON_DEEP * 0.5],
            },
            building::Anchor::Free {
                at: [to, top, crown - MERLON_DEEP * 0.5],
            },
            [0.0, 1.0],
            MERLON_DEEP,
            MERLON_HIGH,
        ));
    }

    // **The crown**: the top course's own crown, so the tower ends in a gabled pair of edges and not in a flat plane.
    if let Some(roof) = skeleton.roofs.last().cloned() {
        if let Some(id) = ids.last() {
            derive::crown_gables(&mut skeleton.masses[id.0], &roof);
        }
    }

    // **The 碉房 beside it.** A 碉楼 is never alone: it stands in a courtyard of stone houses of the same masonry, and a tower on bare
    // ground is a monument. The house is the `tibetan` type's own document — the 羌 and the 藏 build the same coursed-stone box with a
    // flat roof — merged in at the tower's east and turned to face the camera, which is the composition this gallery uses everywhere
    // else: **a settlement is made of types it already has.**
    let mut house = crate::tibetan::document().skeleton;
    ops::rotate(&mut house, [0.0, 0.0], -core::f32::consts::FRAC_PI_2);
    ops::translate(&mut house, [HOUSE_AT[0], HOUSE_AT[1]]);
    assert!(
        ops::merge(&mut skeleton, house),
        "the house places no parts, so the merge always happens"
    );

    Building::new(skeleton)
}
