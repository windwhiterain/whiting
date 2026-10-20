//! A hall: one building derived from `derive::HALL`, and a facade the **style** decides.
//!
//! This is the style slice's proof scene. Nothing here chooses a wall thickness, a pitch or a window's
//! size: `HALL` states its relations — a wall is a twenty-sixth of the smaller span with a floor and a
//! ceiling, a rise is 0.85 of the half-span, a bay is three metres, a window is 0.52 of its bay with its
//! own sill and height — and `HALL::starter`, `HALL::bays` and `HALL::glazing` say what follows for this
//! footprint. What the file adds is the three things a frame needs that a building is not: a ground, a
//! sun, and a camera.
//!
//! # What the frame shows
//!
//! A tall single-storey hall with **three bays a face and one window in each**, all of them the style's
//! own glazing: the same relations that set the walls set the holes, which is what makes a style more
//! than a table of numbers.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, HALL};
use building::grid::{FaceRef, Rhythm};
use building::identity::Identity;
use building::openings::{Inset, Opening};
use building::ops;
use building::{Building, BuildingSkeleton, Mass, PlanNode};

use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, sky, surface,
};
use document::shaders::{Projection, post_shader};

/// What the hall is: `HALL`'s relations at a 10 × 16 footprint, one tall storey.
///
/// # Invariants
///
/// **Every number in the document comes from the style**, which is the point of the scene: the wall, the
/// slab, the eave, the seat, the rise and the roof's thickness are `HALL::starter`'s answers, and the
/// windows are `HALL::glazing`'s. The footprint and the storey count are the scene's, because those are
/// what an author asks for rather than what a style decides.
///
/// **The bays are one count round the building.** A hall's rhythm is uniform, so the count `HALL::bays`
/// gives for the shorter span is used on every face; a scene that measured each face would be choosing a
/// second rhythm the style did not name.
pub fn document_at(width: f32, depth: f32, wing_depth: f32) -> Building {
    // **An L is a host and an attached wing**, not one concave outline: `attach_mass` writes the attachment
    // that authorizes the overlap, and each arm gets its own gable, which is how an L is roofed in practice.
    let plan = HALL.starter(width, depth, 1);
    let mut skeleton = BuildingSkeleton::default();
    let host = ops::add_mass(
        &mut skeleton,
        Mass::new(
            rectangle(-width * 0.5, -depth * 0.5, width * 0.5, depth * 0.5),
            plan.height,
        )
        .with_wall(plan.wall)
        .with_slab(plan.slab),
    );
    let wing = ops::attach_mass(
        &mut skeleton,
        host,
        0.2,
        Mass::new(
            rectangle(
                width * 0.5 - plan.wall,
                -wing_depth * 0.5,
                width * 0.5 + width,
                wing_depth * 0.5,
            ),
            plan.height,
        )
        .with_wall(plan.wall)
        .with_slab(plan.slab),
    )
    .expect("the host outline takes a station off its corners");
    // **A plate for each arm**, over its own footprint, so both wall rings overlap their floors.
    for (arm, x0, z0, x1, z1) in [
        (host, -width * 0.5, -depth * 0.5, width * 0.5, depth * 0.5),
        (
            wing,
            width * 0.5 - plan.wall,
            -wing_depth * 0.5,
            width * 0.5 + width,
            wing_depth * 0.5,
        ),
    ] {
        skeleton.slabs.push(
            building::slab::Slab::plate(
                rectangle(x0, z0, x1, z1)
                    .into_iter()
                    .map(|node| node.position)
                    .collect(),
                -plan.slab,
                0.0,
            )
            .with_host(arm),
        );
    }
    for arm in [host, wing] {
        if let Some(roof) = derive::gable_roof(
            arm,
            &skeleton.masses[arm.0],
            plan.overhang,
            plan.rise,
            plan.sink,
            plan.roof_thickness,
        ) {
            ops::add_roof(&mut skeleton, roof);
        }
    }
    // **Windows only on the faces that meet the weather.** The host east face is where the wing attaches and the
    // wing west face is the shared leaf, so neither takes a window: a hole there would open into the other arm.
    // Every other face takes HALL.bays bays and one window a bay, sized by the style own glazing.
    let bays = HALL.bays(width);
    let (left, right, sill, height) = HALL.glazing(width / bays as f32, plan.height);
    for (arm, faces) in [(host, [0usize, 2, 3]), (wing, [0, 1, 2])] {
        for face in faces {
            let front = FaceRef::new(arm, face);
            ops::split_face(
                &mut skeleton,
                front,
                &Rhythm::Even { count: bays },
                &Rhythm::Even { count: 1 },
            );
            for column in 0..bays {
                let _ = ops::add_opening(
                    &mut skeleton,
                    Opening::new(front, 0, column, Inset::new(left, right, sill, height)),
                );
            }
        }
    }
    Building::new(skeleton)
}

/// Returns a rectangle outline.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<PlanNode> {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, 0.0))
        .collect()
}

/// Returns the hall's document at the size the scene draws: 10 × 16, one tall storey.
pub fn document() -> Building {
    document_at(6.0, 6.0, 4.0)
}

/// Where the sun stands. The hall's own middle is the origin, so this is a direction as much as a place:
/// high and to the left, so the near wall is lit and the far one is not. High rather than grazing on
/// purpose — a low sun rakes the wall and the ground at the angle a shadow map is least able to resolve,
/// and the frame this scene exists to produce is not the place to discover that.
const SUN: [f64; 3] = [-26.0, 34.0, 24.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// The image this scene is drawn into, in pixels.
///
/// # Why this frame is this shape
///
/// **A wing is an L, and an L seen along its length is the widest silhouette in the gallery.** The
/// measured number says by how much: at the square's own lens the building took **8.25 %** of the
/// frame, less than any other scene here, and it was not too small — it was lost. Read every id the
/// frame carries at `FRAC_PI_3 * 1.60` and the silhouette stands `x 405..=628` by `y 455..=570` of
/// the square — **f_w 0.2180 by f_h 0.1124**, a world aspect of **1.940 to one**.
///
/// So the frame takes those proportions, **1.93 to one** at 1424 by 736: 1,048,064 pixels, which is
/// 512 under the square's.
const FRAME_WIDTH: i64 = 1424;
const FRAME_HEIGHT: i64 = 736;

/// The full **vertical** angle this wing is drawn through, in radians.
///
/// # The arithmetic
///
/// ```text
/// tan(new / 2) = tan(1.60 * PI/3 / 2) * 0.1124 / 0.89
///              = 1.324696 * 0.126292 = 0.167301
/// new = 2 * atan(0.167301) = 0.27871 rad = FRAC_PI_3 * 0.2661
/// ```
///
/// **The longest lens in the gallery by a wide margin**, and it is the arithmetic that asks for it:
/// the square lens put the L at 11 % of the frame's height, so making it 89 % is a factor of eight.
/// **The eye and the target did not move** — the wing's own plate is seen through the host's windows,
/// and a plate is in frame because of where the eye stands rather than because of how long the lens
/// is, so the lens here magnifies what was already there and cannot lose it.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.2661;

/// Builds the hall scene.
/// Builds the hall scene at the size the scene draws: 10 × 16, one tall storey.
pub fn wing() -> Value {
    wing_at(6.0, 6.0, 4.0)
}

/// Builds the hall scene for a footprint and a storey count.
pub fn wing_at(width: f32, depth: f32, wing_depth: f32) -> Value {
    let building = document_at(width, depth, wing_depth);
    // The ridge is the number the frame has to fit: the wall top plus the style's own rise.
    let plan = HALL.starter(width, depth, 1);
    let ridge = f64::from(plan.height + plan.rise);
    let extent = f64::from(width.max(depth)) * 1.3;
    let built = building
        .build()
        .expect("the starter house expands: its outline is a quad and its roof's rows are paired");

    // **Every mesh the frame draws carries its elements' ids**, so a pixel of this house can be asked what it
    // is: the walls name their edges, the hip names its skins and its fascia.
    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // One plate an arm, over that arm's own footprint: two solids the document carries at `document_at`,
    // drawn here rather than left in the table where nothing can see them.
    let floors = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Slab(_, _))
    });
    let roof = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Roof(_, _))
    });

    // A plaster wall, a tiled roof and dry ground. The three differ in tint and roughness and in
    // nothing else: what a surface *does* is the shader's business and this scene adds no body to
    // it, so the materials are the standard ones with a colour apiece.
    let plaster = || surface([0.74, 0.70, 0.61], 0.0, 0.85, [0.0; 3]);
    let tiles = || surface([0.33, 0.14, 0.10], 0.0, 0.62, [0.0; 3]);
    let ground = || surface([0.23, 0.21, 0.15], 0.0, 0.95, [0.0; 3]);
    let floor = || surface([0.34, 0.26, 0.18], 0.0, 0.80, [0.0; 3]);

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
    // The ground is not an element of the house, and it still writes the output — with the reserved zero.
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, ground()));
    }

    // Every mesh is authored in world coordinates, so its three instances carry no transform: a
    // house placed by a translation here would be a second copy of where the house already is.
    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("roof", "roof"),
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

    // **Two point schemas**: the walls and the hip carry `authored_id`; the ground carries none and writes
    // the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();
    // **One air, handed to two renderers**: the map a world reads and the light a surface sees. Without the
    // light the shading's sky term is zero, which is why a sun-only scene's shadowed side is black.
    let (sky_image, sky_light) = sky(&atmosphere());

    Value::structure([
        ("name".to_owned(), Value::String("wing".to_owned())),
        // The one field here that is about the picture rather than what is in it: an L is a long
        // building and a long building wants a long frame. A scene that names no frame is drawn into
        // the pipeline's own square, so this is a decision rather than a default.
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            // A three-quarter view from the near corner, high enough that the roof reads as a
            // volume rather than as a silhouette: the eave line and the ridge are both visible,
            // which is what says the roof has a soffit and a thickness. **The eye and the target did
            // not move**; the lens is the free variable, and it is free because the frame is now the
            // silhouette's own shape rather than the pipeline's square.
            Value::List(vec![camera(
                [
                    f64::from(width) * 2.2 + 4.0,
                    ridge * 1.2,
                    f64::from(depth) * 2.4 + 10.0,
                ],
                [f64::from(width) * 0.5, ridge * 0.4, 0.0],
                FIELD_OF_VIEW,
            )]),
        ),
        (
            "lights".to_owned(),
            // A `power` rather than a brightness, derived from the irradiance wanted at the
            // house's own distance from the light: see `.agents/proposals/physical-light-units.md`.
            Value::List(vec![
                light_with_radius(
                    "sun",
                    SUN,
                    [1.0, 0.87, 0.70],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    extent,
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
        // The one image this scene writes beside the frame: which element each pixel shows.
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("roof", "Triangles", point.clone(), roof),
                mesh("floors", "Triangles", point, floors),
                mesh(
                    "ground",
                    "Triangles",
                    plain.point_schema(),
                    crate::ground::mesh(&building, 140, 420.0, |_, _| 0.0),
                ),
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

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The air over the wing: a clear afternoon, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's
/// bloom — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads
/// its sky term from. A wing whose shadows are black is a wing with no air, not a wing with the wrong
/// colours. The sun's bearing is the one the sky law carried, and it is not the sun light's own: the
/// glow is where the sun *is drawn*, and the light is where the shadows fall.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.16, 0.27, 0.50])
        .horizon([0.88, 0.74, 0.56])
        .ground([0.34, 0.30, 0.24])
        .sun([-22.0, 30.0, 20.0], [1.0, 0.86, 0.62], 8.0, 0.35)
        .density(0.0011)
        .scale_height(200.0)
}

/// What the post stage does to the frame.
///
/// # Invariants
///
/// **The air goes on before the curve and only over the geometry.** The sky is already the far
/// distance; what has to be crossed with air is everything between the eye and it, and the distance
/// is the post stage's own recovery of where the pixel is rather than a number assembled out of a
/// near and a far plane.
const GRADE: &str = r#"
    // **The sky is read out of the scene's own environment map**, the same air the bounce reads, so the
    // background and the light cannot be two skies.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * 0.006) * 0.85, 0.0, 0.92);
        // The air in front of a surface takes the colour of the sky *behind* it, so the ray's
        // height is held to the horizon rather than allowed to point into the ground.
        color = mix(
            color,
            environment_sky(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)),
            air,
        );
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.22), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.45, 0.0, 1.0);
"#;
