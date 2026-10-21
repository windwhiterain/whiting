//! A **白族三坊一照壁**: the courtyard of Dali — a two-storey 正房 at its head, a 厢房 on each side, and the **照壁**
//! closing the fourth, which is what the type is named for.
//!
//! # What this scene is for
//!
//! The gallery's **thirteenth type**. The 一颗印 is a Yunnan courtyard that is a *box* — four ranges round a light well. A
//! 三坊一照壁 is a courtyard with **three ranges and one screen wall**: the sun comes in over the wall, the wall is white
//! plastered and painted, and its **tiled coping** is the type's silhouette. Two ranges and a screen wall are the "two 坊 and
//! one 照壁" this scene draws; the third range would stand where the camera is.
//!
//! # The two facts the type has
//!
//! - **The 照壁 is a wall and not a building**, so it is a mass `0.5 m` thick with a `0.18 m` wall in it and a gable roof for
//!   its coping — a wall with a little roof on it is exactly what a 照壁 is.
//! - **The 正房 is two storeys and the 厢房 one**, which is the Bai hierarchy: the head of the court is taller. The 正房 is
//!   two masses carried through `Join::Storey` so each storey takes its own cuts; the 厢房 are single masses.
//!
//! # What it does not draw
//!
//! - **the third 坊** and the 门楼 in the corner, which are the courtyard's other two walls;
//! - **the painted panels** a 照壁 carries, which are a texture and not a shape.

use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, derive, ops};
use definition::Value;

/// The court: `HALF_X` by `HALF_Z` inside the ranges, and the 照壁 on its south side.
const COURT: f32 = 4.20;
const HEAD_DEPTH: f32 = 5.60;
const HEAD_WIDTH: f32 = 9.00;
const WING_DEPTH: f32 = 5.00;
const WING_WIDTH: f32 = 4.40;
const STOREY: f32 = 2.90;
const WALL: f32 = 0.50;
const PLATE: f32 = 0.30;

/// The ranges' roofs and the 照壁's coping.
const OVERHANG: f32 = 0.55;
const RISE: f32 = 1.45;
const SINK: f32 = 0.08;
const ROOF_THICKNESS: f32 = 0.18;
/// The 照壁's own dimensions: `0.5 m` thick, so its `0.18 m` wall cannot fold.
const SCREEN_HALF: f32 = 3.90;
const SCREEN_THICK: f32 = 0.50;
const SCREEN_HIGH: f32 = 3.10;
const SCREEN_WALL: f32 = 0.18;
const SCREEN_RISE: f32 = 0.45;
const SCREEN_EAVE: f32 = 0.28;

const TILES: Covering = Covering {
    period: 0.19,
    fill: 0.72,
    height: 0.045,
};

const SUN: [f64; 3] = [-20.0, 34.0, 26.0];
const SUN_IRRADIANCE: f64 = 3.1;
const SHADOW_EXTENT: f64 = 32.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 55.0;

/// Builds the court: from the 照壁's own corner, so the wall is in the foreground and the 正房 behind the court.
pub fn bai_court() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        // **From beyond the 照壁, looking through it into the court**: the wall is the type`s signature and the view it was
        // built for is the one from the approach — its coping in the foreground and the 正房`s upper storey behind.
        [11.0, 9.4, -19.5],
        [0.0, 1.6, 2.2],
        FIELD_OF_VIEW,
        "bai_court",
    )
}

/// Returns the document: the 正房, the two 厢房, the court's paving and the 照壁.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let head = COURT + HEAD_DEPTH * 0.5;

    // **The 正房: two storeys, the lower carried by the ground and the upper by the lower**, so each takes its own cuts.
    let square = |half_x: f32, half_z: f32, z: f32, elevation: f32| {
        ops::wound(vec![
            [-half_x, z - half_z],
            [half_x, z - half_z],
            [half_x, z + half_z],
            [-half_x, z + half_z],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, elevation))
        .collect::<Vec<_>>()
    };
    let mut lower = Mass::new(
        square(HEAD_WIDTH * 0.5, HEAD_DEPTH * 0.5, head, 0.0),
        STOREY,
    );
    lower.wall = WALL;
    lower.slab = PLATE;
    let lower_id = MassId(skeleton.masses.len());
    skeleton.masses.push(lower);
    let mut upper = Mass::new(
        square(HEAD_WIDTH * 0.5, HEAD_DEPTH * 0.5, head, STOREY),
        STOREY * 2.0,
    );
    upper.wall = WALL;
    upper.slab = PLATE;
    upper.attachment = Some(building::Attachment {
        host: lower_id,
        join: building::Join::Storey,
    });
    let upper_id = MassId(skeleton.masses.len());
    skeleton.masses.push(upper);

    // **The two 厢房, one storey each**, standing on the court's east and west sides and facing it.
    let mut wings = [MassId(0); 2];
    for (index, across) in [-1.0_f32, 1.0].into_iter().enumerate() {
        let x = across * (COURT + WING_WIDTH * 0.5);
        let mut wing = Mass::new(
            ops::wound(vec![
                [x - WING_WIDTH * 0.5, -WING_DEPTH * 0.5],
                [x + WING_WIDTH * 0.5, -WING_DEPTH * 0.5],
                [x + WING_WIDTH * 0.5, WING_DEPTH * 0.5],
                [x - WING_WIDTH * 0.5, WING_DEPTH * 0.5],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            STOREY,
        );
        wing.wall = WALL;
        wing.slab = PLATE;
        wings[index] = MassId(skeleton.masses.len());
        skeleton.masses.push(wing);
    }

    // **The 照壁: a wall with a roof on it**, which is what a screen wall is. It stands on the court's south side, clear of
    // the 厢房's plans, and its coping is a gable over a half-metre span.
    let south = -COURT - SCREEN_THICK * 0.5;
    let mut screen = Mass::new(
        ops::wound(vec![
            [-SCREEN_HALF, south - SCREEN_THICK * 0.5],
            [SCREEN_HALF, south - SCREEN_THICK * 0.5],
            [SCREEN_HALF, south + SCREEN_THICK * 0.5],
            [-SCREEN_HALF, south + SCREEN_THICK * 0.5],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        SCREEN_HIGH,
    );
    screen.wall = SCREEN_WALL;
    screen.slab = 0.22;
    let screen_id = MassId(skeleton.masses.len());
    skeleton.masses.push(screen);

    // **The court is paved**, which is what makes the four walls a court: one slab inside them.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-COURT, -COURT],
            [COURT, -COURT],
            [COURT, COURT],
            [-COURT, COURT],
        ]),
        0.0,
        0.08,
    ));

    // **A roof over each range and a coping over the wall.** `gable_roof` reads the mass's own plan, so the 照壁's coping is
    // the same call as the 正房's roof at a different span.
    for (id, rise, sink, thickness) in [
        (upper_id, RISE, SINK, ROOF_THICKNESS),
        (wings[0], RISE * 0.72, SINK, ROOF_THICKNESS),
        (wings[1], RISE * 0.72, SINK, ROOF_THICKNESS),
        (screen_id, SCREEN_RISE, 0.04, 0.12),
    ] {
        let overhang = if id == screen_id {
            SCREEN_EAVE
        } else {
            OVERHANG
        };
        if let Some(roof) =
            derive::gable_roof(id, &skeleton.masses[id.0], overhang, rise, sink, thickness)
        {
            skeleton.roofs.push(roof.with_covering(TILES));
        }
    }

    // **The 正房's front is three bays**, a door in the middle and a window either side — cut the way round 69 measured, the
    // row before the column and the inset a wall's thickness in from the bay's ends.
    let front = building::FaceRef::new(lower_id, 2);
    let columns = building::Rhythm::Even { count: 3 };
    let rows = building::Rhythm::Even { count: 1 };
    if building::ops::split_face(&mut skeleton, front, &columns, &rows).is_some() {
        for column in 0..3 {
            let inset = if column == 1 {
                building::Inset::new(0.75, 0.75, 0.0, 1.20)
            } else {
                building::Inset::new(0.60, 0.60, 1.00, 1.30)
            };
            let _ = building::ops::add_opening(
                &mut skeleton,
                building::Opening::new(front, 0, column, inset),
            );
        }
    }
    // **Each 厢房 has one door facing the court**, on the face that looks across it.
    for (index, edge) in [(0usize, 1usize), (1, 3)] {
        let face = building::FaceRef::new(wings[index], edge);
        if building::ops::split_face(
            &mut skeleton,
            face,
            &building::Rhythm::Even { count: 1 },
            &building::Rhythm::Even { count: 1 },
        )
        .is_some()
        {
            let _ = building::ops::add_opening(
                &mut skeleton,
                building::Opening::new(face, 0, 0, building::Inset::new(1.10, 1.10, 0.0, 1.30)),
            );
        }
    }

    Building::new(skeleton)
}
