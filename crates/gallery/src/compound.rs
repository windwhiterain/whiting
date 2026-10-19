//! A 三坊一照壁: three two-storey ranges round a court, and a screen wall closing the fourth side.
//!
//! # What this scene is for
//!
//! The stilt house is one building on one slope. This is the **settlement's unit** — the Bai compound
//! of the Dali plain, whose three ranges are the type's whole form and whose fourth side is not a
//! fourth range but a **照壁**, a screen wall standing in front of the gate. Everything it is drawn
//! with already existed before this scene: three masses, three gable roofs, two intermediate floors,
//! a wall, and the covering from the tiles arc. What it proves is that the model *composes* — that a
//! compound is three buildings and a wall, and not a new kind of thing.
//!
//! # Why the court is a gap and not a void
//!
//! A 四合院 with four ranges round the court would be one mass with a courtyard void in it, which the
//! crate has had since it had courtyards. This type is not that: its fourth side is **a single-storey
//! screen**, and a mass's walls are all one height. So the compound is three masses that touch at two
//! corners, and the court is the space between them — the same square of ground, arrived at the other
//! way round, and the one that lets the fourth side be a wall.
//!
//! # What it does not draw
//!
//! - **no openings** — every range is a closed ring; the doors and windows are the interior arc's work;
//! - **no stair** — the two floors are drawn and nothing yet climbs between them;
//! - **no 厦子** — the Yunnan front porch, which is the next arc's plate.

use definition::Value;
use scene::standard::Stage;

use building::derive;
use building::identity::Identity;
use building::wall::Wall;
use building::{Building, BuildingSkeleton, Covering, Mass, PlanNode};

use crate::air::Atmosphere;
use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, sky, surface,
};
use crate::shaders::{Projection, post_shader};

/// The court: its half width, and the two depths it runs between.
const COURT_HALF: f32 = 3.0;
const COURT_BACK: f32 = -3.0;
const COURT_FRONT: f32 = 12.0;

/// How far the ranges reach behind and beside the court.
const SITE_HALF: f32 = 9.0;
const SITE_BACK: f32 = -9.0;

/// The two storeys: the main hall is a little taller than its two wings, which is the type's hierarchy
/// and not a drawing choice.
const MAIN_HEIGHT: f32 = 6.4;
const WING_HEIGHT: f32 = 5.6;
const MAIN_FLOOR: f32 = 3.1;
const WING_FLOOR: f32 = 2.8;

/// The common numbers: a 0.5 m wall, a 0.4 m plate, deep eaves, a gable roof and its rise.
const WALL: f32 = 0.5;
const PLATE: f32 = 0.4;
const OVERHANG: f32 = 0.8;
const RISE: f32 = 2.2;
const SINK: f32 = 0.1;
const ROOF_THICKNESS: f32 = 0.25;

/// The covering every roof in the compound wears, and the 举架 ladder its slopes carry.
const COVERING: Covering = Covering {
    period: 0.32,
    fill: 0.62,
    height: 0.06,
};
const BEAM: f32 = 0.9;
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

/// The sun, its irradiance and the page its shadow needs: the compound is 18 by 21 metres and its
/// walls are two storeys, so the page is wider than the stilt house's.
const SUN: [f64; 3] = [-26.0, 32.0, 28.0];
const SUN_IRRADIANCE: f64 = 3.2;
const SHADOW_EXTENT: f64 = 30.0;

/// The image, and the lens.
const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// How the ground is drawn: one-metre cells over 120 metres, level.
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 60.0;

/// Builds the scene.
pub fn compound() -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the compound expands: three boxes, three roofs, two floors and a wall");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "the compound's own document must be clean, and it is not: {violations:?}"
    );

    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    let roofs = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Roof(_, _) | building::Element::Roll(_, _)
        )
    });
    let floors = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            )
        )
    });
    let rails = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Wall(_, _) | building::Element::Member(_, _)
        )
    });

    let plaster = || surface([0.80, 0.76, 0.68], 0.0, 0.85, [0.0; 3]);
    let tiles = || surface([0.31, 0.30, 0.29], 0.0, 0.58, [0.0; 3]);
    let floor = || surface([0.33, 0.25, 0.17], 0.0, 0.80, [0.0; 3]);
    let timber = || surface([0.20, 0.14, 0.10], 0.0, 0.72, [0.0; 3]);
    let earth = || surface([0.24, 0.22, 0.17], 0.0, 0.94, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("wall", plaster as fn() -> Value),
        ("roof", tiles as fn() -> Value),
        ("floor", floor as fn() -> Value),
        ("timber", timber as fn() -> Value),
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
            ("rails", "timber"),
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
    // **One air, handed to two renderers**: the map a world reads and the light a surface sees. Without the
    // light the shading's sky term is zero, and the compound's own court walls would be black.
    let (sky_image, sky_light) = sky(&atmosphere());

    Value::structure([
        ("name".to_owned(), Value::String("compound".to_owned())),
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            Value::List(vec![camera(
                [10.0, 13.0, 33.0],
                [0.0, 2.5, 1.0],
                FIELD_OF_VIEW,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "sun",
                    SUN,
                    [1.0, 0.9, 0.76],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    160.0,
                ),
                sky_light,
            ]),
        ),
        ("images".to_owned(), Value::List(vec![sky_image])),
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
                mesh("rails", "Triangles", point.clone(), rails),
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

/// Returns the compound's document: three ranges, a screen wall and the floors between their storeys.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    // **The main hall at the back, its two wings down the sides**, and the court between the three.
    // **The main hall's floor has a stairwell through it**, which is the one place this scene draws the
    // interior arc's construct: a slab with a hole is a prism over a ring, so the stair up through it is
    // a hole in the plate and not a second storey's worth of new geometry.
    let main_corners = [
        [-SITE_HALF, SITE_BACK],
        [SITE_HALF, SITE_BACK],
        [SITE_HALF, COURT_BACK],
        [-SITE_HALF, COURT_BACK],
    ];
    let main = range(
        &mut skeleton,
        main_corners,
        MAIN_HEIGHT,
        MAIN_FLOOR,
        Some(stairwell(&main_corners)),
    );
    let west = range(
        &mut skeleton,
        [
            [-SITE_HALF, COURT_BACK],
            [-COURT_HALF, COURT_BACK],
            [-COURT_HALF, COURT_FRONT],
            [-SITE_HALF, COURT_FRONT],
        ],
        WING_HEIGHT,
        WING_FLOOR,
        None,
    );
    let east = range(
        &mut skeleton,
        [
            [COURT_HALF, COURT_BACK],
            [SITE_HALF, COURT_BACK],
            [SITE_HALF, COURT_FRONT],
            [COURT_HALF, COURT_FRONT],
        ],
        WING_HEIGHT,
        WING_FLOOR,
        None,
    );
    let _ = (main, west, east);
    // The 照壁: one wall across the front, from the west wing's end to the east wing's.
    skeleton.walls.push(Wall::new(
        PlanNode::new([-SITE_HALF, COURT_FRONT], 0.0),
        PlanNode::new([SITE_HALF, COURT_FRONT], 0.0),
        0.4,
        3.6,
    ));
    Building::new(skeleton)
}

/// Adds one range — a mass, its intermediate floor and its roof — and returns the mass's id.
///
/// # Invariants
///
/// **The outline is authored positively wound**, which is what a mass's plan is: the four corners of a
/// rectangle taken counter-clockwise in `(x, z)` — and the ranges are built in the order that makes the
/// court the space between them, so two neighbouring ranges **touch** along a line and never overlap.
fn range(
    skeleton: &mut BuildingSkeleton,
    corners: [[f32; 2]; 4],
    height: f32,
    floor: f32,
    stairwell: Option<Vec<building::math::Vec2>>,
) -> building::MassId {
    let mut mass = Mass::new(
        corners
            .iter()
            .map(|at| PlanNode::new(*at, 0.0))
            .collect::<Vec<_>>(),
        height,
    );
    mass.wall = WALL;
    mass.slab = PLATE;
    let id = skeleton.masses.len();
    let mass_id = building::MassId(id);
    // The intermediate floor, seated on the mass's own inner loop so it meets its walls.
    if let Some(storey) = derive::floor_slab(&mass, mass_id, floor, PLATE * 0.75) {
        skeleton.slabs.push(match stairwell {
            Some(void) => storey.with_void(void),
            None => storey,
        });
    }
    let mut roof = derive::gable_roof(mass_id, &mass, OVERHANG, RISE, SINK, ROOF_THICKNESS)
        .expect("a rectangle has an eave loop");
    for plane in &mut roof.planes {
        plane.profile = PROFILE.to_vec();
    }
    roof = roof.with_covering(COVERING);
    skeleton.masses.push(mass);
    skeleton.roofs.push(roof);
    mass_id
}

/// Returns the loop of the stairwell through one range's floor: a hole at its west end.
///
/// # Invariants
///
/// **The hole stays inside the walls**, `WALL` plus a hand's width clear of the outline, which is what
/// keeps it strictly inside the plate and therefore a hole rather than a slit through the wall.
fn stairwell(corners: &[[f32; 2]; 4]) -> Vec<building::math::Vec2> {
    let xs: Vec<f32> = corners.iter().map(|at| at[0]).collect();
    let zs: Vec<f32> = corners.iter().map(|at| at[1]).collect();
    let (low_x, _high_x) = (
        xs.iter().copied().fold(f32::INFINITY, f32::min),
        xs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    );
    let (low_z, high_z) = (
        zs.iter().copied().fold(f32::INFINITY, f32::min),
        zs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    );
    let middle = (low_z + high_z) * 0.5;
    let left = low_x + WALL + 0.35;
    vec![
        [left, middle - 0.7],
        [left + 1.5, middle - 0.7],
        [left + 1.5, middle + 0.7],
        [left, middle + 0.7],
    ]
}

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The air over the compound: a bright highland afternoon, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's bloom
/// — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads its sky
/// term from. A court whose inner walls are black is a court with no air, not a court with the wrong
/// colours.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.22, 0.40, 0.66])
        .horizon([0.84, 0.84, 0.80])
        .ground([0.26, 0.25, 0.20])
        .sun([-26.0, 32.0, 28.0], [1.0, 0.9, 0.72], 8.0, 0.24)
        .density(0.0008)
        .scale_height(240.0)
}

/// What the post stage does to the frame.
const GRADE: &str = r#"
    // **The sky is read out of the scene's own environment map**, the same air the bounce reads.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp((1.0 - exp(-along_ray * 0.0022)) * 0.55, 0.0, 0.8);
        color = mix(
            color,
            environment_sky(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)),
            air,
        );
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.16), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.35, 0.0, 1.0);
"#;
