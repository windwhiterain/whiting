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

    // **The crown**: the top course's own crown, so the tower ends in a gabled pair of edges and not in a flat plane.
    if let Some(roof) = skeleton.roofs.last().cloned() {
        if let Some(id) = ids.last() {
            derive::crown_gables(&mut skeleton.masses[id.0], &roof);
        }
    }

    Building::new(skeleton)
}
