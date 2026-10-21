//! A **屯堡村**: four 石板房 along a stone lane, with the compound's wall behind them and a 朝门 where the lane leaves.
//!
//! # What this scene is for
//!
//! The gallery's **fourth settlement** and the one the twelfth type belongs in. The mountain village is walled houses round a
//! square, the Dai hamlet is a row on a valley floor, the plateau village is four towers on one shelf — a 屯堡 is a
//! **garrison's compound**: one street, two rows of the same house, a wall at the back and a **朝门** at the end, which is
//! the one building a 屯堡 has that is not a house.
//!
//! # What it composes
//!
//! - **four `tunpu::document()`s** in a row, all facing the lane, so their yards are the lane's edge;
//! - **the compound's wall behind them**, two masses with the gate's gap between them: a wall with a gap is two walls, and a
//!   mass whose plan holds another mass's is a mass with a host;
//! - **the 朝门**: two stone piers and a lean-to roof over the gap, which is the one place a 屯堡's lane is a door.
//!
//! # What it does not draw
//!
//! - **the 碉楼** at the village's ends, which is a tower and not a house;
//! - **the second row**: a 屯堡's street has houses on both sides, and one row is this scene's frame. A second row facing the
//!   other way would put its yards under the first's eaves.

use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, derive, site};
use definition::Value;

/// The houses' plots, all facing the lane.
const PLOTS: [f32; 4] = [-16.5, -5.5, 5.5, 16.5];
const HOUSE_Z: f32 = -3.0;

/// The compound's wall: behind the houses, with the gate's gap in it.
const WALL_Z: f32 = 7.0;
/// **A wall`s wall must be thinner than half its own depth**, because the inner loop is offset inward by it: a mass 0.9 m
/// deep with a 0.55 m wall folds on itself, and the crate answered `OffsetEdgeFolded { edge: 1 }` and
/// `OffsetInverted { area: -4.02 }` for each length.
const WALL_HALF: f32 = 0.55;
const WALL_HIGH: f32 = 2.10;
const WALL_THICK: f32 = 0.40;
const WALL_FROM: f32 = -23.0;
const WALL_TO: f32 = 23.0;

/// The 朝门: the gap's width, its two piers and the roof over them.
const GATE_HALF: f32 = 1.80;
const PIER: f32 = 0.45;
const GATE_HEAD: f32 = 2.60;
/// The 门框 the gate`s leaves hang in: its own square section, in metres.
const FRAME: f32 = 0.13;
const GATE_RISE: f32 = 0.55;
const GATE_EAVE: f32 = 0.70;

/// The 碉楼 at the lane`s west end: a square stone tower of two storeys, with arrow slits and a crenellated top.
const TOWER_AT: [f32; 2] = [-27.0, 7.0];
const TOWER_HALF: f32 = 1.60;
const TOWER_STOREY: f32 = 2.80;
const TOWER_WALL: f32 = 0.50;
const TOWER_MERLON: f32 = 0.35;
const TOWER_MERLON_HIGH: f32 = 0.75;

const SUN: [f64; 3] = [-24.0, 28.0, 20.0];
const SUN_IRRADIANCE: f64 = 3.3;
const SHADOW_EXTENT: f64 = 48.0;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the village: four houses, the compound's wall and the gate.
pub fn tunpu_village() -> Value {
    let building = document();
    crate::village::scene_of(
        &building,
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        // **Obliquely down the lane**, so the row recedes and the 朝门 stands between the houses instead of being hidden
        // by them: a camera square to the row sees a wall and four roofs, which is what the first frame was.
        [17.0, 3.8, 15.0],
        [-5.0, 2.2, -2.0],
        FIELD_OF_VIEW,
        "tunpu_village",
    )
}

/// Returns the document: four houses, two lengths of wall and the gate between them.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    for across in PLOTS {
        let mut part = crate::tunpu::document().skeleton;
        building::ops::translate(&mut part, [across, HOUSE_Z]);
        let seated = site::seat(&mut part, &|_: building::math::Vec2| 0.0, 0.0);
        assert_eq!(seated.len(), 1, "a 石板房 is one stone mass");
        assert!(
            building::ops::merge(&mut skeleton, part),
            "a house of the compound places no parts, so the merge always happens"
        );
    }

    // **The compound's wall is two lengths, not one with a hole in it**: a mass with a gap is two masses, and the gap is
    // the gate's own width. Each stands clear of the houses' plans, because a mass inside a mass is a mass with a host.
    let lengths = [(WALL_FROM, -GATE_HALF), (GATE_HALF, WALL_TO)];
    let mut left = MassId(0);
    for (index, (from, to)) in lengths.into_iter().enumerate() {
        let mut wall = Mass::new(
            [
                PlanNode::new([from, WALL_Z - WALL_HALF], 0.0),
                PlanNode::new([to, WALL_Z - WALL_HALF], 0.0),
                PlanNode::new([to, WALL_Z + WALL_HALF], 0.0),
                PlanNode::new([from, WALL_Z + WALL_HALF], 0.0),
            ]
            .to_vec(),
            WALL_HIGH,
        );
        wall.wall = WALL_THICK;
        wall.slab = 0.25;
        let id = MassId(skeleton.masses.len());
        if index == 0 {
            left = id;
        }
        skeleton.masses.push(wall);
    }

    // **The 朝门: two piers and a roof.** The piers stand on the wall's own line at the gap's ends, and the roof is a lean-to
    // that sheds into the lane — a 朝门 is a gate and not a tower.
    // **The piers stand in the gap and not in the wall.** On the wall`s own line they are masses inside masses — measured as
    // `UnattachedOverlap { mass: 4, other: 6 }` and `{ mass: 5, other: 7 }` — and a pier that touches the wall`s end gives
    // its area to no one. Set back by their own width, the clear opening is `2 * (GATE_HALF - PIER)`, which is a gate.
    for across in [-(GATE_HALF - PIER * 0.5), GATE_HALF - PIER * 0.5] {
        let mut pier = Mass::new(
            [
                PlanNode::new([across - PIER * 0.5, WALL_Z - WALL_HALF], 0.0),
                PlanNode::new([across + PIER * 0.5, WALL_Z - WALL_HALF], 0.0),
                PlanNode::new([across + PIER * 0.5, WALL_Z + WALL_HALF], 0.0),
                PlanNode::new([across - PIER * 0.5, WALL_Z + WALL_HALF], 0.0),
            ]
            .to_vec(),
            GATE_HEAD,
        );
        // The pier is 0.45 square, so its own wall is a third of that and not all of it.
        pier.wall = 0.18;
        pier.slab = 0.22;
        skeleton.masses.push(pier);
    }
    // **The 门框: two jambs, a head and a threshold, lining the opening in timber.** The stone piers are the gate's mass and the
    // frame is its joinery: a gate whose leaves hang on bare stone has nothing to hang on, and a gate you walk through without
    // stepping over anything is not a gate. The frame stands `FRAME` square, just inside the opening, and the leaves lie
    // against it.
    let inner = GATE_HALF - PIER;
    let face = WALL_Z + WALL_HALF - 0.12;
    let head = GATE_HEAD - 0.22;
    for across in [-1.0_f32, 1.0] {
        let x = across * (inner - FRAME * 0.5);
        skeleton.members.push(building::Member::new(
            building::Anchor::Free { at: [x, 0.0, face] },
            building::Anchor::Free {
                at: [x, head, face],
            },
            [1.0, 0.0],
            FRAME,
            FRAME,
        ));
    }
    // **The head**, from one jamb to the other; and **the threshold**, the same length a hand's height off the paving, which is
    // what a foot steps over and what a cart is stopped by.
    for (y, high) in [(head, FRAME * 0.9), (0.0, 0.18)] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [-inner, y, face],
            },
            building::Anchor::Free {
                at: [inner, y, face],
            },
            [0.0, 1.0],
            FRAME * 1.3,
            high,
        ));
    }

    // **The 朝门's two leaves, standing open against the piers** — the same panel `derive::leaf` builds for the 白族巷's gate.
    // A 朝门 closes: that is what makes a garrison's street closable, and a gate whose opening is drawn empty is a hole in a
    // wall. Each leaf is a pier's width and the opening's height, a hand's thickness, lying flat against the pier's inner face.
    for across in [-1.0_f32, 1.0] {
        let hinge = across * (GATE_HALF - PIER);
        let towards = -across;
        if let Some(panel) = derive::leaf(
            [hinge, 0.10, WALL_Z + WALL_HALF - 0.02],
            PIER,
            GATE_HEAD - 0.22,
            0.07,
            [0.0, towards],
        ) {
            skeleton.members.push(panel);
        }
    }
    let span = GATE_HALF + PIER * 0.5 + GATE_EAVE;
    let plan = [
        [-span, WALL_Z + WALL_HALF + GATE_EAVE],
        [span, WALL_Z + WALL_HALF + GATE_EAVE],
        [span, WALL_Z - WALL_HALF],
        [-span, WALL_Z - WALL_HALF],
    ];
    if let Some(roof) = derive::lean_to(left, plan, GATE_HEAD - 0.12, GATE_HEAD + GATE_RISE, 0.14) {
        skeleton.roofs.push(roof);
    }

    // **The 碉楼: a square tower at the lane's west end**, two stone storeys with a crenellated top. It is the one building
    // a 屯堡 has that is neither a house nor a wall — the garrison's watch — and it is built the way this crate allows a
    // tall mass: **two masses, the upper carried by the lower through `Join::Storey`**, so each takes its own cuts.
    let [across, along] = TOWER_AT;
    let square = |elevation: f32| {
        [
            PlanNode::new([across - TOWER_HALF, along - TOWER_HALF], elevation),
            PlanNode::new([across + TOWER_HALF, along - TOWER_HALF], elevation),
            PlanNode::new([across + TOWER_HALF, along + TOWER_HALF], elevation),
            PlanNode::new([across - TOWER_HALF, along + TOWER_HALF], elevation),
        ]
        .to_vec()
    };
    let mut lower = Mass::new(square(0.0), TOWER_STOREY);
    lower.wall = TOWER_WALL;
    lower.slab = 0.30;
    let lower_id = MassId(skeleton.masses.len());
    skeleton.masses.push(lower);
    let mut upper = Mass::new(square(TOWER_STOREY), TOWER_STOREY * 2.0);
    upper.wall = TOWER_WALL;
    upper.slab = 0.30;
    upper.attachment = Some(building::Attachment {
        host: lower_id,
        join: building::Join::Storey,
    });
    let upper_id = MassId(skeleton.masses.len());
    skeleton.masses.push(upper);

    // **An arrow slit in every face of both storeys.** A slit's sill is a metre and a half up, and its insets stand a
    // wall's thickness in from the bay's ends — the invariant round 69 measured, because the loop an opening is cut into is
    // mitred inward.
    let slit = building::Inset::new(1.15, 1.15, 1.55, 1.00);
    for mass in [lower_id, upper_id] {
        for edge in 0..4 {
            let face = building::FaceRef::new(mass, edge);
            let split = building::ops::split_face(
                &mut skeleton,
                face,
                &building::Rhythm::Even { count: 1 },
                &building::Rhythm::Even { count: 1 },
            );
            if split.is_none() {
                continue;
            }
            let _ =
                building::ops::add_opening(&mut skeleton, building::Opening::new(face, 0, 0, slit));
        }
    }

    // **The crenellation: a merlon on every edge of the top**, which is four members and not a parapet — a parapet is what
    // the 碉房 wears and a merlon is what a garrison does.
    let crest = TOWER_STOREY * 2.0 + 0.02;
    let corners = [
        [across - TOWER_HALF, along - TOWER_HALF],
        [across + TOWER_HALF, along - TOWER_HALF],
        [across + TOWER_HALF, along + TOWER_HALF],
        [across - TOWER_HALF, along + TOWER_HALF],
    ];
    for index in 0..4 {
        let from = corners[index];
        let to = corners[(index + 1) % 4];
        // **The section is measured across the merlon's own axis**, which is the perpendicular of the edge it follows: a
        // section laid along the axis has no width at all, and the crate answers `MemberDegenerate { length: 3.2 }` — the
        // length reading like a degenerate length and being the merlon's own.
        let axis = [to[0] - from[0], to[1] - from[1]];
        let run = (axis[0] * axis[0] + axis[1] * axis[1]).sqrt();
        let across = [-axis[1] / run, axis[0] / run];
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [from[0], crest, from[1]],
            },
            building::Anchor::Free {
                at: [to[0], crest, to[1]],
            },
            across,
            TOWER_MERLON,
            TOWER_MERLON_HIGH,
        ));
    }

    Building::new(skeleton)
}
