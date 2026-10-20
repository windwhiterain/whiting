//! A **风雨桥**: the covered bridge that stands at the mouth of a 侗族 village, and the drum tower's other half.
//!
//! # What this scene is for
//!
//! The gallery has had a 鼓楼 since the fourth type; a 侗寨 is a tower *and* a bridge, and the bridge is the one
//! piece of the region's architecture that is a **building and a road at once**. It is a timber gallery over a
//! river: stone piers, a deck, two rows of posts carrying a tiled gable, and — the type's own signature — a
//! **pavilion** raised over the middle of the span, which is where the old people sit and where the bridge stops
//! being a crossing and becomes a room.
//!
//! # The three facts the type has
//!
//! - **The deck is a slab and the piers are posts.** A bridge's road is a plate on posts, not a wall: the plate
//!   spans from bank to bank and the piers stand in the water under it.
//! - **The gallery is a mass, because the roof needs a host.** `Roof::new` names the mass it sits on, so the
//!   gallery is a thin-walled volume over the deck — and its roof is a gable running the length of the span.
//! - **The pavilion stands on the gallery through `Join::Storey`**, the relation the drum tower's seven floors
//!   introduced: the middle third of the bridge is a second mass on the first, proud of it on both sides, wearing
//!   its own hip. That is the one place in this scene where a relation is load-bearing.
//!
//! # What it does not draw
//!
//! - **the water**: the ground under the bridge is the harness's own plane, and a river is a surface this scene
//!   does not author;
//! - **the benches**: a 风雨桥's two long benches are where the village sits, and they are not drawn;
//! - **the banks**: the abutments are plate ends and nothing more.

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

use crate::facade;
use crate::ground;

/// The span: `SPAN` long, `WIDTH` wide, with the deck `DECK` above the water.
const SPAN: f32 = 26.0;
const WIDTH: f32 = 4.2;
const DECK: f32 = 3.6;

/// The gallery over the deck: `GALLERY` tall, with walls `GALLERY_WALL` thick and a plate of its own.
const GALLERY: f32 = 2.3;
const GALLERY_WALL: f32 = 0.18;
const GALLERY_PLATE: f32 = 0.24;

/// The pavilion over the middle: `PAVILION_LONG` of the span, proud of the gallery on both sides.
const PAVILION_LONG: f32 = 6.0;
const PAVILION_HIGH: f32 = 2.2;

/// The roofs: a gable the length of the span and a hip over the pavilion, both tiled.
const OVERHANG: f32 = 0.65;
const RISE: f32 = 1.35;
const SINK: f32 = 0.10;
const ROOF_THICKNESS: f32 = 0.15;
const COVERING: building::Covering = building::Covering {
    period: 0.30,
    fill: 0.62,
    height: 0.05,
};

/// The piers: `PIERS` sets of `PIER_POSTS` stone posts, `PIER` square, standing in the water.
const PIERS: [f32; 2] = [-6.5, 6.5];
const PIER_POSTS: [f32; 3] = [-1.4, 0.0, 1.4];
const PIER: f32 = 0.45;

/// How many bays the gallery is open in, down each of its long sides, and how each bay is cut.
///
/// **The margin is the wall's own thickness plus five centimetres**, the rule every opening in this crate
/// follows, and the sill is a hand's height above the deck because a gallery's bay has a bench in it.
const BAYS: usize = 9;
const BAY_MARGIN: f32 = 0.23;
const BAY_SILL: f32 = 0.55;
const BAY_HEAD: f32 = 0.40;

/// The railing on both edges of the deck: a post every `RAIL_SPACING`, and a top rail over them.
const RAIL_SPACING: f32 = 1.7;
const RAIL_HEIGHT: f32 = 1.05;
const RAIL_SECTION: f32 = 0.11;

/// The sun, low along the river so the gallery's two long sides are read against each other.
const SUN: [f64; 3] = [-26.0, 24.0, 16.0];
const SUN_IRRADIANCE: f64 = 3.1;
const SHADOW_EXTENT: f64 = 46.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.62;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 60.0;

/// The air over the river.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.20, 0.36, 0.63])
        .horizon([0.85, 0.82, 0.74])
        .ground([0.32, 0.31, 0.25])
        .density(0.0013)
        .scale_height(200.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the bank, three-quarters along the bridge.
pub fn rain_bridge() -> Value {
    scene(
        [19.0, 6.0, 14.0],
        [-2.0, 5.2, 0.0],
        FIELD_OF_VIEW,
        "rain_bridge",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("a 风雨桥 expands: two masses, their roofs, the deck and the piers");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a 风雨桥's own document must be clean, and it is not: {violations:?}"
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

    let timber = || surface([0.30, 0.20, 0.12], 0.0, 0.78, [0.0; 3]);
    let stone = || surface([0.44, 0.42, 0.38], 0.0, 0.84, [0.0; 3]);
    let tiles = || surface([0.27, 0.27, 0.29], 0.0, 0.58, [0.0; 3]);
    let deck = || surface([0.36, 0.29, 0.20], 0.0, 0.86, [0.0; 3]);
    let water = || surface([0.10, 0.14, 0.15], 0.0, 0.22, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("timber", timber as fn() -> Value),
        ("stone", stone),
        ("tiles", tiles),
        ("deck", deck),
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
        materials.push(mesh_material(&format!("river-{stage}"), shader, water()));
    }

    let placed = |stage: &str| {
        [
            ("walls", "timber"),
            ("roofs", "tiles"),
            ("flats", "deck"),
            ("timbers", "stone"),
            ("river", "river"),
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
    let river = ground::mesh(&building, GROUND_DIVISIONS, GROUND_EXTENT, |_, _| 0.0);

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
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("flats", "Triangles", point.clone(), flats),
                mesh("timbers", "Triangles", point.clone(), timbers),
                mesh("river", "Triangles", plain.point_schema(), river),
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

/// Returns the document: the deck, the two piers, the gallery, its roof, and the pavilion on top of it.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = SPAN * 0.5;
    let deep = WIDTH * 0.5;
    let gallery_base = DECK + GALLERY_PLATE;

    // **The gallery's mass**, which is what the roof is hung on: thin-walled, the length of the span.
    let mut gallery = Mass::new(
        [
            PlanNode::new([-half, -deep], gallery_base),
            PlanNode::new([half, -deep], gallery_base),
            PlanNode::new([half, deep], gallery_base),
            PlanNode::new([-half, deep], gallery_base),
        ]
        .to_vec(),
        gallery_base + GALLERY,
    );
    gallery.wall = GALLERY_WALL;
    gallery.slab = GALLERY_PLATE;
    let gallery_id = MassId(skeleton.masses.len());
    skeleton.masses.push(gallery);

    // **The gable runs the length of the span**, so the eave overhangs the two long sides.
    if let Some(roof) = derive::gable_roof(
        gallery_id,
        &skeleton.masses[gallery_id.0],
        OVERHANG,
        RISE,
        SINK,
        ROOF_THICKNESS,
    ) {
        skeleton.roofs.push(roof.with_covering(COVERING));
    }

    // **The pavilion stands on the gallery** through the storey relation: the middle of the span, proud of the
    // gallery on both sides, with its own hip. Its base is the gallery's top, which is the claim the relation
    // makes and `validate` reads.
    // **The pavilion's walls are flush with the gallery's and its roof is what overhangs.** A `Storey` claims
    // its plan is inside its host's, and a pavilion proud of the gallery is exactly what that claim denies:
    // `StoreyOverhangs { area: 31.8, shared: 25.2 }`, measured. What projects is the roof — `0.75` of overhang —
    // which is also how the type is built.
    let proud = deep;
    let mut pavilion = Mass::new(
        [
            PlanNode::new([-PAVILION_LONG * 0.5, -proud], gallery_base + GALLERY),
            PlanNode::new([PAVILION_LONG * 0.5, -proud], gallery_base + GALLERY),
            PlanNode::new([PAVILION_LONG * 0.5, proud], gallery_base + GALLERY),
            PlanNode::new([-PAVILION_LONG * 0.5, proud], gallery_base + GALLERY),
        ]
        .to_vec(),
        gallery_base + GALLERY + PAVILION_HIGH,
    );
    pavilion.wall = GALLERY_WALL;
    pavilion.slab = GALLERY_PLATE;
    let pavilion_id = ops::stack_mass(&mut skeleton, gallery_id, pavilion)
        .expect("the gallery is in the skeleton");
    if let Some(hip) = derive::hip_roof(
        pavilion_id,
        &skeleton.masses[pavilion_id.0],
        0.75,
        RISE + 0.35,
        0.10,
        ROOF_THICKNESS,
    ) {
        skeleton.roofs.push(hip.with_covering(COVERING));
    }

    // **The gallery is open down both sides**, which is what a 风雨桥 is: an arcade over the water with a bench
    // between its posts, and not a shed. The two long walls are cut into `BAYS` bays each and every bay is
    // opened between a `BAY_SILL` and the wall's own head — the openings machinery of `facade`, whose margins
    // are the wall's own thickness plus five centimetres, the rule every opening in this crate follows.
    for line in [-deep, deep] {
        let Some(edge) = facade::edge_on(&skeleton.masses[gallery_id.0], line, true) else {
            continue;
        };
        let face = building::grid::FaceRef::new(gallery_id, edge);
        if ops::split_face(
            &mut skeleton,
            face,
            &building::grid::Rhythm::Even { count: BAYS },
            &building::grid::Rhythm::Even { count: 1 },
        )
        .is_some()
        {
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

    // **The deck: a plate from bank to bank**, wider than the gallery and under it.
    skeleton.slabs.push(building::Slab::plate(
        vec![
            [-half - 1.2, -deep - 0.45],
            [half + 1.2, -deep - 0.45],
            [half + 1.2, deep + 0.45],
            [-half - 1.2, deep + 0.45],
        ],
        DECK,
        gallery_base,
    ));

    // **The piers: three stone posts to a set, standing in the water** and carrying the deck.
    for x in PIERS {
        for z in PIER_POSTS {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, -0.6, z] },
                building::Anchor::Free { at: [x, DECK, z] },
                [1.0, 0.0],
                PIER,
                PIER,
            ));
        }
    }

    // **The railing down both edges of the deck**: a post every `RAIL_SPACING` and a top rail over them. It
    // stands on the deck's own edge, outside the gallery's wall, which is where a bridge's rail is.
    let rail_at = deep + 0.32;
    let mut tops: Vec<[f32; 3]> = Vec::new();
    let mut x = -half;
    while x <= half + 0.01 {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, gallery_base, -rail_at],
            },
            building::Anchor::Free {
                at: [x, gallery_base + RAIL_HEIGHT, -rail_at],
            },
            [1.0, 0.0],
            RAIL_SECTION,
            RAIL_SECTION,
        ));
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, gallery_base, rail_at],
            },
            building::Anchor::Free {
                at: [x, gallery_base + RAIL_HEIGHT, rail_at],
            },
            [1.0, 0.0],
            RAIL_SECTION,
            RAIL_SECTION,
        ));
        tops.push([x, gallery_base + RAIL_HEIGHT, -rail_at]);
        tops.push([x, gallery_base + RAIL_HEIGHT, rail_at]);
        x += RAIL_SPACING;
    }
    // Two top rails, one to a side, from the first post to the last. **Their section is measured across the
    // span and not along it**: a member whose `across` is parallel to its own axis has no width, and the
    // expansion answers `MemberDegenerate` with the member's length — measured on these two, at 26.0.
    for z in [-rail_at, rail_at] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [-half, gallery_base + RAIL_HEIGHT, z],
            },
            building::Anchor::Free {
                at: [half, gallery_base + RAIL_HEIGHT, z],
            },
            [0.0, 1.0],
            RAIL_SECTION,
            RAIL_SECTION,
        ));
    }
    Building::new(skeleton)
}
