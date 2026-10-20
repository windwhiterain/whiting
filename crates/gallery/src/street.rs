//! A street: five plots on a frontage, one house on each, and the street rising under them.
//!
//! # What this scene is for
//!
//! The building arcs end at one building per scene. This is the **settlement's first row**: a frontage
//! turns a line into plots (`building::frontage`), and the plots that touch are party walls because the
//! masses that stand on them touch. What the scene adds is not a new construct — it is the layer above
//! them: **a street with a slope**, so the row steps up the hill plot by plot exactly as the stilt house
//! steps up its own bank.
//!
//! # The two facts the row has to have
//!
//! - **The eaves face the street.** Every house's ridge runs along its frontage, which is why the plots
//!   are wider than they are deep: `derive::gable_roof` reads the ridge off the longer plan axis, so a
//!   6 x 5 plot gets its ridge down the street and its eaves over the pavement.
//! - **The ground steps.** The street rises along its own length and each house is seated on the highest
//!   ground under its own outline, so the terrace of roofs climbs instead of one long level.
//!
//! # What it does not draw
//!
//! - **no street surface but the ground**: no pavement, no gutter, no arcade (the 骑楼 needs a first
//!   floor standing out over the walk, which is the 挑廊's plate again);
//! - **no openings** — every house is a closed ring, as the compound's ranges are;
//! - **no blind party walls** — a party wall is drawn where the two masses meet, and the crate's joint is
//!   two walls touching rather than one wall shared.

use definition::Value;
use scene::standard::Stage;

use building::derive;
use building::frontage::Frontage;
use building::grid::{FaceRef, Rhythm};
use building::identity::Identity;
use building::openings::{Inset, Opening};
use building::site;
use building::{Building, BuildingSkeleton, Covering, Mass, PlanNode, ops};

use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, sky, surface,
};
use document::shaders::{Projection, post_shader};

/// The frontage: five plots of six metres, five deep, on a street that runs along `x`.
const PLOTS: usize = 5;
const WIDTH: f32 = 6.0;
const DEPTH: f32 = 5.0;

/// How fast the street climbs, and how much of the hill each house's floor sits on.
const SLOPE: f32 = 0.055;

/// The houses: two storeys, a gable, deep eaves and the 举架 ladder on every slope.
const HEIGHT: f32 = 6.2;
const WALL: f32 = 0.4;
const PLATE: f32 = 0.35;
const FLOOR: f32 = 3.0;
const OVERHANG: f32 = 0.7;

/// How far a gable end oversails. **Zero: the street is 硬山**, so each house's gable walls are its party
/// walls and its roof stops at them. With the ends oversailing by the same 0.7 as the eaves, five
/// neighbours' roofs would each reach 0.7 into the next one's — which is what this row drew before the
/// derivation could say which end a number was about.
const GABLE: f32 = 0.0;
const RISE: f32 = 1.9;
const SINK: f32 = 0.1;
const ROOF_THICKNESS: f32 = 0.24;

/// The covering every roof wears, and the 举架 ladder its slopes carry.
const COVERING: Covering = Covering {
    period: 0.32,
    fill: 0.62,
    height: 0.06,
};
const BEAM: f32 = 0.85;
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

/// The sun: from the street's own side, so the eaves cast a band along the fronts.
const SUN: [f64; 3] = [-30.0, 30.0, -26.0];
const SUN_IRRADIANCE: f64 = 3.4;
const SHADOW_EXTENT: f64 = 34.0;

/// The image, and the lens.
const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// How the ground is drawn: one-metre cells over 140 metres.
const GROUND_DIVISIONS: usize = 140;
const GROUND_EXTENT: f64 = 70.0;

/// Builds the scene from outside the row, on the street, looking up it.
pub fn street() -> Value {
    scene(
        [-13.0, 8.5, -24.0],
        [16.0, 3.0, 3.0],
        FIELD_OF_VIEW,
        "street",
    )
}

/// Builds **the same street** from under the 骑楼, which is where a street is walked.
///
/// # Invariants
///
/// **One document and two cameras.** The eyes differ and nothing else: the arcade is the same set of
/// canopies and posts the other view sees from outside, and the rows are the same rows.
pub fn street_arcade() -> Value {
    scene(
        [2.4, 1.65, -1.15],
        [26.0, 2.45, -1.15],
        1.25,
        "street_arcade",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the street expands: five houses, five roofs and five floors");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "the street's own document must be clean, and it is not: {violations:?}"
    );

    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // **The frame is its own mesh**: a post is not a wall with a different elevation, and it is not
    // rendered at all unless a scene asks for it. Nothing drew a member before this one.
    let timbers = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
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

    let plaster = || surface([0.78, 0.74, 0.66], 0.0, 0.86, [0.0; 3]);
    let tiles = || surface([0.30, 0.29, 0.28], 0.0, 0.60, [0.0; 3]);
    let floor = || surface([0.32, 0.24, 0.16], 0.0, 0.80, [0.0; 3]);
    let earth = || surface([0.30, 0.27, 0.21], 0.0, 0.94, [0.0; 3]);
    let timber = || surface([0.34, 0.24, 0.15], 0.0, 0.78, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("wall", plaster as fn() -> Value),
        ("roof", tiles as fn() -> Value),
        ("timber", timber as fn() -> Value),
        ("floor", floor as fn() -> Value),
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
            ("frame", "timber"),
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
    let ground = crate::ground::mesh(&building, GROUND_DIVISIONS, GROUND_EXTENT, |x, _z| {
        f64::from(SLOPE) * x.max(0.0)
    });
    // **One air, handed to two renderers**: the map a world reads and the light a surface sees. Without the
    // light the shading's sky term is zero, which is why a sun-only scene's shadowed side is black.
    let (sky_image, sky_light) = sky(&atmosphere());

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
                    [1.0, 0.92, 0.78],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    190.0,
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
                mesh("frame", "Triangles", point.clone(), timbers),
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

/// Returns the street's document: a frontage's plots, one house on each, and the floors between their
/// storeys.
///
/// # Invariants
///
/// **The houses are built, then seated, then cut.** A face's grid holds *absolute* elevations, so a grid
/// cut before `site::seat` would keep the level the house had before it was moved up the hill, and every
/// opening in it would be that far below its own wall. Seating first is the whole of the order.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let frontage = Frontage::new(
        vec![[0.0, 0.0], [WIDTH * PLOTS as f32, 0.0]],
        // The plots are on the walk's **left**, so the houses stand at positive `z` and the street runs
        // along their fronts at `z = 0`.
        1.0,
        DEPTH,
        vec![WIDTH; PLOTS],
        // A party wall is a gap of zero: the masses touch, which is what the joint is.
        0.0,
    );
    let plots = frontage
        .plots()
        .expect("five six-metre plots fit a thirty-metre line");
    for plot in &plots {
        let mut mass = Mass::new(
            plot.corners()
                .into_iter()
                // **Authored at one level and seated after**, which is what `site::seat` is for: a base
                // band drawn on a slope is a warped band, and the street's step belongs to the mass's
                // elevation rather than to its outline.
                .map(|at| PlanNode::new(at, 0.0))
                .collect::<Vec<_>>(),
            HEIGHT,
        );
        mass.wall = WALL;
        mass.slab = PLATE;
        let mass_id = building::MassId(skeleton.masses.len());
        skeleton.masses.push(mass);
        skeleton
            .roofs
            .push(roof_over(mass_id, &skeleton.masses[mass_id.0]));
    }
    // **The row is seated as one street**, so the houses take the hill's own levels rather than each
    // one's private average: `site::seat` moves every node of every mass to the highest ground it stands
    // on, and the roofs follow their masses.
    let _ = site::seat(&mut skeleton, at_grade, 0.0);

    // **The 骑楼 goes up after the seat**, because a walk is at the street's own level and the street
    // is what the seat reads: five canopies, one to a plot, each shedding away from the shop, and the
    // posts that hold their outer edges at the grade under them.
    arcade(&mut skeleton);
    // The floors are derived from the seated masses, so their elevation is the storey's own.
    for plot in &plots {
        let mass_id = building::MassId(plot.index);
        let mass = &skeleton.masses[mass_id.0];
        if let Some(storey) = derive::floor_slab(mass, mass_id, mass.lowest() + FLOOR, PLATE * 0.8)
        {
            skeleton.slabs.push(storey);
        }
    }
    // **The facades are cut last**, and the front is found by matching the plot's own two corners against
    // the mass's outline: one door and two windows on the street side, in a three-bay two-storey rhythm.
    for plot in &plots {
        let mass_id = building::MassId(plot.index);
        let Some(edge) = front_edge(&skeleton.masses[mass_id.0], plot) else {
            continue;
        };
        let face = FaceRef::new(mass_id, edge);
        let columns = Rhythm::Even { count: 3 };
        let rows = Rhythm::Even { count: 2 };
        if ops::split_face(&mut skeleton, face, &columns, &rows).is_none() {
            continue;
        }
        for opening in [
            Opening::new(face, 0, 1, Inset::new(0.45, 0.45, 0.0, 0.45)),
            Opening::new(face, 0, 0, Inset::new(0.55, 0.55, 1.15, 0.55)),
            Opening::new(face, 0, 2, Inset::new(0.55, 0.55, 1.15, 0.55)),
            Opening::new(face, 1, 1, Inset::new(0.40, 0.40, 0.95, 0.50)),
        ] {
            let _ = ops::add_opening(&mut skeleton, opening);
        }
    }
    Building::new(skeleton)
}

/// Returns the roof over one seated mass: a gable on the 举架 ladder, with its tiles.
/// Builds the **骑楼**: the covered walk the shops stand behind.
///
/// # Invariants
///
/// **One canopy a plot, shedding away from the shop.** The canopy is a lean-to whose crest is on the
/// house's own front wall and whose eave stands `WALK_DEPTH` out over the street, so the water it
/// catches runs to the street and not through the shopfront — which is what the type is for and what a
/// flat awning would not do.
///
/// **Every post stands on the ground under it**, because the street climbs: the posts of one plot do
/// not share an elevation with the next plot's, and a walk built on one level would float at the top of
/// the hill and be buried at the bottom.
///
/// **The canopy is hosted by the mass it covers.** A roof is authorized to touch the mass its `host`
/// names, so a single canopy spanning the row would have to claim one of the five and lie about it.
fn arcade(skeleton: &mut BuildingSkeleton) {
    const DEPTH: f32 = 2.2;
    const LOW: f32 = 2.95;
    const HIGH: f32 = 3.55;
    const POST: f32 = 0.16;
    for index in 0..skeleton.masses.len() {
        let mass_id = building::MassId(index);
        let Some(mass) = skeleton.masses.get(index) else {
            continue;
        };
        let (x0, x1, front) = extents(mass);
        // Half a metre in from the party wall at each end, so two neighbouring canopies are two
        // canopies rather than one with a seam in it.
        let left = x0 + 0.5;
        let right = x1 - 0.5;
        let base = mass.lowest();
        let plan = [
            [left, front - DEPTH],
            [right, front - DEPTH],
            [right, front],
            [left, front],
        ];
        let Some(roof) = derive::lean_to(mass_id, plan, base + LOW, base + HIGH, 0.08) else {
            continue;
        };
        skeleton.roofs.push(roof.with_covering(COVERING));
        for x in [left, right] {
            let ground = grade(x);
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [x, ground, front - DEPTH + POST],
                },
                building::Anchor::Free {
                    at: [x, base + LOW - 0.04, front - DEPTH + POST],
                },
                [1.0, 0.0],
                POST,
                POST,
            ));
        }
    }
}

/// Returns a mass's plan extents along the street and its front line, which is its lowest `z`.
fn extents(mass: &Mass) -> (f32, f32, f32) {
    let mut x0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut front = f32::MAX;
    for node in &mass.outline {
        x0 = x0.min(node.position[0]);
        x1 = x1.max(node.position[0]);
        front = front.min(node.position[1]);
    }
    (x0, x1, front)
}

fn roof_over(mass_id: building::MassId, mass: &Mass) -> building::roof::Roof {
    let mut roof =
        derive::gable_roof_by(mass_id, mass, OVERHANG, GABLE, RISE, SINK, ROOF_THICKNESS)
            .expect("a plot is a rectangle");
    for plane in &mut roof.planes {
        plane.profile = PROFILE.to_vec();
    }
    roof.with_covering(COVERING)
}
/// Returns which edge of a mass is the plot's street front, if one of them is.
///
/// # Invariants
///
/// **The two ends are matched as a pair**, either way round: a plot's corners are wound to the crate's
/// positive area and the front may be the first edge or the third, so the answer is the edge whose two
/// nodes are the front's own two nodes — a question about the plan and not about the winding.
fn front_edge(mass: &Mass, plot: &building::frontage::Plot) -> Option<usize> {
    let same = |left: [f32; 2], right: [f32; 2]| {
        (left[0] - right[0]).abs() < 1.0e-4 && (left[1] - right[1]).abs() < 1.0e-4
    };
    for index in 0..mass.outline.len() {
        let from = mass.outline[index].position;
        let to = mass.outline[(index + 1) % mass.outline.len()].position;
        let forward = same(from, plot.front[0]) && same(to, plot.front[1]);
        let backward = same(from, plot.front[1]) && same(to, plot.front[0]);
        if forward || backward {
            return Some(index);
        }
    }
    None
}

/// Returns the street's grade at one plan coordinate: a straight climb at `SLOPE`, level past the row.
fn grade(x: f32) -> f32 {
    SLOPE * x.max(0.0)
}

/// Returns the street's grade under one plan point, which is what [`site::seat`] asks for.
fn at_grade(at: [f32; 2]) -> f32 {
    grade(at[0])
}

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The air over the street: a clear afternoon, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's
/// bloom — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads
/// its sky term from. A street whose shadows are black is a street with no air, not a street with the
/// wrong colours.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.24, 0.42, 0.68])
        .horizon([0.86, 0.84, 0.78])
        .ground([0.28, 0.26, 0.21])
        .sun([-30.0, 30.0, -26.0], [1.0, 0.92, 0.74], 10.0, 0.30)
        .density(0.0008)
        .scale_height(240.0)
}

/// What the post stage does to the frame.
const GRADE: &str = r#"
    // **The sky is read out of the scene's own environment map**, the same air the bounce reads, so the
    // background and the light cannot be two skies.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp((1.0 - exp(-along_ray * 0.0020)) * 0.5, 0.0, 0.75);
        color = mix(
            color,
            environment_sky(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)),
            air,
        );
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.14), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.32, 0.0, 1.0);
"#;
