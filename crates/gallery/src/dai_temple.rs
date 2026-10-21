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
/// The 佛坛 at the halls far end: its plan, its height, and how many posts stand down the nave and their section.
const ALTAR: [f32; 2] = [4.60, 1.70];
const ALTAR_HIGH: f32 = 0.90;
const NAVE_POSTS: usize = 3;
const NAVE_POST: f32 = 0.22;

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
const YARD: f32 = 5.60;
const WALL_HIGH: f32 = 1.55;
const WALL_THICK: f32 = 0.40;
const WALL_WALL: f32 = 0.16;
/// The walls 压顶: how thick the cap is and how far it stands proud of the wall on either side.
const COPE: f32 = 0.09;
const COPE_PROUD: f32 = 0.11;
const GATE_HALF: f32 = 1.60;
const YARD_PAVING: f32 = 0.06;

/// The 放生池 in the yard: where it stands, its plan, the rims thickness and its height.
const POND_AT: [f32; 2] = [-6.60, 13.40];
const POND: [f32; 2] = [3.40, 2.40];
const POND_WALL: f32 = 0.32;
const POND_HIGH: f32 = 0.46;

/// The 戒堂, the wat`s second building: its plan, its height, its platform, the pitch of its roof and where it stands in the yard.
const UPOSATHA: [f32; 2] = [5.60, 3.90];
const UPOSATHA_HIGH: f32 = 3.10;
const UPOSATHA_PLINTH: f32 = 0.55;
const UPOSATHA_RISE: f32 = 3.20;
const UPOSATHA_OVERHANG: f32 = 0.85;
const UPOSATHA_AT: [f32; 2] = [10.10, -2.20];
/// The 界石 round the 戒堂: one stones plan and its height above the yard.
const SIMA: f32 = 0.30;
const SIMA_HIGH: f32 = 0.58;
/// The 寺门`s piers: how wide one is, in metres.
const GATE_PIER: f32 = 0.52;

/// The 经幡柱 outside the gate: how far either side of the axis, how far out, how high, the poles section, the cross-arms reach
/// and how long a banner hangs.
const FLAG_X: f32 = 4.60;
const FLAG_OUT: f32 = 2.10;
const FLAG_HIGH: f32 = 6.80;
const FLAG_POLE: f32 = 0.17;
const FLAG_ARM: f32 = 1.10;
const FLAG_CLOTH: f32 = 2.60;

/// The 塔: where it stands in the yard, its square, the height of one course and how many, how far each course draws in, the
/// wall inside it and the platform it rises from.
const STUPA_AT: [f32; 2] = [10.40, 5.20];
const STUPA: f32 = 2.90;
const STUPA_COURSE: f32 = 0.95;
const STUPA_COURSES: usize = 4;
const STUPA_BATTER: f32 = 0.21;
const STUPA_WALL: f32 = 0.34;
const STUPA_PLINTH: f32 = 0.32;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the temple: from the porch's corner, so the pitch and the platform's height are both seen.
pub fn dai_temple() -> Value {
    // **The yard's trees**: a wat keeps a **bodhi** at its gate and a mango or two along its wall, and a court with two buildings
    // and no tree is a yard. The vegetation bridge is the hamlet's own, merged here rather than re-authored — the same answer
    // round 117 gave the temple itself.
    let mut extra = crate::settlement_plants::Extra::none();
    extra.merge(crate::settlement_plants::trees(
        &[[-11.4, 12.8], [12.2, 13.4], [-13.2, -9.0]],
        &|_: [f32; 2]| 0.0,
    ));
    crate::village::scene_of(
        &document(),
        &extra,
        &|_, _| 0.0,
        // **Backed off to take the whole wat in**: two buildings, the yard between them and the wall round both. A plate that cuts
        // the 戒堂 at its edge is a plate of one hall.
        [24.0, 10.5, 22.0],
        [0.0, 3.4, -2.0],
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

    // **The inside of the hall: the 佛坛 and the rank of posts down its spine.** A wat's hall is not an empty shell — the image
    // stands on a **dais** at the far end and the roof is carried by a row of posts that the nave is walked between. The dais is a
    // floor slab and not a mass: a plinth is what a dais is, and a mass inside the hall's own plan is a mass that has to name a
    // host it does not have.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [-ALTAR[0] * 0.5, -deep + WALL + 0.20],
            [ALTAR[0] * 0.5, -deep + WALL + 0.20],
            [ALTAR[0] * 0.5, -deep + WALL + 0.20 + ALTAR[1]],
            [-ALTAR[0] * 0.5, -deep + WALL + 0.20 + ALTAR[1]],
        ]),
        PLINTH,
        PLINTH + ALTAR_HIGH,
    ));
    for index in 0..NAVE_POSTS {
        let x = -half + WIDTH * (index + 1) as f32 / (NAVE_POSTS + 1) as f32;
        for z in [-deep + 1.10, deep - 1.10] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, PLINTH, z] },
                building::Anchor::Free {
                    at: [x, PLINTH + HEIGHT - PLATE, z],
                },
                [1.0, 0.0],
                NAVE_POST,
                NAVE_POST,
            ));
        }
    }

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
        // **The 压顶: a tile cap over the wall's top.** A wall that stops in a bare plane is an unfinished wall, and the gate porch
        // above the gap is tiled — a compound whose gate is roofed and whose wall is not is a gate standing in a field. The cap is
        // a roof slab `COPE` thick and a hand wider than the wall on either side, the same answer the 石板街's kerb gave at the
        // other scale.
        skeleton.slabs.push(building::Slab::roof(
            vec![
                [from[0] - COPE_PROUD, from[1] - COPE_PROUD],
                [to[0] + COPE_PROUD, from[1] - COPE_PROUD],
                [to[0] + COPE_PROUD, to[1] + COPE_PROUD],
                [from[0] - COPE_PROUD, to[1] + COPE_PROUD],
            ],
            WALL_HIGH,
            WALL_HIGH + COPE,
        ));
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

    // **The 经幡柱: the flag poles outside the gate.** Every Dai wat has them — two tall poles either side of the entrance, each with
    // a cross-arm, and a banner hanging from each arm. They are members and not masses because a pole has no inside: a post, an arm
    // and a cloth, six members in all, and they are the first thing that tells a traveller that this is a temple and not a house.
    for across in [-1.0_f32, 1.0] {
        let x = across * FLAG_X;
        let pole = gate_z + FLAG_OUT;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free { at: [x, 0.0, pole] },
            building::Anchor::Free {
                at: [x, FLAG_HIGH, pole],
            },
            [1.0, 0.0],
            FLAG_POLE,
            FLAG_POLE,
        ));
        // The arm reaches towards the gate's axis, and the banner hangs off its end.
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, FLAG_HIGH - 0.30, pole],
            },
            building::Anchor::Free {
                at: [x - across * FLAG_ARM, FLAG_HIGH - 0.30, pole],
            },
            [0.0, 1.0],
            FLAG_POLE * 0.5,
            FLAG_POLE * 0.5,
        ));
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x - across * (FLAG_ARM - 0.10), FLAG_HIGH - 0.34, pole],
            },
            building::Anchor::Free {
                at: [
                    x - across * (FLAG_ARM - 0.10),
                    FLAG_HIGH - 0.34 - FLAG_CLOTH,
                    pole,
                ],
            },
            [1.0, 0.0],
            FLAG_POLE * 0.14,
            FLAG_ARM * 0.55,
        ));
    }

    // **The 放生池: the release pond, outside the gate.** A wat has a basin where live things are set free, and it stands where the
    // village comes to it — beside the gate and not behind the wall, where a yard's own wall would hide it from the only view that
    // matters. Its rim, and it is a **mass whose wall
    // is thick enough to be a rim** — `0.32 m` in a `3.4 × 2.4 m` plan, the same answer the wells gave in the two lanes. Its top is
    // the water's level, a hand above the paving, which is the only way this crate can say *water* at all.
    let pond = POND_AT;
    let mut basin = Mass::new(
        ops::wound(vec![
            [pond[0] - POND[0] * 0.5, pond[1] - POND[1] * 0.5],
            [pond[0] + POND[0] * 0.5, pond[1] - POND[1] * 0.5],
            [pond[0] + POND[0] * 0.5, pond[1] + POND[1] * 0.5],
            [pond[0] - POND[0] * 0.5, pond[1] + POND[1] * 0.5],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
        POND_HIGH,
    );
    basin.wall = POND_WALL;
    basin.slab = 0.16;
    skeleton.masses.push(basin);

    // **The 戒堂, the wat's second building.** A wat is not one hall: the 戒堂 is where the monks are ordained, it stands **behind**
    // the 大殿`s platform and to its **east**, so the plate sees both: it is smaller and its platform is lower — and it is closed, because the rite it holds
    // is not a public one. Two buildings on one axis, one open and one shut, is what an ordination hall is for.
    let [uposatha_x, uposatha_z] = UPOSATHA_AT;
    let uposatha_half = UPOSATHA[0] * 0.5;
    let uposatha_deep = UPOSATHA[1] * 0.5;
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [
                uposatha_x - uposatha_half - 0.60,
                uposatha_z - uposatha_deep - 0.60,
            ],
            [
                uposatha_x + uposatha_half + 0.60,
                uposatha_z - uposatha_deep - 0.60,
            ],
            [
                uposatha_x + uposatha_half + 0.60,
                uposatha_z + uposatha_deep + 0.60,
            ],
            [
                uposatha_x - uposatha_half - 0.60,
                uposatha_z + uposatha_deep + 0.60,
            ],
        ]),
        0.0,
        UPOSATHA_PLINTH,
    ));
    let mut uposatha = Mass::new(
        ops::wound(vec![
            [uposatha_x - uposatha_half, uposatha_z - uposatha_deep],
            [uposatha_x + uposatha_half, uposatha_z - uposatha_deep],
            [uposatha_x + uposatha_half, uposatha_z + uposatha_deep],
            [uposatha_x - uposatha_half, uposatha_z + uposatha_deep],
        ])
        .into_iter()
        .map(|at| PlanNode::new(at, UPOSATHA_PLINTH))
        .collect(),
        UPOSATHA_PLINTH + UPOSATHA_HIGH,
    );
    uposatha.wall = WALL;
    uposatha.slab = 0.26;
    let uposatha_id = MassId(skeleton.masses.len());
    skeleton.masses.push(uposatha);

    // **The 界石: the sima stones round the 戒堂.** An ordination hall is not just a building — it stands inside a **consecrated
    // precinct**, marked by a ring of stones set in the ground, and the precinct is the thing that makes a 戒堂 a 戒堂. Six stones
    // stand along its south and east sides, where the plate sees them; the west side is the 大殿's platform and a stone set inside
    // another building's plan is a stone on its floor.
    for (dx, dz) in [
        (-2.40_f32, 3.00_f32),
        (0.00, 3.00),
        (2.40, 3.00),
        (3.85, 1.00),
        (3.85, -1.00),
        (2.40, -3.00),
    ] {
        let mut stone = Mass::new(
            ops::wound(vec![
                [uposatha_x + dx - SIMA * 0.5, uposatha_z + dz - SIMA * 0.5],
                [uposatha_x + dx + SIMA * 0.5, uposatha_z + dz - SIMA * 0.5],
                [uposatha_x + dx + SIMA * 0.5, uposatha_z + dz + SIMA * 0.5],
                [uposatha_x + dx - SIMA * 0.5, uposatha_z + dz + SIMA * 0.5],
            ])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
            SIMA_HIGH,
        );
        stone.wall = SIMA * 0.26;
        stone.slab = 0.10;
        skeleton.masses.push(stone);
    }

    if let Some(roof) = derive::gable_roof_by(
        uposatha_id,
        &skeleton.masses[uposatha_id.0],
        UPOSATHA_OVERHANG,
        UPOSATHA_OVERHANG,
        UPOSATHA_RISE,
        SINK,
        ROOF_THICKNESS * 0.8,
    ) {
        skeleton.roofs.push(roof.with_covering(TILES));
    }
    if let Some(roof) = skeleton.roofs.last() {
        derive::crown_gables(&mut skeleton.masses[uposatha_id.0], roof);
    }

    // **The 塔: the stupa, which is the wat's landmark.** This crate has no solid of revolution, and a 塔 is round — but a Dai
    // stupa is **built in square courses that batter inward**, and that is a stack of masses this crate already has a word for:
    // `derive::stepped_courses` lays exactly that, each course carried by the one below through `Join::Storey` and sharing an
    // edge with it and no volume.
    //
    // The square base, four courses narrowing by `STUPA_BATTER` each, then the bell and the spire on top: eight masses from the
    // yard to `STUPA_TOP`. **A square stupa is not a compromise**, it is the form a brick 塔 of the upper Mekong actually takes.
    let stupa = STUPA_AT;
    let half_stupa = STUPA * 0.5;
    let base = [
        [stupa[0] - half_stupa, stupa[1] - half_stupa],
        [stupa[0] + half_stupa, stupa[1] - half_stupa],
        [stupa[0] + half_stupa, stupa[1] + half_stupa],
        [stupa[0] - half_stupa, stupa[1] + half_stupa],
    ];
    // **The platform it stands on**: a square two steps up, so the 塔 has a base to rise from.
    skeleton.slabs.push(building::Slab::floor(
        ops::wound(vec![
            [base[0][0] - 0.85, base[0][1] - 0.85],
            [base[1][0] + 0.85, base[1][1] - 0.85],
            [base[2][0] + 0.85, base[2][1] + 0.85],
            [base[3][0] - 0.85, base[3][1] + 0.85],
        ]),
        0.0,
        STUPA_PLINTH,
    ));
    let courses = derive::stepped_courses(
        &mut skeleton,
        base,
        STUPA_COURSE,
        STUPA_PLINTH + STUPA_COURSE * STUPA_COURSES as f32,
        STUPA_COURSES,
        STUPA_BATTER,
        STUPA_WALL,
        0.16,
    );
    // **The bell and the spire**, each a mass carried by the one under it: the bell tapers to the spire's foot, and the spire is
    // a third of the whole — which is what a 塔 is: a base, a bell, and a needle.
    if let Some(top) = courses.last() {
        // **The bands stand on the courses' nominal top and each on the one before it.** `Mass::highest` reads the wall's own crown
        // and not the stack's summit, so the first band's foot is the height the courses were laid to; and **each band names the one
        // under it**, because a chain of nested plans is a chain and not a pile — naming the bottom course for all three left every
        // pair unrelated, which is `UnattachedOverlap { 14, 15 }`, `{ 14, 16 }`, `{ 15, 16 }` and three `StoreyNotOnTop`.
        let mut foot = STUPA_PLINTH + STUPA_COURSE * STUPA_COURSES as f32;
        let mut host = *top;
        // **A band's plan sets its wall.** A wall wider than half the plan folds the inner loop (`OffsetEdgeFolded`) and a plan that
        // shrinks past nothing gives a negative wall (`MassThickness { wall: -0.0102 }`, measured here) — so the bell and the spire
        // are `0.62`, `0.46` and `0.30` across their half, and their wall is a fraction of that and never a number of its own.
        // **The bell, its shoulder, the lotus seat, the spire and the needle**, six bands: `(half width, height)`. A stupa's spire
        // is not one column — it is a **lotus seat** where the bell ends and a **tapering needle** above it, and a spire without
        // that pinch is a chimney.
        for (size, high) in [
            (0.62_f32, 0.95_f32),
            (0.46, 0.55),
            (0.44, 0.18),
            (0.30, 0.85),
            (0.20, 0.55),
            (0.11, 0.62),
        ] {
            let mut band = Mass::new(
                ops::wound(vec![
                    [stupa[0] - size, stupa[1] - size],
                    [stupa[0] + size, stupa[1] - size],
                    [stupa[0] + size, stupa[1] + size],
                    [stupa[0] - size, stupa[1] + size],
                ])
                .into_iter()
                .map(|at| PlanNode::new(at, foot))
                .collect(),
                foot + high,
            );
            band.wall = size * 0.34;
            band.slab = 0.14;
            band.attachment = Some(building::Attachment {
                host,
                join: building::Join::Storey,
            });
            host = MassId(skeleton.masses.len());
            skeleton.masses.push(band);
            foot += high;
        }
    }

    Building::new(skeleton)
}
