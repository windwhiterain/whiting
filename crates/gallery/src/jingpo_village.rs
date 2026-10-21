//! A **景颇寨**: three longhouses round a lane, with a gate at the lane's mouth — the first settlement of 滇西.
//!
//! # What this scene is for
//!
//! The gallery's **tenth settlement**. A 景颇 village is not a courtyard and not a street of houses: it is **three or four longhouses** set down
//! on a ridge with a lane between them, and the lane is the whole public space the village has.
//!
//! # What it composes
//!
//! Three `longhouse::document()`s — each already carrying its posts, its 敞廊 and its ladder — on either side of a paved lane, plus the gate
//! this project has built seven times: two lengths of wall either side of a gap, a head across it, two leaves and a lean-to over it. **The
//! settlement adds no type**: the lane, the walls and the gate are the same arrangement the 侗寨 and the 羌寨 use.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The lane: how wide it runs, how far it goes, and where the houses stand either side of it.
const LANE_HALF: f32 = 4.20;
const LANE_FROM: f32 = -26.0;
const LANE_TO: f32 = 20.0;
const PAVING: f32 = 0.06;
/// The houses: where each stands and how it is turned. The lane runs along `x`, so the two flanking houses face across it.
const HOUSES: [(f32, f32, f32); 3] = [
    (-6.0, -10.6, core::f32::consts::PI),
    (-6.0, 10.6, 0.0),
    (-24.0, 0.0, core::f32::consts::FRAC_PI_2),
];
/// The gate at the lane's mouth: how wide the gap, the wall's depth and height, and how deep the gate's head lies.
const GATE_AT: f32 = LANE_TO - 2.20;
const GATE_CLEAR: f32 = 3.60;
const WALL_THICK: f32 = 0.44;
const WALL_HIGH: f32 = 2.10;
const HEAD_HIGH: f32 = 0.46;
/// How far the village's wall runs beyond the lane on each side.
const WALL_SPAN: f32 = 15.0;

/// The 寨门's posts: their section, their height, and the 牛头桩 inside the gate — how many, how far apart, their section and height.
const GATE_POST: f32 = 0.26;
const GATE_POST_HIGH: f32 = 4.40;
const SKULL_POSTS: usize = 3;
const SKULL_GAP: f32 = 2.80;
const SKULL_POST: f32 = 0.20;
const SKULL_HIGH: f32 = 2.35;

/// The 晒台 beside a longhouse: where it stands, its plan, how high its deck is raised, its deck's thickness, how many post pairs and a posts
/// section, and the rails height above the deck.
const PLATFORM_AT: [f32; 2] = [14.60, -9.20];
const PLATFORM: [f32; 2] = [3.60, 3.00];
const PLATFORM_UP: f32 = 1.15;
const PLATFORM_DECK: f32 = 0.14;
const PLATFORM_POSTS: usize = 3;
const PLATFORM_POST: f32 = 0.14;
const PLATFORM_RAIL: f32 = 0.80;

/// The 谷仓 beside the lane: where it stands, its plan, how high its floor is posted and how tall the store is, a posts section and how far
/// its cap stands out.
const GRANARY_AT: [f32; 2] = [7.40, -6.20];
const GRANARY: [f32; 2] = [2.60, 2.10];
const GRANARY_UP: f32 = 0.95;
const GRANARY_HIGH: f32 = 1.85;
const GRANARY_POST: f32 = 0.16;
const GRANARY_EAVE: f32 = 0.42;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the scene: from the lane's mouth, so the gate frames the two long roofs and the third behind them.
pub fn jingpo_village() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [37.0, 13.0, 31.0],
        [-2.0, 4.6, 0.0],
        FIELD_OF_VIEW,
        "jingpo_village",
    )
}

/// The **close plate**: the lane's mouth, where the gate's posts, the skull posts, the granary and the drying deck stand within a few metres of one
/// another.
///
/// # Why a second camera and not a second document
///
/// Round 189 put the drying deck at the north longhouse's own end and recorded that the wide plate could not isolate it. This is round 172's and
/// round 186's move for the third time: **the document is the same one and only the camera changes**.
pub fn jingpo_mouth() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [31.0, 7.0, 12.0],
        [11.0, 2.2, -3.0],
        FIELD_OF_VIEW * 1.15,
        "jingpo_mouth",
    )
}

/// Returns the document: three longhouses, the lane's paving, the gate's walls, its head and its leaves.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The three longhouses**, each the type's own document, turned so the two flanking houses face across the lane.
    for (across, along, turn) in HOUSES {
        let mut part = crate::longhouse::document().skeleton;
        if turn.abs() > 1.0e-3 {
            ops::rotate(&mut part, [0.0, 0.0], turn);
        }
        ops::translate(&mut part, [across, along]);
        assert!(
            ops::merge(&mut skeleton, part),
            "a longhouse places no parts, so the merge always happens"
        );
    }

    // **The lane**, which is the village's only public ground.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [LANE_FROM, -LANE_HALF],
            [LANE_TO, -LANE_HALF],
            [LANE_TO, LANE_HALF],
            [LANE_FROM, LANE_HALF],
        ]),
        0.0,
        PAVING,
    ));

    // **The gate's walls**: two lengths either side of the lane's mouth, meeting the lane's own edge and running out to the village's wall.
    for side in [-1.0_f32, 1.0] {
        let inner = side * LANE_HALF;
        let outer = side * (LANE_HALF + WALL_SPAN);
        let mut wall = Mass::new(
            ops::wound(vec![
                [GATE_AT, inner],
                [GATE_AT, outer],
                [GATE_AT + WALL_THICK, outer],
                [GATE_AT + WALL_THICK, inner],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, 0.0))
            .collect(),
            WALL_HIGH,
        );
        // **The ring's offset stays under half the wall's depth** (round 144).
        wall.wall = WALL_THICK * 0.34;
        wall.slab = 0.16;
        skeleton.masses.push(wall);
    }

    // **The head across the lane**, spanning exactly the gap and resting on the two lengths' own heads (round 157).
    let mut head = Mass::new(
        ops::wound(vec![
            [GATE_AT, -LANE_HALF],
            [GATE_AT, LANE_HALF],
            [GATE_AT + WALL_THICK, LANE_HALF],
            [GATE_AT + WALL_THICK, -LANE_HALF],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, WALL_HIGH))
        .collect(),
        WALL_HIGH + HEAD_HIGH,
    );
    head.wall = WALL_THICK * 0.32;
    head.slab = 0.14;
    skeleton.masses.push(head);
    for side in [-1.0_f32, 1.0] {
        if let Some(leaf) = building::derive::leaf(
            [GATE_AT + WALL_THICK * 0.5, 0.10, side * (LANE_HALF - 0.06)],
            LANE_HALF - 0.12,
            WALL_HIGH - 0.24,
            0.07,
            [0.0, side],
        ) {
            skeleton.members.push(leaf);
        }
    }

    // **The 寨门's posts and the 牛头桩.** A 景颇 village is known by the **tall carved posts** at its gate and by the **skull posts** inside it:
    // every buffalo sacrificed at a 目瑙纵歌 leaves its horns on a post, and a village with many posts is a village with a history. Two gate
    // posts with a beam between them, and three skull posts along the lane's edge.
    for side in [-1.0_f32, 1.0] {
        let z = side * (LANE_HALF + GATE_POST * 0.5);
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [GATE_AT + WALL_THICK * 0.5, 0.0, z],
            },
            building::Anchor::Free {
                at: [GATE_AT + WALL_THICK * 0.5, GATE_POST_HIGH, z],
            },
            [1.0, 0.0],
            GATE_POST,
            GATE_POST,
        ));
    }
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [
                GATE_AT + WALL_THICK * 0.5,
                GATE_POST_HIGH - GATE_POST,
                -LANE_HALF - GATE_POST,
            ],
        },
        building::Anchor::Free {
            at: [
                GATE_AT + WALL_THICK * 0.5,
                GATE_POST_HIGH - GATE_POST,
                LANE_HALF + GATE_POST,
            ],
        },
        [1.0, 0.0],
        GATE_POST * 0.8,
        GATE_POST * 0.9,
    ));
    for index in 0..SKULL_POSTS {
        let x = GATE_AT - 5.60 - SKULL_GAP * index as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, 0.0, LANE_HALF - SKULL_POST * 0.5],
            },
            building::Anchor::Free {
                at: [x, SKULL_HIGH, LANE_HALF - SKULL_POST * 0.5],
            },
            [1.0, 0.0],
            SKULL_POST,
            SKULL_POST,
        ));
    }

    // **The 谷仓: the granary beside the lane.** A hill village keeps its rice out of the house: a **small boxed store raised on posts**, its floor
    // off the ground so the damp and the rats cannot reach it, with a stone cap over it. It is the 侗寨's granary in a second village — the same
    // four posts, the same raised floor, the same cap.
    let [granary_x, granary_z] = GRANARY_AT;
    for bay in 0..2 {
        for side in 0..2 {
            let x = granary_x + (bay as f32 - 0.5) * GRANARY[0] * 0.72;
            let z = granary_z + (side as f32 - 0.5) * GRANARY[1] * 0.72;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, 0.0, z] },
                building::Anchor::Free {
                    at: [x, GRANARY_UP, z],
                },
                [1.0, 0.0],
                GRANARY_POST,
                GRANARY_POST,
            ));
        }
    }
    let mut store = Mass::new(
        ops::wound(vec![
            [granary_x - GRANARY[0] * 0.5, granary_z - GRANARY[1] * 0.5],
            [granary_x + GRANARY[0] * 0.5, granary_z - GRANARY[1] * 0.5],
            [granary_x + GRANARY[0] * 0.5, granary_z + GRANARY[1] * 0.5],
            [granary_x - GRANARY[0] * 0.5, granary_z + GRANARY[1] * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, GRANARY_UP))
        .collect(),
        GRANARY_UP + GRANARY_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
    store.wall = GRANARY[1] * 0.26;
    store.slab = 0.16;
    skeleton.masses.push(store);
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [
                granary_x - GRANARY[0] * 0.5 - GRANARY_EAVE,
                granary_z - GRANARY[1] * 0.5 - GRANARY_EAVE,
            ],
            [
                granary_x + GRANARY[0] * 0.5 + GRANARY_EAVE,
                granary_z - GRANARY[1] * 0.5 - GRANARY_EAVE,
            ],
            [
                granary_x + GRANARY[0] * 0.5 + GRANARY_EAVE,
                granary_z + GRANARY[1] * 0.5 + GRANARY_EAVE,
            ],
            [
                granary_x - GRANARY[0] * 0.5 - GRANARY_EAVE,
                granary_z + GRANARY[1] * 0.5 + GRANARY_EAVE,
            ],
        ]),
        GRANARY_UP + GRANARY_HIGH,
        GRANARY_UP + GRANARY_HIGH + 0.14,
    ));

    // **The 晒台: the drying platform beside the longhouse.** A 景颇 household's grain is dried on a **deck of split bamboo raised on posts** — high
    // enough for the air to move under it and for the fowls not to reach it — and it stands beside the house rather than behind it, because it is
    // used every fine day. A deck, six posts and a rail on the lane's side.
    for post in 0..PLATFORM_POSTS {
        let along = post as f32 / (PLATFORM_POSTS - 1) as f32;
        for z in [
            PLATFORM_AT[1] - PLATFORM[1] * 0.5,
            PLATFORM_AT[1] + PLATFORM[1] * 0.5,
        ] {
            let x = PLATFORM_AT[0] - PLATFORM[0] * 0.5 + PLATFORM[0] * along;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, PAVING, z] },
                building::Anchor::Free {
                    at: [x, PAVING + PLATFORM_UP, z],
                },
                [1.0, 0.0],
                PLATFORM_POST,
                PLATFORM_POST,
            ));
        }
    }
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [
                PLATFORM_AT[0] - PLATFORM[0] * 0.5,
                PLATFORM_AT[1] - PLATFORM[1] * 0.5,
            ],
            [
                PLATFORM_AT[0] + PLATFORM[0] * 0.5,
                PLATFORM_AT[1] - PLATFORM[1] * 0.5,
            ],
            [
                PLATFORM_AT[0] + PLATFORM[0] * 0.5,
                PLATFORM_AT[1] + PLATFORM[1] * 0.5,
            ],
            [
                PLATFORM_AT[0] - PLATFORM[0] * 0.5,
                PLATFORM_AT[1] + PLATFORM[1] * 0.5,
            ],
        ]),
        PAVING + PLATFORM_UP - PLATFORM_DECK,
        PAVING + PLATFORM_UP,
    ));
    // **The rail on the lane's side**, a member with both ends at the deck's own height.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [
                PLATFORM_AT[0] - PLATFORM[0] * 0.5,
                PAVING + PLATFORM_UP + PLATFORM_RAIL,
                PLATFORM_AT[1] + PLATFORM[1] * 0.5,
            ],
        },
        building::Anchor::Free {
            at: [
                PLATFORM_AT[0] + PLATFORM[0] * 0.5,
                PAVING + PLATFORM_UP + PLATFORM_RAIL,
                PLATFORM_AT[1] + PLATFORM[1] * 0.5,
            ],
        },
        [0.0, 1.0],
        PLATFORM_POST * 0.75,
        PLATFORM_POST * 0.8,
    ));

    Building::new(skeleton)
}
