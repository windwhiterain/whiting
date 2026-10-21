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
//! - **the painted panels** a 照壁 carries, which are a texture and not a shape.
//!
//! # What "three ranges" means
//!
//! **The fourth side is the 照壁 and not a range.** The type is named for what it has, so a scene of it has *exactly* three
//! ranges; round 74's own note said the third range "would stand where the camera is", and that was wrong — the camera stands
//! where the screen wall's approach is. What the court was missing is the **门楼**: a 白族 court is entered through the gate in
//! the corner between a 厢房 and the 照壁, and it is a wall with a door in it and a little roof over the door.

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

/// The 门楼: the corner between the east 厢房 and the 照壁, the wall that closes it, and the roof over its door.
const GATE_THICK: f32 = 0.40;
const GATE_WALL: f32 = 0.16;
const GATE_HIGH: f32 = 2.70;
const GATE_EAVE: f32 = 0.55;
const GATE_RISE: f32 = 0.40;
const GATE_DEEP: f32 = 1.10;

/// The well in the court: a stone ring a metre and a bit across, its wall a quarter of that so the hole is a bucket`s.
const WELL_AT: [f32; 2] = [3.10, 3.10];
const WELL_HALF: f32 = 0.62;
const WELL_WALL: f32 = 0.22;
const WELL_HIGH: f32 = 0.80;

/// Where the court plants: the corners the wings leave, and the foot of the 照壁.
const PLANTS: [[f32; 2]; 3] = [[-3.30, 3.10], [-3.20, -3.20], [3.20, -3.10]];

const TILES: Covering = Covering {
    period: 0.19,
    fill: 0.72,
    height: 0.045,
};

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the court: from the 照壁's own corner, so the wall is in the foreground and the 正房 behind the court.
pub fn bai_court() -> Value {
    // **The court's planting**: bamboo at the corners the wings leave and at the foot of the 照壁, from the same bridge the
    // settlements use. A 白族 court grows its flowers against the screen wall, and a court with a well and nothing growing in
    // it is a yard.
    let plants = crate::settlement_plants::bamboo_clumps(&PLANTS, &|_: [f32; 2]| 0.0);
    crate::village::scene_of(
        &document(),
        &plants,
        &|_, _| 0.0,
        // **Along the 照壁 to the gate**: the approach view that showed the wall as a screen now stands off its east end, so
        // the 门楼's own frame — threshold, jambs, lintel and 门罩 — is the foreground and the 正房 rises behind it.
        // **Inside the court, from its south-west corner**: what this scene is a picture of is the *court* — the well, the
        // planting, the 照壁's inner face and the 正房 across the paving — and every shot from beyond the gate showed only the
        // wall that closes it.
        [-3.6, 3.0, -3.2],
        [2.2, 1.8, 4.4],
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

    // **The 门楼: the corner between the east 厢房 and the 照壁.** A 白族 court is entered through a gate in a corner, and the
    // corner is a short wall with a door in it and a roof over the door. The wall runs from the wing's south face to the
    // screen's inner face on the court's east line, so it meets both and shares area with neither.
    let gate_z0 = -COURT;
    let gate_z1 = -WING_DEPTH * 0.5;
    let gate_x = COURT + GATE_THICK * 0.5;
    let mut gate = Mass::new(
        ops::wound(vec![
            [gate_x - GATE_THICK * 0.5, gate_z0],
            [gate_x + GATE_THICK * 0.5, gate_z0],
            [gate_x + GATE_THICK * 0.5, gate_z1],
            [gate_x - GATE_THICK * 0.5, gate_z1],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        GATE_HIGH,
    );
    gate.wall = GATE_WALL;
    gate.slab = 0.20;
    let gate_id = MassId(skeleton.masses.len());
    skeleton.masses.push(gate);

    // **A door through the gate wall**, cut into the face that looks down the approach: one bay, an inset a wall's thickness
    // clear of the bay's ends — round 69's invariant, which the wall's own `0.40 m` depth demands and no more.
    let approach = building::FaceRef::new(gate_id, 3);
    if building::ops::split_face(
        &mut skeleton,
        approach,
        &building::Rhythm::Even { count: 1 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        let _ = building::ops::add_opening(
            &mut skeleton,
            building::Opening::new(approach, 0, 0, building::Inset::new(0.50, 0.50, 0.0, 1.05)),
        );
    }

    // **The door's own frame**: a threshold, two jambs, a lintel and a 门罩 over it. A 白族 gate is framed in stone and wood
    // and the frame is what a person reads at the door — without it the opening is a hole in a wall, which is what round 74
    // and 75 drew. Every member is laid **in the wall's own thickness and proud of it**, because a frame stands in front of
    // what it frames.
    let sill = 0.0_f32;
    let head = GATE_HIGH - 1.05;
    let clear = [gate_z0 + 0.50, gate_z1 - 0.50];
    let middle = gate_x;
    let proud = GATE_THICK * 0.5 + 0.06;
    // The threshold: stone, its top a hand's width above the court.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [middle - proud, sill + 0.14, clear[0]],
        },
        building::Anchor::Free {
            at: [middle + proud, sill + 0.14, clear[1]],
        },
        [1.0, 0.0],
        0.18,
        0.28,
    ));
    // The two jambs: from the threshold to the lintel, one on each side of the opening.
    for z in clear {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [middle - proud, sill + 0.14, z],
            },
            building::Anchor::Free {
                at: [middle + proud, head, z],
            },
            [1.0, 0.0],
            0.16,
            0.16,
        ));
    }
    // The lintel over them, and the 门罩 above it: a band that stands out further than the frame and carries the hood.
    for (height, across, deep, high) in [
        (head + 0.10, 0.10, 0.90, 0.22),
        (head + 0.34, 0.14, 1.05, 0.26),
    ] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [middle + across, height, clear[0] - 0.06],
            },
            building::Anchor::Free {
                at: [middle + across, height, clear[1] + 0.06],
            },
            [1.0, 0.0],
            deep,
            high,
        ));
    }

    // **The gate's hood: a lean-to over the corner**, hosted by the gate wall and shedding into the lane.
    let half = GATE_THICK * 0.5;
    let plan = [
        [gate_x + half + GATE_EAVE, gate_z1],
        [gate_x + half + GATE_EAVE, gate_z0],
        [gate_x - half - GATE_DEEP, gate_z0],
        [gate_x - half - GATE_DEEP, gate_z1],
    ];
    if let Some(hood) =
        derive::lean_to(gate_id, plan, GATE_HIGH - 0.15, GATE_HIGH + GATE_RISE, 0.12)
    {
        skeleton.roofs.push(hood.with_covering(TILES));
    }

    // **The well: a stone ring in the court's paved south-east.** It is a mass whose wall is thick enough to be a ring — the
    // same answer the Dai hamlet's well-head gave — so what the expansion draws is a curb with a hole in plan, which is what a
    // well looks like from beside it. Its wall is `0.22 m` in a `1.24 m` plan, so the inner loop cannot fold (round 73).
    let [across, along] = WELL_AT;
    let mut well = Mass::new(
        ops::wound(vec![
            [across - WELL_HALF, along - WELL_HALF],
            [across + WELL_HALF, along - WELL_HALF],
            [across + WELL_HALF, along + WELL_HALF],
            [across - WELL_HALF, along + WELL_HALF],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        WELL_HIGH,
    );
    well.wall = WELL_WALL;
    well.slab = 0.18;
    skeleton.masses.push(well);

    Building::new(skeleton)
}
