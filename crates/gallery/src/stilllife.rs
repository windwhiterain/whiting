//! A still life by window light.
//!
//! A bowl of lemons and a jug on a table against a wall, lit by a window on the left that is
//! not in the frame. Everything in the picture is a decision about light: where it comes from,
//! how far it carries down the table, what it does in the shadow, and what a lemon, a glaze
//! and a linen cloth do with it when it arrives.
//!
//! # What makes it read as a photograph
//!
//! - **The light is a window and not a lamp.** It is a **grid of thirty five point lights**
//!   standing in the window's opening, because the pipeline has no area light and a window is
//!   the one source a still life is actually lit by. The grid is what the frame is built on:
//!   every light casts its own shadow, so a penumbra is a stack of them rather than a blurred
//!   edge, and the glaze of a bowl shows the window as a smear rather than as a dot. The lights
//!   are **scattered** across the opening rather than set on a lattice, so the steps in a
//!   penumbra do not line up into bands.
//! - **The scene is a hundred units across, and it is a hundred units across because of the
//!   attenuation.** The lighting template's falloff is `1 / (1 + 0.0035 r + 0.00002 r²)`, whose
//!   constants its own comment places in "a scene a couple of hundred units across". At a
//!   tabletop's true size that falloff is flat to within a percent and there is no gradient
//!   down the table at all; at this size the near corner of the table is lit three times as hard
//!   as the far one, which is what a window does to a table. A metre is therefore [`UNIT`]
//!   units, and every number below is written in metres.
//! - **The light is placed where it models.** A window straight in front of the subject is a
//!   flash and a window at right angles is a silhouette; this one stands about thirty degrees
//!   off the camera's own axis and low enough that the light rakes across the table, which is
//!   what puts a shadow beside the bowl and a shadow of the jug up the wall behind it.
//! - **The camera is on a lens.** An 85 mm on a 36 mm frame, looked slightly down, so the table
//!   recedes and the wall rides above the middle. A fitted camera pulls back until the whole
//!   table is in the frame, which is the one composition a still life cannot have.
//! - **It is developed rather than clipped.** The post stage does the two things a camera does
//!   and the pipeline has no field for: it gathers a **circle of confusion** out of the depth
//!   buffer, and it is **exposed once**, through a film curve, and then grained.
//!
//! # The one the framework does not have
//!
//! There is **no environment**: no image based lighting, no probe, and no way to tell a surface
//! what is around it. A still life is largely what things reflect of one another — the window
//! in a glaze, the table in a bowl's underside, the wall in a jug — so the room is written here
//! as a function, [`ROOM_WGSL`], and every surface of the scene samples it. See
//! [the proposal](../../../.agents/proposals/photographic-stilllife.md).

use definition::Value;
use scene::standard::Stage;

use crate::geometry::{cuboid, fractal_noise, hash01, heightfield, lathe, sphere};
use crate::kit::{
    camera_placed_on_lens, gallery_custom_schema, gallery_instance_schema, instance, instances,
    light_with_radius, mesh, mesh_material, post_processes, screen_material, shaded_point_schema,
    shader, surface, surface_reflecting,
};
use crate::shaders::{depthing, post_shader, shadowing};

/// World units in a metre.
///
/// # Why this is not one
///
/// The falloff of a light is a constant of the shading template and not a field of a scene, so
/// the only falloff a scene can have is the one its own size buys. See the module
/// documentation.
const UNIT: f64 = 100.0;

/// The table, in metres: how wide it is, how thick, and how deep.
///
/// It is **short**, and that is a lighting decision: the wall stands just behind its far edge,
/// so the shadows of what is on the table climb the wall instead of running off the end of a
/// table that is longer than the frame.
const TABLE: [f64; 3] = [2.4, 0.07, 1.05];

/// Where the wall's face stands, in metres, and how far up and down it goes.
///
/// # Why the wall is not taller
///
/// It is only ever seen above the table, and every light's shadow volume is fitted around
/// **what the scene draws** with `shadow_extent` as its half width. A wall three metres tall
/// drags the centre of that box a metre and a half into the air and makes every page coarser
/// for nothing. The wall is as tall as the frame can see of it.
const WALL_Z: f64 = -0.54;
const WALL_BASE: f64 = -0.20;
const WALL_TOP: f64 = 0.35;
const WALL_WIDTH: f64 = 7.0;

/// The window: the plane the light comes through, and how it is sampled.
///
/// # Why it is where it is
///
/// It is a plane of constant `x` — a window in the room's left wall — and it stands to the left
/// of and a little behind the camera, which puts the light about thirty degrees off the axis
/// the camera looks down. That is the angle a still life is lit at and it is the angle that
/// makes the picture: head on, the light is a flash and every shadow is hidden behind the thing
/// that cast it; at right angles, the frame is half silhouette.
const WINDOW_X: f64 = -1.45;
const WINDOW_Z: [f64; 2] = [0.78, 1.28];
const WINDOW_Y: [f64; 2] = [0.32, 0.78];
const WINDOW_COLUMNS: usize = 3;
const WINDOW_ROWS: usize = 3;
/// What the whole window puts out, divided among its lights.
const WINDOW_INTENSITY: f64 = 11.0;
/// How far a window light is allowed to reach, which is further than the table is from it.
const WINDOW_REACH: f64 = 4.2;
/// The half width of the volume every light's shadow is rendered over: the region shadows have
/// to land in, which is the table under and around the subject and the wall behind it.
const SHADOW_EXTENT: f64 = 0.75;

/// Where the camera stands, in metres, and what it looks at.
const EYE: [f64; 3] = [-0.50, 0.30, 1.42];
const TARGET: [f64; 3] = [0.05, 0.155, -0.16];
/// The lens: a short telephoto, which is what a still life is usually shot on.
const FOCAL_LENGTH: f64 = 85.0;
const SENSOR_WIDTH: f64 = 36.0;
/// The depth planes, in metres, and they are **close to what the scene draws**.
///
/// # Why this is not a wide range
///
/// A projection's depth is a fraction of the way from the near plane to the far one in `1/d`,
/// so a far plane far enough away collapses the whole scene into the last slice of the cluster
/// grid. A cluster of the **last** slice is the slab at the slice's far boundary, which is the
/// far plane, where a perspective view is widest — so its column is a slab forty metres out and
/// off to one side, and a light standing over the table is further from that slab than its own
/// radius. The cluster is then lit by nothing at all, and the frame is covered in rectangular
/// patches a cluster wide. See the proposal: the scene sets its planes and the framework
/// should not need it to.
const NEAR: f64 = 0.25;
const FAR: f64 = 8.0;

/// Where the bowl and the jug stand, in metres.
const BOWL_AT: [f64; 3] = [-0.10, 0.0, -0.14];
const JUG_AT: [f64; 3] = [0.10, 0.0, 0.04];
/// Where the napkin is laid, how far it reaches, and where the lemon that came out of the bowl
/// rolled to.
const NAPKIN_AT: [f64; 3] = [-0.32, 0.0018, 0.09];
const NAPKIN_EXTENT: f64 = 0.26;
const LOOSE_LEMON: [f64; 3] = [-0.27, 0.031, 0.16];
/// Where the brass spoon lies, in metres: its centre, on the cloth.
const SPOON_AT: [f64; 3] = [-0.26, 0.0060, 0.05];

/// The radius a lemon is, in metres.
const LEMON_RADIUS: f64 = 0.030;

/// Metres to the scene's units.
fn at(value: f64) -> f64 {
    value * UNIT
}

/// A point in metres to a point in the scene's units.
fn place(point: [f64; 3]) -> [f64; 3] {
    [at(point[0]), at(point[1]), at(point[2])]
}

/// A profile in metres as the lathe reads it, in the scene's units.
fn scaled(profile: &[[f64; 2]]) -> Vec<[f64; 2]> {
    profile
        .iter()
        .map(|entry| [at(entry[0]), at(entry[1])])
        .collect()
}

/// Builds the still life.
pub fn stilllife() -> Value {
    let helpers = shared_helpers();
    let point = shaded_point_schema();
    let inputs = crate::attributes::gallery_vertex_inputs(&point);

    let mut shaders = vec![
        shader(
            Stage::Shadowing,
            "shadowing",
            &shadowing(&inputs),
            scene::standard::shadow_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            gallery_custom_schema(),
        ),
        shader(
            Stage::Depthing,
            "depthing",
            &depthing(&inputs),
            scene::standard::depth_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            gallery_custom_schema(),
        ),
    ];
    for (label, displace, body) in surface_bodies() {
        shaders.push(crate::shaders::displaced_lighting_shader(
            label, displace, body, &helpers,
        ));
    }
    shaders.push(post_shader(GRADE, &format!("{helpers}\n{GRADE_HELPERS}")));

    let materials = vec![
        // The two stages that place a point rather than colour it. Their own data is never
        // read, so they carry a surface and nothing else.
        mesh_material(
            "caster",
            "shadowing",
            surface([1.0, 1.0, 1.0], 0.0, 0.9, [0.0; 3]),
        ),
        mesh_material(
            "occluder",
            "depthing",
            surface([1.0, 1.0, 1.0], 0.0, 0.9, [0.0; 3]),
        ),
        // The table: dark walnut under varnish. The one bright thing a varnished board shows is
        // the window lying along it, and `reflect` is how much of that it is allowed to show.
        mesh_material(
            "wood",
            "wood",
            surface_reflecting([0.118, 0.084, 0.060], 0.0, 0.42, [0.0; 3], 0.55),
        ),
        // Linen: the brightest surface in the frame, and the one the light is read off.
        mesh_material(
            "linen",
            "linen",
            surface_reflecting([0.66, 0.60, 0.505], 0.0, 0.78, [0.0; 3], 0.22),
        ),
        // A glaze: smooth, pale, and at a grazing angle nearly a mirror of the room.
        mesh_material(
            "ceramic",
            "ceramic",
            surface_reflecting([0.625, 0.610, 0.560], 0.0, 0.30, [0.0; 3], 1.0),
        ),
        // A lemon's skin is oiled, so it is a rough dielectric with a small sharp reflection
        // sitting on top of a saturated scatter.
        mesh_material(
            "produce",
            "produce",
            surface_reflecting([0.62, 0.52, 0.080], 0.0, 0.36, [0.0; 3], 0.85),
        ),
        // The wall, which is where the shadows are read.
        mesh_material(
            "plaster",
            "plaster",
            surface([0.055, 0.052, 0.049], 0.0, 0.92, [0.0; 3]),
        ),
        // Brass: a metal, so its row carries a metallic of one and a tint that is its own
        // reflectance rather than a colour it scatters.
        mesh_material(
            "brass",
            "metal",
            surface_reflecting([0.62, 0.46, 0.19], 1.0, 0.20, [0.0; 3], 1.0),
        ),
    ];

    let mut shadowing_instances = Vec::new();
    let mut depthing_instances = Vec::new();
    let mut lighting_instances = Vec::new();
    let mut drawn = |object: &str,
                     mesh: &str,
                     translation: [f64; 3],
                     rotation: [f64; 3],
                     scale: [f64; 3],
                     material: &str| {
        let placed = place(translation);
        shadowing_instances.push(instance(
            format!("{object}-cast"),
            mesh,
            placed,
            rotation,
            scale,
            "caster",
        ));

        depthing_instances.push(instance(
            format!("{object}-depth"),
            mesh,
            placed,
            rotation,
            scale,
            "occluder",
        ));
        lighting_instances.push(instance(
            format!("{object}-shade"),
            mesh,
            placed,
            rotation,
            scale,
            material,
        ));
    };

    // The table, which is the lower half of the frame, and the wall, which is the upper part of
    // it and the only surface that shows a shadow as a shape.
    drawn(
        "table",
        "table",
        [0.0, -TABLE[1], 0.0],
        [0.0; 3],
        [1.0; 3],
        "wood",
    );
    drawn(
        "wall",
        "wall",
        [0.0, WALL_BASE, WALL_Z - 0.08],
        [0.0; 3],
        [1.0; 3],
        "plaster",
    );
    drawn("napkin", "napkin", NAPKIN_AT, [0.0; 3], [1.0; 3], "linen");
    drawn("bowl", "bowl", BOWL_AT, [0.0; 3], [1.0; 3], "ceramic");
    drawn("jug", "jug", JUG_AT, [0.0; 3], [1.0; 3], "ceramic");

    // Three lemons in the bowl and one that rolled onto the cloth.
    //
    // The lemon is one mesh the scene places four times rather than four meshes: the sphere is
    // **scaled unevenly** by the instance, and the normal matrix the emitter fills in beside
    // the model is what keeps an unevenly scaled sphere round rather than lit as a ball.
    let lemons: [([f64; 3], [f64; 3]); 5] = [
        (
            [BOWL_AT[0] - 0.042, 0.066, BOWL_AT[2] + 0.030],
            [0.30, 0.60, 1.35],
        ),
        (
            [BOWL_AT[0] + 0.050, 0.064, BOWL_AT[2] - 0.026],
            [0.10, 1.90, 0.40],
        ),
        (
            [BOWL_AT[0] + 0.004, 0.068, BOWL_AT[2] + 0.046],
            [0.55, 0.20, 1.70],
        ),
        (
            [BOWL_AT[0] - 0.004, 0.114, BOWL_AT[2] - 0.008],
            [0.40, 2.60, 0.25],
        ),
        (LOOSE_LEMON, [0.20, 0.35, 1.10]),
    ];
    for (index, (position, rotation)) in lemons.into_iter().enumerate() {
        drawn(
            &format!("lemon-{index}"),
            "lemon",
            position,
            rotation,
            [1.35, 1.0, 1.0],
            "produce",
        );
    }

    // A brass spoon on the cloth, lying at an angle across the near corner. It is the one thing
    // in the frame that is not a scatterer: a metal shows the room and nothing else, and the
    // hot line along its handle is the window in a few millimetres of brass. The handle is a
    // slab and the bowl of it is a sphere laid flat, which is the whole of a spoon's geometry at
    // the distance this one is seen from.
    let spoon_yaw = 0.42;
    drawn(
        "spoon-handle",
        "spoon-handle",
        [SPOON_AT[0], SPOON_AT[1], SPOON_AT[2]],
        [0.0, spoon_yaw, 0.0],
        [1.0, 0.30, 0.72],
        "brass",
    );
    let along = [spoon_yaw.sin(), 0.0, spoon_yaw.cos()];
    drawn(
        "spoon-bowl",
        "spoon-bowl",
        [
            SPOON_AT[0] - along[0] * 0.062,
            SPOON_AT[1] + 0.0035,
            SPOON_AT[2] - along[2] * 0.062,
        ],
        [0.0, spoon_yaw, 0.0],
        [1.0; 3],
        "brass",
    );

    Value::structure([
        ("name".to_owned(), Value::String("stilllife".to_owned())),
        (
            "frame".to_owned(),
            Value::structure([
                ("width".to_owned(), Value::Int(1536)),
                ("height".to_owned(), Value::Int(1024)),
            ]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed_on_lens(
                place(EYE),
                place(TARGET),
                FOCAL_LENGTH,
                SENSOR_WIDTH,
                0.0,
                at(NEAR),
                at(FAR),
            )]),
        ),
        ("lights".to_owned(), Value::List(window_lights())),
        ("shaders".to_owned(), Value::List(shaders)),
        ("meshes".to_owned(), Value::List(meshes())),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("develop", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["develop"])),
        (
            "instances".to_owned(),
            instances(vec![
                (Stage::Shadowing, shadowing_instances),
                (Stage::Depthing, depthing_instances),
                (Stage::Lighting, lighting_instances),
            ]),
        ),
    ])
}

/// Builds the window, and the two lights that stand in for the room around it.
///
/// # Why thirty five lights
///
/// A penumbra cast by a grid of `n` lights is a stack of `n` shadows, so the number is the
/// number of steps a soft edge is drawn with, and the window's light is the only soft edge in
/// the frame. It is also the whole cost: a light that is given a page costs a shadowing pass,
/// so this is thirty five passes to render one shadow, and the pool's default of sixty four
/// pages holds them all. The window is worth it and nothing else in this scene would be.
fn window_lights() -> Vec<Value> {
    let mut lights = Vec::with_capacity(WINDOW_COLUMNS * WINDOW_ROWS + 2);
    let each = WINDOW_INTENSITY / (WINDOW_COLUMNS * WINDOW_ROWS) as f64;
    for row in 0..WINDOW_ROWS {
        for column in 0..WINDOW_COLUMNS {
            let index = (row * WINDOW_COLUMNS + column) as i64;
            // Evenly divided, then **scattered inside its own cell** rather than set on the
            // lattice. Thirty five lights on a lattice put thirty five shadows in a regular
            // stack, and a regular stack of edges is a banded penumbra; scattered, the same
            // thirty five read as one soft edge with a little noise in it, which is what a
            // window's light has when it comes off the sky.
            let across =
                (column as f64 + 0.5 + (scatter(index, 3) - 0.5) * 0.9) / WINDOW_COLUMNS as f64;
            let up = (row as f64 + 0.5 + (scatter(index, 7) - 0.5) * 0.9) / WINDOW_ROWS as f64;
            let z = WINDOW_Z[0] + across * (WINDOW_Z[1] - WINDOW_Z[0]);
            let y = WINDOW_Y[0] + up * (WINDOW_Y[1] - WINDOW_Y[0]);
            // A metre of sky is not the same colour all over it, and the scatter gives the light
            // its own faint unevenness, which is what stops a lit wall from being a wash.
            let tint = 0.94 + 0.12 * scatter(index, 11);
            lights.push(light_with_radius(
                &format!("window-{row}-{column}"),
                place([WINDOW_X, y, z]),
                [tint, tint * 0.995, tint * 0.960],
                each,
                at(SHADOW_EXTENT),
                at(WINDOW_REACH),
            ));
        }
    }
    // The light the room gives back. A still life lit by one window has a shadow side lit by
    // the table, the cloth and the wall, and that light is warm, low and close; it is the only
    // reason the shadow side is not a hole.
    //
    // It stands **on the camera's own axis**, which is where a photographer puts a fill: a light
    // beside the lens casts every shadow it makes directly away from the lens, so the shadows
    // land behind the things that made them and no second set of them is visible. A fill to one
    // side of the camera is a second light with a second set of shadows in the frame — which is
    // what this light was, and what a jug's ghost on the wall taught it not to be.
    lights.push(light_with_radius(
        "bounce",
        place([-0.30, 0.26, 1.30]),
        [1.0, 0.72, 0.46],
        0.45,
        at(SHADOW_EXTENT),
        at(2.6),
    ));
    // And the room's own far corner, which reaches the shadow side of everything as a dim, cool
    // fill from above the camera. A frame whose only fill is warm reads as firelight. It is on
    // the axis too, and for the same reason.
    lights.push(light_with_radius(
        "room",
        place([-0.10, 1.45, 1.15]),
        [0.62, 0.68, 0.80],
        0.35,
        at(SHADOW_EXTENT),
        at(3.4),
    ));
    lights
}

/// Returns a deterministic value in zero to one from two integers.
///
/// # Invariants
///
/// The same window is the same window every time the scene is compiled. A scatter taken from a
/// clock would give a different frame from an unchanged document, and a render is a pure
/// function of the documents and the time.
fn scatter(index: i64, seed: i64) -> f64 {
    hash01(index, seed, 17)
}

/// Builds the meshes the scene draws.
fn meshes() -> Vec<Value> {
    let point = shaded_point_schema();
    vec![
        mesh(
            "table",
            "Triangles",
            point.clone(),
            cuboid(at(TABLE[0]), at(TABLE[1]), at(TABLE[2])),
        ),
        mesh(
            "wall",
            "Triangles",
            point.clone(),
            cuboid(at(WALL_WIDTH), at(WALL_TOP - WALL_BASE), at(0.16)),
        ),
        mesh(
            "napkin",
            "Triangles",
            point.clone(),
            heightfield(72, at(NAPKIN_EXTENT), napkin_fold),
        ),
        mesh(
            "bowl",
            "Triangles",
            point.clone(),
            lathe(&scaled(&BOWL), 72),
        ),
        mesh("jug", "Triangles", point.clone(), lathe(&scaled(&JUG), 64)),
        mesh(
            "lemon",
            "Triangles",
            point.clone(),
            sphere(36, at(LEMON_RADIUS)),
        ),
        mesh(
            "spoon-handle",
            "Triangles",
            point.clone(),
            cuboid(at(0.013), at(0.0035), at(0.125)),
        ),
        mesh("spoon-bowl", "Triangles", point, sphere(24, at(0.020))),
    ]
}

/// The height of the cloth above the table, in the scene's units, at a point of the table.
///
/// # Invariants
///
/// The cloth is **laid and not draped**: a napkin is folded flat and put down, so what it has
/// is a few long creases from the fold and a little unevenness in the weave. A cloth shaped
/// like a wave is a curtain, and a napkin is the one thing in a still life that is flat.
fn napkin_fold(x: f64, z: f64) -> f64 {
    let (x, z) = (x / UNIT, z / UNIT);
    let crease = (x * 9.0).sin() * 0.0055 + (z * 7.0 + 1.2).sin() * 0.0038;
    let uneven = fractal_noise([x * 7.0, z * 7.0], 3, 5) * 0.0028 - 0.0013;
    at(crease + uneven)
}

/// The profile of the bowl, in metres, from the centre of its underside to the centre of the
/// inside of its base.
///
/// # Invariants
///
/// One closed profile rather than two shells: out along the underside, up the outside, over the
/// rim, back down the inside. A bowl drawn as an outside and an inside is a bowl whose two
/// surfaces can be a hair apart at the rim, and a rim a hair thick renders as a crack.
const BOWL: [[f64; 2]; 17] = [
    [0.000, 0.000],
    [0.030, 0.000],
    [0.048, 0.004],
    [0.064, 0.011],
    [0.082, 0.023],
    [0.099, 0.040],
    [0.113, 0.058],
    [0.124, 0.077],
    [0.131, 0.090],
    [0.132, 0.097],
    [0.128, 0.100],
    [0.121, 0.091],
    [0.110, 0.073],
    [0.096, 0.055],
    [0.077, 0.041],
    [0.050, 0.030],
    [0.000, 0.025],
];

/// The profile of the jug, in metres.
const JUG: [[f64; 2]; 17] = [
    [0.000, 0.000],
    [0.030, 0.000],
    [0.046, 0.006],
    [0.058, 0.038],
    [0.062, 0.078],
    [0.052, 0.113],
    [0.033, 0.143],
    [0.030, 0.160],
    [0.038, 0.169],
    [0.036, 0.173],
    [0.030, 0.167],
    [0.026, 0.150],
    [0.030, 0.120],
    [0.040, 0.090],
    [0.045, 0.055],
    [0.035, 0.026],
    [0.000, 0.021],
];

/// The `(label, displace, surface)` of every surface body the scene shades with.
///
/// # Invariants
///
/// One shader a material, because a surface body is a `WGSL` string and a scene has nowhere
/// else to put one. Two materials sharing a pass is the pipeline's arrangement and not this
/// scene's: what differs between a bowl and a lemon is not a row of data, it is everything the
/// fragment computes.
fn surface_bodies() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("wood", WOOD_DISPLACE, WOOD_SURFACE),
        ("metal", "", METAL_SURFACE),
        ("linen", LINEN_DISPLACE, LINEN_SURFACE),
        ("ceramic", "", CERAMIC_SURFACE),
        ("produce", PRODUCE_DISPLACE, PRODUCE_SURFACE),
        ("plaster", "", PLASTER_SURFACE),
    ]
}

/// The `WGSL` every surface body and the grade are written against.
fn shared_helpers() -> String {
    format!("{NOISE_WGSL}\n{ROOM_WGSL}\n{MATERIAL_WGSL}")
}

/// A hash, and the noise built on it.
const NOISE_WGSL: &str = r#"
/// Returns a value in zero to one from a lattice point.
///
/// A hash of the lattice point rather than of the coordinate, so the value is constant within a
/// cell and jumps only at a boundary. A grain built from the continuous coordinate instead is a
/// smooth smear, which reads as a dirty lens rather than as a surface.
fn hash21(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

/// Returns smooth noise in zero to one.
fn noise21(point: vec2<f32>) -> f32 {
    let cell = floor(point);
    let within = fract(point);
    let weight = within * within * (3.0 - 2.0 * within);
    return mix(
        mix(hash21(cell), hash21(cell + vec2<f32>(1.0, 0.0)), weight.x),
        mix(hash21(cell + vec2<f32>(0.0, 1.0)), hash21(cell + vec2<f32>(1.0, 1.0)), weight.x),
        weight.y,
    );
}

/// Returns noise summed over four octaves, in zero to one.
fn fbm21(point: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var at = point;
    for (var octave = 0; octave < 4; octave = octave + 1) {
        sum = sum + amplitude * noise21(at);
        at = at * 2.03;
        amplitude = amplitude * 0.5;
    }
    return sum * 1.05;
}
"#;

/// The room, as the thing a surface reflects and the thing that fills its shadow.
///
/// # Why the room is a function
///
/// The pipeline has no environment and no probe: a surface cannot be told what is around it,
/// and the lighting stage's ambient is a fixed hemisphere of sky. A still life indoors is lit
/// by a room, and almost everything a glaze, a varnish or a skin shows is that room — so the
/// scene describes the one it stands in and hands the same description to every surface body.
/// The material's `reflect` is its say in how much of it it shows.
const ROOM_WGSL: &str = r#"
/// The direction the window is in from the subject, which is the only bright thing in the room.
const TOWARD_WINDOW: vec3<f32> = vec3<f32>(-0.771, 0.281, 0.571);

/// What a surface sees looking along a direction that came off it.
///
/// `roughness` widens the window, because that is what a rough surface does to what it
/// reflects: a mirror shows the window's own edge and a matte wall shows a glow.
fn room_environment(direction: vec3<f32>, roughness: f32) -> vec3<f32> {
    let heading = normalize(direction);
    // The room before the window: the ceiling above, the table and the floor below, and the
    // wall behind the subject. All of it nearly black, because the room has one light in it and
    // the window is it.
    let up = clamp(heading.y, 0.0, 1.0);
    var colour = mix(
        vec3<f32>(0.030, 0.028, 0.026),
        vec3<f32>(0.006, 0.006, 0.008),
        up,
    );
    // The table, seen by everything that hangs over it. It is the second brightest thing in the
    // room and it is warm, and it is what keeps the underside of a bowl from being a hole.
    colour = mix(colour, vec3<f32>(0.055, 0.038, 0.024), clamp(-heading.y * 2.4, 0.0, 1.0));
    // The window itself. Not a point: a window is a rectangle, a glaze shows a rectangle, and
    // the edge of the reflected one softens with the roughness of the surface reflecting it.
    let alignment = dot(heading, TOWARD_WINDOW);
    // The **field**: the whole opening, and how soft its edge is belongs to the roughness of
    // the surface reflecting it.
    let field = smoothstep(0.930 - 0.060 * roughness, 0.995 + 0.004 * roughness, alignment);
    // The **core**: the far side of the sky through the opening, which is the brightest thing in
    // the room by two orders of magnitude. It is narrow, and it is not softened by roughness:
    // what a rough surface does to it is spread it, which the field already is.
    let core = smoothstep(0.976 - 0.030 * roughness, 0.9990, alignment);
    // A window is not one value. What a glaze shows is a **field** with a **core** in it: the
    // far side of the sky through the opening is the brightest thing in the room by two orders,
    // and the sky nearer the frame is what fills the rest of the reflection. The field is what
    // makes a pot read as glazed; the core is what makes it read as lit, because a reflection
    // of four percent of a hundred is four, and four is white after the curve.
    //
    // The numbers are a radiance and the lights are an irradiance at a distance, and there is
    // nowhere in this framework that one is derived from the other — which is the whole of why
    // a scene has to write this function at all. See the proposal.
    let window_radiance = vec3<f32>(11.0, 11.3, 11.8)
        * (0.09 * field + 9.0 * core)
        * (1.0 - 0.15 * roughness);
    return mix(colour, window_radiance, field);
}

/// What the room's own light adds to a surface at a point facing a normal.
///
/// # Why a scene writes this
///
/// The lighting stage adds a hemisphere of sky to every surface before a body runs, and a room
/// has no sky in it: indoors that term is the wrong colour, the wrong size and the wrong shape,
/// and a body is spliced **after** it and cannot choose it. So every body here takes the sky
/// back out and puts the room's own light in. It is a subtract and an add rather than a field
/// of the material because the pipeline offers no field — see the proposal.
fn room_bounce(position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    // What faces the window is lit by the window's own scatter, which is cool and comes from
    // one side; what faces away is lit by whatever in the room the window has already reached.
    let facing = max(dot(normal, TOWARD_WINDOW), 0.0);
    let toward = vec3<f32>(0.045, 0.048, 0.054) * pow(facing, 1.4);
    // The wall and the ceiling, which the window's light rakes across.
    let above = vec3<f32>(0.034, 0.032, 0.030) * clamp(normal.y * 0.5 + 0.5, 0.0, 1.0);
    // The table and the cloth, throwing light back up at whatever hangs over them. It falls off
    // with height, because it is the near table that is doing the throwing.
    let below = clamp(-normal.y, 0.0, 1.0);
    let near_table = clamp(1.0 - position.y / 130.0, 0.0, 1.0);
    let bounce = vec3<f32>(0.090, 0.066, 0.040) * below * near_table;
    return toward + above + bounce;
}
"#;

/// The shapes a still life's own surfaces are built from, as `WGSL`.
const MATERIAL_WGSL: &str = r#"
/// Returns the figure of a board as a value in zero to one.
///
/// # Invariants
///
/// Two scales and **no relief**. A board shows broad bands where the tree grew fast and slow,
/// and a grain of long lines inside them — and at the distance a tabletop is photographed from
/// neither has any height to it worth reflecting: the pores are a tenth of a millimetre and a
/// varnish fills them. A grain carried into the normal instead puts a stripe of specular down
/// every line, and thirty five lights make that stripe a corrugation, which is what a
/// tabletop drawn this way looks like before the relief is taken out.
fn wood_grain(position: vec3<f32>) -> f32 {
    // The figure: a hand wide, running the length of the board, so it is a function of `z` that
    // barely moves along `x`.
    let broad = fbm21(vec2<f32>(position.x * 0.004, position.z * 0.022));
    // The grain itself: twenty to the metre along the board, wandered so the lines are not
    // parallel, and **smooth** rather than a `fract`, because a `fract` has an edge and an edge
    // at this size is a painted stripe.
    let wander = fbm21(vec2<f32>(position.x * 0.012, position.z * 0.006)) * 2.4;
    let lines = 0.5 + 0.5 * sin(position.z * 4.2 + wander);
    // The seam between two boards. A tabletop is boards, and a board edge is a line of shadow
    // and a line of end grain: it is the one thing on a table that says how it was made.
    let seam = 1.0 - smoothstep(0.0, 0.10, abs(position.z - 26.0));
    return clamp(broad * 0.62 + lines * 0.24 + seam * 0.14, 0.0, 1.0);
}

/// Returns the weave of a cloth as a height in zero to one.
///
/// # Invariants
///
/// `width` is how much of the table one pixel covers, and it is what decides how much of the
/// weave is drawn at all. Two threads cross at a right angle and **alternate** which is on top,
/// and a thread is a couple of millimetres: at the distance this cloth is seen from it is four
/// or five pixels across, and at the far corner of it two. A weave sampled below the pixel grid
/// is not a weave, it is a moiré, and a moiré over a lit cloth is the loudest artefact a cloth
/// can have — so the weave **fades out as a thread falls towards a pixel** and what is left is
/// the grain of the cloth, which is coarser and survives.
///
/// The creases and the slubs are the other half of what a cloth is, and both are drawn at a
/// scale where a few pixels are a feature rather than a period.
fn linen_weave(position: vec3<f32>, width: f32) -> f32 {
    // A thread every twelve tenths of a millimetre, and a turn of the sine is
    // `2 pi` of them.
    let at = vec2<f32>(position.x, position.z) * 5.24;
    let warp = 0.5 + 0.5 * sin(at.x);
    let weft = 0.5 + 0.5 * sin(at.y);
    let over = step(0.0, sin(at.x) * sin(at.y));
    let woven = mix(weft, warp, over);
    let slub = fbm21(vec2<f32>(position.x, position.z) * 0.09);
    let crease = fbm21(vec2<f32>(position.x, position.z) * 0.016);
    // One thread is `2 / width` pixels across, and a thread under three is a thread that cannot
    // be drawn.
    let per_thread = 1.2 / max(width, 0.0001);
    let drawn = smoothstep(2.0, 5.0, per_thread);
    return mix(0.5, woven, drawn) * 0.26 + crease * 0.48 + slub * 0.26;
}

/// Returns the skin of a lemon as a height in zero to one.
fn lemon_skin(position: vec3<f32>) -> f32 {
    // The peel's own pitting, and under it the fact that a lemon is not a sphere: it is
    // segmented, and the segments meet along ridges that run from end to end.
    let pitting = fbm21(vec2<f32>(position.x, position.y + position.z) * 0.55);
    let segments = 0.5 + 0.5 * sin(atan2(position.z, position.x) * 9.0);
    return pitting * 0.75 + segments * 0.25;
}
"#;

/// What the lighting stage does to the table.
///
/// # Why there is no displacement here
///
/// A varnished board is flat. Its figure is a colour and its grain has no height left under the
/// film, and every attempt to give it one puts a specular stripe on every line — see
/// [`WOOD_GRAIN_WGSL`]. What makes a tabletop read as wood is the figure, the colour and the
/// sheen of the varnish, and all three are here rather than in the geometry.
const WOOD_DISPLACE: &str = "";

/// What the lighting stage does to the table's colour.
const WOOD_SURFACE: &str = r#"
    let grain = wood_grain(in.world_position);
    // Walnut: the figure is what makes a board read as a board rather than as a brown surface,
    // and its contrast is what a varnish has left of it — a third either way, not a stripe.
    color = color * mix(0.66, 1.20, grain); 
     // The stage added a hemisphere of sky before this body ran, and a room has no sky in it: the
    // term is taken back out and the room's own light put in its place. See the proposal — a
    // body spliced after the term cannot choose it, only correct it.
    color = color - ambient * diffuse_color + room_bounce(in.world_position, normal) * diffuse_color;
    // The varnish. A tabletop is not a diffuse surface: it has a film on it, and what a film
    // shows is the window, lying along the direction its grain runs. The roughness it is asked
    // for is a satin one rather than a glaze's, because a satin finish is what spreads the
    // window along the board instead of showing its shape.
    let to_eye = normalize(uniforms.projection.position - in.world_position);
    let mirrored = reflect(-to_eye, normal);
    color = color + fresnel * room_environment(mirrored, 0.55) * 0.45 * in.material.z;
"#;

/// What the lighting stage does to the cloth, whose weave is the whole of its relief.
const LINEN_DISPLACE: &str = r#"
    let grain_width = length(fwidth(in.world_position.xz));
    let base = linen_weave(in.world_position, grain_width);
    let along_x = linen_weave(in.world_position + vec3<f32>(1.4, 0.0, 0.0), grain_width) - base;
    let along_z = linen_weave(in.world_position + vec3<f32>(0.0, 0.0, 1.4), grain_width) - base;
    // Cloth is soft: the threads lean by a few degrees and the pile does the rest, so the relief
    // is carried into the normal gently rather than as the height itself.
    normal = normalize(
        normal + vec3<f32>(-along_x, 0.0, -along_z) * 14.0 * (1.0 - grain_width * 0.4),
    );
"#;

/// What the lighting stage does to the cloth's colour.
const LINEN_SURFACE: &str = r#"
    let weave_width = length(fwidth(in.world_position.xz));
    let woven = linen_weave(in.world_position, weave_width);
    // The light does not reach into the crossings, which is the whole of why a cloth has a
    // texture in a photograph and a painted board does not.
    color = color * mix(0.80, 1.04, woven); 
     // The stage added a hemisphere of sky before this body ran, and a room has no sky in it: the
    // term is taken back out and the room's own light put in its place. See the proposal — a
    // body spliced after the term cannot choose it, only correct it.
    color = color - ambient * diffuse_color + room_bounce(in.world_position, normal) * diffuse_color;
    // Linen scatters rather than glazes: what it shows of the room is a little, and only where
    // the surface is turned away from the eye.
    let to_eye = normalize(uniforms.projection.position - in.world_position);
    let mirrored = reflect(-to_eye, normal);
    color = color + fresnel * room_environment(mirrored, 0.75) * 0.30 * in.material.z;
"#;

/// What the lighting stage does to a glazed bowl.
///
/// # Why there is no displacement here
///
/// A thrown pot's surface is the profile it was thrown to, and the profile is already in the
/// geometry with its own normals. A bump laid on a turned shape is a bump fighting the shape it
/// was turned to.
const CERAMIC_SURFACE: &str = r#"
    // A glaze is a thin translucent layer over a pale body: light goes into it, scatters, and
    // comes back out deeper where the glaze has pooled, which on a thrown pot is low down.
    let pool = clamp(1.0 - in.world_position.y / 90.0, 0.0, 1.0);
    color = color * mix(1.08, 0.90, pool * pool);
    // A fired glaze is not a smooth surface: it is a glass that ran, and it holds pinholes and
    // a slight unevenness of thickness that only show up close and at the edge of a highlight.
    color = color * (0.975 + 0.05 * fbm21(vec2<f32>(in.world_position.x, in.world_position.y) * 0.9)); 
     // The stage added a hemisphere of sky before this body ran, and a room has no sky in it: the
    // term is taken back out and the room's own light put in its place. See the proposal — a
    // body spliced after the term cannot choose it, only correct it.
    color = color - ambient * diffuse_color + room_bounce(in.world_position, normal) * diffuse_color;
    // The room, and through it the window, upside down in the glaze. This is what a glaze *is*
    // in a photograph: not a colour and not a highlight, but the window in a smear.
    let to_eye = normalize(uniforms.projection.position - in.world_position);
    let mirrored = reflect(-to_eye, normal);
    color = color + fresnel * room_environment(mirrored, 0.10) * 0.75 * in.material.z;
    // The fired body under the glaze is warmer than the glaze, and at the rim the light has the
    // least of it to cross, so the edge of a bowl is a warm line.
    color = color + vec3<f32>(0.10, 0.075, 0.048) * pow(1.0 - normal_dot_view, 4.0) * 0.30;
"#;

/// What the lighting stage does to the brass spoon.
///
/// # Why a metal is a different programme
///
/// A metal has **no diffuse term at all**: it absorbs everything that goes into it and gives
/// back only what it reflects, so the shading template's loop leaves it black but for the point
/// lights' own highlights. Everything a metal *is* comes from the room around it, which is the
/// thing this framework has no way to give a surface — so this body is where the scene's own
/// environment earns its place. A brass spoon in a dark room is a dark object with the window
/// burning along it, and that is what this computes.
const METAL_SURFACE: &str = r#"
    let to_eye = normalize(uniforms.projection.position - in.world_position);
    let mirrored = reflect(-to_eye, normal);
    // What the metal reflects, tinted by the metal. A polished surface takes the window's shape
    // and a satin one smears it along the direction it was finished in, which is what the
    // roughness handed to the environment decides.
    color = color + fresnel * room_environment(mirrored, 0.32) * in.tint * 1.5;
"#;

/// What the lighting stage does to a lemon.
const PRODUCE_DISPLACE: &str = r#"
    // The peel's pitting, carried into the normal along the surface's **own** two tangents: a
    // lemon is a rounded shape, and a bump frame written in the world's axes would put the
    // pitting of the top of the fruit onto its side.
    let base = lemon_skin(in.world_position);
    let tangent = normalize(cross(normal, vec3<f32>(0.0, 1.0, 0.0)) + vec3<f32>(0.001, 0.0, 0.0));
    let bitangent = cross(normal, tangent);
    let along = lemon_skin(in.world_position + tangent * 2.2) - base;
    let across = lemon_skin(in.world_position + bitangent * 2.2) - base;
    normal = normalize(normal - tangent * along * 9.0 - bitangent * across * 9.0);
"#;

/// What the lighting stage does to a lemon's colour.
const PRODUCE_SURFACE: &str = r#"
    let skin = lemon_skin(in.world_position);
    // A lemon is grown and not painted: it is greener where it ripened against something and
    // deeper where it ripened in the light, so the peel's colour follows its own pitting.
    color = color * mix(0.78, 1.22, skin); 
     // The stage added a hemisphere of sky before this body ran, and a room has no sky in it: the
    // term is taken back out and the room's own light put in its place. See the proposal — a
    // body spliced after the term cannot choose it, only correct it.
    color = color - ambient * diffuse_color + room_bounce(in.world_position, normal) * diffuse_color;
    color = mix(color, color * vec3<f32>(0.55, 1.35, 0.18), clamp((1.0 - skin) * 0.5, 0.0, 1.0));
    // The oil in the peel, which holds a small hard reflection rather than a matte scatter.
    let to_eye = normalize(uniforms.projection.position - in.world_position);
    let mirrored = reflect(-to_eye, normal);
    color = color + fresnel * room_environment(mirrored, 0.22) * 0.45 * in.material.z;
"#;

/// What the lighting stage does to the wall.
const PLASTER_SURFACE: &str = r#"
    // Plaster has one thing a flat colour does not: it is not flat. There is a trowel in it and
    // there is a wall behind it, and both of them show once the light rakes across it.
    let trowel = fbm21(vec2<f32>(in.world_position.x, in.world_position.y) * 0.013); 
     // The stage added a hemisphere of sky before this body ran, and a room has no sky in it: the
    // term is taken back out and the room's own light put in its place. See the proposal — a
    // body spliced after the term cannot choose it, only correct it.
    color = color - ambient * diffuse_color + room_bounce(in.world_position, normal) * diffuse_color;
    color = color * mix(0.86, 1.10, trowel);
    color = color + vec3<f32>(0.010, 0.009, 0.008) * fbm21(in.world_position.xy * 0.36);
"#;

/// The constants and the one function the grade is written against.
const GRADE_HELPERS: &str = r#"
/// How far the lens is focused, in the scene's units: the front of the bowl.
const FOCUS: f32 = 154.0;

/// How many pixels of blur an object at **infinity** is drawn with.
///
/// This is the whole of the aperture, and it is the number the thin lens formula wants: a point
/// at a distance `d` is drawn as a disc of `APERTURE * |d - FOCUS| / d`, which is nothing at the
/// focus, grows towards this at infinity, and grows towards it on the near side as well. An
/// 85 mm at f/5.6 on a 36 mm frame is about a hundred pixels; a still life is shot stopped down
/// and the wall of this one is a third of the way out, which is this number times a third.
const APERTURE: f32 = 18.0;

/// The largest blur any pixel is drawn with, in pixels. A defocused highlight is drawn as a disc
/// of about this size, so it is also the size of the brightest disc in the frame.
const LARGEST: f32 = 11.0;

/// How many taps the circle of confusion is gathered with.
const TAPS: i32 = 32;

/// Returns the shaded frame at a pixel, clamped to the frame.
fn frame_at(coordinate: vec2<i32>, size: vec2<f32>) -> vec3<f32> {
    let inside = clamp(
        coordinate,
        vec2<i32>(0, 0),
        vec2<i32>(i32(size.x) - 1, i32(size.y) - 1),
    );
    return textureLoad(color_target, inside, 0).rgb;
}
"#;

/// What the post stage does to the frame the scene produced.
///
/// # Invariants
///
/// The order is the order light takes through a camera, and it is the whole of why this reads as
/// a photograph rather than as a render with a filter over it:
///
/// - **The circle of confusion is gathered first, and it is gathered in radiance.** The taps are
///   summed as the light that reached the film, and the film curve is applied to the sum and not
///   to each tap: a highlight that has been through the curve before it is spread has had its
///   halo taken off it, and a frame whose highlights do not spread is a frame that reads as
///   sharp everywhere.
/// - **A tap counts only if it is at the same distance.** A sharp edge against a defocused
///   background is the one place a screen space gather goes wrong: the dark wall picks up the
///   lit bowl and grows a bright halo it never had. The weight is the **relative** depth
///   difference, so one constant works at every distance in the frame.
/// - **Exposure, then the curve, then out of linear.** A photograph is exposed once. The lights
///   of this scene are radiances at a chosen distance, and the number that decides how bright
///   the frame is has to be one number in one place.
/// - **Vignette and grain last**, because they belong to the camera and the film rather than to
///   the scene, and because a frame without grain has a mathematical smoothness that the eye has
///   no way to account for except by deciding it is not a photograph.
const GRADE: &str = r#"
    let centre = distance_to(depth);
    // The circle of confusion of a thin lens: nothing at the focus, and a disc that grows with
    // how far the **reciprocal** of a distance is from the reciprocal of the focus.
    let coc = min(APERTURE * abs(centre - FOCUS) / max(centre, 1.0), LARGEST);

    // A golden angle walk over the disc, turned by a hash of the pixel, so thirty two taps
    // across an eleven pixel disc spread as noise rather than as thirty two visible arms.
    let turn = hash21(in.uv * size) * 6.2831853;
    var gathered = vec3<f32>(0.0);
    var weight = 0.0;
    for (var tap = 0; tap < TAPS; tap = tap + 1) {
        let index = f32(tap) + 0.5;
        let angle = turn + index * 2.3999632;
        let radius = sqrt(index / f32(TAPS)) * max(coc, 0.35);
        let offset = vec2<f32>(cos(angle), sin(angle)) * radius;
        let at = clamp(
            pixel + vec2<i32>(offset),
            vec2<i32>(0, 0),
            vec2<i32>(i32(size.x) - 1, i32(size.y) - 1),
        );
        let tap_distance = distance_to(textureLoad(depth_buffer, at, 0));
        // A tap is evidence about this pixel only if it is at the same distance, which is what
        // keeps a defocused background from climbing onto the edge of the subject in front of it.
        let apart = abs(tap_distance - centre) / max(centre, 1.0);
        let take = 1.0 / (1.0 + apart * apart * 900.0);
        gathered = gathered + frame_at(at, size) * take;
        weight = weight + take;
    }
    var colour = gathered / max(weight, 0.00001);

    // Nothing was drawn here: the room past the edge of the wall, which is the frame's own black.
    if depth >= 1.0 {
        colour = vec3<f32>(0.0045, 0.0042, 0.0040);
    }

    // The exposure, and the film's own curve, once: what the lens gathered is light, and this is
    // what the film made of the sum of it.
    var developed = colour * 0.46;
    developed = (developed * (2.51 * developed + 0.03)) / (developed * (2.43 * developed + 0.59) + 0.14);
    developed = clamp(developed, vec3<f32>(0.0), vec3<f32>(1.0));

    // Out of linear, and gentler than a bare 0.4545 power: the curve above has already placed the
    // midtone, and a second steep curve darkens the frame twice over.
    developed = pow(developed, vec3<f32>(0.4545));
    // A gentle S about the middle, so the lit side of a lemon and its shadow side are separated
    // rather than sitting at one value.
    developed = developed * developed * (3.0 - 2.0 * developed) * 0.26 + developed * 0.74;

    // The lens: a vignette that is a circle in the space the frame is actually in.
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    developed = developed * clamp(1.0 - dot(centred, centred) * 0.55, 0.0, 1.0);

    // And the grain, scaled by what it lands on, so it shows in the half tones and the deep
    // shadow stays clean, on the pixel lattice so that it does not crawl.
    let lattice = floor(in.uv * size);
    let grain = (hash21(lattice) - 0.5) * 0.040 * smoothstep(0.02, 0.34, dot(developed, vec3<f32>(0.33)));
    color = developed + vec3<f32>(grain);
"#;
