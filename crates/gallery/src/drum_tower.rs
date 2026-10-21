//! A **Dong drum tower** (侗族鼓楼): one timber body and seven skirts of four eaves, the last of them crowned
//! with a hip.
//!
//! # What this scene is for
//!
//! Every building this repository had before this one is a *ring of ranges round a court*, a *row along a
//! street*, or *one house on a bank*. A drum tower is the fourth shape the region builds — **a tower whose
//! eaves are stacked**, one skirt of four lean-tos per storey, read from outside as a pagoda of roofs.
//!
//! # Why the body is one mass and not seven
//!
//! The type's own form is seven storeys, each smaller than the one below and standing on its top. Seven
//! concentric shells are refused: `validate` asks every pair of overlapping plans how they meet, and its two
//! answers are *apart* and *attached at a station on the host's wall*. There is no third relation for *stands
//! on*, so a stack of boxes is a pile of boxes by that reading — correctly. The tower is therefore **one
//! body** and the storeys are read from the **skirts**, which is what a drum tower looks like from outside
//! anyway: one wall and a pagoda of roofs above it.
//!
//! # Why the roofs carry no tiles
//!
//! Every roof wears tiles. That was impossible when this scene was first drawn: a `Covering` lifted a
//! hip's faces out of plane by 0.0102 to 0.0202 of a metre, so the eaves were boarded instead. The roll's
//! own section is now a prism — measured once at the eave rather than taken as a fraction of a band that
//! tapers up a hip's slope — and the covering validates: 0 violations on this tower, where it was 284.
//!   interior;
//! - **no openings**: the body is a closed ring;
//! - **no posts under the lowest eave**: a real drum tower stands on an open colonnade of posts, and this
//!   scene stands it on a stone plinth instead.

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

/// The body: six metres by four and six tenths, seven storeys of two point four, so it stands at 17.25 m.
///
/// **A rectangle and not a square**, because the crown is a hip and a hip needs a ridge: a square plan's
/// crown has none, and `hip_over`'s apex branch is refused as non-planar (measured: departures 0.0098 to
/// 0.0199 on a 6 × 6). A drum tower's plan is square in the street and the shape is worth the compromise.
const SIDE: f32 = 6.0;
const DEPTH: f32 = 4.6;
const STOREYS: usize = 7;
/// How much each storey steps back from the one below, on every side: a tower is read as one storey per
/// eave, and a tower whose storeys did not step back would be a single box wearing seven skirts.
const SHRINK: f32 = 0.34;
const STOREY: f32 = 2.4;
const PLINTH: f32 = 0.45;
const WALL: f32 = 0.20;

/// How many bays each storey is open in, per side, and how each bay is cut.
///
/// **The margin is the wall's thickness plus five centimetres**, the rule every opening in this crate follows;
/// the sill is a rail's height above the floor and the head is the floor above less its plate.
const BAYS: usize = 3;
const BAY_MARGIN: f32 = 0.25;
const BAY_SILL: f32 = 0.75;
const BAY_HEAD: f32 = 0.45;

/// The stair up each storey: how many risers it has, how wide it is, how far it stands off the wall it climbs,
/// and how thick a tread is.
///
/// **The run is computed from the storey's own plan**, so a storey that shrinks as the tower rises gets a
/// steeper flight in the same number of treads rather than a flight that runs through its own wall.
const STAIR_RISERS: usize = 10;
const STAIR_WIDTH: f32 = 1.1;
const STAIR_INSET: f32 = 0.35;
const STAIR_TREAD: f32 = 0.10;
const PLATE: f32 = 0.28;

/// How far a skirt's eave hangs past the wall, and how far below the crest its eave line stands.
const REACH: f32 = 1.5;
const EAVE_DROP: f32 = 0.72;
const SKIRT: f32 = 0.12;

/// The crown.
const CROWN_OVERHANG: f32 = 0.8;
const CROWN_RISE: f32 = 1.8;
const CROWN_THICKNESS: f32 = 0.18;

/// The tiles the eaves and the crown wear, and the 举架 ladder the crown's own slopes carry.
const COVERING: building::Covering = building::Covering {
    period: 0.28,
    fill: 0.62,
    height: 0.05,
};

/// The sun, and the sky the tower stands against.
const SUN: [f64; 3] = [-26.0, 34.0, 22.0];
const SUN_IRRADIANCE: f64 = 3.2;
const SHADOW_EXTENT: f64 = 40.0;
const FRAME_WIDTH: i64 = 1152;
const FRAME_HEIGHT: i64 = 864;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.62;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 80.0;

/// The air over the tower, and the environment map the shading's sky term reads.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.17, 0.33, 0.60])
        .horizon([0.82, 0.79, 0.72])
        .ground([0.35, 0.32, 0.27])
        .density(0.0009)
        .scale_height(220.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the ground, looking up the tower.
pub fn drum_tower() -> Value {
    scene(
        [-22.0, 3.2, 24.0],
        [0.0, 11.0, 0.0],
        FIELD_OF_VIEW,
        "drum_tower",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("a drum tower expands: one body, twenty-eight skirts and a crown");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a drum tower's own document must be clean, and it is not: {violations:?}"
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

    let timber = || surface([0.30, 0.20, 0.12], 0.0, 0.76, [0.0; 3]);
    let boards = || surface([0.26, 0.26, 0.27], 0.0, 0.58, [0.0; 3]);
    let floor = || surface([0.34, 0.26, 0.18], 0.0, 0.82, [0.0; 3]);
    let earth = || surface([0.31, 0.28, 0.22], 0.0, 0.94, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("wall", timber as fn() -> Value),
        ("roof", boards as fn() -> Value),
        ("floor", floor as fn() -> Value),
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
                    [1.0, 0.93, 0.80],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    170.0,
                ),
                environment_light("air", "air", [1.0; 3], 1.10),
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

/// Returns the document: one body, twenty-eight skirts, and a hip on the last.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = SIDE * 0.5;
    let deep = DEPTH * 0.5;
    // **One mass a storey, and each one stands on the one below through the relation.** The plans are
    // concentric and each is smaller than its host, so they overlap; what makes that a tower rather than a
    // pile is `Join::Storey`, and `validate` reads the claim rather than granting it: the base has to be the
    // host's top and the plan has to be inside the host's plan, or the mass is named.
    let mut base: Option<MassId> = None;
    for storey in 0..STOREYS {
        let floor = PLINTH + storey as f32 * STOREY;
        let step = storey as f32 * SHRINK;
        let mut mass = Mass::new(
            [
                PlanNode::new([-half + step, -deep + step], floor),
                PlanNode::new([half - step, -deep + step], floor),
                PlanNode::new([half - step, deep - step], floor),
                PlanNode::new([-half + step, deep - step], floor),
            ]
            .to_vec(),
            floor + STOREY,
        );
        mass.wall = WALL;
        mass.slab = PLATE;
        let id = match base {
            None => {
                skeleton.masses.push(mass);
                MassId(skeleton.masses.len() - 1)
            }
            Some(below) => ops::stack_mass(&mut skeleton, below, mass)
                .expect("the storey below is in the skeleton"),
        };
        // **A skirt at every storey's top, four lean-tos to a skirt**, each with its crest on the storey's
        // own line and its eave hanging `REACH` past the wall. Every one of them wears tiles.
        let line = floor + STOREY;
        let (side, deep_side) = (half - step, deep - step);
        for side_index in 0..4 {
            if let Some(shed) = derive::lean_to(
                id,
                skirt(side, deep_side, side_index),
                line - EAVE_DROP,
                line,
                SKIRT,
            ) {
                skeleton.roofs.push(shed.with_covering(COVERING));
            }
        }
        // **A flight up to the next floor, in every storey but the last.** The storeys are open now, so a tower
        // with no stair is a stack of rooms with no way between them: the flight climbs along the storey's own
        // `+z` wall, one tread a step, from this floor to the next, and its stringer is the beam the treads hang
        // on. It stands inside the storey's plan, `STAIR_INSET` off the wall, so it never reaches a bay.
        if storey + 1 < STOREYS {
            let treads = STAIR_RISERS;
            let rise = STOREY / treads as f32;
            let run = (side * 2.0 - STAIR_INSET * 2.0 - STAIR_WIDTH) / treads as f32;
            let z = deep_side - STAIR_INSET - STAIR_WIDTH * 0.5;
            let from = -side + STAIR_INSET;
            for step in 0..treads {
                let y = floor + rise * (step + 1) as f32;
                let x = from + run * (step as f32 + 0.5);
                skeleton.members.push(building::Member::new(
                    building::Anchor::Free {
                        at: [x, y, z - STAIR_WIDTH * 0.5],
                    },
                    building::Anchor::Free {
                        at: [x, y, z + STAIR_WIDTH * 0.5],
                    },
                    // **The tread's section is measured across its own axis**: the tread runs along `z`, so a
                    // `[0, 1]` across is parallel to it and the member has no width -- `MemberDegenerate` with the
                    // treads' own 1.1, the third time a scene of this crate has made that mistake.
                    [1.0, 0.0],
                    run * 0.92,
                    STAIR_TREAD,
                ));
            }
            // The stringer: one beam under the treads' own ends, from the first riser to the last.
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [from + run * 0.5, floor + rise - STAIR_TREAD, z],
                },
                building::Anchor::Free {
                    at: [from + run * (treads as f32 - 0.5), floor + STOREY, z],
                },
                [0.0, 1.0],
                STAIR_WIDTH * 0.9,
                STAIR_TREAD,
            ));
        }

        // **The crown is a hip on the last storey**, the one form in the crate that closes a plan on all four
        // sides.
        if storey + 1 == STOREYS {
            let crown = derive::hip_roof(
                id,
                &skeleton.masses[id.0],
                CROWN_OVERHANG,
                CROWN_RISE,
                0.08,
                CROWN_THICKNESS,
            )
            .expect("a storey is a rectangle");
            skeleton.roofs.push(crown.with_covering(COVERING));
        }
        base = Some(id);
    }

    // **Every storey is an open pavilion**, which is what a 鼓楼 is: three bays to a side on each floor, cut
    // between a rail's height and the floor above. A tower of closed boxes is a silo; the whole reason the type
    // exists is that the village sits in it, one floor at a time, and looks out.
    //
    // The storeys are cut **after they are all authored**, because a storey's own mass has to exist before its
    // faces can be given a rhythm, and the relation between them says nothing about their walls.
    for storey in 0..STOREYS {
        let id = MassId(storey);
        let edges = skeleton.masses[id.0].outline().len();
        for edge in 0..edges {
            let face = building::grid::FaceRef::new(id, edge);
            if ops::split_face(
                &mut skeleton,
                face,
                &building::grid::Rhythm::Even { count: BAYS },
                &building::grid::Rhythm::Even { count: 1 },
            )
            .is_none()
            {
                continue;
            }
            for column in 0..BAYS {
                let _ = ops::add_opening(
                    &mut skeleton,
                    building::openings::Opening::new(
                        face,
                        0,
                        column,
                        building::openings::Inset::new(BAY_MARGIN, BAY_MARGIN, BAY_SILL, BAY_HEAD),
                    ),
                );
            }
        }
    }

    // **The plinth**, a stone foot the lowest storey stands clear of the ground on.
    let foot = half + 0.45;
    let foot_deep = deep + 0.45;
    skeleton.slabs.push(building::Slab::plate(
        vec![
            [-foot, -foot_deep],
            [foot, -foot_deep],
            [foot, foot_deep],
            [-foot, foot_deep],
        ],
        0.0,
        PLINTH,
    ));
    Building::new(skeleton)
}
/// Returns one side's skirt in plan: **front-left, front-right, back-right, back-left**, with the front edge
/// the eave and the back edge on the wall.
///
/// # Invariants
///
/// **The eave edge is the one pushed out** and it is the same length as the wall it hangs from: the first
/// version flared it, which made the roof's own quad a trapezoid, and a lean-to is one plane over two
/// parallel edges — `validate` answered `RoofDoesNotCover` with the eave's plan area at 11.25 against a skin
/// of 10.34, which is exactly the flare the single plane could not cover.
fn skirt(half: f32, deep: f32, side: usize) -> [[f32; 2]; 4] {
    let out_x = half + REACH;
    let out_z = deep + REACH;
    // **The shed is wider than the wall it hangs from**, by `REACH` at each end, so two neighbouring sheds
    // overlap at the corner and the body is not read between them. A rectangle, still: the eave edge and the
    // crest edge are parallel, which is what the single plane needs.
    match side {
        0 => [
            [-out_x, -out_z],
            [out_x, -out_z],
            [out_x, -deep],
            [-out_x, -deep],
        ],
        1 => [
            [out_x, -out_z],
            [out_x, out_z],
            [half, out_z],
            [half, -out_z],
        ],
        2 => [
            [out_x, out_z],
            [-out_x, out_z],
            [-out_x, deep],
            [out_x, deep],
        ],
        _ => [
            [-out_x, out_z],
            [-out_x, -out_z],
            [-half, -out_z],
            [-half, out_z],
        ],
    }
}
