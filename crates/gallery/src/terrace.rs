//! A terrace: three units of one style side by side, sharing their party walls.
//!
//! This is the slice that asked the vocabulary a question before it drew anything. A terrace's party wall is
//! **two leaves whose outer faces meet** — 0.15 m each, so the wall reads as one 0.3 m party wall — and
//! `examples/terrace_probe.rs` measured that the crate already accepts it: two masses flush at a shared face
//! report **zero violations**, and each stays watertight and manifold on its own.
//!
//! # What the document is
//!
//! Three units from `derive::COTTAGE`'s relations at a 6 × 8 footprint, each its own closed solid, each with its
//! own gable roof running along the row. **The wall is thinner than the style's external wall**: a party leaf is
//! 0.15 m because a party wall is a thin wall, and the style's `1/19` relation describes a wall that faces the
//! weather. That is the one number here the style does not decide, and it is the one the terrace's own type
//! decides.
//!
//! # What the frame shows
//!
//! The row, the party walls between the units, three bays of windows a front, and the roofs' independent ridges —
//! which is what a terrace looks like from the street — and **all three units' floors**, seen through their own
//! windows. The camera stands square in front of the row rather than off its near corner for that reason alone: a
//! row seen down its length puts two of its three floors behind the third, and a floor is only in frame from in
//! front of the wall that holds it.
//!
//! **The frame is the row's own shape and the lens is composed against it.** A row four times as wide as it is tall
//! cannot fill a square frame: the widest crop that holds the whole outline is about 1.21x, which reads about a
//! fifth of the picture, and every step past that takes building off the sides as fast as it takes sky off the top.
//! So this scene asks for three to one, and the lens then does the rest — a vertical angle 2.197x shorter than the
//! square frame's, over the same eye, the same target and the same twenty metres, so every sight line over a window
//! sill that put a floor in a frame still puts one in this one. The building's own pixels go from **25.36 %** of the
//! square to **44.88 %** of the wide one, and the picture itself gets cheaper: 786,432 pixels against 1,048,576.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, COTTAGE};
use building::grid::{FaceRef, Rhythm};
use building::identity::Identity;
use building::openings::{Inset, Opening};
use building::ops;
use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode};

use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, sky, surface,
};
use document::shaders::{Projection, post_shader};

/// One party leaf's thickness. A party wall is thin because nothing weathers it, so it is the one number the
/// style's `1/19` external-wall relation does not decide.
const PARTY_LEAF: f32 = 0.15;

/// How many units the row has.
const UNITS: usize = 3;

/// Each unit's plan size: 6 m across, 8 m deep.
const UNIT_WIDTH: f32 = 6.0;
const UNIT_DEPTH: f32 = 8.0;

/// The image this scene is drawn into: three across to one up.
///
/// # Why the frame is the row's own shape rather than the pipeline's square
///
/// The row is about four times as wide as it is tall, so a square frame holds it at a
/// disadvantage that no lens can undo: at the widest crop that still holds the whole outline the
/// building's own pixels are about a fifth of the picture, and every step past that takes
/// building off the sides as fast as it takes sky off the top. **A frame is a property of the
/// scene** — `scene::standard::frame::Frame::read` takes this pair out of the document and falls
/// back to its own square when a document names none — so a scene whose subject has a shape can
/// ask for that shape and let the lens be worked out against it.
///
/// Three to one is wider than the row and narrower than its own outline in a frame, which is what
/// leaves room: the measured outline stands about 5 % inside the frame's left and right edges and
/// takes two thirds of its height, with a band of sky above the three ridges and a band of ground
/// in front of the wall. The pixels are 786,432 against the square's 1,048,576, so the row's share
/// of the frame rises from a quarter to nearly half while the picture itself gets cheaper to draw.
const FRAME_WIDTH: i64 = 1536;
const FRAME_HEIGHT: i64 = 512;

/// The full **vertical** angle the row is drawn through, in radians.
///
/// # The arithmetic, so that a session changing it does not have to re-derive it
///
/// The square frame this scene replaced held `FRAC_PI_3 * 0.95 / 1.4` = **0.74798 rad** down the
/// frame, and the row's outline ran off both of its sides — the left-hand mass cut at x 0 and the
/// right-hand one at x 1023. Widening the frame to three to one holds the whole outline at that
/// same lens with room to spare (x 450..=1088 of 0..=1535), and everything past that is the lens.
/// Narrowing the vertical angle until the outline stands about 5 % inside the left and right edges
/// puts it at **0.34034 rad = 19.50°**, which is **2.197x shorter** than the square frame's.
///
/// **The magnification is a consequence of the arithmetic and not its aim.** A shorter angle
/// magnifies the world by 2.197x about the view axis, and a frame half as tall spends half of that
/// again on its own size, so what reaches the picture is **1.079x** — measured at 1.079, 1.074 and
/// 1.077 as the square roots of the three plates' pixel counts, which is the same number three
/// independent times. The row did not grow; the frame it sits in stopped being the wrong shape.
///
/// **Only this one number moved.** The eye, the target and the distance are the ones the camera
/// slice measured into place, and a floor is in frame because of where the eye stands and how far
/// it is from the wall — not because of the lens — so every sight line over a sill that reached the
/// back of a room still reaches it.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.325;

/// Returns a rectangular outline from `(x0, z0)` to `(x1, z1)` at the given elevation.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32, elevation: f32) -> Vec<PlanNode> {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, elevation))
        .collect()
}

/// Returns the row's document: `UNITS` units of `COTTAGE`'s proportions, flush at their party walls.
///
/// # Invariants
///
/// - **Each unit is a closed solid of its own**, which is what lets the party wall be two leaves: the row is not
///   one building with two internal walls, it is three buildings that touch.
/// - **Every unit's numbers come from the style except the party leaf.** Height, slab, eave, rise and the roof's
///   thickness are `COTTAGE::starter`'s answers; the wall is `PARTY_LEAF` twice.
/// - **Each unit's roof runs along the row**, so the ridges are parallel and the party walls are visible as the
///   gaps between them — a hip roof per unit would have cut the row into three separate cottages.
pub fn document() -> Building {
    let style = COTTAGE.starter(UNIT_WIDTH, UNIT_DEPTH, 1);
    let mut skeleton = BuildingSkeleton::default();
    for unit in 0..UNITS {
        let centre = (unit as f32 - (UNITS as f32 - 1.0) * 0.5) * UNIT_WIDTH;
        let mass = Mass::new(
            rectangle(
                centre - UNIT_WIDTH * 0.5,
                -UNIT_DEPTH * 0.5,
                centre + UNIT_WIDTH * 0.5,
                UNIT_DEPTH * 0.5,
                0.0,
            ),
            style.height,
        )
        .with_wall(PARTY_LEAF)
        .with_slab(style.slab);
        let id = ops::add_mass(&mut skeleton, mass);
        // **Each unit's plate is its own solid**, spanning the unit's footprint from one plate thickness below the
        // floor level up to it, so the wall ring overlaps it and no floor is split at a wall.
        skeleton.slabs.push(
            building::slab::Slab::plate(
                rectangle(
                    centre - UNIT_WIDTH * 0.5,
                    -UNIT_DEPTH * 0.5,
                    centre + UNIT_WIDTH * 0.5,
                    UNIT_DEPTH * 0.5,
                    0.0,
                )
                .into_iter()
                .map(|node| node.position)
                .collect(),
                -style.slab,
                0.0,
            )
            .with_host(id),
        );
        // The roof is derived from the mass it covers, so a unit's eave follows its own walls rather than a
        // number the scene keeps beside them.
        if let Some(roof) = derive::gable_roof(
            id,
            &skeleton.masses[id.0],
            style.overhang,
            style.rise,
            style.sink,
            style.roof_thickness,
        ) {
            ops::add_roof(&mut skeleton, roof);
        }
    }
    // Three bays of windows a front, sized by the style's own glazing.
    let bays = COTTAGE.bays(UNIT_WIDTH);
    let (left, right, sill, height) = COTTAGE.glazing(UNIT_WIDTH / bays as f32, style.height);
    for unit in 0..UNITS {
        let front = FaceRef::new(MassId(unit), 2);
        ops::split_face(
            &mut skeleton,
            front,
            &Rhythm::Even { count: bays },
            &Rhythm::Even { count: 1 },
        );
        for column in 0..bays {
            let opening = ops::add_opening(
                &mut skeleton,
                Opening::new(front, 0, column, Inset::new(left, right, sill, height)),
            );
            assert!(opening.is_some(), "the opening is one the face accepts");
        }
    }
    Building::new(skeleton)
}

/// Builds the terrace scene.
pub fn terrace() -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the row expands: every unit's outline is a quad and its roof's rows are paired");

    // **Every mesh the frame draws carries its elements' ids**: three masses' walls and three roofs, so a pixel
    // of any unit can be asked which unit and which face it is.
    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // One plate a unit, over the unit's own footprint: three solids `document` authors, drawn here rather
    // than left in the table where nothing can see them.
    let floors = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Slab(_, _))
    });
    let roof = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Roof(_, _))
    });

    // Brick, tile and dry ground; the row is long and low, so the camera stands off its middle.
    let brick = || surface([0.56, 0.34, 0.26], 0.0, 0.88, [0.0; 3]);
    let tile = || surface([0.30, 0.20, 0.16], 0.0, 0.70, [0.0; 3]);
    let ground = || surface([0.26, 0.24, 0.17], 0.0, 0.95, [0.0; 3]);
    let floor = || surface([0.32, 0.24, 0.17], 0.0, 0.82, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("brick", brick as fn() -> Value),
        ("tile", tile),
        ("floor", floor),
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
        materials.push(mesh_material(&format!("ground-{stage}"), shader, ground()));
    }

    let placed = |stage: &str| {
        [
            ("walls", "brick"),
            ("roofs", "tile"),
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
    let ridge = style_ridge();
    let extent = f64::from(UNIT_WIDTH * UNITS as f32) * 0.9;
    // **One air, handed to two renderers**: the map a world reads and the light a surface sees. Without the
    // light the shading's sky term is zero, which is why a sun-only scene's shadowed side is black.
    let (sky_image, sky_light) = sky(&atmosphere());

    Value::structure([
        ("name".to_owned(), Value::String("terrace".to_owned())),
        // The one field here that is about the picture rather than what is in it: a row four times as wide
        // as it is tall in a frame that shape, which the square every other scene keeps cannot hold. A scene that
        // names no frame is drawn into the pipeline's own square, so this is a decision rather than a default.
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            // Square on to the row and above the eaves, a third of a unit in from its middle: a terrace's floors
            // are only in frame from in front of them and high enough for the sight line over a sill to reach the
            // back of a room, and from here every unit is seen through its own windows rather than past its neighbour.
            // Standing off the near corner instead puts two of the three plates behind the row.
            //
            // **The eye and the target are exactly where the camera slice measured them, and only the lens moved.**
            // A floor is in frame because of where the eye stands, how high it is and how far it is from the wall,
            // so those three numbers are what put a plate in each of the three windows and none of this slice's
            // frame work touches them. The lens is the free variable, and it is free because a wide frame gives it
            // the room a square frame spent on sky: **2.197x shorter down the frame**, which is what takes the row's
            // outline from running off both sides of the picture to standing whole inside a frame of its own shape.
            Value::List(vec![camera(
                [
                    f64::from(UNIT_WIDTH) * 0.6,
                    ridge * 1.15,
                    f64::from(UNIT_DEPTH) * 3.0,
                ],
                [0.0, ridge * 0.35, 0.0],
                FIELD_OF_VIEW,
            )]),
        ),
        (
            "lights".to_owned(),
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
                mesh("roofs", "Triangles", point.clone(), roof),
                mesh("floors", "Triangles", point, floors),
                mesh(
                    "ground",
                    "Triangles",
                    plain.point_schema(),
                    crate::ground::mesh(&building, 140, 460.0, |_, _| 0.0),
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

/// Returns the ridge's height the frame has to fit: the style's wall top plus its own rise.
fn style_ridge() -> f64 {
    let style = COTTAGE.starter(UNIT_WIDTH, UNIT_DEPTH, 1);
    f64::from(style.height + style.rise)
}

/// Where the sun stands: high and to the left, so the row's near face is lit and the party walls cast into
/// shadow between the units.
const SUN: [f64; 3] = [-30.0, 36.0, 26.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The air over the row: a clear afternoon, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's
/// bloom — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads
/// its sky term from. A row whose shadows are black is a row with no air, not a row with the wrong
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
