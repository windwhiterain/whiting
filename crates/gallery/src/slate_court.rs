//! A **石板房's court**: the house, its yard, the stone wall that closes the yard and the 门楼 over the gate.
//!
//! # What this scene is for
//!
//! A 屯堡 house does not stand on the lane; it stands **behind a wall in its own court**, and the wall is the same stone as
//! the house. The type's own scene shows the house and its yard and stops there — the wall, the gate and the enclosure are
//! what makes the yard a 院 and not a forecourt.
//!
//! # What it composes
//!
//! - **`tunpu::document()`**, the house, with its yard reaching `DEPTH/2 + YARD` from its own centre;
//! - **three lengths of wall** round the rest of the yard, each a mass with a top and two faces, standing clear of the
//!   house's plan;
//! - **the 门楼**: the gate's two piers in the gap (in it, not on the wall's line) and a lean-to roof over them, which is the
//!   one piece of this scene that is not stone.
//!
//! # What it does not draw
//!
//! - **the second range**: a courtyard house has ranges on its other sides, and a 屯堡 house's court has one house and a
//!   wall — anything more would be the 一颗印 and not this.

use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, derive};
use definition::Value;

/// The court's inner half extents: the house is 8.2 wide, so the wall stands 1.1 m clear of its flanks.
const COURT_X: f32 = 5.20;
/// How far the wall's line stands from the house's own centre along `z`: past the yard, which ends at 5.60.
const COURT_Z: f32 = 6.60;

/// The wall: 1.0 deep, so its own 0.35 wall is well under half of it.
const WALL_HALF: f32 = 0.50;
const WALL_HIGH: f32 = 2.30;
const WALL_THICK: f32 = 0.35;

/// The gate: a gap in the south wall, its two piers and the roof over them.
const GATE_HALF: f32 = 1.30;
const PIER: f32 = 0.40;
const GATE_HEAD: f32 = 2.45;
const GATE_RISE: f32 = 0.50;
const GATE_EAVE: f32 = 0.50;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.60;

/// Builds the court: from the gate's own corner, so the wall runs away and the house stands at the end of the yard.
pub fn slate_court() -> Value {
    let building = document();
    crate::village::scene_of(
        &building,
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        // **From outside the wall and above it**, so the court is read as a court and not as a corner of a wall: a camera
        // inside the yard sees the gate`s roof fill the frame.
        [9.5, 5.4, 18.5],
        [-0.8, 2.0, 1.2],
        FIELD_OF_VIEW,
        "slate_court",
    )
}

/// Returns the document: the house, three lengths of wall and the gate.
pub fn document() -> Building {
    let mut skeleton = crate::tunpu::document().skeleton;

    // **The wall is three lengths, not one loop**: the house itself closes the north side, and a mass that ran through the
    // house's plan would be a mass inside a mass — the refusal round 67 measured, `UnattachedOverlap` against the house.
    // Each length stands from the house's own flank to the court's corner, and the south one leaves the gate's gap.
    // **The side walls stop at the south wall`s inner face and the south wall spans the whole width**: two lengths that
    // each run to the corner share the corner`s area, and the crate answers `UnattachedOverlap { mass: 1, other: 3 }` and
    // `{ mass: 2, other: 4 }` for exactly that. Meeting face to face is the join a wall has.
    let inner = COURT_Z - WALL_HALF;
    let outer = COURT_X + WALL_HALF;
    let lengths = [
        // (from, to): the west flank, the east flank, and the south side in two pieces.
        ([-COURT_X, -2.0], [-COURT_X, inner]),
        ([COURT_X, -2.0], [COURT_X, inner]),
        ([-outer, COURT_Z], [-GATE_HALF, COURT_Z]),
        ([GATE_HALF, COURT_Z], [outer, COURT_Z]),
    ];
    let mut south = MassId(0);
    for (index, (from, to)) in lengths.into_iter().enumerate() {
        // **A wall is a rectangle whose long axis is the run**, and its depth across that run is `2 * WALL_HALF`: the inner
        // loop is offset inward by the wall's own thickness, so the wall must be thinner than the depth or it folds
        // (`OffsetEdgeFolded`, round 70).
        let across = [-((to[1] - from[1]).signum()), (to[0] - from[0]).signum()];
        let normal = [across[0] * WALL_HALF, across[1] * WALL_HALF];
        let plan = if from[0] == to[0] {
            vec![
                [from[0] - normal[0], from[1]],
                [from[0] + normal[0], from[1]],
                [to[0] + normal[0], to[1]],
                [to[0] - normal[0], to[1]],
            ]
        } else {
            vec![
                [from[0], from[1] - normal[1]],
                [from[0], from[1] + normal[1]],
                [to[0], to[1] + normal[1]],
                [to[0], to[1] - normal[1]],
            ]
        };
        // **A plan is wound positively, and which way that is depends on the run's own direction**: the same four corners
        // listed for a wall running east and one running west wind opposite ways, and the crate answers `OffsetInverted`
        // with the plan's area (`-15.81` for the west length, `-7.82` for the short one). `ops::wound` decides it, and that
        // rule lives in the crate now instead of in this scene.
        let mut wall = Mass::new(
            building::ops::wound(plan)
                .into_iter()
                .map(|at| PlanNode::new(at, 0.0))
                .collect(),
            WALL_HIGH,
        );
        wall.wall = WALL_THICK;
        wall.slab = 0.22;
        let id = MassId(skeleton.masses.len());
        if index == 2 {
            south = id;
        }
        skeleton.masses.push(wall);
    }

    // **The gate's piers stand in the gap**, set back by their own width, so they touch the wall's ends without sharing
    // area with them — on the wall's line they would be masses inside masses (round 70).
    for across in [-(GATE_HALF - PIER * 0.5), GATE_HALF - PIER * 0.5] {
        let mut pier = Mass::new(
            [
                PlanNode::new([across - PIER * 0.5, COURT_Z - WALL_HALF], 0.0),
                PlanNode::new([across + PIER * 0.5, COURT_Z - WALL_HALF], 0.0),
                PlanNode::new([across + PIER * 0.5, COURT_Z + WALL_HALF], 0.0),
                PlanNode::new([across - PIER * 0.5, COURT_Z + WALL_HALF], 0.0),
            ]
            .to_vec(),
            GATE_HEAD,
        );
        pier.wall = 0.18;
        pier.slab = 0.20;
        skeleton.masses.push(pier);
    }
    let span = GATE_HALF + PIER * 0.5 + GATE_EAVE;
    let plan = [
        [-span, COURT_Z + WALL_HALF + GATE_EAVE],
        [span, COURT_Z + WALL_HALF + GATE_EAVE],
        [span, COURT_Z - WALL_HALF],
        [-span, COURT_Z - WALL_HALF],
    ];
    if let Some(roof) = derive::lean_to(south, plan, GATE_HEAD - 0.10, GATE_HEAD + GATE_RISE, 0.13)
    {
        skeleton.roofs.push(roof);
    }

    Building::new(skeleton)
}
