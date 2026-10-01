//! A dune field under a low sun.
//!
//! The scene is a heightfield, a low warm light and a cool sky, which is most of what a
//! landscape photograph is. What makes it read as depth rather than as a lit relief map
//! is three things, and all three are decisions a scene makes rather than features the
//! pipeline has:
//!
//! - **A low light.** A sun near the horizon rakes across the surface, so every slope
//!   turns away from it. A light overhead flattens a heightfield into a texture.
//! - **Atmospheric perspective.** Distance is shown by washing colour out towards the
//!   colour of the sky, not by making things smaller. Without it a far dune is exactly as
//!   saturated as a near one and the field has no depth.
//! - **A grade at the end.** A filmic curve on the whole frame rather than a clamp. A
//!   clamped frame is white wherever a highlight lands; a graded one rolls off, and a
//!   sun low in the frame is bright without being a hole in the picture.

use definition::Value;

use crate::gallery::geometry::{fractal_noise, heightfield};
use crate::gallery::kit::{
    camera, instance, instances, light, mesh, mesh_material, post_processes, screen_material,
    shaded_point_schema, surface,
};
use crate::gallery::shaders::{mesh_shaders, post_shader};

/// How far the field reaches, in world units.
const EXTENT: f64 = 165.0;

/// How many cells the field is divided into along each axis.
///
/// This is small for a landscape, and the reason is worth stating: the cost of a frame used to
/// be almost entirely the compile, and the compile was proportional to the number of points *as a
/// tree* rather than to the number that changed. A field twice as fine was four times the compile,
/// for detail the picture is too far away to resolve. The points are packed now, so the compile is
/// proportional to their bytes and a frame is limited by the render rather than by the compile —
/// but the picture is no more able to resolve the detail, so the divisions stay. See
/// [.agents/proposals/compile-cost.md].
const DIVISIONS: usize = 96;

/// Builds the dune field.
pub fn dunes() -> Value {
    let surface_shading = SURFACE;
    let helpers = HELPERS;
    let grade = GRADE;

    Value::structure([
        ("name".to_owned(), Value::String("dunes".to_owned())),
        (
            "cameras".to_owned(),
            // The padding crops in. At one the whole field is held in frame with its edges
            // just inside, which reads as a slab floating in sky; below one the edges fall
            // outside the frame and what is left is a landscape with no edge to it. The
            // camera is above the field looking down across it, because a camera inside
            // the field compresses a heightfield into a band at the horizon.
            Value::List(vec![camera(
                [95.0, 78.0, 190.0],
                [-15.0, 4.0, -25.0],
                std::f64::consts::FRAC_PI_3,
                0.46,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                // The sun, low and off to one side, so the crests catch it and the
                // troughs are in their own shadow. It stands well outside the field: a
                // light inside the volume it lights blows out everything near it.
                // The sun, off to one side, so the crests catch it and the troughs are
                // in their own shadow. It stands well outside the field: a light inside
                // the volume it lights blows out everything near it. The shadow volume is
                // wide enough to hold the whole field, because a shadow map that stops
                // short of the scene's own edge draws a hard line across it.
                light("sun", [-150.0, 125.0, 95.0], [1.0, 0.80, 0.55], 3.6, 175.0),
                // A fill from the opposite side and from above, dim and blue, standing in
                // for the sky. One light with one falloff leaves every shadowed slope
                // black, and black is not what a shadow looks like outdoors.
                light("sky", [130.0, 150.0, -95.0], [0.34, 0.44, 0.68], 0.5, 175.0),
            ]),
        ),
        (
            "shaders".to_owned(),
            Value::List(
                mesh_shaders(surface_shading, helpers)
                    .into_iter()
                    .chain(std::iter::once(post_shader(grade, "")))
                    .collect(),
            ),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![mesh(
                "field",
                "Triangles",
                shaded_point_schema(),
                heightfield(DIVISIONS, EXTENT, dune_height),
            )]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "mesh_materials".to_owned(),
            Value::List(vec![
                // One material per stage, because a material names one shader and a shader
                // belongs to one pass. The shadowing and depthing stages read no custom data,
                // so theirs is a placeholder the shaders ignore.
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
                mesh_material("shade", "lighting", surface(SAND, 0.0, 0.92, [0.0; 3])),
            ]),
        ),
        (
            "instances".to_owned(),
            // One instance per pass, all three drawing the same mesh: the geometry is
            // authored once and drawn three times, which is the whole point of a mesh being
            // separate from a placement. Which pass each is drawn in is the bucket it is in
            // rather than something a material's shader implies.
            instances(vec![
                (
                    "shadowing",
                    vec![instance(
                        "cast",
                        "field",
                        [0.0, 0.0, 0.0],
                        [0.0; 3],
                        [1.0; 3],
                        "cast",
                    )],
                ),
                (
                    "depthing",
                    vec![instance(
                        "depth",
                        "field",
                        [0.0, 0.0, 0.0],
                        [0.0; 3],
                        [1.0; 3],
                        "depth",
                    )],
                ),
                (
                    "lighting",
                    vec![instance(
                        "shade",
                        "field",
                        [0.0, 0.0, 0.0],
                        [0.0; 3],
                        [1.0; 3],
                        "shade",
                    )],
                ),
            ]),
        ),
    ])
}

/// The colour of lit sand, warmer where the light strikes it and cooler where it does not.
const SAND: [f64; 3] = [0.82, 0.58, 0.34];

/// The height of the field at a point on the ground.
///
/// # Invariants
///
/// The two noises are multiplied rather than added, so the crests are long and the
/// hollows between them are broad. Summed, two octaves of noise at different scales make
/// a field of small bumps, which is a gravel path rather than a dune.
fn dune_height(x: f64, z: f64) -> f64 {
    // A long wave along one axis, which gives the field a direction to run in, and a
    // finer wave across it for the ripples.
    let ridge = fractal_noise([x * 0.012, z * 0.004], 4, 11);
    let ripple = fractal_noise([x * 0.06, z * 0.05], 3, 97);
    let crest = ridge * ridge * (3.0 - 2.0 * ridge);
    crest * 26.0 + ripple * 1.8 + 2.0
}

/// What the lighting stage needs beyond the shading itself.
///
/// # Invariants
///
/// The noise is sampled in **world space along the ground**, not in screen space. A
/// cloud shadow in screen space slides across the picture as the camera moves, which
/// reads as a smear on the lens; the same noise in world space stays put on the ground and
/// the camera flies over it.
const HELPERS: &str = r#"
/// Returns a value in zero to one from a two dimensional coordinate.
///
/// This is a hash of the lattice point rather than of the coordinate, so the value is the
/// same everywhere within a cell and jumps only at a cell boundary. The smoothing below is
/// what turns those jumps into a slope.
fn hash(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

/// Returns smooth value noise in zero to one.
fn value_noise(point: vec2<f32>) -> f32 {
    let cell = floor(point);
    let within = fract(point);
    let blend = within * within * (3.0 - 2.0 * within);
    let corner = hash(cell);
    let east = hash(cell + vec2<f32>(1.0, 0.0));
    let north = hash(cell + vec2<f32>(0.0, 1.0));
    let far = hash(cell + vec2<f32>(1.0, 1.0));
    return mix(mix(corner, east, blend.x), mix(north, far, blend.x), blend.y);
}

/// Returns noise summed over three octaves, in roughly zero to one.
fn cloud(point: vec2<f32>) -> f32 {
    var total = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var octave = 0; octave < 3; octave = octave + 1) {
        total = total + amplitude * value_noise(point * frequency);
        frequency = frequency * 2.1;
        amplitude = amplitude * 0.5;
    }
    return total * 1.14;
}
"#;

/// What the lighting stage does to a surface after the light is applied.
///
/// # Invariants
///
/// Aerial perspective is a function of distance from the eye and of how much sky is
/// behind the surface, so a slope facing the camera washes out before one facing away.
/// Washing by distance alone flattens the field, because the far side of a dune and its
/// near side are the same distance from the eye but not the same distance through the air.
const SURFACE: &str = r#"
    // Sand is bright where the light rakes it and dark in the trough, which is a
    // property of the grain rather than of the geometry: the crests are also the
    // places the wind has scoured.
    let crest = clamp(normal.y, 0.0, 1.0);
    color = color * mix(0.82, 1.12, crest);

    // A cloud shadow crossing the field. The noise is sampled where the ground is and
    // drifts with the wind, so the shadow travels over the dunes rather than over the
    // picture. This is the whole of the scene's motion: a landscape has very little of its
    // own, and what it has is mostly weather passing over it.
    let drift = vec2<f32>(uniforms.time * 1.6, uniforms.time * 0.7);
    let cover = cloud(in.world_position.xz * 0.006 + drift);
    let shade = smoothstep(0.44, 0.78, cover);
    color = color * mix(0.30, 1.0, shade);

    // Aerial perspective. The colour a distant surface tends towards is the colour of
    // the sky behind it, and how much of it there is depends on how much of that sky
    // the surface is seen through. The names here are not the lighting stage's: this is
    // written into the same function, so a name used twice is a shader that will not
    // compile.
    let eye = uniforms.projection.position;
    let from_eye = length(eye - in.world_position);
    let view = normalize(in.world_position - eye);
    let facing = 1.0 - abs(dot(normal, view));
    let haze = 1.0 - exp(-from_eye * 0.0006);
    let haze_colour = vec3<f32>(0.58, 0.62, 0.72);
    color = mix(color, haze_colour, clamp(haze * (0.5 + 0.5 * facing), 0.0, 0.62));
"#;

/// What the post stage does to the frame the scene produced.
///
/// # Invariants
///
/// A filmic curve rather than a clamp: the highlight rolls off towards one as it
/// approaches it, so a bright sky keeps its colour all the way to white instead of
/// turning into a flat white shape with an edge. A vignette is applied after the curve,
/// because a vignette applied before is itself curved and the two do not compose.
const GRADE: &str = r#"
    // The sky, for the pixels the scene never reached. A depth of one is the far plane,
    // which is where a depth buffer is cleared, so it is exactly the set of pixels no
    // geometry covered.
    if depth >= 1.0 {
        let ray = ray_through(in.uv);
        // A sky is a gradient taken off the ray rather than off the pixel: a gradient in
        // screen space is a backdrop, and a backdrop does not move when the camera does.
        // Below the horizon the ground haze takes over, which is what stops the bottom of
        // the frame from being the same blue as the top.
        let above = pow(clamp(ray.y, 0.0, 1.0), 0.55);
        let horizon = vec3<f32>(0.70, 0.66, 0.60);
        let zenith = vec3<f32>(0.13, 0.27, 0.56);
        let below = vec3<f32>(0.44, 0.40, 0.36);
        let sky = select(mix(horizon, below, clamp(-ray.y * 4.0, 0.0, 1.0)),
                         mix(horizon, zenith, above),
                         ray.y > 0.0);
        color = sky;
    }

    // Reinhard, applied per channel. The desaturation a plain Reinhard curve causes is
    // put back afterwards, because the curve is there to hold the highlights and not to
    // decide what colour the picture is.
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.22), vec3<f32>(0.0), vec3<f32>(1.0));

    // An S curve about the middle. A tone curve only ever compresses towards the ends, so
    // what it leaves is a picture whose midtones all sit near the same value; the S is
    // what separates the lit crests from the shadowed troughs, and it is applied after
    // the curve because the curve is what puts the midtone in the middle to begin with.
    color = color * color * (3.0 - 2.0 * color) * 0.30 + color * 0.70;

    // A vignette, which is a function of how far the pixel is from the middle of the
    // frame, so it is a circle in the space the frame is actually in. It is a light touch:
    // a vignette strong enough to notice is a vignette that has become the subject.
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    let falloff = 1.0 - dot(centred, centred) * 0.28;
    color = color * clamp(falloff, 0.0, 1.0);
"#;
