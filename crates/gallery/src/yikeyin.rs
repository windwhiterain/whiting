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
use building::grid::{FaceRef, Rhythm};
use building::openings::{Inset, Opening};
use building::site;
use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, ops};

use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, surface,
};
use crate::shaders::post_shader;

/// The site: a nearly square plan, thirteen by fifteen, which is the type's own proportion.
const HALF_X: f32 = 6.5;
const HALF_Z: f32 = 7.5;

/// The court: three metres square, the well the ranges are built round.
const COURT: f32 = 1.5;

/// The depths of the four ranges, and their heights. The 正房 is the tallest, the gate house the lowest.
const MAIN_DEPTH: f32 = 6.0;
const MAIN_HEIGHT: f32 = 6.2;
const WING_HEIGHT: f32 = 5.6;
const GATE_HEIGHT: f32 = 3.6;

/// The common numbers: a wall, a plate, deep eaves, a gable, its rise, and **no overhang at the ends**.
const WALL: f32 = 0.45;
const PLATE: f32 = 0.35;
const OVERHANG: f32 = 0.65;
const GABLE: f32 = 0.0;
const RISE: f32 = 1.8;
const SINK: f32 = 0.1;
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
const SUN_IRRADIANCE: f64 = 3.3;
const SHADOW_EXTENT: f64 = 30.0;

/// The image, and the lens.
const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// How the ground is drawn: one-metre cells over 120 metres, level.
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 60.0;

/// Builds the scene.
pub fn yikeyin() -> Value {
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
    let floors = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            )
        )
    });

    let plaster = || surface([0.80, 0.76, 0.68], 0.0, 0.86, [0.0; 3]);
    let tiles = || surface([0.30, 0.29, 0.28], 0.0, 0.60, [0.0; 3]);
    let floor = || surface([0.33, 0.25, 0.17], 0.0, 0.80, [0.0; 3]);
    let earth = || surface([0.31, 0.28, 0.22], 0.0, 0.94, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("wall", plaster as fn() -> Value),
        ("roof", tiles as fn() -> Value),
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
        ("name".to_owned(), Value::String("yikeyin".to_owned())),
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            Value::List(vec![camera(
                [17.0, 19.0, 23.0],
                [-0.5, 1.2, 0.5],
                FIELD_OF_VIEW,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![light_with_radius(
                "sun",
                SUN,
                [1.0, 0.92, 0.78],
                flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                SHADOW_EXTENT,
                170.0,
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List({
                let mut shaders = stage_set("authored", "", "", &point, &extras);
                shaders.extend(stage_set("plain", "", "", &plain.point_schema(), &plain));
                shaders.push(post_shader(GRADE, SKY, None));
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
    // The one gate: the 门房's south face, one bay wide and two metres tall.
    let gate = FaceRef::new(MassId(3), 3);
    if let Some(edge) = skeleton.masses.get(3).map(|mass| mass.outline.len()) {
        let _ = edge;
    }
    if ops::split_face(
        &mut skeleton,
        gate,
        &Rhythm::Even { count: 3 },
        &Rhythm::Even { count: 1 },
    )
    .is_some()
    {
        let _ = ops::add_opening(
            &mut skeleton,
            Opening::new(gate, 0, 1, Inset::new(0.35, 0.35, 0.0, 0.9)),
        );
    }
    // **The 夏子**, the porch across the 正房's front: a lean-to of its own, two posts at its edge, and two
    // struts pinned to the 正房's wall face — the face anchor's first use in a scene.
    porch(&mut skeleton);
    // **One seat for the whole compound**, so the four ranges share a level rather than each finding its
    // own: a 一颗印 is built flat, and its court is a well and not a slope.
    let _ = site::seat(&mut skeleton, |_| 0.0, 0.0);
    Building::new(skeleton)
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
    let count = mass.outline.len();
    for edge in 0..count {
        let from = mass.outline[edge].position;
        let to = mass.outline[(edge + 1) % count].position;
        if (from[1] - z).abs() < 1.0e-3 && (to[1] - z).abs() < 1.0e-3 {
            let length = (to[0] - from[0]).abs();
            if length > 1.0 {
                return Some((edge, from[0], length));
            }
        }
    }
    None
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
        mass.crowns = vec![height; mass.outline.len()];
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
        crown_gables(&mut skeleton.masses[mass_id.0], &roof);
    }
    skeleton.roofs.push(roof);
    if let Some(storey) = derive::floor_slab(
        &skeleton.masses[mass_id.0],
        mass_id,
        height * 0.48,
        PLATE * 0.8,
    ) {
        skeleton.slabs.push(storey);
    }
}

/// Splits the two gable ends of a range at their middles and raises those nodes to the roof's apex.
///
/// # Invariants
///
/// **The ends are the two edges the ridge does not run along**, read from the plan as the shorter pair of
/// the loop; each is split at its own middle station, **far one first**, because an insertion moves every
/// station after it. The apex is the roof's own highest underside, so the wall meets the roof instead of
/// stopping below it.
fn crown_gables(mass: &mut Mass, roof: &building::roof::Roof) {
    let apex = roof
        .crest
        .iter()
        .map(|point| point.elevation)
        .fold(f32::NEG_INFINITY, f32::max)
        - roof.thickness
        - 0.01;
    let Some(stations) = gable_stations(mass) else {
        return;
    };
    for station in stations {
        if let Some(index) = ops::split_outline(mass, station) {
            if let Some(crown) = mass.crowns.get_mut(index) {
                *crown = apex;
            }
        }
        // The split rewrites the crown vector with the loop, so a node that is not crowned yet keeps the
        // wall top it had; the vector grows by one entry per split.
    }
}

/// Returns the two stations at which the gable ends stand, **far one first**.
///
/// # Invariants
///
/// A gable end is one of the two edges the ridge does not run along: the ridge follows the longer pair of
/// edges, so the ends are the other pair. Each answers its own middle station as a fraction of the
/// **closed** loop before either insertion, which is why they are sorted descending.
fn gable_stations(mass: &Mass) -> Option<Vec<f32>> {
    let count = mass.outline.len();
    if count != 4 {
        return None;
    }
    let spans: Vec<f32> = (0..count)
        .map(|edge| {
            let from = mass.outline[edge].position;
            let to = mass.outline[(edge + 1) % count].position;
            ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt()
        })
        .collect();
    let total: f32 = spans.iter().sum();
    if total <= f32::EPSILON {
        return None;
    }
    // The pair of edges the ridge runs along is the longer pair: `0, 2` or `1, 3`.
    let (first_pair, second_pair) = (spans[0] + spans[2], spans[1] + spans[3]);
    let ends: Vec<usize> = if first_pair >= second_pair {
        vec![1, 3]
    } else {
        vec![0, 2]
    };
    let mut stations: Vec<f32> = ends
        .iter()
        .map(|edge| {
            let walked: f32 = spans[..*edge].iter().sum();
            (walked + spans[*edge] * 0.5) / total
        })
        .collect();
    stations.sort_by(|left, right| right.partial_cmp(left).unwrap_or(std::cmp::Ordering::Equal));
    Some(stations)
}

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The sky this scene draws: a highland afternoon over a sealed court.
const SKY: &str = r#"
fn sky_at(ray: vec3<f32>) -> vec3<f32> {
    let above = pow(clamp(ray.y, 0.0, 1.0), 0.52);
    let horizon = vec3<f32>(0.85, 0.84, 0.79);
    let zenith = vec3<f32>(0.23, 0.41, 0.67);
    let below = vec3<f32>(0.27, 0.25, 0.20);
    let sun = normalize(vec3<f32>(-18.0, 40.0, 24.0));
    let glow = pow(clamp(dot(normalize(ray), sun), 0.0, 1.0), 9.0) * 0.26;
    let sky = select(mix(horizon, below, clamp(-ray.y * 4.0, 0.0, 1.0)),
                     mix(horizon, zenith, above),
                     ray.y > 0.0);
    return sky + vec3<f32>(1.0, 0.92, 0.74) * glow;
}
"#;

/// What the post stage does to the frame.
const GRADE: &str = r#"
    if depth >= 1.0 {
        color = sky_at(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp((1.0 - exp(-along_ray * 0.0021)) * 0.52, 0.0, 0.78);
        color = mix(color, sky_at(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)), air);
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.15), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.34, 0.0, 1.0);
"#;
