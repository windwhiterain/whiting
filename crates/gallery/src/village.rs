//! A **village square**: a drum tower at its head, two earth houses to the sides, two log houses behind, and the
//! paving all four face onto.
//!
//! # What this scene is for
//!
//! Every other scene in this gallery is **one type**. This one is a *place*, and the place is the next scale the
//! repository had never drawn: the six types are compositions of masses, roofs, slabs and members, and a village
//! is a composition of **types**. What makes that possible is two operations, both new here —
//! `ops::translate`, which moves a document's plans, and `ops::merge`, which appends one document's skeleton to
//! another and re-bases every index it names.
//!
//! # The arrangement, and why it is this one
//!
//! - **The tower is at the head of the square**, north of it, facing south down the open ground: a 侗寨's 鼓楼 is
//!   the centre of the village and its square is the room in front of it;
//! - **the two 土掌房 stand east and west**, their doors on the square, because the flat-roofed house belongs to
//!   the same highland economy the tower does and its 晒场 faces the light;
//! - **the two 木楞房 stand south**, the village's own edge, where the log houses of a Naxi or Pumi village
//!   actually are;
//! - **the square is paved**, with one grating at its middle: the piece of ground the four types share.
//!
//! # What it does not draw
//!
//! - **no ground of its own** beyond the harness's own mesh: the village sits on level earth, and a terraced
//!   village on a slope would be a scene of its own;
//! - **no people, no trees, no well**: the square is the subject;
//! - **no lanes**: the square is bounded by the four buildings and open at its two ends, and the streets that
//!   run out of it belong to the settlement scene that does not exist yet.

use building::math::Vec2;
use building::site;
use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, derive, ops};
use definition::Value;
use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::kit::{
    camera, environment_light, flux_for_irradiance, frame, image, instance, instances,
    light_with_radius, mesh, mesh_material, post_processes, screen_material, surface,
};
use document::shaders::{Projection, post_shader};
use scene::standard::Stage;

use crate::ground;

/// The square: sixteen metres by twelve, and the grating at its middle.
const SQUARE: [f32; 4] = [-8.0, -6.0, 8.0, 6.0];
const GRATE: f32 = 0.7;
/// How far south the lane runs from the square's own edge, in metres of plan.
const LANE_END: f32 = 15.0;

/// The 寨门 at the lane's mouth: how wide it is, the two plan lines it spans, how tall, and how thick its walls
/// are.
///
/// **The passage is cut on both faces at one place**, because a mass is a shell: two holes in line are one tunnel
/// through it, and a gate is a tunnel with a roof on it.
const GATE_WIDTH: f32 = 4.4;
const GATE_DEPTH: [f32; 2] = [8.0, 9.6];
const GATE_HEIGHT: f32 = 3.4;
const GATE_WALL: f32 = 0.42;
const GATE_HEAD: f32 = 0.25;

/// The 寨门's two wings: how far they run out from its corners, and how high they stand.
const GATE_WING: f32 = 5.4;
const GATE_WING_HEIGHT: f32 = 1.9;

/// How steeply the ground falls away outside the square, in metres of drop per metre, and how far it is
/// allowed to fall.
///
/// **The square is a terrace cut into a hillside**: its own plan is level, and the ground outside it falls away
/// on all four sides. That is what a village square is in this country — the one piece of flat ground a slope
/// can be made to give — and it is also what lets the four types stand at four different heights without any of
/// them being moved by hand.
const SLOPE: f64 = 0.12;
const DROP_LIMIT: f64 = 2.4;

/// The hill the hillside village stands on: **a massif, read onto a lattice and eroded**, which is the pipeline
/// `mountain` and `terrain` exist for and which no scene of a building has ever stood on.
///
/// # Invariants
///
/// **Nothing here is a formula.** A fan of ridges is authored by `mountain`'s own starter, expanded into a facet
/// mesh, rasterized onto a two-metre lattice and solved by the hydraulic erosion; the village then reads the
/// solved field through [`field_height`]. The three crates meet in the order their documents state —
/// `mountain` makes the rock, `terrain` reads and erodes it, and a building stands on the result — and this is
/// the first place all three are in one frame.
///
/// **The field's own plain is zero**, because the massif's rim is levelled to it and the solve is faded back to
/// the massif at that rim. The village's ground outside the lattice is therefore the plain, and no fallback
/// formula is needed or wanted.
fn hillside() -> terrain::HeightField {
    let massif = mountain::starter(
        HILL_SEED,
        mountain::Starter {
            ridges: 5,
            nodes: 6,
            reach: 120.0,
            summit: 34.0,
            tip: 1.5,
            bulge: 9.0,
            ..mountain::Starter::default()
        },
    );
    let mesh = massif
        .build()
        .expect("a fan of five ridges is a mesh with a footprint");
    let read =
        terrain::rasterize(&mesh, HILL_CELL).expect("a 240 m hill is far larger than two metres");
    let (eroded, _books) = terrain::erode_traced(
        &terrain::ErodeParams::default(),
        &read.field,
        &terrain::Hydraulic,
    )
    .expect("a height field is a height field");
    eroded
}

/// Returns the hill's elevation at a plan point: the village reads the hill's south-western flank, so its own
/// plan is offset by `HILL_AT` into the massif's.
fn field_height(field: &terrain::HeightField, x: f64, z: f64) -> f64 {
    field.sample([x as f32 + HILL_AT[0], z as f32 + HILL_AT[1]]) as f64
}

/// Returns the **platform**'s level for the hillside village: the hill's own elevation at the square's centre,
/// which is the level a terrace cut into that slope would be cut to.
fn platform_level(field: &terrain::HeightField) -> f32 {
    let middle = [
        ((SQUARE[0] + SQUARE[2]) * 0.5) as f64,
        ((SQUARE[1] + SQUARE[3]) * 0.5) as f64,
    ];
    field_height(field, middle[0], middle[1]) as f32
}

/// Returns the hillside's grade: the platform inside the square and the hill outside it.
///
/// # Invariants
///
/// **The cut is a step and not a ramp**, and that is what a terrace is: inside the square the ground is one
/// level, outside it the hill is itself, and the retaining wall in [`compose`] is what stands in the face
/// between them.
fn hill_grade(field: &terrain::HeightField, platform: f32, x: f64, z: f64) -> f64 {
    let inside = x >= SQUARE[0] as f64
        && x <= SQUARE[2] as f64
        && z >= SQUARE[1] as f64
        && z <= SQUARE[3] as f64;
    if inside {
        platform as f64
    } else {
        field_height(field, x, z)
    }
}
/// Returns the massif field's own shape, for the probe that measures it.
pub fn field_report() -> String {
    let field = hillside();
    format!(
        "origin {:?} cell {} {}x{} base {} relief {:?}",
        field.origin(),
        field.cell(),
        field.width(),
        field.height(),
        field.base(),
        field.relief(),
    )
}

/// Returns the field's own `sample` at a point, for the probe that measures it.
pub fn field_sample_at(x: f64, z: f64) -> f64 {
    let field = hillside();
    field_height(&field, x, z)
}

/// Returns the hillside village's grade at a point, for the probe that measures it.
pub fn hillside_at(x: f64, z: f64) -> f64 {
    let field = hillside();
    let platform = platform_level(&field);
    hill_grade(&field, platform, x, z)
}

/// Returns the terrace village's grade at a point, for the probe that measures it.
pub fn grade_at(x: f64, z: f64) -> f64 {
    grade(x, z)
}
/// Returns the ground's own elevation at a plan point.
///
/// # Invariants
///
/// **The square is the bottom of a lane that climbs east**, so the ground rises past the square's east edge at
/// the lane's own grade and falls away on the other three sides. The two halves meet at `x = SQUARE[2]`, where
/// both are `0.0`, which is what makes one function serve the lane's seating and the village's: the street is
/// authored with `0.055 * x` from its own origin, and this is the same line read from the square's edge.
fn grade(x: f64, z: f64) -> f64 {
    let climbing = LANE_SLOPE * (x - SQUARE[2] as f64).max(0.0);
    let beyond = [
        (SQUARE[0] as f64 - x).max(0.0),
        (z - SQUARE[3] as f64).max(0.0),
        (SQUARE[1] as f64 - z).max(0.0),
    ];
    let out = beyond.iter().copied().fold(0.0, f64::max);
    let falling = -(out * SLOPE).min(DROP_LIMIT);
    if climbing > 0.0 { climbing } else { falling }
}

/// The **lane**'s own grade, which is the street scene's own number: a straight climb east of `0.055` a metre.
/// The village reads it from the square's edge, so the lane's first plot stands exactly where the square ends.
const LANE_SLOPE: f64 = 0.055;

/// The massif the hillside village stands on: its seed, the lattice it is read onto, and where the village sits
/// on it.
///
/// **`HILL_AT` is the village`s own plan offset into the massif**, so the square stands on a flank and the summit
/// is out of frame to the north-east — a village on a summit is a different building type.
const HILL_SEED: i64 = 11;
const HILL_CELL: f32 = 16.0;
const HILL_AT: [f32; 2] = [-30.0, -34.0];

/// How far the terrace's kerb stands above the square's own level.
const KERB: f32 = 0.35;

/// Where the lane is joined on: the offset its own document is moved by, so its first frontage is the square's
/// east side.
const LANE_AT: [f32; 2] = [8.0, 0.0];
/// The fill: `PAD_SPACING` between posts, `PAD_SECTION` square, `PAD_EMBED` into the ground at the foot.
///
/// **A house pad on a slope is a cut and a fill**, and the fill is a forest of short posts rather than a wall:
/// the same construction the 吊脚楼 makes of its whole floor, at the scale of one corner of a house.
const PAD_SPACING: f32 = 1.6;
/// How far a house's stone plinth stands proud of its own plan, in metres.
const PLINTH_MARGIN: f32 = 0.28;
const PAD_SECTION: f32 = 0.16;
const PAD_EMBED: f32 = 0.03;

/// Where each building stands, as the offset its own document is moved by.
const TOWER_AT: [f32; 2] = [0.0, -13.0];
const EARTHEN_WEST: [f32; 2] = [-13.0, 0.0];
const EARTHEN_EAST: [f32; 2] = [-13.0, -13.0];
const LOG_WEST: [f32; 2] = [-10.5, 12.0];
const LOG_EAST: [f32; 2] = [10.5, 12.0];
/// **The 蘑菇房 stands down the hill, west of the square**, which is where a Hani house belongs in this village: not
/// on the terrace with the tower, but below it on the slope the terraces are cut into.
const MUSHROOM_AT: [f32; 2] = [-24.0, -5.0];

/// The sun, low from the south-west, so the tower's seven skirts are modelled against each other and the log
/// walls are raked.
const SUN: [f64; 3] = [-26.0, 26.0, -14.0];
const SUN_IRRADIANCE: f64 = 3.1;
const SHADOW_EXTENT: f64 = 60.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.62;
const GROUND_DIVISIONS: usize = 160;
const GROUND_EXTENT: f64 = 90.0;

/// The air over the village.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.19, 0.35, 0.62])
        .horizon([0.84, 0.80, 0.71])
        .ground([0.35, 0.31, 0.24])
        .density(0.0011)
        .scale_height(200.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the south, across the square, at the tower.
pub fn village_square() -> Value {
    scene(
        [0.0, 7.5, 30.0],
        [0.0, 7.0, -12.0],
        FIELD_OF_VIEW,
        "village_square",
    )
}

/// Builds the village from the west, where the sixth building stands, instead of from the south.
///
/// # Invariants
///
/// **A village of six buildings needs a view that holds six.** The south view is the one the square earned — the tower
/// on its axis, the gate behind the camera — and it stands 16 m east of the 蘑菇房's own ground, so the third house
/// type is behind the western 土掌房 in it. This view is the same document and the same terrace seen along the other
/// diagonal: the 蘑菇房 in the foreground, the earthen houses and the tower behind it.
pub fn village_west() -> Value {
    scene(
        [-34.0, 9.5, 22.0],
        [-4.0, 6.0, -8.0],
        FIELD_OF_VIEW,
        "village_west",
    )
}

/// Builds the scene from one camera, on the terrace.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|x, z| grade(x, z),
        eye,
        target,
        field_of_view,
        name,
    )
}

/// Builds the hillside's scene: the same composition on the field, from a camera standing on the slope below it.
pub fn hillside_village() -> Value {
    let field = hillside();
    let platform = platform_level(&field);
    let ground = move |x: f64, z: f64| hill_grade(&field, platform, x, z);
    // **The camera stands on the hill and not at an elevation written by hand.** The massif's flank is sixteen
    // metres above the plain, so a camera at y = 6 was under the ground it came to look at -- which is what the
    // first frame of this scene showed: a village seen through the underside of its own hill.
    // **The camera is placed above the ground under its own feet**, which is the only statement about height
    // that survives a terrain: a fixed elevation is a statement about the plain, and this village is not on
    // one. It also stands outside the hill's shoulder and looks along the slope, so the frame is the village
    // and not the underside of the ground that carries it.
    // **The camera stands above the ground under its own feet**, and the village is framed from the south-west
    // as the terrace's village is: a terrain has no side an architect would have picked, so the framing is the
    // one the flat scene already earned and the terrain is what moves under it.
    let eye_plan = [-18.0_f64, 34.0_f64];
    let eye = [
        eye_plan[0],
        ground(eye_plan[0], eye_plan[1]) + 7.0,
        eye_plan[1],
    ];
    let target = [2.0, f64::from(platform) + 4.0, -6.0];
    // **The scrub is scattered on the terrain, and a seat is refused twice.** Round 61 put five clumps round this village
    // by hand and every one drew above the horizon: the seats had been read off a frame, and a frame is not a place. A seat
    // is kept only if the **ground under it is the village's own shelf** — the platform the houses stand on, within a
    // metre and a half — and if it is **outside every building's plan**, which is the same refusal a seat inside a wall
    // would get from the expansion and cheaper to make here.
    let scattered = hillside_document();
    let mut plans: Vec<([f32; 2], [f32; 2])> = Vec::new();
    for mass in &scattered.skeleton.masses {
        let mut low = [f32::INFINITY; 2];
        let mut high = [f32::NEG_INFINITY; 2];
        for node in mass.outline() {
            for axis in 0..2 {
                low[axis] = low[axis].min(node.position[axis]);
                high[axis] = high[axis].max(node.position[axis]);
            }
        }
        plans.push((low, high));
    }
    let mut seats: Vec<[f32; 2]> = Vec::new();
    for across in 0..11 {
        for along in 0..11 {
            let at = [
                (SQUARE[0] - 6.0) + across as f32 * 3.0,
                (SQUARE[1] - 6.0) + along as f32 * 3.0,
            ];
            // **The shelf is the square's plan plus a margin, and then the level.** The level alone is not enough: the
            // massif has ground at the platform's own height elsewhere on its flank, and that is where round 61's
            // hand-written seats landed — a cluster of clumps drew against the sky above the ridge. A terrace is a
            // *place* as well as a height.
            if at[0] < SQUARE[0] - 3.0
                || at[0] > SQUARE[2] + 3.0
                || at[1] < SQUARE[1] - 3.0
                || at[1] > SQUARE[3] + 3.0
            {
                continue;
            }
            let level = ground(at[0] as f64, at[1] as f64);
            if (level - f64::from(platform)).abs() > 1.5 {
                continue;
            }
            let inside = plans.iter().any(|(low, high)| {
                at[0] > low[0] - 1.6
                    && at[0] < high[0] + 1.6
                    && at[1] > low[1] - 1.6
                    && at[1] < high[1] + 1.6
            });
            if inside {
                continue;
            }
            seats.push(at);
        }
    }
    // **A row of clumps and not a lawn**: every third candidate, so the shelf keeps its open ground between the houses.
    let seats: Vec<[f32; 2]> = seats.into_iter().step_by(3).take(9).collect();
    if std::env::var("SEATS_DEBUG").is_ok() {
        eprintln!("  platform {platform}");
        for at in &seats {
            eprintln!("  seat {at:?} y {:.3}", ground(at[0] as f64, at[1] as f64));
        }
    }
    // **Every third seat is a tree and the rest are clumps**: a canopy over the yards and brush along the wall, which is
    // what a village's own planting looks like.
    let mut clump_seats: Vec<[f32; 2]> = Vec::new();
    let mut tree_seats: Vec<[f32; 2]> = Vec::new();
    for (index, at) in seats.iter().enumerate() {
        if index % 3 == 1 {
            tree_seats.push(*at);
        } else {
            clump_seats.push(*at);
        }
    }
    let mut scrub = crate::settlement_plants::bamboo_clumps(&clump_seats, &|at: [f32; 2]| {
        ground(at[0] as f64, at[1] as f64) as f32
    });
    scrub.merge(crate::settlement_plants::trees(
        &tree_seats,
        &|at: [f32; 2]| ground(at[0] as f64, at[1] as f64) as f32,
    ));
    scene_of(
        &scattered,
        &scrub,
        &ground,
        eye,
        target,
        FIELD_OF_VIEW,
        "hillside_village",
    )
}

/// Returns the divisions and the extent of the ground mesh a scene needs.
///
/// # Invariants
///
/// **A landscape's mesh is sized by the landscape and not by the building on it.**
fn ground_scale(name: &str) -> (usize, f64) {
    if name == "hillside_village" {
        (600, 260.0)
    } else {
        (GROUND_DIVISIONS, GROUND_EXTENT)
    }
}
/// Builds one scene of a village: a building and the ground it stands on, which is the only pair of facts the
/// two variants of this village disagree about.
/// **The scene shell is the crate's**, shared with the Dai hamlet: one list of meshes an authored building may
/// produce — walls, roofs and their rolls, slabs and members — and one set of earthy materials for them. A second
/// settlement reuses it rather than writing a fourth copy of the same twelve material declarations.
pub(crate) fn scene_of(
    building: &Building,
    plants: &crate::settlement_plants::Extra,
    ground_at: &dyn Fn(f64, f64) -> f64,
    eye: [f64; 3],
    target: [f64; 3],
    field_of_view: f64,
    name: &str,
) -> Value {
    let built = building
        .build()
        .expect("the village expands: every type it is made of expands on its own");
    let violations = building.validate();
    for violation in &violations {
        if let building::validate::Violation::FaceNotPlanar { face, .. } = violation {
            let held = built.faces[*face];
            eprintln!("  warped face {face}: {:?}", held.element);
        }
    }
    assert!(
        violations.is_empty(),
        "the village's own document must be clean, and it is not: {violations:?}"
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
    let flats = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            )
        )
    });
    let timbers = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
    });

    let plaster = || surface([0.74, 0.70, 0.60], 0.0, 0.88, [0.0; 3]);
    let earth = || surface([0.54, 0.43, 0.30], 0.0, 0.92, [0.0; 3]);
    let tiles = || surface([0.28, 0.28, 0.29], 0.0, 0.58, [0.0; 3]);
    let paving = || surface([0.50, 0.48, 0.43], 0.0, 0.86, [0.0; 3]);
    let timber = || surface([0.29, 0.19, 0.11], 0.0, 0.78, [0.0; 3]);
    let ground_earth = || surface([0.33, 0.29, 0.22], 0.0, 0.96, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("wall", plaster as fn() -> Value),
        ("earthen", earth),
        ("roof", tiles),
        ("paving", paving),
        ("timber", timber),
    ] {
        materials.push(mesh_material(
            &format!("{label}-cast"),
            "authored_shadowing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{label}-depth"),
            "authored_depthing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{label}-shade"),
            "authored_lighting",
            body(),
        ));
    }
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(
            &format!("ground-{stage}"),
            shader,
            ground_earth(),
        ));
    }

    // **A mesh that is not declared is not instanced either.** The two lists have to agree, and the document writer says so
    // twice: `mesh roofs has no points` when one was declared empty, and `instance roofs-depth names mesh roofs, which the
    // scene does not declare` when the instance outlived it. One table of what this building has decides both.
    let has = [
        ("walls", "wall", !walls.is_empty()),
        ("roofs", "roof", !roofs.is_empty()),
        ("flats", "paving", !flats.is_empty()),
        ("timber", "timber", !timbers.is_empty()),
        ("ground", "ground", true),
    ];
    let placed = |stage: &str| {
        has.into_iter()
            .filter(|(_, _, present)| *present)
            .map(|(label, material, _)| {
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
    // **A settlement's plants are placed per stage like everything else**, and a scene with none adds nothing: the
    // extra's instance list is empty and the stage's list is the one it always was.
    let grown = |stage: &str| {
        let mut all = placed(stage);
        all.extend(plants.instances(stage));
        all
    };

    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();
    // **The ground is the scene's own grade**, not a level plane: the same function the houses were seated
    // on, so a house cannot float and the hill cannot disagree with the building standing on it. Its mesh is
    // sized by the landscape: the terrace's plateau is 90 m across, and a hill with a 62 m shoulder needs a box
    // that reaches past its own shoulder, or the frame shows the edge of the mesh as a cliff.
    let (divisions, extent) = ground_scale(name);
    let ground = ground::mesh(building, divisions, extent, |x, z| ground_at(x, z));

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
                    SUN,
                    [1.0, 0.93, 0.80],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    170.0,
                ),
                environment_light("air", "air", [1.0; 3], 1.05),
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
                shaders.push(post_shader("", &helpers, Some(Projection::Latlong)));
                shaders
            }),
        ),
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List({
                // **A mesh with no points is not declared.** A settlement whose houses wear no roof — the plateau village,
                // whose towers are flat-topped — has nothing for a roofs mesh to draw, and a scene that declares one with no
                // points is refused by the document writer: `mesh roofs has no points`.
                let mut all = Vec::new();
                for (label, packed) in [
                    ("walls", walls),
                    ("roofs", roofs),
                    ("flats", flats),
                    ("timber", timbers),
                ] {
                    if packed.is_empty() {
                        continue;
                    }
                    all.push(mesh(label, "Triangles", point.clone(), packed));
                }
                all.push(mesh("ground", "Triangles", plain.point_schema(), ground));
                for (label, packed) in plants.meshes() {
                    all.push(mesh(&label, "Triangles", plain.point_schema(), packed));
                }
                all
            }),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "mesh_materials".to_owned(),
            Value::List({
                let mut all = materials;
                all.extend(plants.materials());
                all
            }),
        ),
        (
            "instances".to_owned(),
            instances(vec![
                (Stage::Shadowing, grown("cast")),
                (Stage::Depthing, grown("depth")),
                (Stage::Lighting, grown("shade")),
            ]),
        ),
    ])
}

/// Returns the document: five buildings and the paving they face onto.
///
/// # Invariants
///
/// **Each building is the scene that already draws it**, moved and merged rather than rebuilt: a village that
/// re-derived a tower would be a second implementation of the tower, and the two would drift the first time one
/// of them was edited. The document is the composition of five documents, which is what `merge` and `translate`
/// exist for.
///
/// **A merge appends where the document was authored**, so each one is translated first — and translated by its
/// own offset, which is the only thing this function says about where anything stands.
/// Gives one seated house its **stone plinth**: the pad it is cut into and filled up to, under the posts.
///
/// # Invariants
///
/// **The plinth is the house's own plan, proud by [`PLINTH_MARGIN`] on every side**, from the lowest ground under
/// that plan up to the sill. It therefore has no thickness of its own: it is deep where the fill is deep and a
/// step where the ground is level, which is what a plinth is — the masonry between a building and a slope, with
/// the slope's own shape in its underside.
///
/// **A house whose sill is already on the ground gets no plate at all.** A plate thinner than two centimetres is
/// a face at a face, and the expansion would report the two as one solid or as a crack.
fn pad_plinth(part: &mut BuildingSkeleton, ground: &impl Fn(Vec2) -> f32) {
    let under = part
        .masses
        .iter()
        .flat_map(|mass| mass.outline().iter().map(|node| node.position))
        .map(ground)
        .fold(f32::INFINITY, f32::min);
    let plans: Vec<(Vec<[f32; 2]>, f32)> = part
        .masses
        .iter()
        .map(|mass| {
            let sill = mass.lowest();
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
                let grow = (length + PLINTH_MARGIN) / length;
                *at = [middle[0] + out[0] * grow, middle[1] + out[1] * grow];
            }
            if loop_area(&plan) < 0.0 {
                plan.reverse();
            }
            (plan, sill)
        })
        .collect();
    for (plan, sill) in plans {
        let foot = under.min(sill) - PLINTH_MARGIN;
        if sill - foot > 0.02 {
            part.slabs.push(building::Slab::plate(plan, foot, sill));
        }
    }
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
pub fn document() -> Building {
    compose(grade, true, 0.0)
}

/// Returns the document of the village **on a hillside**: the same five buildings, seated on a real
/// [`terrain::HeightField`] instead of on the terrace's rule, with the square cut into the slope.
///
/// # Invariants
///
/// **The two villages are one composition and two grounds.** `compose` is the composition; the terrace's own
/// `grade` and the hillside's field are the only things that differ, and neither variant re-derives a tower or
/// a house. A second composition would be a second village, and the two would drift the first time one of them
/// was edited.
pub fn hillside_document() -> Building {
    let field = hillside();
    let platform = platform_level(&field);
    compose(
        move |x, z| hill_grade(&field, platform, x, z),
        false,
        platform,
    )
}

/// The composition, given its ground and whether the lane is part of it.
///
/// # Invariants
///
/// **The ground is a function of plan and nothing else**, so it is the same one the houses are seated on and the
/// one the frame draws. `lane` decides whether the street scene is joined on: the terrace's village has it, and
/// the hillside's does not, because the street is authored on its own straight grade and a hillside is not one.
pub fn compose(ground: impl Fn(f64, f64) -> f64, lane: bool, platform: f32) -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let seated_ground = move |at: Vec2| ground(at[0] as f64, at[1] as f64) as f32;
    // **The tower stands on the terrace and the houses stand on the hill.** A seat moves every mass's nodes to
    // the ground under them, which is exactly what a house needs and exactly what a tower of stacked storeys
    // must not have: seating the tower would flatten its seven floors onto one elevation. So the tower is
    // translated and merged as it is, and the four houses are seated on their own ground before they are merged
    // — which is the same `site::seat` every single-building scene calls, read once per building.
    for (mut part, at, seated) in [
        (crate::drum_tower::document().skeleton, TOWER_AT, false),
        (crate::earthen::document().skeleton, EARTHEN_WEST, true),
        (crate::earthen::document().skeleton, EARTHEN_EAST, true),
        // **The lane is the street scene, joined on.** Its document seats its own five plots on its own grade,
        // and that grade is the one this village's `grade` continues past the square, so the join is a
        // continuation rather than two hills meeting: the street's `0.055 * x` from its own origin is this
        // document's `0.055 * (x - 8)` from the square's edge.
        (crate::street::document().skeleton, LANE_AT, false),
        (crate::log_house::document().skeleton, LOG_WEST, true),
        (crate::log_house::document().skeleton, LOG_EAST, true),
        // **The 蘑菇房 is the village's sixth building and its third house type**, and it is seated like the others.
        // **Its terrace edge is dropped**, because a kerb is site work: the document authors one for the scene whose
        // ground steps down in front of the house, and on this hill the seat gives that kerb's two ends two different
        // grounds. **Its straw stays on**: the covering was dropped in round 45 on a `1.09e-3` planarity report that
        // turned out to be the validator's own plane fit cancelling itself to zero (round 54), so the bundles are
        // geometry again and the village gets them.
        (
            {
                let mut part = crate::mushroom_house::document().skeleton;
                part.walls.clear();
                part
            },
            MUSHROOM_AT,
            true,
        ),
    ] {
        if !lane && at == LANE_AT {
            continue;
        }
        ops::translate(&mut part, at);
        if seated {
            let report = site::seat(&mut part, &seated_ground, 0.0);
            assert!(
                report.len() == part.masses.len(),
                "every mass of a house is seated on the ground this grade reads"
            );
            // **A house on a slope is cut in on one side and filled on the other**, and the fill is what stands
            // the house up: `site::posts` puts a post under the outline wherever the ground has fallen away
            // from the pad, which is the same op the 吊脚楼 uses for its whole floor. Seating the house on the
            // highest ground under it — which is what `seat` does — leaves the downhill side in the air, and a
            // house standing on nothing is a house drawn halfway.
            //
            // **The posts are placed before the merge**, so their anchors resolve against the house's own mass
            // and not against a village that has five of them.
            let stilt = site::posts(
                &mut part,
                &seated_ground,
                PAD_SPACING,
                PAD_SECTION,
                PAD_EMBED,
            );
            assert!(
                stilt.refused == 0,
                "the grade answers under every post position of every house"
            );
            pad_plinth(&mut part, &seated_ground);
        }
        assert!(
            ops::merge(&mut skeleton, part),
            "a building of the village places no parts, so the merge always happens"
        );
    }
    // **The 寨门: the village's gate, at the mouth of its lane.** A village with a square and no gate is a
    // clearing; the gate is what makes the lane an entrance and the square a place inside something. It is a
    // mass a lane wide, pierced on both faces, with a gable over it — the three pieces every gate in this country
    // is made of — and it stands on the lane's own ground so the passage is level with the paving it opens onto.
    {
        let middle = [
            (SQUARE[0] + SQUARE[2]) * 0.5,
            (SQUARE[3] + GATE_DEPTH[1]) * 0.5 + GATE_DEPTH[1] * 0.5,
        ];
        let level = seated_ground(middle) as f32;
        let half_gate = GATE_WIDTH * 0.5;
        let deep_gate = (GATE_DEPTH[1] - GATE_DEPTH[0]) * 0.5;
        let mut gate = Mass::new(
            [
                PlanNode::new([middle[0] - half_gate, GATE_DEPTH[0]], level),
                PlanNode::new([middle[0] + half_gate, GATE_DEPTH[0]], level),
                PlanNode::new([middle[0] + half_gate, GATE_DEPTH[1]], level),
                PlanNode::new([middle[0] - half_gate, GATE_DEPTH[1]], level),
            ]
            .to_vec(),
            level + GATE_HEIGHT,
        );
        gate.wall = GATE_WALL;
        gate.slab = 0.30;
        let gate_id = MassId(skeleton.masses.len());
        skeleton.masses.push(gate);
        // The passage, cut through **both** faces at the same place: a mass is a shell, so two holes in line are
        // one tunnel through it, which is what a gate is.
        for edge in [0_usize, 2] {
            let face = building::grid::FaceRef::new(gate_id, edge);
            if ops::split_face(
                &mut skeleton,
                face,
                &building::grid::Rhythm::Even { count: 3 },
                &building::grid::Rhythm::Even { count: 1 },
            )
            .is_some()
            {
                let opening = ops::add_opening(
                    &mut skeleton,
                    building::openings::Opening::new(
                        face,
                        0,
                        1,
                        building::openings::Inset::new(
                            GATE_WALL + 0.05,
                            GATE_WALL + 0.05,
                            0.0,
                            GATE_HEIGHT - GATE_HEAD - 0.4,
                        ),
                    ),
                );
                assert!(opening.is_some(), "the opening is one the face accepts");
            }
        }
        if let Some(roof) =
            derive::gable_roof(gate_id, &skeleton.masses[gate_id.0], 0.55, 1.1, 0.08, 0.14)
        {
            skeleton.roofs.push(roof.with_covering(building::Covering {
                period: 0.30,
                fill: 0.62,
                height: 0.05,
            }));
        }
        let _ = deep_gate;
        // **And the gate's two wings**: a gate standing alone in a field is a porch, and what makes it a gate is
        // that it closes something. Two runs of wall go out from its own corners along the lane, on the ground at
        // their own feet — the same construction as the terrace's kerb, because a village wall is a fence and not a
        // building, and this crate keeps the two apart.
        for side in [-1.0_f32, 1.0] {
            let x = middle[0] + side * half_gate;
            let from = [x, GATE_DEPTH[1]];
            let to = [x, GATE_DEPTH[1] + GATE_WING];
            let foot = seated_ground(from) as f32;
            skeleton.walls.push(building::wall::Wall::new(
                PlanNode::new(from, foot),
                PlanNode::new(to, foot),
                0.42,
                foot + GATE_WING_HEIGHT,
            ));
        }
    }
    // **The terrace is held up by a wall**, one run to a side, standing on the ground at the foot of each drop:
    // a plateau cut into a hill is a cut face, and the village's own edge is what a person on the square sees
    // when they look over it. The wall is the site's, not a mass's — it is not a building.
    for (from, to) in [
        (
            [SQUARE[0] - 0.3, SQUARE[1] - 0.3],
            [SQUARE[2] + 0.3, SQUARE[1] - 0.3],
        ),
        (
            [SQUARE[2] + 0.3, SQUARE[1] - 0.3],
            [SQUARE[2] + 0.3, SQUARE[3] + 0.3],
        ),
        (
            [SQUARE[2] + 0.3, SQUARE[3] + 0.3],
            [SQUARE[0] - 0.3, SQUARE[3] + 0.3],
        ),
        (
            [SQUARE[0] - 0.3, SQUARE[3] + 0.3],
            [SQUARE[0] - 0.3, SQUARE[1] - 0.3],
        ),
    ] {
        let foot = seated_ground([from[0], from[1]]) as f32;
        skeleton.walls.push(building::wall::Wall::new(
            building::PlanNode::new([from[0], from[1]], foot),
            building::PlanNode::new([to[0], to[1]], foot),
            0.45,
            // **The wall stands a kerb above the terrace and not level with it**: its top is an absolute
            // elevation, and a top of `0.0` sat below the ground on the east side once the lane began to climb
            // there -- `WallTooLow { top: 0.0, highest: 0.0165 }`, measured.
            // **The wall stands a kerb above whichever is higher: the platform or its own foot.** On the
            // downhill side the foot is below the platform and the wall becomes a retaining wall; on the uphill
            // side the hill itself rises above the platform, and a top of `platform + KERB` would be *under* the
            // ground it stands in -- `WallTooLow { top: 0.35, highest: 5.57 }`, measured on all four.
            foot.max(platform) + KERB,
        ));
    }
    // **The square is paved, with one grating at its middle**: the piece of ground the four types share, and
    // the hole the village's water leaves by.
    if let Some(paving) = derive::court_paving(
        [
            [SQUARE[0], SQUARE[1]],
            [SQUARE[2], SQUARE[1]],
            [SQUARE[2], SQUARE[3]],
            [SQUARE[0], SQUARE[3]],
        ],
        0.02,
        0.12,
        GRATE,
        0.40,
    ) {
        skeleton.slabs.push(paving);
        // **The lane steps down from the square.** The square opens south between the two log houses, and the
        // way out of it is a stone stair: one plate a step, each landing on the grade at its own run. A ramp is a
        // surface this crate cannot cut, and a stair is what a village on a hillside lays down at the end of its
        // square.
        let lane = 1.6;
        let step = 1.1;
        let mut z = SQUARE[3];
        while z < LANE_END {
            // **The lane is laid on the scene's own ground and not on the terrace's rule**: `grade` is the
            // flat village's arithmetic, and a stair cut from it in a hillside village is a stair under the hill
            // -- which is where it was, nineteen metres down, until this was read.
            let level = seated_ground([0.0, z]) as f32;
            skeleton.slabs.push(building::Slab::plate(
                vec![[-lane, z], [lane, z], [lane, z + step], [-lane, z + step]],
                level - 0.16,
                level + 0.10,
            ));
            z += step;
        }
    }
    Building::new(skeleton)
}
