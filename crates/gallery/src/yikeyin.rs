//! A 一颗印: the Yunnan courtyard house — twelve by fifteen, a three-metre light well in the middle, two
//! storeys round it, and gable walls that rise to their roofs.
//!
//! # What this scene is for
//!
//! The compound is the Bai 三坊一照壁, drawn as three ranges and a screen wall. This is the other Yunnan
//! type and the tighter one: **一颗印**, "a seal", so called because its plan is nearly square and its
//! court is a well rather than a garden. Four ranges round a 3 × 3 court, the 正房 at the back, two 厢房
//! down the sides, and the 门房 across the front with the one gate in it.
//!
//! It is the first scene to use both constructs the last two arcs added: **硬山** gable ends, so each
//! range's roof stops at its own gable wall, and **山墙冠**, so each gable wall rises to the ridge instead
//! of stopping at the wall top. A 一颗印 is read by exactly those two.
//!
//! # What it does not draw
//!
//! - **no court void** — the court is the gap between four ranges, as in the compound, because a roof over
//!   a *ring* is a derivation this crate does not have (四水归堂 is four ranges whose eaves meet, and that
//!   is the composition this scene is);
//! - **one opening** — the gate, in the 门房's street face; the doors and windows behind it are the
//!   interior arc's work;
//! - **no 夏子** — the porch the research notes describe on the 正房, which is a plate on posts.

use definition::Value;
use scene::standard::Stage;

use building::derive;
use building::grid::FaceRef;
use building::openings::Inset;
use building::site;
use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, ops};

use crate::facade::{self};
use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::kit::{
    camera, environment_light, flux_for_irradiance, frame, image, instance, instances,
    light_with_radius, mesh, mesh_material, post_processes, screen_material, surface,
};
use document::shaders::{Projection, post_shader};

/// The air over the block, and **the half of the light the sun is not**.
///
/// A well three metres across under one directional lamp is a hole with nothing in it: the shading
/// already carries a sky term, but it reads that term out of the scene's own environment map, and a scene
/// that declares none answers zero there. This is that map's own sky, and the same one the frame is graded
/// against, so the ambient and the background cannot disagree.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.18, 0.34, 0.60])
        .horizon([0.80, 0.77, 0.71])
        .ground([0.36, 0.33, 0.28])
        .density(0.0009)
        .scale_height(220.0)
}

/// The site: a nearly square plan, thirteen by fifteen, which is the type's own proportion.
const HALF_X: f32 = 6.5;
const HALF_Z: f32 = 7.5;

/// The 走马转角楼: the first-floor gallery round the court — its level, how deep it runs into the well, its decks thickness, its posts section
/// and the rails height above the deck.
const GALLERY_AT: f32 = 2.95;
const GALLERY_DEEP: f32 = 0.34;
const GALLERY_DECK: f32 = 0.10;
const GALLERY_POST: f32 = 0.13;
const GALLERY_RAIL: f32 = 0.80;

/// The court: three metres square, the well the ranges are built round.
const COURT: f32 = 1.5;

/// The grating through the court's paving, at its middle, and the stone it stands clear of.
const GRATE: f32 = 0.45;

/// The depths of the four ranges, and their heights. The 正房 is the tallest, the gate house the lowest.
const MAIN_DEPTH: f32 = 6.0;
const MAIN_HEIGHT: f32 = 6.2;
const WING_HEIGHT: f32 = 5.6;
const GATE_HEIGHT: f32 = 3.6;

/// The gateway: how high its threshold stands, how far its 门罩 reaches out, and the head its door is hung at.
///
/// **The sill is a length and not a detail**: an opening that reaches the floor is a hole in the bottom of the
/// wall, and a board standing in one splits the wall into two shells. `GATE_HEAD` is the door's own head, read
/// off the same inset the cut uses.
/// The platform's own height above the court, and how far it stands proud of the range it carries.
///
/// **The height is the seat's offset**, so the ranges and their platforms cannot disagree: the seat moves the
/// building up by TAI_MING and the platforms fill exactly that gap.
const TAI_MING: f32 = 0.45;
const TAI_MING_PROUD: f32 = 0.30;

/// How wide the 踏步 at each door are, in metres.
const STEP_WIDTH: f32 = 1.9;

const GATE_SILL: f32 = 0.15;
const GATE_HOOD: f32 = 0.85;
const GATE_HEAD: f32 = GATE_HEIGHT - 0.9;

/// The boards of the gate: four planks, `0.13` wide and `0.07` thick, so the gate reads as a door and not as a
/// grille.
const GATE_BOARDS: facade::Bar = facade::Bar {
    count: 4,
    width: 0.13,
    depth: 0.07,
};

/// The common numbers: a wall, a plate, deep eaves, a gable, its rise, and **no overhang at the ends**.
const WALL: f32 = 0.45;
const PLATE: f32 = 0.35;
/// The 照壁 in front of the gate: its width, its depth, its height, its own wall, and the gap it stands off the range.'n///'n/// **The depth is five tenths and the wall is one and eight tenths of a tenth of that**, so the inner loop cannot fold.'nconst SCREEN_HALF: f32 = 2.60;'nconst SCREEN_THICK: f32 = 0.50;'nconst SCREEN_HIGH: f32 = 3.10;'nconst SCREEN_WALL: f32 = 0.18;'nconst SCREEN_GAP: f32 = 1.70;'nconst SCREEN_EAVE: f32 = 0.28;'nconst SCREEN_RISE: f32 = 0.45;
/// The 照壁`s framed panel: how far the moulding stands proud, its own square section, and the margin it keeps off the wall`s
/// own edges.
const PANEL_PROUD: f32 = 0.035;
const PANEL_BAR: f32 = 0.055;
const PANEL_MARGIN: f32 = 0.42;

/// The 照壁 in front of the gate: its width, its depth, its height, its own wall, and the gap it stands off the range.
///
/// **The depth is five tenths and the wall is under half of it**, so the inner loop cannot fold (round 73's invariant).
const SCREEN_HALF: f32 = 2.60;
const SCREEN_THICK: f32 = 0.50;
const SCREEN_HIGH: f32 = 3.10;
const SCREEN_WALL: f32 = 0.18;
const SCREEN_GAP: f32 = 1.70;
const SCREEN_EAVE: f32 = 0.28;
const SCREEN_RISE: f32 = 0.45;
/// The 照壁`s framed panel: how far the moulding stands proud, its own square section, and the margin it keeps off the wall`s
/// own edges.

const OVERHANG: f32 = 0.65;
const GABLE: f32 = 0.0;
const RISE: f32 = 1.8;
const SINK: f32 = 0.1;

/// The rafter tails under every eave: how far apart they stand, and the wood's own section.
const RAFTER_SPACING: f32 = 0.42;
const RAFTER_WIDTH: f32 = 0.09;
const RAFTER_DEPTH: f32 = 0.11;
const ROOF_THICKNESS: f32 = 0.22;

/// The covering every roof wears, and the 举架 ladder its slopes carry.
const COVERING: Covering = Covering {
    period: 0.30,
    fill: 0.62,
    height: 0.06,
};
const BEAM: f32 = 0.8;
const PROFILE: [building::roof::Step; 3] = [
    building::roof::Step {
        fraction: 0.25,
        rise: BEAM * 0.50,
    },
    building::roof::Step {
        fraction: 0.50,
        rise: BEAM * 0.50 + BEAM * 0.60,
    },
    building::roof::Step {
        fraction: 0.75,
        rise: BEAM * 0.50 + BEAM * 0.60 + BEAM * 0.75,
    },
];

/// The sun, high and from the gate's own side, so the court is lit down one wall and not the other.
const SUN: [f64; 3] = [-18.0, 40.0, 24.0];

/// The court's own sun: high and from the south, so the shaft reaches the platform and the steps rather than
/// stopping on the four roofs. **It is a scene's sun and not the building's** — the same document, lit for the
/// one frame that is about the court.
const COURT_SUN: [f64; 3] = [-6.0, 46.0, 10.0];
const SUN_IRRADIANCE: f64 = 3.3;
const SHADOW_EXTENT: f64 = 30.0;

/// The image, and the lens.
const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// How the ground is drawn: one-metre cells over 120 metres, level.
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 60.0;

/// Builds **the same building** from inside its court, which is the only place its own faces can be seen.
///
/// # Invariants
///
/// **It is one document and two cameras**: the expansion, its rows and its meshes are the same, and only
/// the eye differs. The eye stands in the court at a person's height, in the corner the sun does not fill,
/// looking at the 正房 and the 夏子 across the well.
pub fn yikeyin_court() -> Value {
    // **The court has a sun of its own, and it is nearly overhead.** A 一颗印's court is a well: the four roofs
    // leave it in shadow all afternoon, which is the type''s own fact (四水归堂 is the light coming down the one
    // hole) and also why an eye-level frame of it rendered almost black — measured, the round that put the
    // 台明's steps in found them in the identity table and could not show them. At `COURT_SUN` the light reaches
    // the platform, the steps and the porch's posts, and the court is photographed as the room it is.
    scene(
        // **The court is three metres across and six metres deep in walls**, so its photograph needs a wide lens
        // and a place to stand: from the court's own south side, looking up at the 正房's platform and eaves
        // through a 97-degree field, which is the only framing in which one frame holds the steps, the porch's
        // posts and the sky-well together.
        // **The camera looks down at the platform and not up at the eaves**: the steps are on the floor, and a
        // court frame aimed at the sky-well is a frame of the sky-well.
        [0.2, 2.1, 1.3],
        [-0.2, 0.35, -1.4],
        1.70,
        "yikeyin_court",
        COURT_SUN,
    )
}

/// Builds the scene, from outside the block.
pub fn yikeyin() -> Value {
    scene(
        [17.0, 19.0, 23.0],
        [-0.5, 1.2, 0.5],
        FIELD_OF_VIEW,
        "yikeyin",
        SUN,
    )
}

/// Builds the scene from one camera: the document, its meshes, and the sky it hangs under.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str, sun: [f64; 3]) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("a 一颗印 expands: four ranges, four roofs and four floors");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a 一颗印's own document must be clean, and it is not: {violations:?}"
    );

    let table = building::identity::Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    let roofs = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Roof(_, _) | building::Element::Roll(_, _)
        )
    });
    // **The court's paving is not a floor**, and it is drawn as what it is: the four 台明 and the storeys are the
    // `floors` mesh, and the stone the court is laid with is its own — a surface a person walks on in the open, with the
    // grate through it, and not the underside of a room.
    let paved: Vec<usize> = document()
        .skeleton
        .slabs
        .iter()
        .enumerate()
        .filter(|(_, slab)| slab.role == building::slab::SlabRole::Paving)
        .map(|(index, _)| index)
        .collect();
    let is_paving = |element: &building::Element| matches!(element, building::Element::Slab(id, _) if paved.contains(&id.0));
    let floors = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            )
        ) && !is_paving(element)
    });
    let paving = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            )
        ) && is_paving(element)
    });

    // **The bars of the 直棂窗**, which are members and had no mesh before: a document that carries them and a
    // scene that does not draw them is a bar nobody can see.
    let bars = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
    });

    let plaster = || surface([0.80, 0.76, 0.68], 0.0, 0.86, [0.0; 3]);
    let tiles = || surface([0.30, 0.29, 0.28], 0.0, 0.60, [0.0; 3]);
    let floor = || surface([0.33, 0.25, 0.17], 0.0, 0.80, [0.0; 3]);
    // **Wet stone**: the court is the one surface in a 一颗印 that rain reaches, so its paving is darker and much
    // smoother than the platforms around it.
    let paved_stone = || surface([0.24, 0.23, 0.21], 0.0, 0.34, [0.0; 3]);
    // **Wet stone**: the court is the one surface in a 一颗印 that rain reaches, so its paving is darker and much
    // smoother than the platforms around it.
    let paved_stone = || surface([0.24, 0.23, 0.21], 0.0, 0.34, [0.0; 3]);
    let earth = || surface([0.31, 0.28, 0.22], 0.0, 0.94, [0.0; 3]);
    // The bars are wood, not plaster: a dark warm brown that reads against the wall behind them.
    let timber = || surface([0.22, 0.14, 0.08], 0.0, 0.72, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("wall", plaster as fn() -> Value),
        ("roof", tiles as fn() -> Value),
        ("floor", floor as fn() -> Value),
        ("paved", paved_stone as fn() -> Value),
        ("bars", timber as fn() -> Value),
    ] {
        materials.push(mesh_material(
            &format!("{name}-cast"),
            "authored_shadowing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{name}-depth"),
            "authored_depthing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{name}-shade"),
            "authored_lighting",
            body(),
        ));
    }
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, earth()));
    }

    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("roofs", "roof"),
            ("floors", "floor"),
            ("paving", "paved"),
            ("bars", "bars"),
            ("ground", "ground"),
        ]
        .into_iter()
        .map(|(label, material)| {
            instance(
                format!("{label}-{stage}"),
                label,
                [0.0; 3],
                [0.0; 3],
                [1.0; 3],
                &format!("{material}-{stage}"),
            )
        })
        .collect::<Vec<_>>()
    };

    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();
    let ground = crate::ground::mesh(&building, GROUND_DIVISIONS, GROUND_EXTENT, |_, _| 0.0);

    Value::structure([
        ("name".to_owned(), Value::String(name.to_owned())),
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            Value::List(vec![camera(eye, target, field_of_view)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "sun",
                    sun,
                    [1.0, 0.92, 0.78],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(sun)),
                    SHADOW_EXTENT,
                    170.0,
                ),
                // **The sky as a light**, which is the term the court lives on: a well under one lamp is a
                // hole, and its own walls see the dome instead. The pixels below are the same air, so the
                // ambient and the background are one sky and cannot drift apart.
                // **The sky is the court's only other light, and a well's own opening is bright**: the court is
                // photographed with the environment turned up, which is what a sky-well does to a small court and
                // is a scene's own number, not a change to any building's material.
                environment_light(
                    "air",
                    "air",
                    [1.0; 3],
                    if name == "yikeyin_court" { 2.6 } else { 1.06 },
                ),
            ]),
        ),
        (
            "images".to_owned(),
            Value::List(vec![image(
                "air",
                256,
                128,
                atmosphere().environment_pixels(256, 128),
                Projection::Latlong,
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List({
                let helpers = atmosphere().wgsl();
                let mut shaders = stage_set("authored", "", helpers.as_str(), &point, &extras);
                shaders.extend(stage_set(
                    "plain",
                    "",
                    helpers.as_str(),
                    &plain.point_schema(),
                    &plain,
                ));
                shaders.push(post_shader(GRADE, &helpers, Some(Projection::Latlong)));
                shaders
            }),
        ),
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("floors", "Triangles", point.clone(), floors),
                mesh("paving", "Triangles", point.clone(), paving),
                mesh("bars", "Triangles", point.clone(), bars),
                mesh("ground", "Triangles", plain.point_schema(), ground),
            ]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "instances".to_owned(),
            instances(vec![
                (Stage::Shadowing, placed("cast")),
                (Stage::Depthing, placed("depth")),
                (Stage::Lighting, placed("shade")),
            ]),
        ),
    ])
}

/// Returns the document: four ranges round a three-metre court, each crowned at its gable ends.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    // **The 正房 at the back**, its ridge along `x`, so its gable ends face east and west.
    let ranges = [
        (
            [
                [-HALF_X, -HALF_Z],
                [HALF_X, -HALF_Z],
                [HALF_X, -HALF_Z + MAIN_DEPTH],
                [-HALF_X, -HALF_Z + MAIN_DEPTH],
            ],
            MAIN_HEIGHT,
            true,
        ),
        (
            [
                [-HALF_X, -HALF_Z + MAIN_DEPTH],
                [-COURT, -HALF_Z + MAIN_DEPTH],
                [-COURT, HALF_Z],
                [-HALF_X, HALF_Z],
            ],
            WING_HEIGHT,
            true,
        ),
        (
            [
                [COURT, -HALF_Z + MAIN_DEPTH],
                [HALF_X, -HALF_Z + MAIN_DEPTH],
                [HALF_X, HALF_Z],
                [COURT, HALF_Z],
            ],
            WING_HEIGHT,
            true,
        ),
        (
            [
                [-COURT, COURT],
                [COURT, COURT],
                [COURT, HALF_Z],
                [-COURT, HALF_Z],
            ],
            GATE_HEIGHT,
            false,
        ),
    ];
    for (corners, height, crowned) in ranges {
        range(&mut skeleton, corners, height, crowned);
    }
    let _ = skeleton.masses.len();
    // **The gate is a door and not a hole.** A 一颗印 is entered through its 门房, and what makes that an
    // entrance is the three things a doorway has: a **threshold** to step over, **boards** that close it, and a
    // **门罩** over it. The threshold is the wall's own foot — the opening starts `GATE_SILL` above the ground
    // and the wall below it is the stone sill - because a member standing in an opening that reaches the floor
    // splits the wall's own shell into two loops, which `validate` names `SolidOpen` (measured on the street's
    // shopfront before it was given its own threshold).
    if let Some(face) = facade::edge_on(&skeleton.masses[3], HALF_Z, true) {
        let gate_face = FaceRef::new(MassId(3), face);
        // **The gate house is one bay**, not three: a 3 m front cut into three leaves a 0.30 m middle bay,
        // which is a cat flap, and the boards in it were skipped by the very guard that refuses bars in a hole
        // too narrow for them -- measured as `yikeyin` gaining three identity rows for a gate it did not get.
        if ops::split_face(
            &mut skeleton,
            gate_face,
            &building::grid::Rhythm::Even { count: 1 },
            &building::grid::Rhythm::Even { count: 1 },
        )
        .is_some()
        {
            let _ = facade::open_with_bars(
                &mut skeleton,
                gate_face,
                0,
                0,
                // **The margin is the wall's own thickness plus five centimetres**, the rule every other
                // opening in this crate follows: the face an opening is measured against is the inner one, and
                // a 0.35 margin on a 0.45 wall overran it by 0.101 -- `OpeningOutsideWall`, measured.
                Inset::new(0.50, 0.50, GATE_SILL, 0.9),
                GATE_BOARDS,
            );
        }
        // **The 门罩**: a small shed over the door, hosted by the gate house, shedding away from the wall.
        // Its crest stands on the wall above the door's head and its eave reaches out over the step, which is
        // what a 门罩 is for — the rain never falls on the person waiting for the gate to open.
        let half_gate = 1.1;
        let plan = [
            [-half_gate, HALF_Z + GATE_HOOD],
            [half_gate, HALF_Z + GATE_HOOD],
            [half_gate, HALF_Z],
            [-half_gate, HALF_Z],
        ];
        use building::derive;
        if let Some(hood) =
            derive::lean_to(MassId(3), plan, GATE_HEAD - 0.15, GATE_HEAD + 0.35, 0.10)
        {
            skeleton.roofs.push(hood);
        }
    }
    // **Every range opens onto the court**, which is what a 一颗印's four inner faces are: bays of doors
    // and windows, two storeys of them, and the court is the circulation they open onto.
    court_faces(&mut skeleton);
    // **The 夏子**, the porch across the 正房's front: a lean-to of its own, two posts at its edge, and two
    // struts pinned to the 正房's wall face — the face anchor's first use in a scene.
    porch(&mut skeleton);
    // **One seat for the whole compound**, so the four ranges share a level rather than each finding its
    // own: a 一颗印 is built flat, and its court is a well and not a slope.
    //
    // **And the seat is `TAI_MING` off the ground**, because a Chinese courtyard's ranges do not stand on the
    // paving: they stand on a stone platform — the 台明 — and the court is the floor below it, which is what makes
    // the court a room and the ranges its walls. The offset is the seat's own argument and the platform is what
    // fills the gap it opens.
    let _ = site::seat(&mut skeleton, |_| 0.0, TAI_MING);
    // The four platforms, one under each range: the range's own plan proud of it by `TAI_MING_PROUD`, from the
    // court's own level up to the seat.
    for index in 0..4 {
        let Some(mass) = skeleton.masses.get(index) else {
            continue;
        };
        let mut plan: Vec<[f32; 2]> = mass.outline().iter().map(|node| node.position).collect();
        let middle = {
            let sum = plan.iter().fold([0.0_f32, 0.0_f32], |sum, at| {
                [sum[0] + at[0], sum[1] + at[1]]
            });
            [sum[0] / plan.len() as f32, sum[1] / plan.len() as f32]
        };
        for at in &mut plan {
            let out = [at[0] - middle[0], at[1] - middle[1]];
            let length = (out[0] * out[0] + out[1] * out[1]).sqrt().max(1.0e-3);
            let grow = (length + TAI_MING_PROUD) / length;
            *at = [middle[0] + out[0] * grow, middle[1] + out[1] * grow];
        }
        if loop_area(&plan) < 0.0 {
            plan.reverse();
        }
        skeleton
            .slabs
            .push(building::Slab::plate(plan, 0.0, TAI_MING));
    }
    // **The 踏步: two stone steps at every door.** A 台明 without steps is a platform a person cannot reach, and
    // the steps are the half of it the eye reads first. Each range's court-facing wall is found by taking the
    // edge whose own middle stands nearest the court's centre — the court is the square the four ranges face —
    // and the flight is laid out along that edge's outward normal, one tread a step, from the court's level up
    // to the platform.
    for index in 0..4 {
        let Some(mass) = skeleton.masses.get(index) else {
            continue;
        };
        let mut best: Option<([f32; 2], [f32; 2], f32)> = None;
        for edge in 0..mass.outline().len() {
            let here = mass.outline()[edge].position;
            let there = mass.outline()[(edge + 1) % mass.outline().len()].position;
            let middle = [(here[0] + there[0]) * 0.5, (here[1] + there[1]) * 0.5];
            let distance = middle[0] * middle[0] + middle[1] * middle[1];
            let axis = [there[0] - here[0], there[1] - here[1]];
            let length = (axis[0] * axis[0] + axis[1] * axis[1]).sqrt().max(1.0e-3);
            // The outward normal, taken towards the court: the edge's own left-hand normal points away from a
            // positively wound loop's solid, and which way is "away" is decided by the edge's middle.
            let mut normal = [-axis[1] / length, axis[0] / length];
            if normal[0] * middle[0] + normal[1] * middle[1] > 0.0 {
                normal = [-normal[0], -normal[1]];
            }
            if best.map_or(true, |(_, _, was)| distance < was) {
                best = Some((middle, normal, distance));
            }
        }
        let Some((middle, normal, _)) = best else {
            continue;
        };
        // The flight: `STEPS` treads, each a plate `STEP_WIDTH` wide, stepping out along the normal.
        let risers = 2;
        let rise = TAI_MING / risers as f32;
        let run = 0.34;
        for step in 0..risers {
            let out = 0.12 + run * (step as f32 + 0.5);
            let at = [middle[0] + normal[0] * out, middle[1] + normal[1] * out];
            let across = [-normal[1], normal[0]];
            let level = rise * (step + 1) as f32;
            let half = STEP_WIDTH * 0.5;
            let mut plan = vec![
                [
                    at[0] - across[0] * half - normal[0] * run * 0.5,
                    at[1] - across[1] * half - normal[1] * run * 0.5,
                ],
                [
                    at[0] + across[0] * half - normal[0] * run * 0.5,
                    at[1] + across[1] * half - normal[1] * run * 0.5,
                ],
                [
                    at[0] + across[0] * half + normal[0] * run * 0.5,
                    at[1] + across[1] * half + normal[1] * run * 0.5,
                ],
                [
                    at[0] - across[0] * half + normal[0] * run * 0.5,
                    at[1] - across[1] * half + normal[1] * run * 0.5,
                ],
            ];
            if loop_area(&plan) < 0.0 {
                plan.reverse();
            }
            skeleton.slabs.push(building::Slab::plate(plan, 0.0, level));
        }
    }

    // **And the court is paved, with its grating at the middle** — which is where 四水归堂's water ends:
    // the four roofs shed inward and the paving's one hole is what they shed into.
    let court = [
        [-COURT, -COURT],
        [COURT, -COURT],
        [COURT, COURT],
        [-COURT, COURT],
    ];
    if let Some(paving) = derive::court_paving(court, 0.02, 0.08, GRATE, 0.30) {
        skeleton.slabs.push(paving);
    }

    // **The 照壁: the screen wall the courtyard sets in front of its gate.** A 一颗印's gate opens straight onto the lane,
    // and what a Yunnan courtyard sets against the lane is a **照壁** — a white screen wall, head high and wider than the
    // gate, wearing a tiled coping. It is a wall and not a building: a mass `SCREEN_THICK` deep with a `SCREEN_WALL` in it,
    // so the inner loop is a third of the depth and cannot fold (round 73's invariant), and a gable over a half-metre span is
    // its coping, which is the same `gable_roof` call the ranges wear at a different span.
    let screen_z = HALF_Z + SCREEN_GAP;
    let mut screen = Mass::new(
        [
            PlanNode::new([-SCREEN_HALF, screen_z - SCREEN_THICK * 0.5], 0.0),
            PlanNode::new([SCREEN_HALF, screen_z - SCREEN_THICK * 0.5], 0.0),
            PlanNode::new([SCREEN_HALF, screen_z + SCREEN_THICK * 0.5], 0.0),
            PlanNode::new([-SCREEN_HALF, screen_z + SCREEN_THICK * 0.5], 0.0),
        ]
        .to_vec(),
        SCREEN_HIGH,
    );
    screen.wall = SCREEN_WALL;
    screen.slab = 0.22;
    let screen_id = MassId(skeleton.masses.len());
    skeleton.masses.push(screen);
    if let Some(coping) = derive::gable_roof(
        screen_id,
        &skeleton.masses[screen_id.0],
        SCREEN_EAVE,
        SCREEN_RISE,
        0.04,
        0.12,
    ) {
        skeleton.roofs.push(coping.with_covering(COVERING));
    }

    // **The 照壁's panel: a frame of four thin mouldings on its face towards the gate.** A white screen wall is not blank — it
    // carries a **framed panel** where the four characters of the household's motto are written, and the frame is what makes the
    // wall a page instead of a surface. Four members `PANEL_PROUD` proud of the plaster, `PANEL_BAR` square, set in from the
    // wall's ends by `PANEL_MARGIN` and from its foot by the same.
    // **The panel faces the lane and not the gate**, because this screen stands *outside* the gate: it is a 外照壁, and what a
    // 外照壁 is for is the street`s own view. Its face is the `+z` one, which is the one the plate is taken from.
    let face = screen_z + SCREEN_THICK * 0.5 + PANEL_PROUD;
    let half = SCREEN_HALF - PANEL_MARGIN;
    let foot = PANEL_MARGIN;
    let head = SCREEN_HIGH - PANEL_MARGIN;
    for (from, to) in [
        ([-half, foot], [half, foot]),
        ([-half, head], [half, head]),
        ([-half, foot], [-half, head]),
        ([half, foot], [half, head]),
    ] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [from[0], from[1], face],
            },
            building::Anchor::Free {
                at: [to[0], to[1], face],
            },
            [0.0, 1.0],
            PANEL_BAR,
            PANEL_BAR,
        ));
    }

    // **The 走马转角楼: the gallery that walks round the court.** A 一颗印 is read by its two gable walls from outside and by this from inside — a
    // **first-floor corridor running round three sides of the well**, carried on posts, so the household can walk from the 正房 to either 厢房 without
    // coming down. It is three decks, a post at each corner and a rail on the court's edge, and it is the one thing this type was missing.
    //
    // **It is `GALLERY_DEEP` and no deeper**: the court is three metres across, and a corridor wide enough to walk in on three sides of a three-metre
    // well would close the well. At this depth the light still reaches the paving and the corridor still reads as a way round.
    let level = GALLERY_AT;
    let decks: [([f32; 2], [f32; 2]); 3] = [
        ([COURT, -COURT], [-COURT, -COURT]),
        ([-COURT, -COURT], [-COURT, COURT]),
        ([COURT, -COURT], [COURT, COURT]),
    ];
    for (from, to) in decks {
        let inward: [f32; 2] = if (to[0] - from[0]).abs() < 1.0e-3 {
            [if from[0] > 0.0 { -1.0 } else { 1.0 }, 0.0]
        } else {
            [0.0, 1.0]
        };
        let [ax, az] = from;
        let [bx, bz] = to;
        let [dx, dz] = inward;
        let plan = if dz.abs() > 0.5 {
            vec![
                [ax, az],
                [bx, bz],
                [bx, bz + GALLERY_DEEP],
                [ax, az + GALLERY_DEEP],
            ]
        } else {
            vec![
                [ax, az],
                [bx, bz],
                [bx + dx * GALLERY_DEEP, bz],
                [ax + dx * GALLERY_DEEP, az],
            ]
        };
        skeleton.slabs.push(building::Slab::paving(
            ops::wound(plan.clone()),
            level,
            level + GALLERY_DECK,
        ));
        // **The rail on the court's edge**, a member from one end of the deck to the other.
        let rail = [
            [
                ax + dx * (GALLERY_DEEP - 0.06),
                az + dz * (GALLERY_DEEP - 0.06),
            ],
            [
                bx + dx * (GALLERY_DEEP - 0.06),
                bz + dz * (GALLERY_DEEP - 0.06),
            ],
        ];
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [rail[0][0], level + GALLERY_RAIL, rail[0][1]],
            },
            building::Anchor::Free {
                at: [rail[1][0], level + GALLERY_RAIL, rail[1][1]],
            },
            // A rail's `across` is its run turned a quarter (round 134): a rail along `x` takes `[0, 1]`, one along `z` takes `[1, 0]`.
            if dz.abs() > 0.5 {
                [0.0, 1.0]
            } else {
                [1.0, 0.0]
            },
            GALLERY_POST * 0.6,
            GALLERY_POST * 0.7,
        ));
    }
    // **The posts under the corridor's court-side edge**, one at each end of each deck.
    for [x, z] in [
        [
            COURT - GALLERY_DEEP + GALLERY_POST * 0.5,
            -COURT + GALLERY_DEEP - GALLERY_POST * 0.5,
        ],
        [
            -COURT + GALLERY_DEEP - GALLERY_POST * 0.5,
            -COURT + GALLERY_DEEP - GALLERY_POST * 0.5,
        ],
        [
            -COURT + GALLERY_DEEP - GALLERY_POST * 0.5,
            COURT - GALLERY_DEEP + GALLERY_POST * 0.5,
        ],
        [
            COURT - GALLERY_DEEP + GALLERY_POST * 0.5,
            COURT - GALLERY_DEEP + GALLERY_POST * 0.5,
        ],
    ] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free { at: [x, 0.0, z] },
            building::Anchor::Free { at: [x, level, z] },
            [1.0, 0.0],
            GALLERY_POST,
            GALLERY_POST,
        ));
    }

    Building::new(skeleton)
}

/// Cuts every range's court face into bays and puts a door and windows in them.
///
/// # Invariants
///
/// **Each face is found by position, not by index**, and each is gridded three bays wide by two storeys
/// tall: a door in the middle bay of the ground storey, and a window in every bay of the upper one — which
/// is what a 一颗印's four inner walls are. The 门房 is the exception and it is the type's own: one storey,
/// one bay, one door, because it is a gate house and not a range.
fn court_faces(skeleton: &mut BuildingSkeleton) {
    // The four court faces, as (mass, the plan line they stand on, whether the line is a `z`, its front).
    let faces = [
        (MassId(0), -HALF_Z + MAIN_DEPTH, true, facade::Front::RANGE),
        (MassId(1), -COURT, false, facade::Front::RANGE),
        (MassId(2), COURT, false, facade::Front::RANGE),
        (MassId(3), COURT, true, facade::Front::GATE),
    ];
    for (mass, line, on_z, front) in faces {
        facade::open_court_face(skeleton, mass, line, on_z, front);
    }
}

/// Adds the **夏子**: the porch roof, its two posts and the two struts that hold it to the wall.
///
/// # Invariants
///
/// **Its roof is a lean-to and its struts are pinned to a face.** The porch stands in front of the 正房's
/// court wall (mass 0's edge 2, which runs from `x = +6.5` to `x = -6.5`), so a strut's wall end is an
/// [`Anchor::OnFace`] at a station along that edge and its outer end is free at the porch's own eave —
/// which is the whole point of the face anchor: the strut follows the wall rather than a coordinate that
/// has to be kept in step with it.
fn porch(skeleton: &mut BuildingSkeleton) {
    let wall = -HALF_Z + MAIN_DEPTH;
    let depth = 1.2;
    let low = 2.9;
    let high = 3.55;
    let half = 4.5;
    let Some(roof) = derive::lean_to(
        MassId(0),
        [
            [-half - 0.15, wall + depth + 0.15],
            [half + 0.15, wall + depth + 0.15],
            [half + 0.15, wall],
            [-half - 0.15, wall],
        ],
        low,
        high,
        0.1,
    ) else {
        return;
    };
    skeleton.roofs.push(roof.with_covering(COVERING));
    // The two posts at the porch's own edge, and the two struts from the wall to them.
    for side in [-1.0_f32, 1.0] {
        let x = side * (half - 0.3);
        let edge = wall + depth + 0.1;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free { at: [x, 0.0, edge] },
            building::Anchor::Free {
                at: [x, low - 0.05, edge],
            },
            [1.0, 0.0],
            0.16,
            0.16,
        ));
        // The strut: from below the porch roof's high edge, down and out to the post's head. **The
        // court edge is found by position and not by index**: the 正房's outline was split when its gable
        // ends were crowned, so edge 2 is a three-metre gable edge now and the court wall is somewhere else
        // in the loop.
        let Some((wall_edge, start, _length)) = court_edge(&skeleton.masses[0], wall) else {
            continue;
        };
        skeleton.members.push(building::Member::new(
            building::Anchor::OnFace {
                face: FaceRef::new(MassId(0), wall_edge),
                along: (start - x).abs(),
                up: high - 0.35,
            },
            building::Anchor::Free {
                at: [x, low - 0.05, edge],
            },
            [0.0, 1.0],
            0.12,
            0.12,
        ));
    }
}

/// Returns the mass's edge that lies on the plan line `z`, its start node's `x`, and its length.
///
/// # Invariants
///
/// **The edge is matched by its two nodes' own `z`**, both of them on the line, which is the only reading
/// that survives a split: an index into the outline changes when a gable end is split, and a scene that
/// remembered one would name a different wall the moment a crown moved.
fn court_edge(mass: &building::Mass, z: f32) -> Option<(usize, f32, f32)> {
    let count = mass.outline().len();
    for edge in 0..count {
        let from = mass.outline()[edge].position;
        let to = mass.outline()[(edge + 1) % count].position;
        if (from[1] - z).abs() < 1.0e-3 && (to[1] - z).abs() < 1.0e-3 {
            let length = (to[0] - from[0]).abs();
            if length > 1.0 {
                return Some((edge, from[0], length));
            }
        }
    }
    None
}

/// Returns a plan loop's signed area, which is how this file tells a positive winding from a reversed one.
fn loop_area(plan: &[[f32; 2]]) -> f32 {
    if plan.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..plan.len() {
        let here = plan[index];
        let next = plan[(index + 1) % plan.len()];
        area += here[0] * next[1] - next[0] * here[1];
    }
    area * 0.5
}
/// Adds one range: a mass, its intermediate floor and its crowned 硬山 roof, in that order.
///
/// # Invariants
///
/// **The crown comes after the roof and before the seat.** A gable wall rises to the *roof's* underside,
/// so the height its middle node takes is the roof's own apex less the thickness — a number only the roof
/// can answer — and `site::seat` moves crowns with their masses, so the two must both be in place first.
/// The gable ends are found from the **plan**, by which axis is longer, which is the same reading
/// `derive::gable_roof` makes.
fn range(skeleton: &mut BuildingSkeleton, corners: [[f32; 2]; 4], height: f32, crowned: bool) {
    let mut mass = Mass::new(
        corners.iter().map(|at| PlanNode::new(*at, 0.0)).collect(),
        height,
    );
    mass.wall = WALL;
    mass.slab = PLATE;
    // The wall top starts level and is crowned after the roof exists, because the height a gable's middle
    // node takes is the roof's own apex. The vector is authored before any split, so ops::split_outline
    // keeps it in step with the loop.
    if crowned {
        {
            let count = mass.outline().len();
            for node in &mut mass.crown_mut().outline {
                node.elevation = height;
            }
            let _ = count;
        }
    }
    let mass_id = MassId(skeleton.masses.len());
    skeleton.masses.push(mass);
    let mut roof = derive::gable_roof_by(
        mass_id,
        &skeleton.masses[mass_id.0],
        OVERHANG,
        GABLE,
        RISE,
        SINK,
        ROOF_THICKNESS,
    )
    .expect("a range is a rectangle");
    for plane in &mut roof.planes {
        plane.profile = PROFILE.to_vec();
    }
    roof = roof.with_covering(COVERING);
    if crowned {
        building::derive::crown_gables(&mut skeleton.masses[mass_id.0], &roof);
    }
    let roof_id = skeleton.roofs.len();
    skeleton.roofs.push(roof);
    // **The rafter tails under that roof**: the piece of every rafter the eave exposes, from the wall's
    // own top out and down to the eave's line. They are derived from the roof the mass already carries,
    // so a tail cannot disagree with the slope above it.
    for tail in derive::rafter_tails(
        &skeleton.roofs[roof_id],
        &skeleton.masses[mass_id.0],
        RAFTER_SPACING,
        RAFTER_WIDTH,
        RAFTER_DEPTH,
    ) {
        skeleton.members.push(tail);
    }
    if let Some(storey) = derive::floor_slab(
        &skeleton.masses[mass_id.0],
        mass_id,
        height * 0.48,
        PLATE * 0.8,
    ) {
        skeleton.slabs.push(storey);
    }
}

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// What the post stage does to the frame.
const GRADE: &str = r#"
    // **The sky is read out of the scene's own environment map**, which is the same air the ambient term
    // reads: a sky law written here instead would give the background and the bounce two different skies,
    // and no number could bring them back together.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp((1.0 - exp(-along_ray * 0.0021)) * 0.52, 0.0, 0.78);
        color = mix(
            color,
            environment_sky(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)),
            air,
        );
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.15), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.34, 0.0, 1.0);
"#;
