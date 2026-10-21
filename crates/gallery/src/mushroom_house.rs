//! A **蘑菇房**: the Hani house of the 元阳 terraces — a square rammed-earth room under a straw cap that overhangs it
//! on all four sides, with a lower wing beside it.
//!
//! # What this scene is for
//!
//! Every type in this gallery so far has had a roof whose eave is a *detail* of a wall. A 蘑菇房's roof is the
//! building: a four-slope cap whose projection is more than a metre past the wall on every side, thick because the
//! straw is thick, and steep because the rain on that hillside is measured in metres. It is the one type here where
//! **the overhang is the form** and the wall beneath it is a small, dark, square room.
//!
//! # The three facts of the type
//!
//! - **the cap is a hip roof and it projects on all four sides**, which is what [`derive::hip_roof`] builds from the
//!   mass's own plan — one call, and the `OVERHANG` is the type;
//! - **the straw is thick**: `ROOF_THICKNESS` is more than twice a tile roof's, and the covering's ribs are wide and
//!   shallow, so the surface reads as bundles rather than as courses of tile;
//! - **the wing is lower and its roof is a lean-to**, which is what a 蘑菇房's side room is: one slope, its crest on
//!   the main wall and its eave over a small yard.
//!
//! # What it does not draw
//!
//! - **the terraces**: a 蘑菇房 stands in a field of them, and this scene's ground is flat;
//! - **the thatch's bedding**: the straw is bound in courses whose laps are geometry this type would need its own
//!   covering for.

use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, derive, ops};
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

use crate::facade;
use crate::ground;

/// The room: `WIDTH` by `DEPTH`, one storey of `STOREY`, with walls `WALL` thick.
/// **The room is longer than it is deep**, and not by taste: a hip over a nearly square plan has a ridge of almost
/// no length, and the four planes then meet close enough to a point that one of them departs from its own plane by
/// `1.2e-3 m` — measured as `FaceNotPlanar` at the validator's `1e-3`, with the cap the only roof in the document.
const WIDTH: f32 = 8.2;
const DEPTH: f32 = 6.6;
const STOREY: f32 = 2.75;
const WALL: f32 = 0.52;
const PLATE: f32 = 0.35;

/// The cap: how far it projects past the wall, how high it rises, and how thick the straw lies.
const OVERHANG: f32 = 1.15;
const RISE: f32 = 2.50;
const SINK: f32 = 0.08;
const ROOF_THICKNESS: f32 = 0.42;

/// The straw: wide, shallow bundles rather than tile courses.
const COVERING: Covering = Covering {
    period: 0.55,
    fill: 0.34,
    height: 0.10,
};

/// The wing beside the room: how deep it is, how high, and how thick its walls.
const WING_DEPTH: f32 = 2.3;
const WING_LONG: f32 = 3.8;
const WING_HIGH: f32 = 2.15;
const WING_WALL: f32 = 0.40;

/// How far the wing's lean-to rises from its eave to its crest, in metres.
const LEAN_RISE: f32 = 0.85;

/// The door and the two windows, on the room's one long face.
const DOOR_WIDTH: f32 = 1.05;
const DOOR_HIGH: f32 = 1.95;
const WINDOW_SILL: f32 = 0.95;
const WINDOW_HEAD: f32 = 1.75;
const OPENING_MARGIN: f32 = 0.60;

/// The stone foot under the room: how deep it stands below the ground, and how far it is proud of the wall.
const PLINTH: f32 = 0.30;
const PLINTH_PROUD: f32 = 0.22;

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

/// The sun, low enough that the cap's projection is what casts the shadow the frame reads.
const SUN: [f64; 3] = [-22.0, 26.0, 18.0];
const SUN_IRRADIANCE: f64 = 3.2;
const SHADOW_EXTENT: f64 = 34.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 44.0;

/// The terrace the house stands on: its own edge, the distance the ground takes to fall, and how far it falls.
///
/// **A 蘑菇房 stands in a field of terraces and not on a plain.** The drop is in front of the house, where the frame
/// can see it, and it is one step and not a slope: a terrace is a level the water is held on.
const TERRACE_EDGE: f64 = 4.6;
const TERRACE_FALL: f64 = 0.9;
const TERRACE_DROP: f64 = 1.35;

/// Returns the ground's elevation at a plan point: level to the house, then down one terrace, then level again.
fn terrace(_x: f64, z: f64) -> f64 {
    if z <= TERRACE_EDGE {
        0.0
    } else if z >= TERRACE_EDGE + TERRACE_FALL {
        -TERRACE_DROP
    } else {
        -TERRACE_DROP * (z - TERRACE_EDGE) / TERRACE_FALL
    }
}

/// The air over the hillside.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.21, 0.37, 0.64])
        .horizon([0.86, 0.83, 0.75])
        .ground([0.30, 0.29, 0.24])
        .density(0.0012)
        .scale_height(220.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the yard, with the cap's projection between the camera and the sun.
pub fn mushroom_house() -> Value {
    // **From the wing's own side**, so the frame holds both roofs: the cap, and the lean-to falling from the room's
    // wall over the lower room beside it.
    scene(
        [-13.5, 5.6, 12.0],
        [-0.6, 2.4, 0.0],
        FIELD_OF_VIEW,
        "mushroom_house",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("a 蘑菇房 expands: a room, a wing, two roofs and the door");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a 蘑菇房's own document must be clean, and it is not: {violations:?}"
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

    let earth = || surface([0.55, 0.47, 0.36], 0.0, 0.90, [0.0; 3]);
    let straw = || surface([0.62, 0.51, 0.28], 0.0, 0.72, [0.0; 3]);
    let stone = || surface([0.42, 0.40, 0.36], 0.0, 0.86, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("earth", earth as fn() -> Value),
        ("straw", straw),
        ("stone", stone),
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
        materials.push(mesh_material(&format!("yard-{stage}"), shader, earth()));
    }

    let placed = |stage: &str| {
        [
            ("walls", "earth"),
            ("roofs", "straw"),
            ("flats", "stone"),
            ("yard", "yard"),
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
    let yard = ground::mesh(&building, GROUND_DIVISIONS, GROUND_EXTENT, terrace);

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
                    [1.0, 0.95, 0.84],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    160.0,
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
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("flats", "Triangles", point.clone(), flats),
                mesh("yard", "Triangles", plain.point_schema(), yard),
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

/// Returns the document: the room under its cap, the wing beside it, and the door and windows.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = WIDTH * 0.5;
    let deep = DEPTH * 0.5;

    // **The room**, a square of rammed earth with one plate.
    let mut room = Mass::new(
        [
            PlanNode::new([-half, -deep], 0.0),
            PlanNode::new([half, -deep], 0.0),
            PlanNode::new([half, deep], 0.0),
            PlanNode::new([-half, deep], 0.0),
        ]
        .to_vec(),
        STOREY,
    );
    room.wall = WALL;
    room.slab = PLATE;
    let room_id = MassId(skeleton.masses.len());
    skeleton.masses.push(room);

    // **The cap**, and it is the type: one hip roof projecting `OVERHANG` past the wall on all four sides, steep and
    // thick, with the straw's wide shallow bundles.
    let cap = derive::hip_roof(
        room_id,
        &skeleton.masses[room_id.0],
        OVERHANG,
        RISE,
        SINK,
        ROOF_THICKNESS,
    )
    .expect("a room is a rectangle, so its cap is a hip");
    skeleton.roofs.push(cap.with_covering(COVERING));
    // **The cap is built before anything splits the room's own loop**, because a hip is read off that loop and a
    // fifth node on the wall warps its eave by a millimetre -- FaceNotPlanar { departure: 0.0012 }, measured.

    // **The wing**, on the room's own left: a lower mass on its wall, with a lean-to whose crest is on the room and
    // whose eave is over the yard.
    let wing_x = -half - WING_DEPTH + WALL;
    let mut wing = Mass::new(
        [
            PlanNode::new([wing_x, -WING_LONG * 0.5], 0.0),
            PlanNode::new([wing_x + WING_DEPTH, -WING_LONG * 0.5], 0.0),
            PlanNode::new([wing_x + WING_DEPTH, WING_LONG * 0.5], 0.0),
            PlanNode::new([wing_x, WING_LONG * 0.5], 0.0),
        ]
        .to_vec(),
        WING_HIGH,
    );
    wing.wall = WING_WALL;
    wing.slab = 0.28;
    // **The station is the middle of the room's own left wall** and not a corner: a station on a corner splits the
    // outline twice at one point, which the expander reports as `OutlineEdgeDegenerate`. The perimeter is walked
    // from `(-half, -deep)` and the left wall is the last edge, so its middle is `(3*WIDTH + 3*DEPTH + DEPTH/2)` of
    // the way round.
    let perimeter = 2.0 * (WIDTH + DEPTH);
    let station = (2.0 * WIDTH + 3.0 * DEPTH + DEPTH * 0.5) / perimeter;
    let wing_id = ops::attach_mass(&mut skeleton, room_id, station, wing)
        .expect("the room is in the skeleton and its wall takes a wing");
    // **The lean-to**: its crest on the room's wall at `WING_HIGH + LEAN_RISE`, its eave out over the wing's own
    // plate. A lean-to falls away from the taller wall, so its crest is above the wing's own top and below the room's.
    if let Some(shed) = derive::lean_to(
        wing_id,
        [
            [wing_x, -WING_LONG * 0.5],
            [wing_x + WING_DEPTH, -WING_LONG * 0.5],
            [wing_x + WING_DEPTH, WING_LONG * 0.5],
            [wing_x, WING_LONG * 0.5],
        ],
        WING_HIGH + 0.55,
        WING_HIGH + LEAN_RISE,
        0.30,
    ) {
        skeleton.roofs.push(shed.with_covering(COVERING));
    }

    // **A stone foot under the room**, `PLINTH` deep and proud of the wall: a rammed-earth house on that hillside
    // stands on one, and it is also the document's only slab — a mesh a scene declares but does not fill is a
    // document Cycles refuses outright, which this scene learned by declaring `flats` and authoring none.
    let mut foot = vec![
        [-half - PLINTH_PROUD, -deep - PLINTH_PROUD],
        [half + PLINTH_PROUD, -deep - PLINTH_PROUD],
        [half + PLINTH_PROUD, deep + PLINTH_PROUD],
        [-half - PLINTH_PROUD, deep + PLINTH_PROUD],
    ];
    if loop_area(&foot) < 0.0 {
        foot.reverse();
    }
    skeleton
        .slabs
        .push(building::Slab::plate(foot, -PLINTH, 0.0));
    // **The terrace's edge is stone**, because a step in a field is a retaining wall: two runs along `TERRACE_EDGE`,
    // standing from the lower level up to a kerb above the house's own floor, which is what holds the upper level.
    let edge = TERRACE_EDGE as f32;
    for side in [-1.0_f32, 1.0] {
        skeleton.walls.push(building::wall::Wall::new(
            PlanNode::new([side * (half + 1.4), edge], -TERRACE_DROP as f32 - 0.2),
            PlanNode::new([side * 0.2, edge], -TERRACE_DROP as f32 - 0.2),
            0.46,
            0.16,
        ));
    }
    // **The door and the two windows**, on the room's own long face at `+z`: three bays, the door in the middle.
    if let Some(edge) = facade::edge_on(&skeleton.masses[room_id.0], deep, true) {
        let face = building::grid::FaceRef::new(room_id, edge);
        if ops::split_face(
            &mut skeleton,
            face,
            &building::grid::Rhythm::Even { count: 3 },
            &building::grid::Rhythm::Even { count: 1 },
        )
        .is_some()
        {
            let _ = ops::add_opening(
                &mut skeleton,
                building::openings::Opening::new(
                    face,
                    0,
                    1,
                    building::openings::Inset::new(
                        (WIDTH / 3.0 - DOOR_WIDTH) * 0.5,
                        (WIDTH / 3.0 - DOOR_WIDTH) * 0.5,
                        0.0,
                        STOREY - 0.15 - DOOR_HIGH,
                    ),
                ),
            );
            for column in [0_usize, 2] {
                let _ = ops::add_opening(
                    &mut skeleton,
                    building::openings::Opening::new(
                        face,
                        0,
                        column,
                        building::openings::Inset::new(
                            (WIDTH / 3.0 - 0.7) * 0.5,
                            (WIDTH / 3.0 - 0.7) * 0.5,
                            WINDOW_SILL,
                            STOREY - 0.15 - WINDOW_HEAD,
                        ),
                    ),
                );
            }
        }
    }
    let _ = OPENING_MARGIN;
    Building::new(skeleton)
}
