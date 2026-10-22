//! A **木楞房**: the log house of the Naxi, Pumi and Nu villages — walls of stacked logs whose ends cross at
//! every corner, and a board roof that is not much more than the logs continued upward.
//!
//! # What this scene is for
//!
//! It is the gallery's **sixth type** and the only one whose walls are not built: a 木楞房 is a stack of logs,
//! course on course, each course laid the other way round from the one below it so that the ends cross at the
//! corners and lock the building together without a single joint cut. The crossing ends are the type's whole
//! signature, and they are what this scene exists to draw.
//!
//! # The two facts the type has
//!
//! - **Every course is two logs, and the courses alternate.** One course lies along `x` on the two `z` walls,
//!   the next along `z` on the two `x` walls; the ends run `LOG_PROJECT` past the corner, which is the crossing
//!   that holds the wall together. Eighteen members carry the whole shell.
//! - **The roof is board and it is low.** A 木楞房 is a single storey with a gentle gable and shingles or boards
//!   on it — no tiles, no 举架, and an overhang big enough to keep the log ends dry.
//!
//! # What it does not draw
//!
//! - **the chinking**: the moss and clay packed between the logs is a material the frame cannot show;
//! - **the notched corners**: the logs cross and stop; a real one is notched half through at each end, and a
//!   member here is a square box;
//! - **the 木楞 floor**: the upper floor is a plate in this document, and in the type it is more logs.

use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, derive};
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

use crate::facade::{self, Bar, Front};
use crate::ground;

/// The house: `WIDTH` by `DEPTH`, one storey of `COURSES` logs, each log `LOG` square.
const WIDTH: f32 = 7.2;
const DEPTH: f32 = 5.4;
const COURSES: usize = 9;
const COURSE: f32 = 0.26;
const LOG: f32 = 0.22;

/// How far a log's end runs past the corner, which is the notch the next course crosses it at.
const LOG_PROJECT: f32 = 0.38;

/// The wall the logs are carried on and the plate under them: a thin shell inside the log's own thickness.
const WALL: f32 = 0.16;
const PLATE: f32 = 0.30;

/// The roof: a gentle gable, low overhang, and boards rather than tiles.
const OVERHANG: f32 = 0.55;
const RISE: f32 = 1.35;
const SINK: f32 = 0.08;
const ROOF_THICKNESS: f32 = 0.14;
const STARTER_HEIGHT: f32 = COURSES as f32 * COURSE;

/// The sun, low from the corner the logs cross on, so the ends cast their own shadows down the wall.
const SUN: [f64; 3] = [-20.0, 24.0, 22.0];
const SUN_IRRADIANCE: f64 = 3.0;
const SHADOW_EXTENT: f64 = 24.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 50.0;

/// The air over the village.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.20, 0.35, 0.62])
        .horizon([0.85, 0.81, 0.72])
        .ground([0.34, 0.30, 0.23])
        .density(0.0010)
        .scale_height(200.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the corner where the logs cross.
pub fn log_house() -> Value {
    scene(
        [12.0, 3.4, 12.5],
        [0.0, 2.4, 0.0],
        FIELD_OF_VIEW,
        "log_house",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("a 木楞房 expands: one mass, its board roof and eighteen logs");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a 木楞房's own document must be clean, and it is not: {violations:?}"
    );

    let table = building::identity::Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    let logs = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
    });
    let roofs = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Roof(_, _) | building::Element::Roll(_, _)
        )
    });

    let plaster = || surface([0.70, 0.62, 0.48], 0.0, 0.88, [0.0; 3]);
    let timber = || surface([0.30, 0.20, 0.12], 0.0, 0.78, [0.0; 3]);
    let board = || surface([0.34, 0.31, 0.26], 0.0, 0.72, [0.0; 3]);
    let earth = || surface([0.32, 0.28, 0.21], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("wall", plaster as fn() -> Value),
        ("log", timber),
        ("roof", board),
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
        materials.push(mesh_material(&format!("ground-{stage}"), shader, earth()));
    }

    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("logs", "log"),
            ("roofs", "roof"),
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
    let ground = ground::mesh(&building, GROUND_DIVISIONS, GROUND_EXTENT, |_, _| 0.0);

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
                    [1.0, 0.94, 0.82],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    170.0,
                ),
                environment_light("air", "air", [1.0; 3], 1.06),
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
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("logs", "Triangles", point.clone(), logs),
                mesh("roofs", "Triangles", point.clone(), roofs),
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

/// How wide the house is between its two log ends, which is the span a course is laid over.
const SPAN: f32 = WIDTH * 0.5 + LOG_PROJECT;

/// The door and the window of the front wall, as `(from, to, sill, head)` in plan and elevation.
///
/// # Invariants
///
/// **The numbers are the ones the openings were cut with**, so a log stops at the jamb the wall stops at: the
/// door is the middle bay's width less its margin, and its head is the storey's own top less the inset's own
/// head. A second reading of the same numbers is what would drift.
const OPENINGS: [(f32, f32, f32, f32); 2] = [
    // The door: the middle bay (1.2 m either side of the centre) less a 0.55 margin, from the floor to
    // `STARTER_HEIGHT` less the door's own 0.40 head.
    (-0.65, 0.65, 0.0, STARTER_HEIGHT - 0.40),
    // The window in the first bay: its own margin and its own sill and head.
    (-3.05, -1.75, 1.30, STARTER_HEIGHT - 0.60),
];

/// Returns the segments one front-wall log is laid in at elevation `y`.
///
/// # Invariants
///
/// **A log is cut where it would cross an opening**, and only there: the front wall's `z` and the opening
/// list are the whole of the reading, so the back wall is one log from end to end and the front is as many
/// pieces as the openings leave. A course whose own band does not reach an opening's sill or head is not cut
/// by it — a log above a door is a lintel and stays whole.
fn log_spans(z: f32, y: f32) -> Vec<(f32, f32)> {
    let band = [y - LOG * 0.5, y + LOG * 0.5];
    let mut blocked: Vec<(f32, f32)> = Vec::new();
    if z > 0.0 {
        for (from, to, sill, head) in OPENINGS {
            if band[0] < head && band[1] > sill {
                blocked.push((from, to));
            }
        }
    }
    blocked.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut spans = Vec::new();
    let mut at = -SPAN;
    for (from, to) in blocked {
        if from > at {
            spans.push((at, from));
        }
        at = at.max(to);
    }
    if at < SPAN {
        spans.push((at, SPAN));
    }
    spans
}

/// Returns the document: a shell, its board roof, and the logs the shell is built of.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = WIDTH * 0.5;
    let deep = DEPTH * 0.5;
    let mut mass = Mass::new(
        [
            PlanNode::new([-half, -deep], 0.0),
            PlanNode::new([half, -deep], 0.0),
            PlanNode::new([half, deep], 0.0),
            PlanNode::new([-half, deep], 0.0),
        ]
        .to_vec(),
        STARTER_HEIGHT,
    );
    mass.wall = WALL;
    mass.slab = PLATE;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(mass);

    // **The roof is board and it is low**: a gable with no 举架 ladder and no covering, which is what a
    // 木楞房 wears. Its overhang is the one number that keeps the log ends dry.
    if let Some(roof) = derive::gable_roof(
        id,
        &skeleton.masses[id.0],
        OVERHANG,
        RISE,
        SINK,
        ROOF_THICKNESS,
    ) {
        skeleton.roofs.push(roof);
    }

    // **Every course is two logs, and the courses alternate.** Course `even` lies along `x` on the two `z`
    // walls and runs `LOG_PROJECT` past both corners; course `odd` lies along `z` on the two `x` walls and does
    // the same the other way. The wall the logs are carried on is inside their own thickness, so a log is the
    // outer face, not a decoration on one.
    for course in 0..COURSES {
        let y = course as f32 * COURSE + COURSE * 0.5;
        if course % 2 == 0 {
            // Along `x`: two logs, one on each `z` wall. **The front one is cut at the openings it crosses**,
            // because a log that runs over a doorway is a doorway with a log in it: a 木楞房's logs stop at
            // the jamb and start again on the other side, which is what leaves the hole in the wall.
            for z in [-deep, deep] {
                for (from, to) in log_spans(z, y) {
                    skeleton.members.push(building::Member::new(
                        building::Anchor::Free { at: [from, y, z] },
                        building::Anchor::Free { at: [to, y, z] },
                        [0.0, 1.0],
                        LOG,
                        LOG,
                    ));
                }
            }
        } else {
            // Along `z`: two logs, one on each `x` wall, crossing the ends of the course below.
            for x in [-half, half] {
                skeleton.members.push(building::Member::new(
                    building::Anchor::Free {
                        at: [x, y, -deep - LOG_PROJECT],
                    },
                    building::Anchor::Free {
                        at: [x, y, deep + LOG_PROJECT],
                    },
                    [1.0, 0.0],
                    LOG,
                    LOG,
                ));
            }
        }
    }

    // **A door in the middle bay of the one storey**, and a small window in a side bay: the type's front is
    // its door, and a 木楞房's window is a pane set between two courses.
    let opened = facade::open_court_face(
        &mut skeleton,
        id,
        deep,
        true,
        Front {
            storeys: 1,
            bays: 3,
            door: true,
        },
    );
    assert!(opened, "the opening is one the wall accepts");
    if let Some(face) = facade::edge_on(&skeleton.masses[id.0], deep, true) {
        let opened = facade::open_with_bars(
            &mut skeleton,
            building::grid::FaceRef::new(id, face),
            0,
            0,
            building::openings::Inset::new(0.55, 0.55, 1.30, 0.60),
            Bar::NONE,
        );
        assert!(opened, "the opening is one the wall accepts");
    }
    Building::new(skeleton)
}
