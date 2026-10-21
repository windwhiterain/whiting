//! A **穿斗式民居**: the timber-framed house of Sichuan and Guizhou — a body of board walls, a **colonnade of slender posts**
//! down its front, and a roof whose eaves reach out over them.
//!
//! # What this scene is for
//!
//! It is the gallery's **fifteenth type**, and the one structural tradition the other fourteen had left out. The 抬梁 frame
//! carries its roof on beams that span bay to bay; **穿斗** carries it on a **row of thin posts set close together**, tied by
//! purlins pushed through them, and the wall between the posts is board and not structure. That is why a 穿斗 house's front is
//! a colonnade: the posts are the wall's own rhythm, and the bays between them are opened to the street.
//!
//! # The three facts the type has
//!
//! - **the posts are close**, two metres or less apart, because each one is a point load and the purlin is what spans;
//! - **the eaves are deep**, because a board wall needs the rain kept off it — `OVERHANG` is twice the 竹楼's;
//! - **the front is open**: the ground floor is a colonnade and the boards begin behind it.
//!
//! # What it does not draw
//!
//! - **the 穿枋**: the tie beams that run *through* the posts, which are the detail the tradition is named for — a member laid
//!   through another member's own section is not something this crate's members can say;
//! - **the interior**: a 穿斗 frame's posts continue up through the floors, and this crate has no interior.

use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, derive, ops};
use definition::Value;

/// The house: five bays of a 穿斗 frame, one storey and a loft.
const WIDTH: f32 = 11.00;
const DEPTH: f32 = 7.00;
const HEIGHT: f32 = 3.70;
/// How far the colonnade stands out from the body's wall, and how many posts it is.
const PORCH: f32 = 1.60;
const POSTS: usize = 6;
const POST: f32 = 0.17;

/// The body's boards and its floor plate: a board wall is thin, which is why the eaves are deep.
const WALL: f32 = 0.22;
const PLATE: f32 = 0.26;

/// The roof: a 穿斗 house's eaves are its shelter, so they are deep and the pitch is moderate.
const OVERHANG: f32 = 1.30;
const RISE: f32 = 1.85;
const SINK: f32 = 0.10;
const ROOF_THICKNESS: f32 = 0.16;

/// The tiles: `period` of plan, `fill` of that wide, `height` proud of the skin — small grey 小青瓦.
const TILES: Covering = Covering {
    period: 0.17,
    fill: 0.70,
    height: 0.045,
};

const SUN: [f64; 3] = [-24.0, 32.0, 20.0];
const SUN_IRRADIANCE: f64 = 3.0;
const SHADOW_EXTENT: f64 = 30.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 55.0;

/// Builds the scene: from the front-left, where the colonnade and the eaves' shadow are both seen.
pub fn chuandou_house() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [14.0, 4.6, 15.0],
        [-1.0, 2.4, 0.0],
        FIELD_OF_VIEW,
        "chuandou_house",
    )
}

/// Returns the document: the body, the colonnade and the roof.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = WIDTH * 0.5;
    let deep = DEPTH * 0.5;

    // **The body: one mass of boards**, standing back from the colonnade by `PORCH`. A board wall is thin and it is not the
    // structure — the posts are — so the mass is a wall and its own openings, and the posts stand in front of it.
    let mut body = Mass::new(
        ops::wound(vec![
            [-half, -deep],
            [half, -deep],
            [half, deep],
            [-half, deep],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        HEIGHT,
    );
    body.wall = WALL;
    body.slab = PLATE;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(body);

    // **The colonnade: `POSTS` posts down the front**, each standing its own height and carrying the eaves that the roof
    // already has. Their spacing is `WIDTH / (POSTS - 1)` — the rhythm the board walls behind them take.
    let front = deep + PORCH;
    for index in 0..POSTS {
        let x = -half + WIDTH * index as f32 / (POSTS - 1) as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, 0.0, front - POST * 0.5],
            },
            building::Anchor::Free {
                at: [x, HEIGHT, front - POST * 0.5],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }
    // **The porch's own beam**: one member laid across the posts' heads, which is what a 穿斗 frame's purlin is at this scale.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-half - POST, HEIGHT - 0.18, front - POST * 0.5],
        },
        building::Anchor::Free {
            at: [half + POST, HEIGHT - 0.18, front - POST * 0.5],
        },
        [0.0, 1.0],
        0.16,
        0.22,
    ));

    // **The roof**, with `PORCH` added to its overhang at the front: the eaves of a 穿斗 house reach past the colonnade, which
    // is the whole reason the colonnade can exist.
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

    // **The boards behind the colonnade**: a door in the middle bay and a window either side, cut the way the crate asks —
    // the face split first, then an inset a wall's thickness clear of the bay's ends.
    // **The gable ends rise, then the front is found, then the boards are cut.** A crown splits the wall's own loop, so an edge
    // index named before it points at half a face after it: the crate answered `GridOnACrownedFace`, and the split that came
    // before it had silently made a one-column grid — which is why three hand-cut openings failed by exactly one bay each.
    // The front is the edge whose midpoint stands furthest along `+z`, read after the crown.
    if let Some(roof) = skeleton.roofs.first() {
        derive::crown_gables(&mut skeleton.masses[id.0], roof);
    }
    let front = {
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
    let face = building::FaceRef::new(id, front);
    let columns = building::Rhythm::Even { count: 3 };
    let rows = building::Rhythm::Even { count: 1 };
    ops::split_face(&mut skeleton, face, &columns, &rows)
        .expect("a 穿斗 house's front takes its three bays");
    for column in 0..3 {
        // **An opening stands at least one wall's thickness clear of the face's mitred ends.** A 0.34 m inset in a 0.22 m wall
        // came back `past: 0.048` three times, which is how much it was short; 0.48 is the wall and a hand's width besides.
        let inset = if column == 1 {
            building::Inset::new(0.48, 0.48, 0.0, 1.10)
        } else {
            building::Inset::new(0.48, 0.48, 1.05, 1.05)
        };
        ops::add_opening(
            &mut skeleton,
            building::Opening::new(face, 0, column, inset),
        )
        .expect("the bay is one the grid has");
    }
    Building::new(skeleton)
}
