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
/// The 丽江 gate: how wide a gap beside the screen it stands in, and how deep the head lies on the wall's own head.
const GATE_CLEAR: f32 = 2.40;
const GATE_HEAD: f32 = 0.46;

/// The 厦子 in front of the 正房: its half width, its edge (where its posts stand), how far back it reaches, its height and its posts.
const XIAZI_HALF: f32 = 5.20;
const XIAZI_EDGE: f32 = -COURT_Z + 2.30;
const XIAZI_BACK: f32 = -COURT_Z + 0.10;
const XIAZI_HIGH: f32 = 3.05;
const XIAZI_POSTS: usize = 4;
const XIAZI_POST: f32 = 0.17;

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
    // **And the lengths that close the rest of the south side.** The left one runs from the screen's end to the corner; the right one starts
    // **`GATE_CLEAR` past the screen** and leaves the gap a 丽江 gate stands in — because a court closed on four sides is a court with no
    // door in it, which is what round 162 shipped.
    for side in [-1.0_f32, 1.0] {
        let inner = side * SCREEN * 0.5;
        let gate = side * (SCREEN * 0.5 + GATE_CLEAR);
        let (from, _) = if side < 0.0 {
            (inner, gate)
        } else {
            (gate, inner)
        };
        let mut wall = Mass::new(
            ops::wound(vec![
                [from, COURT_Z - SCREEN_THICK * 0.5],
                [side * (COURT_X + 0.45), COURT_Z - SCREEN_THICK * 0.5],
                [side * (COURT_X + 0.45), COURT_Z + SCREEN_THICK * 0.5],
                [from, COURT_Z + SCREEN_THICK * 0.5],
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

    // **The 丽江门楼: the gate house in that gap.** A stone head across it, resting on the screen's end and the wall's end; two leaves that
    // close it; and a lean-to over it that sheds into the lane — the head is the host and the roof is `derive::lean_to`'s, which is the
    // pair every gate in this project has used since round 89.
    let gate_x = SCREEN * 0.5 + GATE_CLEAR * 0.5;
    let mut head = Mass::new(
        ops::wound(vec![
            [SCREEN * 0.5, COURT_Z - SCREEN_THICK * 0.5],
            [SCREEN * 0.5 + GATE_CLEAR, COURT_Z - SCREEN_THICK * 0.5],
            [SCREEN * 0.5 + GATE_CLEAR, COURT_Z + SCREEN_THICK * 0.5],
            [SCREEN * 0.5, COURT_Z + SCREEN_THICK * 0.5],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, SOUTH_HIGH))
        .collect(),
        SOUTH_HIGH + GATE_HEAD,
    );
    head.wall = SCREEN_THICK * 0.32;
    head.slab = 0.14;
    let head_id = building::MassId(skeleton.masses.len());
    skeleton.masses.push(head);
    for side in [-1.0_f32, 1.0] {
        if let Some(leaf) = building::derive::leaf(
            [gate_x + side * (GATE_CLEAR * 0.5 - 0.05), 0.10, COURT_Z],
            GATE_CLEAR * 0.5 - 0.10,
            SOUTH_HIGH - 0.24,
            0.07,
            [-side, 0.0],
        ) {
            skeleton.members.push(leaf);
        }
    }
    let plan = [
        [SCREEN * 0.5 - 0.30, COURT_Z - 1.05],
        [SCREEN * 0.5 + GATE_CLEAR + 0.30, COURT_Z - 1.05],
        [SCREEN * 0.5 + GATE_CLEAR + 0.30, COURT_Z + 1.05],
        [SCREEN * 0.5 - 0.30, COURT_Z + 1.05],
    ];
    if let Some(roof) = building::derive::lean_to(
        head_id,
        plan,
        SOUTH_HIGH + GATE_HEAD,
        SOUTH_HIGH + GATE_HEAD + 0.75,
        0.13,
    ) {
        skeleton.roofs.push(roof);
    }
    let _ = screen_id;

    // **The 厦子: the deep veranda in front of the 正房.** A 纳西 house is not a box with a porch — its front is a **second roof**, lower and
    // shallower, carried on a row of posts and reaching out over the court. It is what a 丽江 court is seen by from inside, and the court
    // plate's whole near edge was bare without it.
    //
    // It is **posts and a roof slab** and not a `lean_to`: a lean-to needs a host mass and this veranda is carried by its own row, which is
    // what a 厦子 is.
    for index in 0..XIAZI_POSTS {
        let x = -XIAZI_HALF + 2.0 * XIAZI_HALF * index as f32 / (XIAZI_POSTS - 1) as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, 0.0, XIAZI_EDGE],
            },
            building::Anchor::Free {
                at: [x, XIAZI_HIGH, XIAZI_EDGE],
            },
            [1.0, 0.0],
            XIAZI_POST,
            XIAZI_POST,
        ));
    }
    // **The corners wind counter-clockwise in `(x, z)`**: listed the other way the veranda's slab came out inside out
    // (`SolidInsideOut { volume: -1.94 }`, `SolidOpen { loops: 2 }`), which is a winding and not a shape.
    skeleton.slabs.push(building::Slab::roof(
        vec![
            [-XIAZI_HALF - 0.35, XIAZI_BACK],
            [XIAZI_HALF + 0.35, XIAZI_BACK],
            [XIAZI_HALF + 0.35, XIAZI_EDGE - 0.45],
            [-XIAZI_HALF - 0.35, XIAZI_EDGE - 0.45],
        ],
        XIAZI_HIGH,
        XIAZI_HIGH + 0.15,
    ));

    Building::new(skeleton)
}
