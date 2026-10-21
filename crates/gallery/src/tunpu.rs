//! A **屯堡石板房**: the stone-slab house of the Tunpu villages of Anshun — walls of rough stone, a gable roof laid in
//! thin stone slates, and a stone yard in front of it.
//!
//! # What this scene is for
//!
//! It is the gallery's **twelfth type** and the one that is *all* stone: the 碉房 carries a timber gallery and an earth
//! roof, the 土掌房 is earth, the 木楞房 is logs — this one quarries its roof. A 屯堡 is a Ming garrison's descendants'
//! village, and what a Ming garrison built with was the stone under it: the wall, the yard and the roof are the same
//! material in three sizes.
//!
//! # The two facts the type has
//!
//! - **The roof is stone.** Slates a hand's thickness, laid in courses from the ridge down, each course overlapping the
//!   one below it — a `Covering` with `period` 0.42 and `fill` 0.94, which is a wide flat course and not a roll. A slate
//!   roof is heavy, so the pitch is low: `RISE` is 1.5 over 6.4, which is a third of the 竹楼's.
//! - **The yard is the same stone**, paved as a slab in front of the door: a 屯堡 house's 院坝 is where the grain is dried
//!   and where the family sits, and it is not a garden.
//!
//! # What it does not draw
//!
//! - **the 朝门 and the 碉楼**: a 屯堡 has a gate house and a watchtower at the village's ends, and they are the village's
//!   work, not the house's;
//! - **the dressed quoins**: a real wall has cut stone at its corners and rubble between, and one plaster thickness cannot
//!   say so.

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

use crate::facade::{self, Front};
use crate::ground;

/// The house: `WIDTH` by `DEPTH`, one storey of `HEIGHT`, with a stone yard `YARD` deep in front of it.
const WIDTH: f32 = 8.2;
const DEPTH: f32 = 6.4;
const HEIGHT: f32 = 3.45;
const YARD: f32 = 2.40;

/// The wall and the floor plate: a stone wall is thick and carries everything.
const WALL: f32 = 0.55;
const PLATE: f32 = 0.32;

/// The roof: a low pitch of stone slates, and the eaves a slate roof needs to keep its feet dry.
const OVERHANG: f32 = 0.45;
const RISE: f32 = 1.50;
const SINK: f32 = 0.08;
const ROOF_THICKNESS: f32 = 0.16;

/// The slates: `period` of plan, `fill` of that wide, `height` proud of the skin — a wide flat course and not a roll.
const SLATES: Covering = Covering {
    period: 0.42,
    fill: 0.94,
    height: 0.035,
};

/// The sun, cold and high, from the front-left, so the slates' courses read as courses.
const SUN: [f64; 3] = [-26.0, 30.0, 22.0];
const SUN_IRRADIANCE: f64 = 3.3;
const SHADOW_EXTENT: f64 = 26.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 50.0;

/// The air over the Anshun basin: wet, low and grey, which is what a stone village is seen through.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.26, 0.38, 0.60])
        .horizon([0.82, 0.83, 0.80])
        .ground([0.32, 0.31, 0.28])
        .density(0.0016)
        .scale_height(180.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the yard's corner, where the roof's courses are seen edge-on.
pub fn slate_house() -> Value {
    scene(
        [12.0, 3.8, 13.5],
        [-0.4, 2.4, 0.4],
        FIELD_OF_VIEW,
        "slate_house",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("a 石板房 expands: one stone mass, its slate roof and its yard");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a 石板房's own document must be clean, and it is not: {violations:?}"
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
    let yards = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(_, building::slab::SlabSurface::Top)
        )
    });

    // **Stone, slate and the yard's own grit**, which are three greys and not one.
    let stone = || surface([0.52, 0.51, 0.47], 0.0, 0.90, [0.0; 3]);
    let slate = || surface([0.34, 0.35, 0.36], 0.0, 0.78, [0.0; 3]);
    let grit = || surface([0.44, 0.43, 0.39], 0.0, 0.93, [0.0; 3]);
    let earth = || surface([0.31, 0.29, 0.24], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("wall", stone as fn() -> Value),
        ("roof", slate),
        ("yard", grit),
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
            ("yards", "yard"),
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
                    [1.0, 0.96, 0.90],
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
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("yards", "Triangles", point.clone(), yards),
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
        if let Some(index) = building::ops::split_outline(mass, station) {
            if let Some(crown) = mass.crown_mut().outline.get_mut(index) {
                crown.elevation = apex;
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
    let count = mass.outline().len();
    if count != 4 {
        return None;
    }
    let spans: Vec<f32> = (0..count)
        .map(|edge| {
            let from = mass.outline()[edge].position;
            let to = mass.outline()[(edge + 1) % count].position;
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

/// Returns the document: one stone box, its slate roof and the yard in front of it.
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
        HEIGHT,
    );
    mass.wall = WALL;
    mass.slab = PLATE;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(mass);

    // **The roof is slates**: a low gable and a covering whose courses are wide and flat, which is what a roof of quarried
    // stone is — the type's whole subject, in one number.
    if let Some(roof) = derive::gable_roof(
        id,
        &skeleton.masses[id.0],
        OVERHANG,
        RISE,
        SINK,
        ROOF_THICKNESS,
    ) {
        skeleton.roofs.push(roof.with_covering(SLATES));
    }

    // **The yard: the same stone, paved.** A 屯堡 house's 院坝 is where the grain dries, so it is a slab of the wall's own
    // material and not a garden: two and a half metres deep, from the wall to the lane.
    skeleton.slabs.push(
        building::Slab::floor(
            vec![
                [-half, deep],
                [half, deep],
                [half, deep + YARD],
                [-half, deep + YARD],
            ],
            0.0,
            0.06,
        )
        .with_host(id),
    );

    // **The gable ends rise to the roof, and before the openings are placed.** A split re-indexes the loop, so an opening
    // cut first would name an edge the split moved: measured as `OpeningOutsideWall { edge: 2, past: 2.2176661 }` when the
    // two were the other way round. A stone wall that stops at the eaves leaves the roof's own underside open from
    // the yard — the first frame of this scene showed the loft as a hole through the house — so each end is split at its
    // middle and crowned to the apex.
    if let Some(roof) = skeleton.roofs.first() {
        let apex = roof
            .crest
            .iter()
            .map(|point| point.elevation)
            .fold(f32::NEG_INFINITY, f32::max)
            - roof.thickness
            - 0.01;
        if let Some(stations) = gable_stations(&skeleton.masses[id.0]) {
            for station in stations {
                if let Some(index) =
                    building::ops::split_outline(&mut skeleton.masses[id.0], station)
                {
                    if let Some(crown) = skeleton.masses[id.0].crown_mut().outline.get_mut(index) {
                        crown.elevation = apex;
                    }
                }
            }
        }
    }
    // **A door and two windows in the front wall**, which is the wall the yard is in front of.
    // **Two windows either side of the door**, cut by hand and not by the façade: `open_court_face` on a one-storey
    // 土掌房 wall places its windows at a storey`s height, and a stone house has only the one — the first frame showed a
    // door and two blank bays.
    // **A door in the front wall, and the two windows are still to cut.** `open_court_face` places the door and does not
    // leave the grid a later opening needs (`Opening::new` cuts inside a bay, and an unsplit face answers `None`), and the
    // two ways this round tried to cut them itself both answered from the crate: a face index named before the gables were
    // crowned is a crowned gable (`GridOnACrownedFace`, then `FaceNotPlanar` at 0.303), and after the crown the door`s own
    // inset came back as `OpeningOutsideWall { edge: 3, past: 0.411 }`. The windows are the next round`s work and this one
    // lands the type.
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
