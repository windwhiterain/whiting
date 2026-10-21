//! A **白族本主庙**: the village's own temple — a 大殿 on its plinth at the head of a court, a 戏台 facing it across the
//! paving, and the walls that close the two sides.
//!
//! # What this scene is for
//!
//! The gallery's **fourteenth type**, and the first that is a *public* building. A 本主庙 is where a 白族 village keeps its
//! 本主 — its own protecting deity — and it has two buildings because the village has two duties: the 大殿 where the god is
//! and the 戏台 where the god is *entertained*. That pair facing each other across a court is the whole type.
//!
//! # The four facts the type has
//!
//! - **the 大殿 stands on a plinth** and the 戏台 on a lower one: a temple's floor is a step above the court it looks on;
//! - **the 大殿's front is a colonnade**, not a wall: the god is behind it and the village stands in front of it;
//! - **the 戏台 is open on its court side**: its mouth is a hole the width of the stage, because it is a stage;
//! - **the two are on one axis**, and the courtyard between them is the nave.
//!
//! # What it does not draw
//!
//! - **the god**: an image is a sculpture and not a building;

use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, derive, ops};
use definition::Value;

/// The court: `COURT_X` each side of the axis, and the two buildings at `HALL_Z` and `STAGE_Z`.
const COURT_X: f32 = 6.60;
const HALL_Z: f32 = 7.60;
const STAGE_Z: f32 = -7.20;

/// The 大殿: ten metres across, six deep, one tall storey on a half-metre plinth.
const HALL_WIDTH: f32 = 10.00;
const HALL_DEPTH: f32 = 6.00;
const HALL_HIGH: f32 = 4.30;
const HALL_PLINTH: f32 = 0.50;

/// The 戏台: seven across, five and a half deep, a little lower and on a lower plinth.
const STAGE_WIDTH: f32 = 7.00;
const STAGE_DEPTH: f32 = 5.40;
const STAGE_HIGH: f32 = 3.60;
const STAGE_PLINTH: f32 = 0.90;

const WALL: f32 = 0.50;
const PLATE: f32 = 0.30;

/// The roofs: a temple's are bigger than a house's, and they are where the type's rank shows.
const OVERHANG: f32 = 0.85;
const HALL_RISE: f32 = 2.10;
const STAGE_RISE: f32 = 1.55;
const SINK: f32 = 0.10;
const ROOF_THICKNESS: f32 = 0.22;

/// The walls that close the court's two sides.
const SIDE_THICK: f32 = 0.40;
const SIDE_WALL: f32 = 0.16;
const SIDE_HIGH: f32 = 2.60;

const TILES: Covering = Covering {
    period: 0.20,
    fill: 0.74,
    height: 0.05,
};

/// The 香炉 in the court`s middle: where, its plan, the wall it is made of, the step it stands on and its height.
const BURNER_AT: [f32; 2] = [0.0, -0.40];
const BURNER: f32 = 1.25;
const BURNER_WALL: f32 = 0.17;
const BURNER_FOOT: f32 = 0.16;
/// The lid over the burner: how thick the stone cap is.
const LID: f32 = 0.14;
const BURNER_HIGH: f32 = 1.05;

/// The 旗杆石 pair: their plan, their height, and how far either side of the axis they stand.
const FLAGSTONE: f32 = 0.58;
const FLAGSTONE_HIGH: f32 = 0.86;
const FLAGSTONE_AT: f32 = 3.40;

/// The 台口栏杆 on the stages mouth: how high it stands above the stages floor, how many bays, and a posts section.
const STAGE_RAIL_HIGH: f32 = 0.86;
const STAGE_RAIL_BAYS: usize = 6;
const STAGE_RAIL_POST: f32 = 0.085;

/// The 庑: how far the colonnade stands out from the side wall, how many posts carry it, and a posts section.
const GALLERY: f32 = 1.60;
const GALLERY_POSTS: usize = 5;
const GALLERY_POST: f32 = 0.15;

/// The 坐凳栏杆 under the 庑: how high the seat is and how wide.
const BENCH_HIGH: f32 = 0.46;
const BENCH_WIDE: f32 = 0.34;

/// The 匾额: its width, its height, and how high its centre stands above the halls plinth — and how far the boards stand proud of
/// the face they hang on.
const PLAQUE: [f32; 3] = [2.60, 0.78, 3.05];
const BOARD_PROUD: f32 = 0.07;

/// The 山门: how wide it is either side of the axis, how thick, how high, the wall in it, and where it stands.
const SHANMEN_HALF: f32 = 6.20;
const SHANMEN_THICK: f32 = 0.60;
const SHANMEN_HIGH: f32 = 3.10;
const SHANMEN_WALL: f32 = 0.26;
const SHANMEN_Z: f32 = -12.40;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the temple: from the court's south-west corner, so the 戏台 is at one side and the 大殿 at the other.
pub fn benzu_miao() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        // **High and three-quarters from the south-west**: the 戏台`s mouth faces the 大殿 and a low camera from the south sees
        // only its back, so the shot looks over its roof into the court and down onto the 大殿`s colonnade.
        // **Backed off to take the 山门 in**: the gate closes the court at its south end, and a plate of a temple that leaves its
        // own entrance out of the frame is a plate of a courtyard.
        [20.5, 15.0, -21.0],
        [-0.5, 2.0, -1.4],
        FIELD_OF_VIEW,
        "benzu_miao",
    )
}

/// The court's own plate: **from the 大殿's step, looking at the 戏台's mouth** — the seat the audience takes. A temple's compound plate
/// sees roofs; what a stage's mouth is for is only visible from where the village sits, so the type has two scenes: the compound and
/// the court.
pub fn court() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [0.9, 3.10, 5.40],
        [-0.4, 2.05, -7.40],
        FIELD_OF_VIEW * 0.92,
        "benzu_miao_court",
    )
}
/// Returns the document: the two plinths, the 大殿, the 戏台, the court's paving and the two side walls.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    let rectangle = |half_x: f32, half_z: f32, x: f32, z: f32, elevation: f32| {
        ops::wound(vec![
            [x - half_x, z - half_z],
            [x + half_x, z - half_z],
            [x + half_x, z + half_z],
            [x - half_x, z + half_z],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, elevation))
        .collect::<Vec<_>>()
    };

    // **The 大殿 on its plinth.** The plinth is a floor slab a little larger than the hall, because a temple's floor is a
    // step above the court it looks on.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [-HALL_WIDTH * 0.5 - 0.65, HALL_Z - HALL_DEPTH * 0.5 - 0.65],
            [HALL_WIDTH * 0.5 + 0.65, HALL_Z - HALL_DEPTH * 0.5 - 0.65],
            [HALL_WIDTH * 0.5 + 0.65, HALL_Z + HALL_DEPTH * 0.5 + 0.65],
            [-HALL_WIDTH * 0.5 - 0.65, HALL_Z + HALL_DEPTH * 0.5 + 0.65],
        ]),
        0.0,
        HALL_PLINTH,
    ));
    let mut hall = Mass::new(
        rectangle(HALL_WIDTH * 0.5, HALL_DEPTH * 0.5, 0.0, HALL_Z, HALL_PLINTH),
        HALL_PLINTH + HALL_HIGH,
    );
    hall.wall = WALL;
    hall.slab = PLATE;
    let hall_id = MassId(skeleton.masses.len());
    skeleton.masses.push(hall);

    // **The 戏台 on its own plinth**, facing the hall. Its plinth is higher, because a stage's floor is a stage's floor.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [
                -STAGE_WIDTH * 0.5 - 0.55,
                STAGE_Z - STAGE_DEPTH * 0.5 - 0.55,
            ],
            [STAGE_WIDTH * 0.5 + 0.55, STAGE_Z - STAGE_DEPTH * 0.5 - 0.55],
            [STAGE_WIDTH * 0.5 + 0.55, STAGE_Z + STAGE_DEPTH * 0.5 + 0.55],
            [
                -STAGE_WIDTH * 0.5 - 0.55,
                STAGE_Z + STAGE_DEPTH * 0.5 + 0.55,
            ],
        ]),
        0.0,
        STAGE_PLINTH,
    ));
    let mut stage = Mass::new(
        rectangle(
            STAGE_WIDTH * 0.5,
            STAGE_DEPTH * 0.5,
            0.0,
            STAGE_Z,
            STAGE_PLINTH,
        ),
        STAGE_PLINTH + STAGE_HIGH,
    );
    stage.wall = WALL;
    stage.slab = PLATE;
    let stage_id = MassId(skeleton.masses.len());
    skeleton.masses.push(stage);

    // **A roof over each and the court paved between them**, which is what makes the two buildings one temple.
    for (id, rise) in [(hall_id, HALL_RISE), (stage_id, STAGE_RISE)] {
        if let Some(roof) = derive::gable_roof(
            id,
            &skeleton.masses[id.0],
            OVERHANG,
            rise,
            SINK,
            ROOF_THICKNESS,
        ) {
            skeleton.roofs.push(roof.with_covering(TILES));
        }
    }
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-COURT_X, STAGE_Z + STAGE_DEPTH * 0.5 + 0.55],
            [COURT_X, STAGE_Z + STAGE_DEPTH * 0.5 + 0.55],
            [COURT_X, HALL_Z - HALL_DEPTH * 0.5 - 0.65],
            [-COURT_X, HALL_Z - HALL_DEPTH * 0.5 - 0.65],
        ]),
        0.0,
        0.06,
    ));

    // **The two side walls**, from the 戏台's plinth to the 大殿's, each a mass thinner than half its own depth.
    let from = STAGE_Z + STAGE_DEPTH * 0.5 + 0.55;
    let to = HALL_Z - HALL_DEPTH * 0.5 - 0.65;
    for across in [-COURT_X - SIDE_THICK * 0.5, COURT_X + SIDE_THICK * 0.5] {
        let mut wall = Mass::new(
            ops::wound(vec![
                [across - SIDE_THICK * 0.5, from],
                [across + SIDE_THICK * 0.5, from],
                [across + SIDE_THICK * 0.5, to],
                [across - SIDE_THICK * 0.5, to],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            SIDE_HIGH,
        );
        wall.wall = SIDE_WALL;
        wall.slab = 0.20;
        skeleton.masses.push(wall);
        let wall_id = MassId(skeleton.masses.len() - 1);

        // **The 庑: a colonnade and a lean-to in front of that wall.** A temple's court is flanked by **covered galleries**, not by
        // a fence: the wall is their back, `GALLERY` posts stand in front of it, and a lean-to sheds from the wall's head into the
        // court. Without them the court is a yard between two buildings; with them it is a courtyard, which is what a 本主庙 is
        // arranged as.
        let inward = if across < 0.0 { 1.0_f32 } else { -1.0 };
        let face = across + inward * GALLERY;
        for index in 0..=GALLERY_POSTS {
            let z = from + (to - from) * index as f32 / GALLERY_POSTS as f32;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [face, 0.0, z] },
                building::Anchor::Free {
                    at: [face, SIDE_HIGH - 0.55, z],
                },
                [1.0, 0.0],
                GALLERY_POST,
                GALLERY_POST,
            ));
        }
        // The gallery's beam, and its roof: a lean-to from the wall's head down over the posts.
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [face, SIDE_HIGH - 0.62, from],
            },
            building::Anchor::Free {
                at: [face, SIDE_HIGH - 0.62, to],
            },
            // **The section's `across` is the run's perpendicular**: the beam runs along `z`, so its across is `x`. Passed as the run
            // itself it is `MemberDegenerate { length: 7.9 }` — the lesson rounds 100, 106 and 128 all paid for.
            [1.0, 0.0],
            GALLERY_POST * 1.2,
            GALLERY_POST * 1.5,
        ));
        let roof_plan = [
            [across - inward * SIDE_THICK * 0.5, from - 0.30],
            [face + inward * 0.55, from - 0.30],
            [face + inward * 0.55, to + 0.30],
            [across - inward * SIDE_THICK * 0.5, to + 0.30],
        ];
        if let Some(roof) =
            derive::lean_to(wall_id, roof_plan, SIDE_HIGH - 0.60, SIDE_HIGH + 0.10, 0.12)
        {
            skeleton.roofs.push(roof.with_covering(TILES));
        }

        // **The 坐凳栏杆: the bench rail along the colonnade.** A 庑 is where the village sits to watch the 戏台 — the rail's top is
        // the seat, a plank `BENCH_WIDE` across at `BENCH_HIGH`, carried on a leg at every post. A gallery with no bench is a
        // corridor; a gallery with one is the place a temple fair is watched from.
        let bench_x = face + inward * 0.10;
        for index in 0..=GALLERY_POSTS {
            let z = from + (to - from) * index as f32 / GALLERY_POSTS as f32;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [bench_x, 0.0, z],
                },
                building::Anchor::Free {
                    at: [bench_x, BENCH_HIGH - 0.04, z],
                },
                [1.0, 0.0],
                GALLERY_POST * 0.55,
                GALLERY_POST * 0.55,
            ));
        }
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [bench_x, BENCH_HIGH, from],
            },
            building::Anchor::Free {
                at: [bench_x, BENCH_HIGH, to],
            },
            // **The plank`s across is `x` because its run is `z`.** This is the fifth time a member's `across` has been written as its
            // own run in this project (rounds 100, 106, 128, 133 and here), so the rule is stated as a rule: **a member's across is
            // the run turned a quarter** — `[dy, -dx]` or its negative — and never the run.
            [1.0, 0.0],
            BENCH_WIDE,
            0.07,
        ));
    }

    // **The 大殿's front is a colonnade**: three bays, and the middle one is a door two bays wide, because the god is behind
    // it and the village stands in front of it. The face that looks down the court is the plan's `-z` side, `edge 0`.
    let front = building::FaceRef::new(hall_id, 0);
    if building::ops::split_face(
        &mut skeleton,
        front,
        &building::Rhythm::Even { count: 3 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        for column in 0..3 {
            let inset = building::Inset::new(0.62, 0.62, 0.0, 1.10);
            let _ = building::ops::add_opening(
                &mut skeleton,
                building::Opening::new(front, 0, column, inset),
            );
        }
    }

    // **The 匾额 and the 楹联.** A 大殿's colonnade is not bare: over the middle door hangs the **plaque** that names the god, and on
    // each flanking bay a pair of **couplet boards**. They are members — a board has no inside — set `BOARD_PROUD` off the front face,
    // and the face they hang on is the one the court plate looks at, which is why they are worth drawing at all.
    let face_z = HALL_Z - HALL_DEPTH * 0.5 - BOARD_PROUD;
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-PLAQUE[0] * 0.5, HALL_PLINTH + PLAQUE[2], face_z],
        },
        building::Anchor::Free {
            at: [PLAQUE[0] * 0.5, HALL_PLINTH + PLAQUE[2], face_z],
        },
        [0.0, 1.0],
        0.09,
        PLAQUE[1],
    ));
    for across in [-1.0_f32, 1.0] {
        let x = across * HALL_WIDTH / 6.0;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, HALL_PLINTH + 1.15, face_z],
            },
            building::Anchor::Free {
                at: [x, HALL_PLINTH + 2.65, face_z],
            },
            [1.0, 0.0],
            0.075,
            0.34,
        ));
    }

    // **The 戏台's mouth**: one opening the width of the stage, on the face that looks at the 大殿 (`+z`, `edge 2`).
    let mouth = building::FaceRef::new(stage_id, 2);
    if building::ops::split_face(
        &mut skeleton,
        mouth,
        &building::Rhythm::Even { count: 1 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        let _ = building::ops::add_opening(
            &mut skeleton,
            building::Opening::new(mouth, 0, 0, building::Inset::new(0.62, 0.62, 0.0, 0.70)),
        );
    }

    // **The 上下场门: the two doors in the stage's back wall.** An actor enters by one and leaves by the other — 出将 and 入相 — and a
    // 戏台 with no back doors is a stage nobody can walk onto. They are cut on `edge 0`, the face away from the court, which is the
    // face the **compound** plate looks at: two small openings in the outer thirds of the wall, a metre and a half either side of the
    // axis, each as tall as a man.
    let back = building::FaceRef::new(stage_id, 0);
    if building::ops::split_face(
        &mut skeleton,
        back,
        &building::Rhythm::Even { count: 3 },
        &building::Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        for column in [0usize, 2] {
            let _ = building::ops::add_opening(
                &mut skeleton,
                building::Opening::new(
                    back,
                    0,
                    column,
                    building::Inset::new(0.55, 0.55, 0.0, 1.35),
                ),
            );
        }
    }

    // **The 台口栏杆 across the stage's mouth.** A stage a metre up with an open mouth is a stage a child walks off the front of; the
    // rail runs across the mouth inside it, on the stage's own floor, with a post every `STAGE_RAIL_BAY` and two rails. It is the
    // same edge-given-a-rail the 竹楼's deck got in round 128 and the 塔's platform in round 130 — only here the material is timber
    // and the rail stands where the audience sees it.
    let rail_half = STAGE_WIDTH * 0.5 - 0.70;
    let rail_z = STAGE_Z + STAGE_DEPTH * 0.5 - 0.30;
    for index in 0..=STAGE_RAIL_BAYS {
        let x = -rail_half + rail_half * 2.0 * index as f32 / STAGE_RAIL_BAYS as f32;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, STAGE_PLINTH, rail_z],
            },
            building::Anchor::Free {
                at: [x, STAGE_PLINTH + STAGE_RAIL_HIGH, rail_z],
            },
            [1.0, 0.0],
            STAGE_RAIL_POST,
            STAGE_RAIL_POST,
        ));
    }
    for high in [STAGE_RAIL_HIGH, STAGE_RAIL_HIGH * 0.52] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [-rail_half, STAGE_PLINTH + high, rail_z],
            },
            building::Anchor::Free {
                at: [rail_half, STAGE_PLINTH + high, rail_z],
            },
            [0.0, 1.0],
            STAGE_RAIL_POST * 0.6,
            STAGE_RAIL_POST * 0.8,
        ));
    }

    // **The 香炉 in the court's middle**: a square stone box on a step, which is what a temple's court is arranged around. Four
    // members, the same shape the 碉房's 煨桑炉 has — a mass inside a court would be a mass inside nothing, and a burner is four
    // walls. **Both of a member`s anchors are at the same elevation**: a pair that differ is a post, and four posts at a
    // burner`s corners are not a box — the section`s height is `BURNER_HIGH` and the run is the wall`s own length.
    let [burner_x, burner_z] = BURNER_AT;
    let half = BURNER * 0.5;
    let corners = [
        ([-half, -half], [half, -half]),
        ([half, -half], [half, half]),
        ([half, half], [-half, half]),
        ([-half, half], [-half, -half]),
    ];
    // **The 基座 and the 炉盖, and the walls between them.** The walls' two anchors are at **one elevation** — a pair that differ
    // is a post, and four posts at a box's corners are not a box (rounds 90, 100 and 105 all made that mistake, and the last of
    // them left it in *this* loop). The step they stand on is a floor slab and the lid over them is a roof slab, which is what a
    // burner is: two slabs and four walls.
    let step = BURNER * 0.5 + 0.22;
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [burner_x - step, burner_z - step],
            [burner_x + step, burner_z - step],
            [burner_x + step, burner_z + step],
            [burner_x - step, burner_z + step],
        ]),
        0.0,
        BURNER_FOOT,
    ));
    skeleton.slabs.push(building::Slab::roof(
        vec![
            [burner_x - half - 0.05, burner_z - half - 0.05],
            [burner_x + half + 0.05, burner_z - half - 0.05],
            [burner_x + half + 0.05, burner_z + half + 0.05],
            [burner_x - half - 0.05, burner_z + half + 0.05],
        ],
        BURNER_FOOT + BURNER_HIGH,
        BURNER_FOOT + BURNER_HIGH + LID,
    ));
    for (from, to) in corners {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [burner_x + from[0], BURNER_FOOT, burner_z + from[1]],
            },
            building::Anchor::Free {
                at: [burner_x + to[0], BURNER_FOOT, burner_z + to[1]],
            },
            // **The section`s `across` is the run`s perpendicular**: a member whose across lies along its own axis is a
            // degenerate one, answered as `MemberDegenerate { length: 1.25 }` — the same lesson as the door leaf`s.
            [-(to[1] - from[1]) / BURNER, (to[0] - from[0]) / BURNER],
            BURNER_WALL,
            BURNER_HIGH,
        ));
    }

    // **The 山门: the temple's own gate, closing the court at its south end.** The module has said since round 79 that the 山门 is
    // "where the camera is"; it is a wall across the court with a door in it now, `SHANMEN_HALF` either side of the axis and
    // `SHANMEN_THICK` deep, and the door cut in its court face the way every other opening in this repository is cut — the face
    // split first, then an inset a wall's thickness clear of the bay's ends.
    let mut shanmen = Mass::new(
        rectangle(SHANMEN_HALF, SHANMEN_THICK * 0.5, 0.0, SHANMEN_Z, 0.0),
        SHANMEN_HIGH,
    );
    shanmen.wall = SHANMEN_WALL;
    shanmen.slab = 0.24;
    let shanmen_id = MassId(skeleton.masses.len());
    skeleton.masses.push(shanmen);
    let gate_face = building::FaceRef::new(shanmen_id, 2);
    let columns = building::Rhythm::Even { count: 3 };
    let rows = building::Rhythm::Even { count: 1 };
    if ops::split_face(&mut skeleton, gate_face, &columns, &rows).is_some() {
        let _ = ops::add_opening(
            &mut skeleton,
            building::Opening::new(gate_face, 0, 1, building::Inset::new(0.30, 0.30, 0.0, 0.90)),
        );
    }
    // **The gate's two leaves, swung out into the lane, and its threshold.** A 山门 is a gate and not an arch: the plates stand
    // open, one against each jamb, and a foot steps over the threshold between them. The leaves are `derive::leaf`'s panels —
    // both anchors at one elevation, the section's height the opening's — the same call the 白族巷's gate and the 屯堡's 朝门
    // hang their own with.
    let clear = SHANMEN_HALF / 3.0 - 0.30;
    for across in [-1.0_f32, 1.0] {
        let hinge = across * clear;
        if let Some(panel) = derive::leaf(
            [hinge, 0.06, SHANMEN_Z - SHANMEN_THICK * 0.5 + 0.06],
            clear,
            SHANMEN_HIGH - 0.90 - 0.12,
            0.07,
            [0.0, -across.signum()],
        ) {
            skeleton.members.push(panel);
        }
    }
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [-clear, 0.0, SHANMEN_Z],
        },
        building::Anchor::Free {
            at: [clear, 0.0, SHANMEN_Z],
        },
        [0.0, 1.0],
        0.26,
        0.18,
    ));

    // **Its roof**: a lean-to over the wall, shedding into the court, which is what makes a wall a gate house.
    let span = SHANMEN_HALF + 0.55;
    let plan = [
        [-span, SHANMEN_Z + SHANMEN_THICK * 0.5 + 0.55],
        [span, SHANMEN_Z + SHANMEN_THICK * 0.5 + 0.55],
        [span, SHANMEN_Z - SHANMEN_THICK * 0.5],
        [-span, SHANMEN_Z - SHANMEN_THICK * 0.5],
    ];
    if let Some(roof) = derive::lean_to(
        shanmen_id,
        plan,
        SHANMEN_HIGH - 0.55,
        SHANMEN_HIGH + 0.05,
        0.14,
    ) {
        skeleton.roofs.push(roof.with_covering(TILES));
    }

    // **The 旗杆石 a pair, flanking the 大殿's steps.** A temple that has a court has two stones in it where the flags stand on
    // festival days, and they are masses because a stone with a foot and a top is one.
    for across in [-1.0_f32, 1.0] {
        let mut stone = Mass::new(
            rectangle(
                FLAGSTONE * 0.5,
                FLAGSTONE * 0.5,
                across * FLAGSTONE_AT,
                HALL_Z - HALL_DEPTH * 0.5 - 1.55,
                0.0,
            ),
            FLAGSTONE_HIGH,
        );
        stone.wall = FLAGSTONE * 0.22;
        stone.slab = 0.12;
        skeleton.masses.push(stone);
    }

    Building::new(skeleton)
}
