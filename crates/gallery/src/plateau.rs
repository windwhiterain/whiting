//! A **高原小村**: four 碉房 on a plateau, two facing the lane and two turned away from it, with the trees a household
//! plants and the wall that holds the terrace up.
//!
//! # What this scene is for
//!
//! The gallery's **third settlement**, and the one the eleventh type belongs in. The mountain village is a terrace village
//! of walled houses round a square; the Dai hamlet is a row on a valley floor; a plateau village is neither — its ground is
//! **one shelf at one height**, it has no square, and its houses are **one type at one size**, four of them, which is what a
//! 碉房 village is: a handful of towers in sight of one another and of nothing else.
//!
//! # What it composes
//!
//! - **four `tibetan::document()`s**, each turned to its own plot: the two south of the lane face it, the two north face
//!   away, so every gallery is on the sunny side and every lane front is a wall;
//! - **two trees**, from the same `derive::tree` the hamlet uses, because a plateau household plants one;
//! - **one retaining wall**, which is what makes the shelf a shelf: a terrace on a slope is held by something, and this
//!   village's north edge is a course of stone one metre high.
//!
//! # What it does not draw
//!
//! - **the fields**: a plateau village's barley is the agricultural arc's work;
//! - **the 玛尼堆 and the prayer flags are drawn** (round 217): a stone heap and a rope of cloths, out of masses and members. This bullet said they

use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, ops, site};
use definition::Value;

/// **One row, all four facing south**, which is what a plateau household does: the gallery is the only timber and it goes
/// where the sun is, whatever the lane does. The first draft put two rows either side of the lane and every outside camera
/// saw one row`s back wall.
const PLOTS: [(f32, f32, f32); 4] = [
    (-16.5, -2.0, 0.0),
    (-5.5, -2.0, 0.0),
    (5.5, -2.0, 0.0),
    (16.5, -2.0, 0.0),
];

/// The retaining wall along the north edge of the shelf.
/// The wall stands clear of the north row, because a retaining wall inside a house`s plan is a mass inside a mass:
/// measured as `UnattachedOverlap { mass: 4..7, other: 8 }` when it was laid on the row`s own line.
const WALL_ALONG: f32 = 22.0;
const WALL_AT: f32 = -10.5;
const WALL_HALF: f32 = 1.2;
const WALL_HIGH: f32 = 1.0;
const WALL_THICK: f32 = 0.55;

/// The 玛尼堆: where it stands in the lane, its half width, its height, how many stones stand on it and how big one is.
const MANI_AT: [f32; 2] = [10.40, 3.60];
const MANI_HALF: f32 = 0.85;
const MANI_HIGH: f32 = 0.95;
const MANI_STONES: usize = 7;
const MANI_STONE: f32 = 0.20;

/// The prayer flags: where the line stands, how long, the poles' height and section, the rope's section, how many cloths and how far each hangs.
const FLAGS_AT: [f32; 2] = [-10.60, 3.80];
const FLAGS_LONG: f32 = 7.40;
const FLAGS_HIGH: f32 = 3.30;
const FLAG_POLE: f32 = 0.10;
const FLAG_ROPE: f32 = 0.03;
const FLAGS: usize = 9;
const FLAG_DROP: f32 = 0.46;
const FLAG_CLOTH: f32 = 0.20;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the village: four towers, two trees and the wall that holds the shelf.
pub fn plateau_village() -> Value {
    let building = document();
    let mut plants =
        crate::settlement_plants::trees(&[[-22.0, -3.0], [22.0, -3.0]], &|at: [f32; 2]| {
            grade(at[0] as f64, at[1] as f64) as f32
        });
    plants.merge(crate::settlement_plants::bamboo_clumps(
        &[[-11.0, 2.0], [0.0, 3.0], [11.0, 2.0]],
        &|at: [f32; 2]| grade(at[0] as f64, at[1] as f64) as f32,
    ));
    crate::village::scene_of(
        &building,
        &plants,
        &grade,
        [9.0, 6.0, 25.0],
        [0.0, 4.6, -2.0],
        FIELD_OF_VIEW,
        "plateau_village",
    )
}

/// Returns the ground: the shelf, and the fall off its north edge.
///
/// # Invariants
///
/// **The shelf is level and the lane is the only thing off it.** A plateau village stands on one height; what falls away
/// is the ground beyond the retaining wall, which is what the wall is for.
fn grade(x: f64, z: f64) -> f64 {
    let _ = x;
    if z > 11.0 {
        // Beyond the wall the ground drops a metre and a half, which is the terrace's own height.
        -1.5 * ((z - 11.0) / 6.0).min(1.0)
    } else {
        0.0
    }
}

/// Returns the document: four towers and the retaining wall.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    for (across, along, turn) in PLOTS {
        let mut part = crate::tibetan::document().skeleton;
        if turn.abs() > 1.0e-3 {
            building::ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        building::ops::translate(&mut part, [across, along]);
        // **Each house is seated on the shelf under it**, and its posts are what the seat opens: the same pair of calls
        // every settlement in this crate makes, read once per house.
        let seated = site::seat(
            &mut part,
            &|at: building::math::Vec2| grade(at[0] as f64, at[1] as f64) as f32,
            0.0,
        );
        // **A tower house is five masses now** — a ground course, three battered courses and the third storey — and the
        // settlement seats each of them. The assertion names what it needs: a house with no seated mass is a house that
        // floats, and a count would have to be rewritten every time the type is laid differently.
        assert!(
            !seated.is_empty(),
            "a 碉房 is at least one stone mass, and this one seated {}",
            seated.len()
        );
        let _ = site::posts(
            &mut part,
            &|at: building::math::Vec2| grade(at[0] as f64, at[1] as f64) as f32,
            2.0,
            0.18,
            0.06,
        );
        assert!(
            building::ops::merge(&mut skeleton, part),
            "a house of the plateau places no parts, so the merge always happens"
        );
    }

    // **The retaining wall: one mass along the north edge of the shelf.** It is a mass and not a member because it holds
    // ground rather than carrying a roof, and a wall with two faces and a top is a mass's own shape.
    let mut wall = Mass::new(
        [
            PlanNode::new([-WALL_ALONG, WALL_AT - WALL_HALF], -WALL_HIGH),
            PlanNode::new([WALL_ALONG, WALL_AT - WALL_HALF], -WALL_HIGH),
            PlanNode::new([WALL_ALONG, WALL_AT + WALL_HALF], -WALL_HIGH),
            PlanNode::new([-WALL_ALONG, WALL_AT + WALL_HALF], -WALL_HIGH),
        ]
        .to_vec(),
        0.0,
    );
    wall.wall = WALL_THICK;
    wall.slab = 0.20;
    let wall_id = MassId(skeleton.masses.len());
    let _ = wall_id;
    skeleton.masses.push(wall);
    // **The 玛尼堆 and the prayer flags.** This file said they *"need a decorator this crate does not have"*, and round 215 found the same kind of note
    // to be false in another file. They need **no decorator at all**: a 玛尼堆 is a pile of stones, which is a low mass with a few stones standing on it,
    // and prayer flags are a rope between two poles with cloth tied along it, which is a row of members. Both are drawn here in the village's own
    // vocabulary — mass, slab, member — and neither asked the crate for anything it does not have.
    let [mani_x, mani_z] = MANI_AT;
    // **The heap**: a battered pile, drawn as a mass whose ring is offset well inside half its depth.
    let mut mani = Mass::new(
        ops::wound(vec![
            [mani_x - MANI_HALF, mani_z - MANI_HALF],
            [mani_x + MANI_HALF, mani_z - MANI_HALF],
            [mani_x + MANI_HALF, mani_z + MANI_HALF],
            [mani_x - MANI_HALF, mani_z + MANI_HALF],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        MANI_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
    mani.wall = MANI_HALF * 0.55;
    mani.slab = 0.12;
    skeleton.masses.push(mani);
    // **The stones that have been put on it**, one by one: a scatter of small members standing on the heap's own top, which is what a 玛尼堆 is — a pile
    // that grows by a stone at a time.
    for stone in 0..MANI_STONES {
        let angle = core::f32::consts::TAU * stone as f32 / MANI_STONES as f32;
        let at = [
            mani_x + MANI_HALF * 0.42 * angle.cos(),
            mani_z + MANI_HALF * 0.42 * angle.sin(),
        ];
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [at[0], MANI_HIGH, at[1]],
            },
            building::Anchor::Free {
                at: [at[0], MANI_HIGH + MANI_STONE, at[1]],
            },
            [1.0, 0.0],
            MANI_STONE,
            MANI_STONE,
        ));
    }
    // **The prayer flags**: two poles and a rope, with a cloth tied along it every `FLAG_GAP`. The rope sags in a real one; a member is straight, so the
    // cloth is hung from a straight line and the flags themselves are what says the thing is a 经幡.
    let [flag_x, flag_z] = FLAGS_AT;
    for side in [-1.0_f32, 1.0] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [flag_x + side * FLAGS_LONG * 0.5, 0.0, flag_z],
            },
            building::Anchor::Free {
                at: [flag_x + side * FLAGS_LONG * 0.5, FLAGS_HIGH, flag_z],
            },
            [0.0, 1.0],
            FLAG_POLE,
            FLAG_POLE,
        ));
    }
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [flag_x - FLAGS_LONG * 0.5, FLAGS_HIGH - FLAG_POLE, flag_z],
        },
        building::Anchor::Free {
            at: [flag_x + FLAGS_LONG * 0.5, FLAGS_HIGH - FLAG_POLE, flag_z],
        },
        [0.0, 1.0],
        FLAG_ROPE,
        FLAG_ROPE,
    ));
    for flag in 0..FLAGS {
        let at = flag_x - FLAGS_LONG * 0.5 + FLAGS_LONG * (flag as f32 + 0.5) / FLAGS as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [at, FLAGS_HIGH - FLAG_POLE, flag_z],
            },
            building::Anchor::Free {
                at: [at, FLAGS_HIGH - FLAG_POLE - FLAG_DROP, flag_z],
            },
            [1.0, 0.0],
            FLAG_CLOTH,
            FLAG_CLOTH,
        ));
    }

    Building::new(skeleton)
}
