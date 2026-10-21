//! A **屯堡石板街**: a street of slate houses — two rows facing each other across a flagged lane, and its own well.
//!
//! # What this scene is for
//!
//! The gallery's **sixth settlement**, and the first that is a **street** and not a compound. The 屯堡 village is a garrison's
//! yard with a wall and a 朝门; a 屯堡 *street* is what a growing one becomes: houses on both sides of a flagged lane, each
//! turning its 院坝 onto it, and the lane itself the village's front room.
//!
//! # What it composes
//!
//! - **six `tunpu::document()`s in two rows**, the north row turned a half so both rows face the lane. A 石板房's yard is at
//!   `+z`, so the south row takes no turn and the north row does — one fact about which way a yard looks, exactly as the 白族
//!   lane's gates are placed by one fact about which way a gate looks;
//! - **the lane, laid in flags** across its whole length, because a 屯堡's street is the same stone as its houses;
//! - **a wall at each end** with the lane's own width left open, so the street has two mouths and no third.
//!
//! # What it does not draw
//!
//! - **the gate towers**: a 屯堡 street's 碉楼 stands at the village's corner and not on its street, and the compound scene has
//!   it;
//! - **the culverts** the channel would empty into: a slab drains to its lowest edge and this crate has no word for a mouth.

use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, ops};
use definition::Value;

/// The plots: three to a row, and how far each row stands off the lane's centre line.
const PLOTS: [f32; 3] = [-11.0, 0.0, 11.0];
const ROW: f32 = 9.40;

/// The lane: half its width, and how far it runs past the last house.
const LANE_HALF: f32 = 3.10;
const LANE_FROM: f32 = -26.0;
const LANE_TO: f32 = 26.0;
/// How many flags it is laid in, the joint between them, and how thick a flag is.
const LANE_FLAGS: usize = 18;
const JOINT: f32 = 0.05;
const FLAG: f32 = 0.07;
/// The channel down the lane`s middle: half its width, and how deep it stands below the walked strips.
const CHANNEL: f32 = 0.34;
/// The 街沿石: how wide a kerb strip is and how high it stands above the flags.
const KERB: f32 = 0.26;
const KERB_HIGH: f32 = 0.13;
/// The streets water: where the well stands, its half width, its wall and its curbs height, and the trough beside it.
const WELL_AT: [f32; 2] = [7.4, 2.05];
const WELL_HALF: f32 = 0.72;
const WELL_WALL: f32 = 0.24;
const WELL_HIGH: f32 = 0.82;
/// The 井台`s own height: how far the whole of the street`s water stands above the flags.
const PLINTH: f32 = 0.30;
/// How deep one 踏步 in front of the platform is, in metres.
const STEP: f32 = 0.26;
const TROUGH_LONG: f32 = 1.85;
const TROUGH_DEEP: f32 = 0.62;
const TROUGH_WALL: f32 = 0.16;
const TROUGH_HIGH: f32 = 0.55;
const GUTTER: f32 = 0.015;

/// The walls that close the street's two ends: their height and thickness, and the gap the lane leaves open.
const END_HALF: f32 = 0.45;
const END_HIGH: f32 = 2.30;
const END_THICK: f32 = 0.36;
const MOUTH: f32 = 3.30;
/// The 街门 over the east mouth: how high its wall stands to, how square a pier is, and how far in front of the wall it stands.
const GATE_HEAD: f32 = 3.05;
const GATE_PIER: f32 = 0.52;
const GATE_OUT: f32 = 0.66;

const SUN: [f64; 3] = [-26.0, 30.0, 18.0];
const SUN_IRRADIANCE: f64 = 3.3;
const SHADOW_EXTENT: f64 = 52.0;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the street: from its south mouth, looking north between the two rows.
pub fn slate_street() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        // **Inside the street, under the flags`, looking north between the two rows**: from outside its mouth the end wall is the
        // whole foreground, and what the scene is a picture of is the lane between the houses.
        [-22.0, 3.0, 0.6],
        [6.0, 2.0, -0.6],
        FIELD_OF_VIEW,
        "slate_street",
    )
}

/// Returns the document: six houses, the flags between them, and the two end walls.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    // **Two rows facing each other.** A 石板房's yard is at `+z`, so the south row is placed as it is built and the north row is
    // turned a half — the same answer the 白族 lane gave, where the turn is what puts a gate on the lane.
    for (along, turn) in [(-ROW, 0.0_f32), (ROW, core::f32::consts::PI)] {
        for across in PLOTS {
            let mut part = crate::tunpu::document().skeleton;
            if turn.abs() > 1.0e-3 {
                ops::rotate(&mut part, [0.0, 0.0], turn);
            }
            ops::translate(&mut part, [across, along]);
            assert!(
                ops::merge(&mut skeleton, part),
                "a house of the street places no parts, so the merge always happens"
            );
        }
    }

    // **The lane, laid in flags with a channel down its middle.** One slab from mouth to mouth reads as concrete; the joint is
    // where a mason's day ended, and the **channel** is where the water goes. Each flag is three bands: two walked strips at
    // `FLAG` and the gutter between them at `GUTTER`, a hand's depth lower — which is how a stone street drains without a pipe.
    let run = (LANE_TO - LANE_FROM) / LANE_FLAGS as f32;
    for flag in 0..LANE_FLAGS {
        let from = LANE_FROM + run * flag as f32 + JOINT * 0.5;
        let to = LANE_FROM + run * (flag + 1) as f32 - JOINT * 0.5;
        for (near, far, top) in [
            (-LANE_HALF, -CHANNEL, FLAG),
            (-CHANNEL, CHANNEL, GUTTER),
            (CHANNEL, LANE_HALF, FLAG),
        ] {
            skeleton.slabs.push(building::Slab::paving(
                ops::wound(vec![[from, near], [to, near], [to, far], [from, far]]),
                0.0,
                top,
            ));
        }
    }

    // **The 街沿石: a kerb down each side of the lane.** A street laid in flags and stopping at the yards' own paving has no edge —
    // it bleeds into them. Two strips of stone run the lane's whole length, a hand's height proud of the flags, and they are the
    // line the street is read by: the flags between them, the yards beyond them.
    for (near, far) in [
        (-LANE_HALF, -LANE_HALF + KERB),
        (LANE_HALF - KERB, LANE_HALF),
    ] {
        skeleton.slabs.push(building::Slab::paving(
            ops::wound(vec![
                [LANE_FROM, near],
                [LANE_TO, near],
                [LANE_TO, far],
                [LANE_FROM, far],
            ]),
            FLAG,
            KERB_HIGH,
        ));
    }

    // **A wall at each end, with the lane's width left open.** A street with two mouths and no third is a street a garrison
    // could close, which is what a 屯堡 is for; the gap is the lane's own width plus a hand's breadth either side.
    for x in [LANE_FROM + END_THICK, LANE_TO - END_THICK] {
        let lengths = [
            (-LANE_HALF - 9.0, -MOUTH * 0.5),
            (MOUTH * 0.5, LANE_HALF + 9.0),
        ];
        for (from, to) in lengths {
            let mut wall = Mass::new(
                ops::wound(vec![
                    [x - END_HALF, from],
                    [x + END_HALF, from],
                    [x + END_HALF, to],
                    [x - END_HALF, to],
                ])
                .into_iter()
                .map(|at| PlanNode::new(at, 0.0))
                .collect(),
                END_HIGH,
            );
            wall.wall = END_THICK;
            wall.slab = 0.22;
            skeleton.masses.push(wall);
        }
    }
    // **The 街门 at the east mouth: a lintel over two piers, set out in front of the wall.** A street with two mouths and no gate is
    // a street a garrison cannot close, and this is a 屯堡 — the wall's gap was the one thing the street was missing. The lintel is
    // the **host** and the piers are `Under` it, as round 89 established for the 白族巷's gate; and the gate stands `GATE_OUT` in
    // front of the wall's face for the reason round 114 measured: a lintel wide enough to contain two piers reaches past the gap's
    // own edges, and the wall either side owns the square metre it reaches into.
    let gate_x = LANE_TO - END_THICK + GATE_OUT;
    let mut lintel = Mass::new(
        ops::wound(vec![
            [gate_x - MOUTH * 0.5 - 0.55, -0.42],
            [gate_x + MOUTH * 0.5 + 0.55, -0.42],
            [gate_x + MOUTH * 0.5 + 0.55, 0.42],
            [gate_x - MOUTH * 0.5 - 0.55, 0.42],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, GATE_HEAD))
        .collect(),
        GATE_HEAD + 0.52,
    );
    lintel.wall = 0.26;
    lintel.slab = 0.20;
    let lintel_id = MassId(skeleton.masses.len());
    skeleton.masses.push(lintel);
    for across in [-1.0_f32, 1.0] {
        let x = gate_x + across * (MOUTH * 0.5 - GATE_PIER * 0.5);
        let mut pier = Mass::new(
            ops::wound(vec![
                [x - GATE_PIER * 0.5, -0.36],
                [x + GATE_PIER * 0.5, -0.36],
                [x + GATE_PIER * 0.5, 0.36],
                [x - GATE_PIER * 0.5, 0.36],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            GATE_HEAD,
        );
        pier.wall = 0.18;
        pier.slab = 0.16;
        pier.attachment = Some(building::Attachment {
            host: lintel_id,
            join: building::Join::Under,
        });
        skeleton.masses.push(pier);
    }

    // **The street's water: a well and a trough, at the east mouth.** A 屯堡 street's houses have no wells of their own — the
    // water is at the place the street widens, and it is one place. The well is a **mass whose wall is thick enough to be a ring**
    // (`0.24 m` in a `1.44 m` plan), the same answer the 白族 lane's well gave; the trough beside it is the same idea at a
    // rectangle, and it is what the animals drink from.
    // **The 井台: the stepped stone apron the water stands on.** A well's ring does not sit on the street — it sits on a
    // platform a hand's height up, so that the water running down the channel cannot run back into it. The apron is a floor
    // slab, and the curb and the trough stand on its top: their nodes are at `PLINTH` and not at nothing.
    let well = WELL_AT;
    let apron = [
        [well[0] - WELL_HALF - 0.62, well[1] - WELL_HALF - 0.40],
        [
            well[0] + WELL_HALF + TROUGH_LONG + 1.05,
            well[1] - WELL_HALF - 0.40,
        ],
        [
            well[0] + WELL_HALF + TROUGH_LONG + 1.05,
            well[1] + WELL_HALF + 0.40,
        ],
        [well[0] - WELL_HALF - 0.62, well[1] + WELL_HALF + 0.40],
    ];
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(apron.to_vec()),
        0.0,
        PLINTH,
    ));
    // **The 踏步 in front of it**: a platform a hand's height up with no step is a kerb you climb, and a well is approached by
    // two shallow steps and not by one. Each is a slab a hand's width deep, half the height of the one behind it — one step for
    // the platform's own edge and one for the street.
    for (band, top) in [(0usize, PLINTH * 0.66), (1, PLINTH * 0.33)] {
        let south = apron[0][1] - STEP * (band + 1) as f32;
        skeleton.slabs.push(building::Slab::floor(
            ops::wound(vec![
                [apron[0][0], south],
                [apron[1][0], south],
                [apron[1][0], south + STEP],
                [apron[0][0], south + STEP],
            ]),
            0.0,
            top,
        ));
    }
    let mut ring = Mass::new(
        ops::wound(vec![
            [well[0] - WELL_HALF, well[1] - WELL_HALF],
            [well[0] + WELL_HALF, well[1] - WELL_HALF],
            [well[0] + WELL_HALF, well[1] + WELL_HALF],
            [well[0] - WELL_HALF, well[1] + WELL_HALF],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, PLINTH))
        .collect(),
        PLINTH + WELL_HIGH,
    );
    ring.wall = WELL_WALL;
    ring.slab = 0.18;
    skeleton.masses.push(ring);

    // **The trough**: a hollow box `TROUGH_LONG` by `TROUGH_DEEP`, its wall `TROUGH_WALL` thick, standing on the paving at the
    // well's own side. A trough is a well laid on its side: the same ring of stone, shorter and longer.
    let mut trough = Mass::new(
        ops::wound(vec![
            [well[0] + WELL_HALF + 0.55, well[1] - TROUGH_DEEP * 0.5],
            [
                well[0] + WELL_HALF + 0.55 + TROUGH_LONG,
                well[1] - TROUGH_DEEP * 0.5,
            ],
            [
                well[0] + WELL_HALF + 0.55 + TROUGH_LONG,
                well[1] + TROUGH_DEEP * 0.5,
            ],
            [well[0] + WELL_HALF + 0.55, well[1] + TROUGH_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        TROUGH_HIGH,
    );
    trough.wall = TROUGH_WALL;
    trough.slab = 0.14;
    skeleton.masses.push(trough);

    Building::new(skeleton)
}
