//! A **傣族缅寺**: the village temple of the Dai of Xishuangbanna — a long hall raised on a platform, under a roof whose pitch is
//! the steepest in the gallery, entered through a porch of slender posts.
//!
//! # What this scene is for
//!
//! It is the gallery's **sixteenth type**, and the second public building after the 白族本主庙. Where a 白族 temple is axial and
//! walled, a 缅寺 is **one great roof**: the hall is a single volume, the roof takes two thirds of the building's height, and the
//! walls under it are a screen rather than a structure.
//!
//! # The three facts the type has
//!
//! - **the roof is steep** — `RISE` is more than half the hall's height — because the rain of the tropics is the building's first
//!   problem, and a steep pitch is a fast drain;
//! - **the hall stands on a platform** a metre up, and every approach to a 缅寺 is up steps;
//! - **the porch is posts and not walls**: the front of the hall is open, and the roof comes down over it.
//!
//! # What it does not draw
//!
//! - **the 塔**: a 缅寺's spire stands beside its hall and not on it, and a spire is a solid of revolution this gallery has no
//!   primitive for (the drum tower's own summit is the closest it comes);
//! - **the 佛像**: an image is a sculpture and not a building.

use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, derive, ops};
use definition::Value;

/// The hall: longer than it is wide, because a 缅寺's hall is a nave.
/// **The hall is wider than it is deep, and that is a structural fact and not a taste**: the ridge runs along the longer pair of
/// edges, so the gable ends stand at `±x` and the face the village enters by — the one at `+z` — is a **long side**. Swapped the
/// other way the porch stood against a gable, and the crate answered `GridOnACrownedFace { edge: 3 }`.
const WIDTH: f32 = 13.60;
const DEPTH: f32 = 9.20;
const HEIGHT: f32 = 4.10;
/// The platform it stands on: a metre up, with steps at the front.
const PLINTH: f32 = 1.05;
const PLINTH_PROUD: f32 = 0.85;

/// How far the porch stands out from the front wall, and how many posts carry it.
const PORCH: f32 = 2.30;
const POSTS: usize = 4;
const POST: f32 = 0.19;

/// The walls and the plate: a screen wall, thick enough to be stone and brick in alternate courses.
const WALL: f32 = 0.42;
const PLATE: f32 = 0.30;

/// The roof: the steepest in the gallery, with deep eaves on every side.
const OVERHANG: f32 = 1.45;
const RISE: f32 = 4.60;
const SINK: f32 = 0.12;
const ROOF_THICKNESS: f32 = 0.18;

/// The tiles: small, close, and proud — a tropical roof sheds rather than holds.
const TILES: Covering = Covering {
    period: 0.15,
    fill: 0.76,
    height: 0.05,
};

/// The steps at the porch`s middle: how wide they are and how deep a tread is.
const STEPS_HALF: f32 = 1.70;
const STEP: f32 = 0.32;

/// The compound: how far the yard stands beyond the platform, the walls height, its thickness and the wall inside it, the
/// gates half width, and how thick the yards paving is.
const YARD: f32 = 2.40;
const WALL_HIGH: f32 = 1.55;
const WALL_THICK: f32 = 0.40;
const WALL_WALL: f32 = 0.16;
const GATE_HALF: f32 = 1.60;
const YARD_PAVING: f32 = 0.06;
/// The 寺门`s piers: how wide one is, in metres.
const GATE_PIER: f32 = 0.52;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the temple: from the porch's corner, so the pitch and the platform's height are both seen.
pub fn dai_temple() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [15.5, 6.2, 14.0],
        [-1.4, 4.2, -0.8],
        FIELD_OF_VIEW,
        "dai_temple",
    )
}

/// Returns the document: the platform, the hall, its roof, the porch and the steps.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = WIDTH * 0.5;
    let deep = DEPTH * 0.5;
    let front = deep + PORCH;

    // **The platform**, a floor slab a metre up with a margin round the hall on three sides and the full porch at the front.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [-half - PLINTH_PROUD, -deep - PLINTH_PROUD],
            [half + PLINTH_PROUD, -deep - PLINTH_PROUD],
            [half + PLINTH_PROUD, front + PLINTH_PROUD],
            [-half - PLINTH_PROUD, front + PLINTH_PROUD],
        ]),
        0.0,
        PLINTH,
    ));
    // **The steps up to it**: three treads at the porch's middle, each a third of the platform's height.
    for (index, top) in [(0usize, PLINTH * 0.34), (1, PLINTH * 0.67)] {
        let south = front + PLINTH_PROUD + STEP * index as f32;
        skeleton.slabs.push(building::Slab::floor(
            ops::wound(vec![
                [-STEPS_HALF, south],
                [STEPS_HALF, south],
                [STEPS_HALF, south + STEP],
                [-STEPS_HALF, south + STEP],
            ]),
            0.0,
            top,
        ));
    }

    // **The hall**: one volume, a screen wall and its own openings.
    let mut hall = Mass::new(
        ops::wound(vec![
            [-half, -deep],
            [half, -deep],
            [half, deep],
            [-half, deep],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, PLINTH))
        .collect(),
        PLINTH + HEIGHT,
    );
    hall.wall = WALL;
    hall.slab = PLATE;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(hall);

    // **The porch's posts**, standing on the platform at the front edge, with a beam across their heads.
    for index in 0..POSTS {
        let x = -half + WIDTH * index as f32 / (POSTS - 1) as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, PLINTH, front - POST * 0.5],
            },
            building::Anchor::Free {
                at: [x, PLINTH + HEIGHT, front - POST * 0.5],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-half - POST, PLINTH + HEIGHT - 0.22, front - POST * 0.5],
        },
        building::Anchor::Free {
            at: [half + POST, PLINTH + HEIGHT - 0.22, front - POST * 0.5],
        },
        [0.0, 1.0],
        0.18,
        0.26,
    ));

    // **The roof**: steep, deep-eaved, and crowned at its gable ends so the wall meets it.
    if let Some(roof) = derive::gable_roof_by(
        id,
        &skeleton.masses[id.0],
        OVERHANG,
        OVERHANG + PORCH,
        RISE,
        SINK,
        ROOF_THICKNESS,
    ) {
        skeleton.roofs.push(roof.with_covering(TILES));
    }
    if let Some(roof) = skeleton.roofs.first() {
        derive::crown_gables(&mut skeleton.masses[id.0], roof);
    }

    // **The front's openings**, cut after the crown on the edge whose midpoint stands furthest along `+z`: a door in the middle
    // bay and a window either side.
    let face_index = {
        let outline = skeleton.masses[id.0].outline().to_vec();
        let mut best = (0usize, f32::NEG_INFINITY);
        for index in 0..outline.len() {
            let a = outline[index].position;
            let b = outline[(index + 1) % outline.len()].position;
            let run = (a[0] - b[0]).abs() + (a[1] - b[1]).abs();
            if run > 1.0 && (a[1] + b[1]) * 0.5 > best.1 {
                best = (index, (a[1] + b[1]) * 0.5);
            }
        }
        best.0
    };
    let face = building::FaceRef::new(id, face_index);
    let columns = building::Rhythm::Even { count: 3 };
    let rows = building::Rhythm::Even { count: 1 };
    if ops::split_face(&mut skeleton, face, &columns, &rows).is_some() {
        for column in 0..3 {
            let inset = if column == 1 {
                building::Inset::new(0.55, 0.55, 0.0, 1.15)
            } else {
                building::Inset::new(0.55, 0.55, 1.05, 1.35)
            };
            ops::add_opening(
                &mut skeleton,
                building::Opening::new(face, 0, column, inset),
            )
            .expect("the bay is one the grid has");
        }
    }

    // **The compound: a low wall round the platform and a paved yard inside it.** A 缅寺 is not a hall standing in a field — it
    // is a **wat**, and a wat is an enclosure: a wall a metre and a half high, a gate in its front, and the hall on its platform
    // in the middle of the yard. Five lengths — two sides, a back and two front pieces round the gate — and none of them
    // overlaps the platform, which is a slab and nothing else.
    let yard_x = half + PLINTH_PROUD + YARD;
    let yard_z = deep + PLINTH_PROUD + YARD;
    let half_wall = WALL_THICK * 0.5;
    let lengths: [([f32; 2], [f32; 2]); 5] = [
        // **The sides stop at the back and front pieces' inner faces**, so two lengths never share a corner: a corner two masses
        // both own is two masses in each other's way, which round 72 measured on the 石板房院's walls.
        (
            [-yard_x - half_wall, -yard_z + WALL_THICK],
            [-yard_x + half_wall, yard_z - WALL_THICK],
        ),
        (
            [yard_x - half_wall, -yard_z + WALL_THICK],
            [yard_x + half_wall, yard_z - WALL_THICK],
        ),
        (
            [-yard_x, -yard_z - half_wall],
            [yard_x, -yard_z + half_wall],
        ),
        (
            [-yard_x, yard_z - half_wall],
            [-GATE_HALF, yard_z + half_wall],
        ),
        (
            [GATE_HALF, yard_z - half_wall],
            [yard_x, yard_z + half_wall],
        ),
    ];
    for (from, to) in lengths {
        let mut wall = Mass::new(
            ops::wound(vec![
                [from[0], from[1]],
                [to[0], from[1]],
                [to[0], to[1]],
                [from[0], to[1]],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            WALL_HIGH,
        );
        wall.wall = WALL_WALL;
        wall.slab = 0.18;
        skeleton.masses.push(wall);
    }
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-yard_x, -yard_z],
            [yard_x, -yard_z],
            [yard_x, yard_z],
            [-yard_x, yard_z],
        ]),
        0.0,
        YARD_PAVING,
    ));

    // **The 寺门: the compound's own gate, over the gap the wall leaves.** A lintel is the **host** and the two piers are `Under`
    // it — the crate's reading of a carried mass, which round 89 found by needing it: a mass whose top meets the host's foot and
    // whose plan stands inside the host's plan is a mass that *carries* the host, so the thing over the gap names the things under
    // it. That is the only relation this crate has for a gateway, and it is why a gate is a lintel with piers beneath and not two
    // piers with a beam between.
    //
    // **The gate stands in front of the wall and not in its gap**: a lintel wide enough to contain two piers reaches past the
    // gap`s own edges, and the wall lengths either side of it own that square metre — measured as
    // `UnattachedOverlap { mass: 4, other: 6 }` and `{ 5, 6 }`. A gate porch set `0.62 m` out is the same gate and overlaps
    // nothing.
    let gate_z = yard_z + 0.62;
    let mut lintel = Mass::new(
        ops::wound(vec![
            [-GATE_HALF - 0.55, gate_z - 0.42],
            [GATE_HALF + 0.55, gate_z - 0.42],
            [GATE_HALF + 0.55, gate_z + 0.42],
            [-GATE_HALF - 0.55, gate_z + 0.42],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, WALL_HIGH + 1.20))
        .collect(),
        WALL_HIGH + 1.20 + 0.55,
    );
    lintel.wall = 0.26;
    lintel.slab = 0.20;
    let lintel_id = MassId(skeleton.masses.len());
    skeleton.masses.push(lintel);
    for across in [-1.0_f32, 1.0] {
        let x = across * (GATE_HALF - GATE_PIER * 0.5);
        let mut pier = Mass::new(
            ops::wound(vec![
                [x - GATE_PIER * 0.5, gate_z - 0.36],
                [x + GATE_PIER * 0.5, gate_z - 0.36],
                [x + GATE_PIER * 0.5, gate_z + 0.36],
                [x - GATE_PIER * 0.5, gate_z + 0.36],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            WALL_HIGH + 1.20,
        );
        pier.wall = 0.18;
        pier.slab = 0.16;
        pier.attachment = Some(building::Attachment {
            host: lintel_id,
            join: building::Join::Under,
        });
        skeleton.masses.push(pier);
    }
    // **The gate's roof**: a lean-to over the lintel, shedding away from the yard, hosted by the lintel itself.
    let plan = [
        [-GATE_HALF - 0.95, gate_z + 0.80],
        [GATE_HALF + 0.95, gate_z + 0.80],
        [GATE_HALF + 0.95, gate_z - 0.42],
        [-GATE_HALF - 0.95, gate_z - 0.42],
    ];
    if let Some(roof) = derive::lean_to(
        lintel_id,
        plan,
        WALL_HIGH + 1.20 + 0.55,
        WALL_HIGH + 1.20 + 1.05,
        0.14,
    ) {
        skeleton.roofs.push(roof.with_covering(TILES));
    }

    Building::new(skeleton)
}
