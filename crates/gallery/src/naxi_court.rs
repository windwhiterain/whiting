//! A **纳西院落**: a courtyard of three timber houses round a paved court, with a screen wall facing the chief one — the **三坊一照壁**
//! of the 丽江 basin.
//!
//! # What this scene is for
//!
//! The gallery's **eighteenth type** and its first from the **northwest of Yunnan**. The 白族 build the same three-sided court across the
//! mountain and the gallery already has theirs (`bai_court`); the 纳西's differs in three things that show: the houses are **taller and
//! shallower** with a deep front veranda, the court is **paved**, and the screen wall opposite the chief house is **flat and white with a
//! tiled cap**, not a gate.
//!
//! # What it composes
//!
//! Three `chuandou::document()`s — a 纳西 house is a timber frame with a deep eave, and the 穿斗 type is the gallery's own timber frame —
//! set on three sides of the court and turned to face into it, plus the court's paving and the screen wall.
//!
//! # What it does not draw
//!
//! - **the 丽江 gate house**: the south side is a wall with a gap, because a 纳西 gate is a small roofed house of its own and it deserves
//!   its own round rather than a gap with a name;
//! - **the water channel**: a 丽江 court often runs water through it, and water is the landscape arc's work.

use building::{Building, BuildingSkeleton, Mass, PlanNode, ops};
use definition::Value;

/// The court: half its width in `x` and half its depth in `z`, and the paving's thickness.
const COURT_X: f32 = 11.60;
const COURT_Z: f32 = 9.40;
const PAVING: f32 = 0.06;
/// Where the three houses stand, and which way each is turned so its porch faces the court.
const HOUSES: [(f32, f32, f32); 3] = [
    // The 正房, across the court's far side, facing south into it.
    (-0.40, -COURT_Z - 4.10, core::f32::consts::PI),
    // The two 厢房, facing each other across the court.
    (-COURT_X - 3.90, 0.60, -core::f32::consts::FRAC_PI_2),
    (COURT_X + 3.90, 0.60, core::f32::consts::FRAC_PI_2),
];
/// The screen wall on the court's near side: how wide, how thick and how high, and its cap.
const SCREEN: f32 = 9.20;
const SCREEN_THICK: f32 = 0.42;
const SCREEN_HIGH: f32 = 3.10;
const SCREEN_CAP: f32 = 0.16;
/// The south wall either side of the screen, and its height.
const SOUTH_HIGH: f32 = 2.30;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the scene: from the court's south-east, where the three roofs and the screen read as one room without a ceiling.
pub fn naxi_court() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [26.0, 12.5, 30.0],
        [0.0, 4.2, -1.0],
        FIELD_OF_VIEW,
        "naxi_court",
    )
}

/// Returns the document: three houses, the court's paving, the screen wall and the south wall.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The three houses.** Each is the 穿斗 type's own document, turned so its porch faces the court.
    for (across, along, turn) in HOUSES {
        let mut part = crate::chuandou::document().skeleton;
        if turn.abs() > 1.0e-3 {
            ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        ops::translate(&mut part, [across, along]);
        assert!(
            ops::merge(&mut skeleton, part),
            "a house of the court places no parts, so the merge always happens"
        );
    }

    // **The court's paving**, which is what makes three houses a court and not three houses.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-COURT_X, -COURT_Z],
            [COURT_X, -COURT_Z],
            [COURT_X, COURT_Z],
            [-COURT_X, COURT_Z],
        ]),
        0.0,
        PAVING,
    ));

    // **The 照壁: the screen wall opposite the 正房.** It is the court's fourth side and it is **not a gate**: a white wall with a tiled
    // cap, standing where the 白族 put theirs, and the thing a 纳西 court is recognised by from inside.
    let mut screen = Mass::new(
        ops::wound(vec![
            [-SCREEN * 0.5, COURT_Z - SCREEN_THICK],
            [SCREEN * 0.5, COURT_Z - SCREEN_THICK],
            [SCREEN * 0.5, COURT_Z],
            [-SCREEN * 0.5, COURT_Z],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        SCREEN_HIGH,
    );
    // **The ring's offset stays under half the wall's depth** — the depth is `SCREEN_THICK`, the offset is a fraction of it (round 144).
    screen.wall = SCREEN_THICK * 0.36;
    screen.slab = 0.18;
    let screen_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(screen);
    // Its cap: a roof slab over its head, proud of it either side, which is the shape a 照壁's tiled top takes.
    skeleton.slabs.push(building::Slab::roof(
        vec![
            [-SCREEN * 0.5 - 0.22, COURT_Z - SCREEN_THICK - 0.10],
            [SCREEN * 0.5 + 0.22, COURT_Z - SCREEN_THICK - 0.10],
            [SCREEN * 0.5 + 0.22, COURT_Z + 0.12],
            [-SCREEN * 0.5 - 0.22, COURT_Z + 0.12],
        ],
        SCREEN_HIGH,
        SCREEN_HIGH + SCREEN_CAP,
    ));
    // **And the two lengths that close the rest of the south side**, from the screen's ends out to the court's corners.
    for side in [-1.0_f32, 1.0] {
        let inner = side * SCREEN * 0.5;
        let outer = side * (COURT_X + 0.45);
        let mut wall = Mass::new(
            ops::wound(vec![
                [inner, COURT_Z - SCREEN_THICK * 0.5],
                [outer, COURT_Z - SCREEN_THICK * 0.5],
                [outer, COURT_Z + SCREEN_THICK * 0.5],
                [inner, COURT_Z + SCREEN_THICK * 0.5],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            SOUTH_HIGH,
        );
        wall.wall = SCREEN_THICK * 0.30;
        wall.slab = 0.16;
        skeleton.masses.push(wall);
    }
    let _ = screen_id;

    Building::new(skeleton)
}
