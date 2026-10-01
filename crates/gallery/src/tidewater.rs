//! A lake at first light.
//!
//! The picture is a mirror: still water under a sky that is still dark overhead and already
//! burning along its bearing, mist lying on the surface, and a far shore reduced to a
//! silhouette. Almost nothing in it is an object. The whole frame is made of four things —
//! water, shore, reeds, air — and every one of them is a decision about light rather than
//! about geometry.
//!
//! # What makes it read as a photograph
//!
//! - **The horizon rides high.** A camera tilted down puts it there, and a camera tilted
//!   down means the near water is seen steeply and the far water is seen edge on. A steep
//!   angle is where you look *into* water and it is dark; a grazing angle is where it
//!   mirrors the sky and it is bright. The frame therefore runs from a dark foreground to a
//!   bright middle distance, which is what a lake looks like from a bank, and no amount of
//!   geometry produces it without the camera being placed rather than fitted.
//! - **One sky, seen twice.** The water reflects the sky and the frame's background *is*
//!   the sky. Written as two functions they will not agree, and where the far water meets
//!   the air there will be a seam — a bright line along the horizon, which is the commonest
//!   way a rendered landscape announces itself. [`sky_wgsl`] is written once and handed to
//!   both the lighting stage and the post stage, so the seam is not avoided but made
//!   impossible.
//! - **The air is in the picture.** The mist is a function of height above the water, so it
//!   lies on the lake and thins up the face of the far shore. The shore's own colour is then
//!   mostly whatever the air between the eye and it has made of it, which is most of what
//!   separates a silhouette from a black shape.
//!
//! # The two things that make the water
//!
//! The lake is a flat plane. Everything that makes it water happens per fragment: a normal
//! displaced by a few crossing swells, and a colour that is the sky reflected off it, mixed
//! by Fresnel against the dark body of the lake.
//!
//! The swells are **faded with distance from the eye**, and that is the whole trick. Past a
//! few hundred units a swell is smaller than a pixel, and sampling one anyway is not a
//! shimmer — it is a field of aliased sparkle over the far half of the lake, which is the
//! loudest artefact a water surface can have. Fading them leaves the far water a clean
//! mirror, which is both what a photograph shows and what an eye actually resolves at that
//! range.

use definition::Value;
use scene::standard::Stage;

use crate::geometry::{cone, fractal_noise, hash01, heightfield, normalized, ridged_noise};
use crate::kit::{
    camera_placed_on_lens, instance, instances, light, mesh, mesh_material, post_processes,
    screen_material, shaded_point_schema, surface,
};
use crate::shaders::{emission_shader, mesh_shaders_with_normal, post_shader};

use std::f64::consts::TAU;

/// Where the sun is, in world space.
///
/// # Invariants
///
/// This is the **only** place the sun is written down. The light that lights the scene and
/// the sky the water reflects are both derived from it, so they cannot disagree. A scene
/// that put a direction in its sky and a position in its light would be a scene in which
/// the two were permitted to drift apart, and at a low sun that drift is a visible seam
/// running the length of the horizon.
const SUN: [f64; 3] = [-150.0, 20.0, -700.0];

/// How far the lake reaches, in world units.
const EXTENT: f64 = 1400.0;

/// How many cells the lake's surface is divided into.
///
/// The water is flat, so its tessellation buys nothing geometrically and costs vertex
/// processing for the whole plane. What it has to do is reach past the far shore, so that
/// the lake's own edge is never the line at which the sky begins.
const WATER_DIVISIONS: usize = 64;

/// How high the far shore stands above the water.
const RIDGE_HEIGHT: f64 = 44.0;

/// How far off the lake the far shore stands.
const RIDGE_DISTANCE: f64 = -700.0;

/// The height of the far shore at a point.
///
/// # Invariants
///
/// Ridged noise rather than smooth, because a wooded skyline is a line and not a swell, and
/// the **same** function places the trees on it — a tree standing at a height the ground is
/// not floats, and one buried in it is a stump.
///
/// The falloff in `z` is what stops the mesh being a wall. A profile extruded along the view
/// axis is a plateau with a cliff at its far edge, and the silhouette of a plateau is not a
/// thing. The land is a hill instead, which comes to the water at both ends of its mesh, and
/// the `-6` term puts everything the hill does not cover well under the lake so the near end
/// is not a hummock standing out of the water beside the reed bed.
fn shore_height(x: f64, z: f64) -> f64 {
    let along = ridged_noise([x * 0.0024, 0.0], 4, 31) * 0.84
        + ridged_noise([x * 0.0125, 4.0], 3, 77) * 0.16;
    // The crests are raised towards their own maximum, so the skyline has peaks rather than
    // a row of similar bumps. A ridged noise spends most of its range low; taken straight it
    // gives a shore that is a plateau with a ragged edge, which is not a landscape.
    let across = (-(z - RIDGE_DISTANCE).powi(2) / (2.0 * 210.0 * 210.0)).exp();
    along.powf(0.55) * RIDGE_HEIGHT * across - 9.0 * (1.0 - across)
}

/// Builds the lake.
pub fn tidewater() -> Value {
    // The same air, the same sun and the same hash in the surface that lights the scene and
    // in the grade that finishes it. Three shader bodies, one source for the things they
    // must agree about.
    let helpers = shared_helpers();

    Value::structure([
        ("name".to_owned(), Value::String("tidewater".to_owned())),
        // The hour the scene is drawn at, and the only reason the mist and the water move.
        // A scene that names no time is drawn at a moment that is not a number, and every
        // stage that reads one gets a value nothing can be computed from.
        ("time".to_owned(), Value::Float(0.0)),
        // Three by two rather than the square the pipeline has always drawn: a horizon on
        // the upper third of a square is a horizon in the middle of a photograph, and the
        // aspect of a frame is a compositional decision rather than a constant.
        (
            "frame".to_owned(),
            Value::structure([
                ("width".to_owned(), Value::Int(1536)),
                ("height".to_owned(), Value::Int(1024)),
            ]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![
                // Standing on the bank, four units up, tilted down until the far shore rides
                // on the upper third. A fitted camera pulls back until the reeds and the
                // shore both fit, which is a view of a lake from a helicopter.
                //
                // The eye height is the one that matters. Two and a half units put the near
                // water almost edge on, which is the mirror a photographer wants and the
                // reed bed a curtain they do not: at that height a reed forty units away
                // stands higher in the frame than the far waterline, and the frame stops
                // reading as sky, shore, lake and foreground in four separate bands. Four
                // units is what a photographer standing on a bank actually has.
                //
                // Thirty five millimetres on a full frame negative, which is the lens a
                // photographer would name for a view this wide. The angle it works out to
                // is not written here: it is the vertical angle of that lens on that
                // negative in *this* frame's aspect, and an author who wrote the angle
                // would be quoting a number off a calculator rather than a lens.
                camera_placed_on_lens(
                    [-30.0, 9.00, 240.0],
                    [-26.0, -33.0, -160.0],
                    35.0,
                    36.0,
                    0.0,
                    0.4,
                    2400.0,
                ),
                // The same bank on a longer lens, framed on the treeline. A second camera
                // is a second frame of the same scene rather than a second scene, which is
                // what makes it a lens choice instead of a re-render.
                //
                // A hundred and fourteen millimetres is the same view narrowed to about a
                // third of the angle, and it is the lens that turns the treeline from a
                // band into a subject. Nothing about the scene changed between the two
                // frames but the number the camera was holding.
                camera_placed_on_lens(
                    [-30.0, 9.00, 240.0],
                    [-110.0, 22.0, -700.0],
                    114.0,
                    36.0,
                    0.0,
                    0.4,
                    2400.0,
                ),
            ]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                // The sun, just behind the far shore and a long way off, so it is very
                // nearly parallel and everything it lights is rimmed rather than filled.
                // The intensity is large because the falloff below is a constant of the
                // standard shading rather than anything the document can set: at this range
                // it attenuates by something over twenty, and what a scene sets is therefore
                // a radiance at a chosen distance, not a power. See the proposal.
                light("sun", SUN, [1.0, 0.55, 0.28], 34.0, 900.0),
                // The sky, which at this hour is doing most of the lighting. It is dim, cold
                // and from above: a scene with one light leaves every shadowed slope black,
                // and black is not what a shadow looks like outdoors.
                //
                // It is also the whole of the difference between a reed bed and a fence of
                // white sticks. The standard shading's Fresnel goes to one at a grazing
                // angle, and a reed is seen almost entirely edge on, so a thin near object
                // takes the full specular term off this light while its diffuse term is a
                // fiftieth of it. Dim the fill and the reeds go dark; leave it and no tint
                // on the material can bring them back.
                light(
                    "sky",
                    [260.0, 420.0, 300.0],
                    [0.30, 0.42, 0.70],
                    13.0,
                    1400.0,
                ),
            ]),
        ),
        (
            "shaders".to_owned(),
            Value::List(
                mesh_shaders_with_normal(REED_SWAY, SHORE, &helpers)
                    .into_iter()
                    .chain(std::iter::once(emission_shader(
                        "water",
                        &water_body(),
                        &helpers,
                    )))
                    .chain(std::iter::once(post_shader(GRADE, &helpers)))
                    .collect(),
            ),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![
                // The lake: a flat heightfield, which is the cheapest honest plane there
                // is. Every normal in it is the same, which is what leaves each fragment
                // free to decide for itself what the water is doing.
                mesh(
                    "water",
                    "Triangles",
                    shaded_point_schema(),
                    heightfield(WATER_DIVISIONS, EXTENT, |_, _| 0.0),
                ),
                mesh(
                    "shore",
                    "Triangles",
                    shaded_point_schema(),
                    heightfield(128, 760.0, shore_height),
                ),
                // The near bank: the one dark shape in the corner that the rest of the frame
                // is composed around.
                mesh(
                    "bank",
                    "Triangles",
                    shaded_point_schema(),
                    heightfield(72, BANK_EXTENT, bank_height),
                ),
                // A tree on the far shore: further away than anything else in the frame,
                // and correspondingly forgiving about what shape it is.
                mesh(
                    "tree",
                    "Triangles",
                    shaded_point_schema(),
                    cone(7, 1.0, 1.0),
                ),
            ]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "mesh_materials".to_owned(),
            Value::List(vec![
                mesh_material(
                    "cast",
                    "shadowing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                mesh_material(
                    "depth",
                    "depthing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                // The shore and the reeds are one material between them: both are matte,
                // both are nearly black, and the only thing that separates them is how much
                // air lies between the eye and them. They are one pass.
                mesh_material(
                    "shore",
                    "lighting",
                    surface(SHORE_TINT, 0.0, 0.97, [0.0; 3]),
                ),
                // The water is not lit, and saying so is the whole trick. A mirror's colour is
                // almost entirely the sky it reflects, so drawing it in a **lighting** pass
                // adds that colour once per light: under this scene's two lights the lake is
                // twice as bright as the sky above it, which is how it went white the first
                // time. The emission stage runs exactly once, which is what a surface whose
                // colour does not come from a light is for.
                mesh_material("water", "water", surface([0.0; 3], 0.0, 0.02, [0.0; 3])),
            ]),
        ),
        ("instances".to_owned(), instances(instance_buckets())),
    ])
}

/// The colour of the shore and the reeds: the colour of a thing with no light on it.
const SHORE_TINT: [f64; 3] = [0.010, 0.011, 0.014];

/// Where the near bank is placed, and how far its mesh reaches.
const BANK_AT: [f64; 3] = [-40.0, 0.0, 196.0];

/// How far the near bank reaches, in world units.
const BANK_EXTENT: f64 = 60.0;

/// The height of the near bank above the water.
///
/// # Invariants
///
/// The bank is a **mound that falls away to below the waterline** at every edge, rather than
/// a patch with a rim. A patch with a rim is a raft: its edge is a vertical wall standing in
/// the lake, and no amount of shading makes that read as ground. Falling away means the last
/// row of points is under the surface and the shoreline is wherever the height crosses zero,
/// which is the only way a heightfield can have one.
///
/// It is a *bank* rather than a bed of reeds, and the reason is photographic before it is
/// technical. A foreground wants one dark shape in a corner; three hundred thin stalks want
/// three hundred shapes spread across the frame, and a stalk seen edge on takes the whole of
/// the standard shading's grazing Fresnel, so it renders as a pale stick however dark its
/// material is. A broad surface seen at an angle is a shape, and it stays one.
fn bank_height(x: f64, z: f64) -> f64 {
    let radius = ((x * x) / (28.0 * 28.0) + (z * z) / (24.0 * 24.0)).sqrt();
    // A smooth dome that goes under the water before it reaches the mesh's edge.
    let dome = (1.0 - radius * radius).max(0.0);
    let rise = 4.5 * dome * dome;
    // Two octaves of noise on the top of it, so the crest is a landform and not a hemisphere.
    let rough = fractal_noise([x * 0.075, z * 0.075], 3, 53) * 1.1
        - fractal_noise([x * 0.22, z * 0.22], 2, 91) * 0.4;
    rise + rough * dome - 2.4
}

/// Builds the instance buckets: the water, and everything that stands in it.
///
/// # Invariants
///
/// The lake is **not** in the `shadowing` bucket, and it is drawn by the `emission` stage
/// rather than the `lighting` one.
///
/// A shadow map holds what is put in it, and a flat plane a thousand units across and a
/// couple of degrees off the light is the worst case for shadow acne there is: its own depth
/// changes by more than a shadow texel across every texel, and the bias that would fix it
/// detaches every other shadow in the frame. Leaving the water out costs nothing, because a
/// flat lake casts no shadow a viewer could see — and what the lookup then returns is the far
/// shore's shadow lying across it, which is the effect actually worth having.
///
/// The stage is the other half of the same decision, and the reason it took two attempts to
/// get right: a mirror's colour is almost entirely the sky it reflects, so a lighting pass
/// adds that colour once per light. Under two lights the lake came out twice as bright as
/// the sky above it, and it went white.
///
/// Everything else is in all three buckets, so the geometry is authored once and drawn three
/// times, which is the whole of what a mesh being separate from a placement is for. The
/// three lists are the same placements under three materials, because a material names one
/// shader and a shader belongs to one stage — and under three sets of labels, because a
/// label names one instance in the whole scene rather than one per pass.
fn instance_buckets() -> Vec<(Stage, Vec<Value>)> {
    let water = |material: &str| {
        instance(
            format!("{material}-water"),
            "water",
            [0.0, 0.0, 0.0],
            [0.0; 3],
            [1.0; 3],
            material,
        )
    };
    let mut depth = vec![water("depth")];
    depth.extend(standing_instances("depth"));
    let lighting = standing_instances("shore");

    vec![
        (Stage::Shadowing, standing_instances("cast")),
        (Stage::Depthing, depth),
        (Stage::Lighting, lighting),
        (Stage::Emission, vec![water("water")]),
    ]
}

/// The far shore, its trees, and the near reed bed, drawn with one material.
///
/// # Invariants
///
/// Every placement is a deterministic function of its index, because a scene that scatters
/// a thousand things has to draw the same thousand things on every compile. A scatter from
/// a random source is a scene whose delta between two otherwise identical frames is the
/// whole document.
fn standing_instances(material: &str) -> Vec<Value> {
    let mut standing = vec![instance(
        format!("{material}-shore"),
        "shore",
        [0.0, 0.0, 0.0],
        [0.0; 3],
        [1.0; 3],
        material,
    )];

    // The treeline. Each cone stands on the heightfield it sits on, and the scatter is
    // dense enough to close into a line and irregular enough not to read as one.
    for index in 0..210i64 {
        let along = (index as f64 / 210.0 - 0.5) * 1500.0 + (hash01(index, 7, 5) - 0.5) * 11.0;
        let z = RIDGE_DISTANCE + (hash01(index, 11, 9) - 0.5) * 60.0;
        let height = 6.0 + hash01(index, 3, 13) * 6.0;
        standing.push(instance(
            format!("{material}-tree-{index}"),
            "tree",
            [along, shore_height(along, z) - 2.0, z],
            [0.0, hash01(index, 2, 17) * TAU, 0.0],
            [height * 0.32, height, height * 0.32],
            material,
        ));
    }

    // The near bank, in the corner the frame is built around. It is the darkest thing in the
    // picture and the only thing near enough to be sharp, which is what anchors the
    // composition: a frame of pure distance has nothing to stand on.
    standing.push(instance(
        format!("{material}-bank"),
        "bank",
        BANK_AT,
        [0.0, 0.34, 0.0],
        [1.0; 3],
        material,
    ));
    standing
}

/// The WGSL the lighting stage and the post stage share.
///
/// # Invariants
///
/// Everything here is a pure function of `uniforms`, and `uniforms` is the one thing both
/// stages have in common apart from the projection. That constraint is what lets a single
/// copy be declared once and used by a lighting pass and a post pass at the same time —
/// which is the only way two stages that draw the same air can be made to agree about it.
fn shared_helpers() -> String {
    format!("{}\n{}\n{}", HASH_WGSL, sky_wgsl(), AERIAL_WGSL)
}

/// The sun's direction and the sky it lights, as WGSL both stages use.
///
/// # Invariants
///
/// The gradient is taken off the **ray** and not off the pixel. A gradient in screen space
/// is a backdrop: it does not move when the camera does, it does not compress towards the
/// horizon as the camera tilts, and a water surface that reflects it is reflecting a wall.
/// A ray is what the sky is a function of.
fn sky_wgsl() -> String {
    let sun = normalized(SUN);
    let azimuth = normalized([sun[0], 0.0, sun[2]]);
    format!(
        r#"
/// The sun, as a direction. The same number the scene placed its light at.
const SUN_DIRECTION: vec3<f32> = vec3<f32>({sun_x:.9}, {sun_y:.9}, {sun_z:.9});
const SUN_BEARING: vec3<f32> = vec3<f32>({bearing_x:.9}, 0.0, {bearing_z:.9});

/// The colour of the sky along a ray from the eye.
fn sky_colour(ray: vec3<f32>) -> vec3<f32> {{
    let up = clamp(ray.y, -1.0, 1.0);

    // The vertical gradient, compressing towards the horizon rather than running straight
    // there: the air is deep, and a sky changes fastest in the last few degrees above the
    // ground.
    let height = pow(clamp(up, 0.0, 1.0), 0.42);
    let zenith = vec3<f32>(0.010, 0.022, 0.062);
    let horizon = vec3<f32>(0.062, 0.076, 0.125);
    var colour = mix(horizon, zenith, height);

    // The warm band, lying along the sun's bearing and hugging the horizon. This is the
    // part of a dawn that is a photograph's subject and a gradient's footnote, and it is
    // what the water carries back down to the eye along the whole length of the frame.
    //
    // The exponent is very high on purpose. The mist takes its colour from whatever is
    // behind it, so the width of this band is the width of the warmth in the *whole* frame:
    // a gentle falloff spreads it across the visible sky, the lake carries it back, and a
    // dawn becomes a noon with a warm filter over it. It has to be a halo, not a wash.
    let bearing = max(dot(normalize(vec3<f32>(ray.x, 0.0, ray.z) + vec3<f32>(1e-5)), SUN_BEARING), 0.0);
    let band = pow(bearing, 260.0) * exp(-max(up, 0.0) * 11.0);
    colour = mix(colour, vec3<f32>(0.58, 0.240, 0.080), clamp(band * 0.92, 0.0, 1.0));

    // Below the horizon there is no sky, only more air, so the gradient runs on into a
    // ground haze. Stopping it at the horizon leaves a hard line down the middle of a frame
    // whose whole subject is the air.
    colour = mix(colour, vec3<f32>(0.042, 0.045, 0.055), smoothstep(0.0, -0.045, up));

    // The sun as a small disc, and behind the far shore here — what a low sun contributes
    // to a frame is the band, not the disc. Half a degree is the real one; drawn larger it
    // is the one element in a dawn photograph that immediately looks drawn.
    colour = colour
        + vec3<f32>(1.0, 0.68, 0.40) * smoothstep(0.999955, 0.999985, dot(ray, SUN_DIRECTION)) * 14.0;
    return colour;
}}
"#,
        sun_x = sun[0],
        sun_y = sun[1],
        sun_z = sun[2],
        bearing_x = azimuth[0],
        bearing_z = azimuth[2],
    )
}

/// A hash of a lattice point, shared by the surface and the grade.
const HASH_WGSL: &str = r#"
/// Returns a value in zero to one from a two dimensional coordinate.
///
/// A hash of the lattice point rather than of the coordinate, so the value is the same
/// everywhere within a cell and jumps only at a boundary. A grain built from the continuous
/// coordinate instead is a smooth smear, and reads as a dirty lens rather than as film.
fn hash(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
"#;

/// The mist, as WGSL both stages use.
///
/// # Invariants
///
/// Density falls off with **height above the water**, which is the only reason mist lies on
/// a lake rather than hanging in it, and the falloff is steep enough that the far shore
/// stands clear of it. A fog with no height term is a wash of grey over everything at every
/// distance, which reads as a dirty lens rather than as weather.
const AERIAL_WGSL: &str = r#"
/// How much of the sky a point at this distance through this air is seen through.
///
/// `height` is the point's own height above the water and not its distance from the eye. A
/// mist that is a function of distance alone is a fog that is a function of the camera, and
/// moving the camera changes the weather.
fn aerial_penetration(distance: f32, height: f32, facing: f32) -> f32 {
    // The mist burns off. This and the water are the scene's whole motion, and a dawn whose
    // fog never lifts is a fog rather than a morning: what changes over half a minute is the
    // air, not the land.
    let thinning = 1.0 - 0.55 * smoothstep(0.0, 24.0, uniforms.time);
    // The height term is what makes this mist rather than fog. A lake's mist lies in the
    // first few units of air above the water and a treeline stands well clear of it, so the
    // falloff has to be steep enough that a tree fifteen units up is barely in it at all —
    // otherwise the far shore dissolves into the sky at exactly the distance where a
    // silhouette is the whole of the photograph, and the frame has no dark in it anywhere.
    let density = exp(-max(height, 0.0) * 0.22) * 0.00026 * thinning;
    // More air along a line that grazes the surface than along one that goes into it, which
    // is the same reason a distant road is paler than a near one.
    let slant = 1.0 + facing * 2.4;
    return 1.0 - exp(-distance * density * slant);
}

/// The colour the air has made a surface at this distance, seen along this way.
///
/// The fog takes the colour of the sky **in the direction of the surface** and not a
/// constant. This is what closes the horizon: a far surface and the air it is seen against
/// are then the same colour by the same function, and a mist of one flat grey is the thing
/// that produces a visible seam instead.
fn aerial_colour(eye: vec3<f32>, point: vec3<f32>) -> vec3<f32> {
    let ray = normalize(point - eye);
    return sky_colour(vec3<f32>(ray.x, max(ray.y, -0.03), ray.z));
}
"#;

/// The normal a wind-blown surface carries, displaced per fragment.
///
/// # Invariants
///
/// The swells are summed as **slopes** rather than as heights, because a surface's normal is
/// the gradient of its height and taking the gradient directly is cheaper than adding four
/// waves up and differentiating the sum.
///
/// The amplitude is faded with distance from the eye. A swell shorter than a pixel cannot be
/// resolved, and sampling one anyway is not a shimmer: it is a field of aliased sparkle over
/// the far half of every lake. Fading it leaves the far water a mirror, which is both what a
/// photograph shows and what the eye resolves at that range.
const WATER_NORMAL: &str = r#"
    // Four crossing swells: two long ones that give a lake its slow breathing, and two short
    // ones that break the reflection up close and are nothing at all a long way off.
    //
    // The wavelengths are the part that matters and the part that is easiest to get wrong. A
    // swell tens of units long is not a wave on a lake, it is a **fold in the ground**: at
    // sixty units from the eye one of them spans a sixth of the frame, and four of them
    // read as a picket fence standing in the water rather than as a surface. A lake's
    // texture is a unit or two across, and everything longer than that is the shape of the
    // lake, not of its surface.
    let drift = vec2<f32>(uniforms.time * 0.9, uniforms.time * 0.35);
    let point = in.world_position.xz + drift;
    var slope = vec2<f32>(0.0, 0.0);

    slope += vec2<f32>(0.82, 0.57) * 0.0130
        * cos(dot(point, vec2<f32>(0.82, 0.57)) * 0.55 - uniforms.time * 0.55);
    slope += vec2<f32>(-0.44, 0.90) * 0.0078
        * cos(dot(point, vec2<f32>(-0.44, 0.90)) * 0.95 + uniforms.time * 0.72);
    slope += vec2<f32>(0.95, -0.31) * 0.0034
        * cos(dot(point, vec2<f32>(0.95, -0.31)) * 3.00 - uniforms.time * 1.90);
    slope += vec2<f32>(0.13, 0.99) * 0.0021
        * cos(dot(point, vec2<f32>(0.13, 0.99)) * 5.50 + uniforms.time * 2.40);

    // What is resolved is what is drawn. Past a couple of hundred units a swell is smaller
    // than a pixel, and sampling one anyway is not a shimmer: it is a field of aliased
    // sparkle over the far half of every lake. Fading it leaves the far water a mirror,
    // which is both what a photograph shows and what the eye resolves at that range.
    let wave_range = distance(uniforms.projection.position, in.world_position);
    slope = slope * exp(-wave_range * 0.0090);
    wave_normal = normalize(vec3<f32>(-slope.x, 1.0, -slope.y));
"#;

/// The body the emission stage runs for the lake, the waves and the colour together.
///
/// # Invariants
///
/// Water is a mirror mixed against the body of the lake by Fresnel, and those two terms are
/// the whole of it: the reflection is the sky, and the body is what little light comes back
/// out of the lake. Water is unusual in reflecting only a few percent of what is above it
/// looking straight down and very nearly all of it at a grazing angle, and that difference is
/// the entire reason a lake reads as a lake — from above you see into it, and along it you
/// see only sky. One Fresnel term produces both, and two hand-written ones would not agree
/// at the angle where they meet.
///
/// The glitter is taken against the sun's **direction** and not against a light's position.
/// A sun a thousand units away is very nearly parallel, and the difference between the two is
/// the difference between a glitter path that runs to the horizon and one that stops halfway
/// across the lake.
fn water_body() -> String {
    format!(
        r#"
    // The surface the lake is drawn with. The mesh is a flat plane and every normal in it is
    // the same, so whatever the water is doing is decided here, per fragment.
    var wave_normal = in.normal;
{WATER_NORMAL}
    let eye = uniforms.projection.position;
    let to_eye = eye - in.world_position;
    let from_eye = length(to_eye);
    let view = to_eye / from_eye;

    let facing = 1.0 - clamp(dot(wave_normal, view), 0.0, 1.0);
    let water_fresnel = 0.02 + 0.98 * pow(facing, 5.0);

    // The sky, reflected. The reflected ray is the one that leaves the surface, so the waves
    // are what break the reflection up; where they are not resolved the reflection is whole
    // again and the far lake goes to glass.
    let reflected = reflect(-view, wave_normal);
    let mirrored = sky_colour(vec3<f32>(reflected.x, max(reflected.y, 0.003), reflected.z));

    // The body of the lake: what is under the surface, which is nearly nothing, and which
    // is tinted by whatever the water is carrying this morning.
    let deep = vec3<f32>(0.010, 0.016, 0.021);

    var water = mix(deep, mirrored, water_fresnel);

    // The sun's own glitter: a specular lobe on the displaced normal, so it is the one place
    // the swells show up as light rather than as a broken reflection.
    //
    // The exponent is moderate and the amplitude small, and both matter. A very tight lobe on
    // a wave-modulated normal lights up wherever a wave's slope happens to match the half
    // vector, which is not a glitter path but a field of hard streaks all across the lake;
    // a loose one spreads the sun's reflection over the whole surface and there is no path at
    // all. The sun here is behind the far shore, so what this buys is the column of light
    // running from the horizon towards the eye, which is the thing a photograph of water at
    // dawn is actually made of.
    let glitter_half = normalize(SUN_DIRECTION + view);
    let glitter = pow(max(dot(wave_normal, glitter_half), 0.0), 240.0) * 0.5;
    water = water + vec3<f32>(1.0, 0.66, 0.40) * glitter;

    // The mist, over the top of both. A lake with air lying on it is not a mirror but a
    // sheet, and the reflection softens into the distance rather than stopping at it.
    let penetration = aerial_penetration(from_eye, in.world_position.y, facing);
    emitted = mix(water, aerial_colour(eye, in.world_position), clamp(penetration, 0.0, 0.97));
"#
    )
}

/// The sway a reed carries, displacing the normal it is lit by.
///
/// # Invariants
///
/// The first thing that shows when a reed moves is the light on it changing, and that is the
/// half of the motion a displaced normal can honestly express: nothing here moves a point, so
/// the silhouette is still and only the shading breathes. A scene that wanted the outline to
/// move as well would be displacing in the vertex stage, which is a different hook and is
/// what the proposal asks for.
const REED_SWAY: &str = r#"
    let sway = sin(uniforms.time * 0.85
        + in.world_position.x * 0.42
        + in.world_position.z * 0.27) * 0.055;
    let turned = vec2<f32>(
        normal.x * cos(sway) + normal.z * sin(sway),
        -normal.x * sin(sway) + normal.z * cos(sway));
    normal = normalize(vec3<f32>(turned.x, normal.y, turned.y));
"#;

/// What the lighting stage makes of a fragment of shore or reed.
///
/// # Invariants
///
/// Neither is lit by much but the sun, and both are seen against a sky far brighter than
/// they are, so most of what a fragment of shore looks like is the air in front of it. The
/// tint is nearly black on purpose: a silhouette is not a dark object but an object with
/// the sky's colour *around* it and very little of its own, and a shore given a legible
/// surface colour reads as a painted backdrop instead.
const SHORE: &str = r#"
    // A backlit shore is a silhouette, and a silhouette is mostly the air in front of it.
    //
    // The shading is scaled down rather than the tint taken to nothing, and the reason is
    // the standard shading's specular: it is not normalised by roughness, so a rough surface
    // has a *broad* lobe at the *same* peak, and its Fresnel goes to one at a grazing angle.
    // A thin vertical stalk is seen almost entirely edge on, so it takes the whole of that
    // peak while its diffuse term is a fiftieth of it — and no tint on the material can
    // bring it down, because the tint is not in the specular.
    color = color * 0.55;

    let eye = uniforms.projection.position;
    let to_eye = eye - in.world_position;
    let from_eye = length(to_eye);
    let view = to_eye / from_eye;

    // And then the air puts the sky back, more the further off and the lower down it is.
    // This is the whole of what a distant shore is made of.
    let facing = 1.0 - clamp(dot(normal, view), 0.0, 1.0);
    let penetration = aerial_penetration(from_eye, in.world_position.y, facing);
    color = mix(color, aerial_colour(eye, in.world_position), clamp(penetration, 0.0, 0.985));
"#;

/// What the post stage does to the frame the scene produced.
///
/// # Invariants
///
/// The order is the reason this reads as a photograph rather than as a render with a filter
/// on it:
///
/// - **The sky, from the same function the water reflected.** Everything the depth prepass
///   never reached.
/// - **Exposure, then white balance, then a filmic curve.** A curve is what lets a sun that
///   is genuinely brighter than the frame roll off into white with its colour still in it.
/// - **Grain and a vignette last**, because they belong to the film rather than to the
///   scene. Grain over a clean gradient is the whole difference between an image and a
///   picture of an image: a frame without it has a mathematical smoothness the eye has no
///   other way to account for than by deciding it is not a photograph.
const GRADE: &str = r#"
    // The sky, and only the sky: the pixels no geometry covered. A depth of one is the far
    // plane, which is where a depth buffer is cleared.
    if depth >= 1.0 {
        color = sky_colour(ray_through(in.uv));
    }

    // Exposure. A photograph is exposed once and this is the one place it is decided; the
    // lights above are a radiance at a chosen distance, which is a different thing.
    color = color * 1.00;

    // The balance of an hour before sunrise: the frame overall cool, and the warmth kept in
    // the highlights, which is where a warm light actually lands.
    let before = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = color * mix(vec3<f32>(0.90, 0.97, 1.15), vec3<f32>(1.06, 1.00, 0.94),
        clamp(before * 2.2, 0.0, 1.0));

    // A filmic curve rather than a clamp, per channel. A clamped frame is white wherever a
    // highlight lands, with an edge to it; a curved one holds the colour all the way up.
    color = (color * (2.51 * color + 0.03)) / (color * (2.43 * color + 0.59) + 0.14);
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    // The transfer out of linear, which is deliberately gentler than the 0.4545 power the
    // earlier scenes use: a filmic curve has already placed the midtone, and applying a
    // second, steeper curve to it darkens the whole frame twice over.
    color = pow(color, vec3<f32>(0.4545));

    // A gentle S about the middle. A tone curve only ever compresses towards the ends, so
    // what it leaves is a frame whose midtones all sit at one value; the S is what
    // separates a lit crest from a shadowed one.
    color = color * color * (3.0 - 2.0 * color) * 0.22 + color * 0.78;

    // A vignette, a function of how far the pixel is from the middle of the frame, so it is
    // a circle in the space the frame is actually in. Light: a vignette strong enough to
    // notice is a vignette that has become the subject.
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.40, 0.0, 1.0);

    // Grain, and it is last for the reason the vignette is. It is scaled by the brightness
    // of whatever it lands on, so it shows in the sky and the mist, which is where a
    // photographer's own grain shows, and is nearly absent in the blacks, which is where
    // theirs is not. The lattice is the pixel grid, so a grain that shifts with the camera
    // does not move across the frame.
    let seed = floor(in.uv * size) + vec2<f32>(fract(uniforms.time * 0.37) * 311.0);
    let grain = (hash(seed) - 0.5) * 0.055 * smoothstep(0.0, 0.28, dot(color, vec3<f32>(0.33)));
    color = color + vec3<f32>(grain);
"#;
