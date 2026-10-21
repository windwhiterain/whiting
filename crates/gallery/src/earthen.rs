//! A **土掌房**: the flat-roofed earth house of the Yunnan and Guizhou highlands — thick rammed-earth walls,
//! a roof that is a floor, and an outside stair up to it.
//!
//! # What this scene is for
//!
//! Every other building in this gallery is a **pitched** building: the roof sheds water away from the plan,
//! and the family's life happens under it. A 土掌房 is the region's other answer, and it is the one the crate
//! had never drawn: the walls are earth half a metre thick, the roof is a **slab of earth on joists**, and that
//! slab is the household's **晒场** — the drying floor for grain, chillies and maize. The roof is a room.
//!
//! # The three facts the type has
//!
//! - **The roof is flat and it is a floor.** `Slab::roof` with a `Plate` role, hosting the mass, and a
//!   **parapet** of four low members standing on its edge: without the parapet the roof is a ledge, and the
//!   whole point is that a person walks on it.
//! - **The stair is outside.** A run of stone treads climbs the downhill wall to the roof, because the roof is
//!   reached from the yard and not through the house.
//! - **The walls are earth and they are thick** — 0.55 m, four times a timber frame's wall. The thickness is
//!   what makes a small window a hole through a metre of wall in the frame.
//!
//! # What it does not draw
//!
//! - **no batter**: a rammed-earth wall leans in as it rises, and this crate's walls are vertical. The
//!   thickness carries the type's mass; the lean is a shape the mass model has no field for;
//! - **no 土 and no joists**: the roof slab is one plate, and the earth on it and the timber under it are
//!   materials the scene gives it rather than structure;
//! - **no 厦子**: the lean-to porch most of these houses carry on one side.

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

/// The house: nine metres by six and a half, two storeys of 2.8, and **earth walls 0.55 thick**.
const WIDTH: f32 = 9.0;
const DEPTH: f32 = 6.5;
const STOREY: f32 = 2.8;

/// How far the upper room stands back from the terrace's edge, which is what makes the lower roof a 晒场.
const SETBACK: f32 = 0.55;
const STOREYS: usize = 2;
const WALL: f32 = 0.55;
const PLATE: f32 = 0.40;

/// The roof slab: an earth floor `ROOF_THICK` deep on the mass's own top.
const ROOF_THICK: f32 = 0.35;

/// The parapet on the roof's edge: `PARAPET` thick and `PARAPET_HIGH` tall, standing on the slab.
const PARAPET: f32 = 0.28;
const PARAPET_HIGH: f32 = 0.55;

/// The 厦子: how far its roof reaches out from the door's wall, and the elevations of its eave and its crest.
const PORCH_DEPTH: f32 = 1.9;
const PORCH_LOW: f32 = 2.55;
const PORCH_HIGH: f32 = 3.15;
const POST: f32 = 0.18;

/// The outside stair: `STEPS` treads of `TREAD` going, up the gable end and onto the roof.
const STEPS: usize = 14;
/// The second flight, from the terrace to the room`s own roof: the storey above the 晒场 is half the first climb.
const SECOND_STEPS: usize = 8;
/// How far along the wall the second flight starts, so it clears the first flight`s own stringer.
const STEP_BACK: f32 = 1.10;
const TREAD: f32 = 0.32;
const TREAD_THICK: f32 = 0.14;

/// The sun, low and from the yard's own side, so the thick wall's reveal and the stair's shadow are both read.
const SUN: [f64; 3] = [-24.0, 22.0, 20.0];
const SUN_IRRADIANCE: f64 = 3.1;
const SHADOW_EXTENT: f64 = 30.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.60;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 60.0;

/// The air over the village.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.20, 0.36, 0.62])
        .horizon([0.84, 0.80, 0.72])
        .ground([0.38, 0.33, 0.25])
        .density(0.0012)
        .scale_height(200.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the yard's corner, the stair side toward the camera.
pub fn earthen_house() -> Value {
    scene(
        [13.5, 3.0, 15.0],
        [0.0, 3.2, 0.0],
        FIELD_OF_VIEW,
        "earthen_house",
    )
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("a 土掌房 expands: one mass, its roof slab, its parapet and its stair");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a 土掌房's own document must be clean, and it is not: {violations:?}"
    );

    let table = building::identity::Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // **The flat roof and the porch's shed are one mesh**, because they are one material: the earth the
    // 晒场 is made of. The scene had no `Roof` in it until the 厦子 arrived, so a roof element would have
    // been drawn by nothing.
    let roofs = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            ) | building::Element::Roof(_, _)
        )
    });
    let timbers = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
    });

    let earth = || surface([0.56, 0.44, 0.30], 0.0, 0.92, [0.0; 3]);
    let roof_earth = || surface([0.48, 0.38, 0.26], 0.0, 0.94, [0.0; 3]);
    let stone = || surface([0.42, 0.40, 0.36], 0.0, 0.80, [0.0; 3]);
    let ground_earth = || surface([0.34, 0.29, 0.21], 0.0, 0.96, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("wall", earth as fn() -> Value),
        ("roof", roof_earth),
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
        materials.push(mesh_material(
            &format!("ground-{stage}"),
            shader,
            ground_earth(),
        ));
    }

    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("roofs", "roof"),
            ("stone", "stone"),
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
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("stone", "Triangles", point.clone(), timbers),
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

/// Returns how wide the stair is, measured from the wall it climbs.
///
/// # Invariants
///
/// **One number, three readers** — the treads, their stringer and the parapet's landing gap — so a stair
/// narrowed in one place cannot leave a gap in the parapet that no longer fits it.
fn stair_width() -> f32 {
    1.1
}

/// Returns the plan direction perpendicular to a member's own axis, which is what its section is measured
/// across.
///
/// # Invariants
///
/// **A section laid along the axis has no width at all**, and the expansion answers `MemberDegenerate` with the
/// member's own length — a number that reads like a degenerate *length* and is not one. Every member here is
/// axis-aligned, but the parapet's four sides run two ways, so the direction is read from the pair rather than
/// written per side.
fn across_of(from: [f32; 2], to: [f32; 2]) -> [f32; 2] {
    let axis = [to[0] - from[0], to[1] - from[1]];
    let length = (axis[0] * axis[0] + axis[1] * axis[1]).sqrt();
    if length <= 1.0e-3 {
        return [1.0, 0.0];
    }
    [-axis[1] / length, axis[0] / length]
}

/// Returns the document: one earth box, its roof floor, the parapet round it and the stair up to it.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = WIDTH * 0.5;
    let deep = DEPTH * 0.5;
    let top = STOREYS as f32 * STOREY;
    // **Two masses and a terrace between them.** A 土掌房 is built in two steps and the step is the type: the lower wall
    // rises to the first floor's roof, that roof is a **晒场** left open round the room above, and the room above stands
    // back from the edge by `SETBACK` and is carried by the wall below. Each is a two-section mass, so **each takes its
    // own openings**: a mass drawn in several bands refuses a cut by name (`BandedMassCut`), and the scene that only
    // needed one band per storey never meets that refusal.
    let terrace = top - STOREY;
    let mut lower = Mass::new(
        [
            PlanNode::new([-half, -deep], 0.0),
            PlanNode::new([half, -deep], 0.0),
            PlanNode::new([half, deep], 0.0),
            PlanNode::new([-half, deep], 0.0),
        ]
        .to_vec(),
        terrace,
    );
    lower.wall = WALL;
    lower.slab = PLATE;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(lower);
    let mut upper = Mass::new(
        [
            PlanNode::new([-half + SETBACK, -deep + SETBACK], terrace),
            PlanNode::new([half - SETBACK, -deep + SETBACK], terrace),
            PlanNode::new([half - SETBACK, deep - SETBACK], terrace),
            PlanNode::new([-half + SETBACK, deep - SETBACK], terrace),
        ]
        .to_vec(),
        top,
    );
    upper.wall = WALL;
    upper.slab = PLATE;
    upper.attachment = Some(building::Attachment {
        host: id,
        join: building::Join::Storey,
    });
    let upper_id = MassId(skeleton.masses.len());
    skeleton.masses.push(upper);

    // **The lower roof is the terrace**, and the upper room stands on it: an earthen slab like the top one.
    skeleton.slabs.push(
        building::Slab::roof(
            vec![[-half, -deep], [half, -deep], [half, deep], [-half, deep]],
            terrace - ROOF_THICK,
            terrace,
        )
        .with_host(id),
    );

    // **The roof is a floor.** An earth slab on the upper mass's own top, hosting it, so the family's 晒场 is
    // drawn as the plate it is rather than as a pitched roof that the type does not have.
    skeleton.slabs.push(
        building::Slab::roof(
            vec![
                [-half + SETBACK, -deep + SETBACK],
                [half - SETBACK, -deep + SETBACK],
                [half - SETBACK, deep - SETBACK],
                [-half + SETBACK, deep - SETBACK],
            ],
            top - ROOF_THICK,
            top,
        )
        .with_host(upper_id),
    );

    // **The parapet: four low members standing on the slab's own edge**, one to a side. They are members and
    // not a second mass because a mass is a closed volume with its own floor, and a parapet has no floor —
    // it is a wall standing on another wall's top.
    let base = top + 0.02;
    // **The stair's own side is two segments and not one**, because a parapet with no gap is a parapet you
    // cannot step over: the landing's opening is the width of the stair, and it stands where the last tread
    // does.
    let landing = -deep + TREAD * STEPS as f32;
    let gap = landing - stair_width() * 0.5..landing + stair_width() * 0.5;
    let mut sections = vec![
        ([-half, -deep], [half, -deep]),
        ([half, deep], [-half, deep]),
        ([-half, deep], [-half, -deep]),
        // The stair's side, in two pieces: from the low end to the gap, and from the gap to the high end.
        ([half, -deep], [half, gap.start]),
        ([half, gap.end], [half, deep]),
    ];
    sections.retain(|(from, to)| (to[1] - from[1]).abs() > 0.05 || (to[0] - from[0]).abs() > 0.05);
    for (from, to) in sections {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [from[0], base, from[1]],
            },
            building::Anchor::Free {
                at: [to[0], base, to[1]],
            },
            across_of(from, to),
            PARAPET,
            PARAPET_HIGH,
        ));
    }

    // **The treads climb along the wall, they do not stick out of it.** A tread runs across the stair's width
    // — 1.1 m out from the wall — and each one steps along the wall's own face. The first version ran every
    // tread through the whole depth of the house and climbed in `x`, which drew a fan of fins rather than a
    // stair, and the frame said so before any validator could.
    // **The stair climbs to the terrace and not to the roof**, because the terrace is the first floor: a 土掌房`s
    // stair reaches the 晒场 and goes on to the room above by a second flight this document does not draw.
    let terrace_level = top - STOREY;
    let rise = (terrace_level - ROOF_THICK) / STEPS as f32;
    let stair_width = stair_width();
    for step in 0..STEPS {
        let y = rise * (step + 1) as f32;
        let z = -deep + TREAD * step as f32 + TREAD * 0.5;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [half + 0.05, y, z],
            },
            building::Anchor::Free {
                at: [half + stair_width, y, z],
            },
            [0.0, 1.0],
            TREAD,
            TREAD_THICK,
        ));
    }
    // **The second flight**: from the terrace to the top roof, **on the same flank as the first** — one straight run up the
    // Its treads run across the terrace's own walkway, which is exactly `SETBACK` wide, and it lands at the gap the top
    // roof's parapet has always had — the gap was made for this stair and has waited for it.
    let upper_rise = (top - terrace_level - ROOF_THICK) / SECOND_STEPS as f32;
    let upper_width = SETBACK - 0.08;
    for step in 0..SECOND_STEPS {
        let y = terrace_level + upper_rise * (step + 1) as f32;
        let z = -deep + STEP_BACK + TREAD * step as f32 + TREAD * 0.5;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [half - SETBACK + 0.04, y, z],
            },
            building::Anchor::Free {
                at: [half - SETBACK + 0.04 + upper_width, y, z],
            },
            [0.0, 1.0],
            TREAD,
            TREAD_THICK,
        ));
    }

    // **The stringer**, one sloped beam under the treads' outer edge: a run of floating treads reads as steps
    // only because they are evenly spaced, and a stair that carries nothing is a ladder lying down. The beam
    // starts under the first tread and ends under the last, so its own slope *is* the stair's.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [
                half + stair_width - TREAD_THICK,
                rise - TREAD_THICK,
                -deep + TREAD * 0.5,
            ],
        },
        building::Anchor::Free {
            at: [
                half + stair_width - TREAD_THICK,
                top - ROOF_THICK - TREAD_THICK,
                -deep + TREAD * (STEPS as f32 - 0.5),
            ],
        },
        [1.0, 0.0],
        TREAD_THICK,
        0.30,
    ));

    // **The 厦子, the lean-to every one of these houses carries over its door.** It sheds away from the wall
    // and stands on two posts at the yard, because a porch that hangs off a wall with nothing under it is a
    // canopy, and the type's porch is where the year's work is done out of the rain.
    let porch = deep + PORCH_DEPTH;
    let plan = [
        [-half + 0.6, porch],
        [half - 0.6, porch],
        [half - 0.6, deep],
        [-half + 0.6, deep],
    ];
    if let Some(shed) = derive::lean_to(id, plan, PORCH_LOW, PORCH_HIGH, 0.16) {
        skeleton.roofs.push(shed);
    }
    // **Two posts at the eave's own ends**, each standing on the ground under it: the porch's own bearers,
    // and the reason a 厦子 reads as a place rather than as a lip of roof.
    for x in [-half + 0.6, half - 0.6] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, 0.0, porch - 0.12],
            },
            building::Anchor::Free {
                at: [x, PORCH_LOW - 0.16, porch - 0.12],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }
    // **A door and two small windows in the front**, which for a wall this thick are holes a metre deep: the
    // reveal is the wall, and that is what the type's windows look like from outside.
    // **The lower floor is opened onto its porch and the upper room onto its terrace**, each in its own mass: a
    // 竹楼 opens onto its veranda, a 土掌房 onto its 厦子 and its 晒场, and the two storeys are two masses here.
    let _ = facade::open_court_face(&mut skeleton, id, deep, true, Front::RANGE);
    let _ = facade::open_court_face(
        &mut skeleton,
        upper_id,
        deep - SETBACK,
        true,
        Front {
            storeys: 1,
            bays: 3,
            door: true,
        },
    );
    let _ = Bar::NONE;
    Building::new(skeleton)
}
