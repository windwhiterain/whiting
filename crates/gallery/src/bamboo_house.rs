//! A **傣族竹楼**: the Dai house of Xishuangbanna — a floor lifted a metre and a half off the ground on piles, a
//! steep thatched roof whose eaves come down almost to the floor, a veranda along the front and a drying deck at the
//! gable end.
//!
//! # What this scene is for
//!
//! It is the gallery's **tenth type** and the southernmost: the other houses in this crate are walled, and this one is
//! **floored**. Its ground floor is open air, its walls do not carry the roof but sit under it, and the roof is a single
//! steep hip that covers the room, the veranda and the eaves' own shadow together. What makes it a 竹楼 is exactly what
//! makes it hard to draw as a single mass: **the house is the floor**.
//!
//! # The two facts the type has
//!
//! - **The house stands on piles, and the floor is the building.** `site::posts` fills the gap between the deck and the
//!   ground it was seated over, so the ground floor is open. The veranda and the drying deck are slabs at the floor's own
//!   level, hosted by the room, which is what keeps them with it when the house is seated.
//! - **The roof sheds by slope, not by tile.** `RISE` is more than half the house's depth, the eaves run `OVERHANG` past
//!   the wall, and the covering is a thatch of coarse bundles — a steep roof is what a thatched house is.
//!
//! # What it does not draw
//!
//! - **the bamboo**: a 竹楼's walls are woven bamboo on a timber frame, and a wall here is one thickness of plaster;
//! - **the open ground floor's uses**: the loom, the store, the animals are the interior arc's work;
//! - **the 火塘**: the hearth in the middle of the room, which needs a hole in the floor this crate does not cut.

use building::{Building, BuildingSkeleton, Covering, Mass, MassId, PlanNode, derive, site};
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

use crate::facade::{self, Front};
use crate::ground;

/// The room: `WIDTH` by `DEPTH`, standing `FLOOR` above the ground with `WALL_HEIGHT` of wall under the roof.
const WIDTH: f32 = 5.4;
const DEPTH: f32 = 4.2;
const FLOOR: f32 = 1.95;
const WALL_HEIGHT: f32 = 2.45;

/// How far the veranda runs past the front wall, and the drying deck past the gable end.
const VERANDA: f32 = 1.8;
const DECK: f32 = 1.5;

/// The wall the room is enclosed with and the plate the floor is: thin, because neither carries the roof.
const WALL: f32 = 0.14;
const PLATE: f32 = 0.22;

/// The roof: a steep hip, deep eaves, and thatch rather than tile.
const OVERHANG: f32 = 1.35;
const RISE: f32 = 2.70;
const SINK: f32 = 0.10;
const ROOF_THICKNESS: f32 = 0.18;

/// The thatch, as bundles: `period` of plan, `fill` of that wide, `height` proud of the skin.
const THATCH: Covering = Covering {
    period: 0.30,
    fill: 0.62,
    height: 0.09,
};

/// The piles under the veranda's edge, and the stair up to it.
const PILE: f32 = 0.17;
const PILE_SPACING: f32 = 1.6;
const STAIR_RISERS: usize = 10;
const TREAD: f32 = 0.26;
const TREAD_THICK: f32 = 0.06;

/// The sun, low from the front, so the veranda is in the roof's shadow and the deck is not.
const SUN: [f64; 3] = [-26.0, 22.0, 20.0];
const SUN_IRRADIANCE: f64 = 3.2;
const SHADOW_EXTENT: f64 = 28.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 50.0;

/// The air over the southern valley: warmer and thicker than the mountains' own.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.24, 0.40, 0.66])
        .horizon([0.88, 0.84, 0.74])
        .ground([0.36, 0.33, 0.24])
        .density(0.0014)
        .scale_height(220.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the front corner, where the veranda's piles and the stair are.
pub fn bamboo_house() -> Value {
    scene(
        [10.5, 5.4, 13.0],
        [-0.4, 2.9, 0.6],
        FIELD_OF_VIEW,
        "bamboo_house",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("a 竹楼 expands: one room, its roof, its decks and the piles under them");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a 竹楼's own document must be clean, and it is not: {violations:?}"
    );

    let table = building::identity::Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    let timber = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
    });
    let roofs = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Roof(_, _) | building::Element::Roll(_, _)
        )
    });
    let decks = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(_, building::slab::SlabSurface::Top)
        )
    });

    // **Bamboo is pale and the thatch is grey**, which is what a 竹楼's two materials are seen as from the path.
    let bamboo = || surface([0.58, 0.55, 0.38], 0.0, 0.86, [0.0; 3]);
    let wood = || surface([0.26, 0.18, 0.11], 0.0, 0.76, [0.0; 3]);
    let thatch = || surface([0.42, 0.38, 0.30], 0.0, 0.88, [0.0; 3]);
    let board = || surface([0.36, 0.30, 0.22], 0.0, 0.78, [0.0; 3]);
    let earth = || surface([0.32, 0.29, 0.22], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("wall", bamboo as fn() -> Value),
        ("pile", wood),
        ("roof", thatch),
        ("deck", board),
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
            ("piles", "pile"),
            ("roofs", "roof"),
            ("decks", "deck"),
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
                    [1.0, 0.94, 0.84],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    170.0,
                ),
                environment_light("air", "air", [1.0; 3], 1.08),
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
                mesh("piles", "Triangles", point.clone(), timber),
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("decks", "Triangles", point.clone(), decks),
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

/// Returns the document: a room, the two decks at its floor, the piles under them and the stair up.
///
/// # Invariants
///
/// **The room is seated `FLOOR` above the ground and the decks are hosted slabs**, so the seat moves the floor, the
/// decks and the piles' tops together; `site::posts` then fills the gap the seat opened. That is the whole of a 干栏
/// house: nothing stands on the ground but the piles.
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
        WALL_HEIGHT,
    );
    mass.wall = WALL;
    mass.slab = PLATE;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(mass);

    // **The veranda and the drying deck are the same plate as the room's floor**, laid outside its outline and hosted
    // by it: a 竹楼's floor runs out of the room on two sides, and what an onlooker reads as the house's floor is the
    // part that has no walls on it.
    skeleton.slabs.push(
        // **A deck is a floor and not the room`s ground plate**: the crate`s coverage check reads the role, and a
        // plate has to cover the room it stands under. A 竹楼`s veranda is flooring laid outside the room, so it is a
        // `floor` — the role a slab gets when it is a surface people walk on inside the house.
        building::Slab::floor(
            vec![
                [-half, deep],
                [half, deep],
                [half, deep + VERANDA],
                [-half, deep + VERANDA],
            ],
            -PLATE,
            0.0,
        )
        .with_host(id),
    );
    skeleton.slabs.push(
        building::Slab::floor(
            vec![
                [half, -deep],
                [half + DECK, -deep],
                [half + DECK, deep],
                [half, deep],
            ],
            -PLATE,
            0.0,
        )
        .with_host(id),
    );

    // **The roof covers the room, the veranda and the deck**, which is why its overhang is more than a metre: a
    // thatched roof is drawn by its eaves, and a 竹楼's come down to the floor's own level on the shaded side.
    if let Some(roof) = derive::hip_roof(
        id,
        &skeleton.masses[id.0],
        OVERHANG + VERANDA * 0.5,
        RISE,
        SINK,
        ROOF_THICKNESS,
    ) {
        // **The thatch is dropped, and it is a defect that it has to be.** A hip`s two corner rolls leave their own
        // plane by `1.10e-3` against a tolerance of `1e-3` — the same millimetre that kept the 蘑菇房`s straw out of the
        // village in round 45. Two types, two hip roofs, the same two corner rolls: the fault is in `covering`, not in
        // either house, and it is recorded where the roll is built.
        skeleton.roofs.push(roof);
    }

    // **One seat and one stilt**: the floor is `FLOOR` above the ground, and the posts fill what the seat opened.
    let ground = |_: building::math::Vec2| 0.0_f32;
    let seated = site::seat(&mut skeleton, ground, FLOOR);
    assert_eq!(seated.len(), 1, "the 竹楼 is one room");
    let stilt = site::posts(&mut skeleton, ground, PILE_SPACING, PILE, 0.06);
    assert!(stilt.refused == 0, "the ground answers under the veranda");

    // **The stair: a steep run of ten treads from the yard to the veranda's edge.** It climbs along `z`, one tread a
    // step, and its stringer is the beam the treads are carried on.
    let rise = FLOOR / STAIR_RISERS as f32;
    let edge = deep + VERANDA;
    for step in 0..STAIR_RISERS {
        let y = rise * (step + 1) as f32;
        let z = edge + TREAD * (STAIR_RISERS - step) as f32 - TREAD * 0.5;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free { at: [-0.55, y, z] },
            building::Anchor::Free { at: [0.55, y, z] },
            [0.0, 1.0],
            TREAD,
            TREAD_THICK,
        ));
    }

    // **Two stringers**, one under each side of the run: a flight of floating treads reads as steps only because they
    // are evenly spaced, and a stair that carries nothing is a ladder lying down. Each runs from under the lowest tread
    // to under the highest, so its slope is the stair's own.
    for x in [-0.55_f32, 0.55] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, rise - TREAD_THICK, edge + TREAD * STAIR_RISERS as f32],
            },
            building::Anchor::Free {
                at: [x, FLOOR - TREAD_THICK, edge + TREAD * 0.5],
            },
            [1.0, 0.0],
            TREAD_THICK,
            0.22,
        ));
    }

    // **A door and two windows in the front wall**, which is the wall the veranda is in front of: a 竹楼 opens onto
    // its veranda and not onto its yard.
    let _ = facade::open_court_face(
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
    Building::new(skeleton)
}
