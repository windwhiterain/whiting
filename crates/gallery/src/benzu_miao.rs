//! A **白族本主庙**: the village's own temple — a 大殿 on its plinth at the head of a court, a 戏台 facing it across the
//! paving, and the walls that close the two sides.
//!
//! # What this scene is for
//!
//! The gallery's **fourteenth type**, and the first that is a *public* building. A 本主庙 is where a 白族 village keeps its
//! 本主 — its own protecting deity — and it has two buildings because the village has two duties: the 大殿 where the god is
//! and the 戏台 where the god is *entertained*. That pair facing each other across a court is the whole type.
//!
//! # The four facts the type has
//!
//! - **the 大殿 stands on a plinth** and the 戏台 on a lower one: a temple's floor is a step above the court it looks on;
//! - **the 大殿's front is a colonnade**, not a wall: the god is behind it and the village stands in front of it;
//! - **the 戏台 is open on its court side**: its mouth is a hole the width of the stage, because it is a stage;
//! - **the two are on one axis**, and the courtyard between them is the nave.
//!
//! # What it does not draw
//!
//! - **the god**: an image is a sculpture and not a building;
//! - **the 山门**, which stands where the camera is.

use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, derive, ops};
use definition::Value;

/// The court: `COURT_X` each side of the axis, and the two buildings at `HALL_Z` and `STAGE_Z`.
const COURT_X: f32 = 6.60;
const HALL_Z: f32 = 7.60;
const STAGE_Z: f32 = -7.20;

/// The 大殿: ten metres across, six deep, one tall storey on a half-metre plinth.
const HALL_WIDTH: f32 = 10.00;
const HALL_DEPTH: f32 = 6.00;
const HALL_HIGH: f32 = 4.30;
const HALL_PLINTH: f32 = 0.50;

/// The 戏台: seven across, five and a half deep, a little lower and on a lower plinth.
const STAGE_WIDTH: f32 = 7.00;
const STAGE_DEPTH: f32 = 5.40;
const STAGE_HIGH: f32 = 3.60;
const STAGE_PLINTH: f32 = 0.90;

const WALL: f32 = 0.50;
const PLATE: f32 = 0.30;

/// The roofs: a temple's are bigger than a house's, and they are where the type's rank shows.
const OVERHANG: f32 = 0.85;
const HALL_RISE: f32 = 2.10;
const STAGE_RISE: f32 = 1.55;
const SINK: f32 = 0.10;
const ROOF_THICKNESS: f32 = 0.22;

/// The walls that close the court's two sides.
const SIDE_THICK: f32 = 0.40;
const SIDE_WALL: f32 = 0.16;
const SIDE_HIGH: f32 = 2.60;

const TILES: Covering = Covering {
    period: 0.20,
    fill: 0.74,
    height: 0.05,
};

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the temple: from the court's south-west corner, so the 戏台 is at one side and the 大殿 at the other.
pub fn benzu_miao() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        // **High and three-quarters from the south-west**: the 戏台`s mouth faces the 大殿 and a low camera from the south sees
        // only its back, so the shot looks over its roof into the court and down onto the 大殿`s colonnade.
        [17.0, 13.0, -18.0],
        [-0.5, 2.0, 1.2],
        FIELD_OF_VIEW,
        "benzu_miao",
    )
}

/// Returns the document: the two plinths, the 大殿, the 戏台, the court's paving and the two side walls.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    let rectangle = |half_x: f32, half_z: f32, x: f32, z: f32, elevation: f32| {
        ops::wound(vec![
            [x - half_x, z - half_z],
            [x + half_x, z - half_z],
            [x + half_x, z + half_z],
            [x - half_x, z + half_z],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, elevation))
        .collect::<Vec<_>>()
    };

    // **The 大殿 on its plinth.** The plinth is a floor slab a little larger than the hall, because a temple's floor is a
    // step above the court it looks on.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [-HALL_WIDTH * 0.5 - 0.65, HALL_Z - HALL_DEPTH * 0.5 - 0.65],
            [HALL_WIDTH * 0.5 + 0.65, HALL_Z - HALL_DEPTH * 0.5 - 0.65],
            [HALL_WIDTH * 0.5 + 0.65, HALL_Z + HALL_DEPTH * 0.5 + 0.65],
            [-HALL_WIDTH * 0.5 - 0.65, HALL_Z + HALL_DEPTH * 0.5 + 0.65],
        ]),
        0.0,
        HALL_PLINTH,
    ));
    let mut hall = Mass::new(
        rectangle(HALL_WIDTH * 0.5, HALL_DEPTH * 0.5, 0.0, HALL_Z, HALL_PLINTH),
        HALL_PLINTH + HALL_HIGH,
    );
    hall.wall = WALL;
    hall.slab = PLATE;
    let hall_id = MassId(skeleton.masses.len());
    skeleton.masses.push(hall);

    // **The 戏台 on its own plinth**, facing the hall. Its plinth is higher, because a stage's floor is a stage's floor.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [
                -STAGE_WIDTH * 0.5 - 0.55,
                STAGE_Z - STAGE_DEPTH * 0.5 - 0.55,
            ],
            [STAGE_WIDTH * 0.5 + 0.55, STAGE_Z - STAGE_DEPTH * 0.5 - 0.55],
            [STAGE_WIDTH * 0.5 + 0.55, STAGE_Z + STAGE_DEPTH * 0.5 + 0.55],
            [
                -STAGE_WIDTH * 0.5 - 0.55,
                STAGE_Z + STAGE_DEPTH * 0.5 + 0.55,
            ],
        ]),
        0.0,
        STAGE_PLINTH,
    ));
    let mut stage = Mass::new(
        rectangle(
            STAGE_WIDTH * 0.5,
            STAGE_DEPTH * 0.5,
            0.0,
            STAGE_Z,
            STAGE_PLINTH,
        ),
        STAGE_PLINTH + STAGE_HIGH,
    );
    stage.wall = WALL;
    stage.slab = PLATE;
    let stage_id = MassId(skeleton.masses.len());
    skeleton.masses.push(stage);

    // **A roof over each and the court paved between them**, which is what makes the two buildings one temple.
    for (id, rise) in [(hall_id, HALL_RISE), (stage_id, STAGE_RISE)] {
        if let Some(roof) = derive::gable_roof(
            id,
            &skeleton.masses[id.0],
            OVERHANG,
            rise,
            SINK,
            ROOF_THICKNESS,
        ) {
            skeleton.roofs.push(roof.with_covering(TILES));
        }
    }
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-COURT_X, STAGE_Z + STAGE_DEPTH * 0.5 + 0.55],
            [COURT_X, STAGE_Z + STAGE_DEPTH * 0.5 + 0.55],
            [COURT_X, HALL_Z - HALL_DEPTH * 0.5 - 0.65],
            [-COURT_X, HALL_Z - HALL_DEPTH * 0.5 - 0.65],
        ]),
        0.0,
        0.06,
    ));

    // **The two side walls**, from the 戏台's plinth to the 大殿's, each a mass thinner than half its own depth.
    let from = STAGE_Z + STAGE_DEPTH * 0.5 + 0.55;
    let to = HALL_Z - HALL_DEPTH * 0.5 - 0.65;
    for across in [-COURT_X - SIDE_THICK * 0.5, COURT_X + SIDE_THICK * 0.5] {
        let mut wall = Mass::new(
            ops::wound(vec![
                [across - SIDE_THICK * 0.5, from],
                [across + SIDE_THICK * 0.5, from],
                [across + SIDE_THICK * 0.5, to],
                [across - SIDE_THICK * 0.5, to],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            SIDE_HIGH,
        );
        wall.wall = SIDE_WALL;
        wall.slab = 0.20;
        skeleton.masses.push(wall);
    }

    // **The 大殿's front is a colonnade**: three bays, and the middle one is a door two bays wide, because the god is behind
    // it and the village stands in front of it. The face that looks down the court is the plan's `-z` side, `edge 0`.
    let front = building::FaceRef::new(hall_id, 0);
    if building::ops::split_face(
        &mut skeleton,
        front,
        &building::Rhythm::Even { count: 3 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        for column in 0..3 {
            let inset = building::Inset::new(0.62, 0.62, 0.0, 1.10);
            let _ = building::ops::add_opening(
                &mut skeleton,
                building::Opening::new(front, 0, column, inset),
            );
        }
    }

    // **The 戏台's mouth**: one opening the width of the stage, on the face that looks at the 大殿 (`+z`, `edge 2`).
    let mouth = building::FaceRef::new(stage_id, 2);
    if building::ops::split_face(
        &mut skeleton,
        mouth,
        &building::Rhythm::Even { count: 1 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        let _ = building::ops::add_opening(
            &mut skeleton,
            building::Opening::new(mouth, 0, 0, building::Inset::new(0.62, 0.62, 0.0, 0.70)),
        );
    }

    Building::new(skeleton)
}
