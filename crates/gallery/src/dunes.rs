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
//!   saturated as a near one and the field has no depth. The colour it washes towards is
//!   [`Atmosphere`]'s, which is the same number the sky is drawn with — see [`crate::air`]
//!   for what that is worth, and for the seam this scene used to have.
//! - **A grade at the end.** A filmic curve on the whole frame rather than a clamp. A
//!   clamped frame is white wherever a highlight lands; a graded one rolls off, and a
//!   sun low in the frame is bright without being a hole in the picture.

use definition::Value;
use scene::standard::Stage;

use document::air::Atmosphere;
use document::geometry::{fractal_noise, heightfield};
use document::kit::{
    camera, environment_light, image, instance, instances, light_with_radius, mesh, mesh_material,
    post_processes, screen_material, shaded_point_schema, surface,
};
use document::shaders::{Projection, mesh_shaders, post_shader};

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

/// The air this field is seen through.
///
/// # Invariants
///
/// The horizon colour is **the colour of the air**, not a second number that happens to look
/// like one. This scene's two copies of it were `vec3(0.58, 0.62, 0.72)` in the lighting
/// body and `vec3(0.70, 0.66, 0.60)` in the grade, under a comment in the first saying that a
/// distant surface tends towards the colour of the sky behind it. They are the same physical
/// quantity and they differed by a fifth, which is what put a bright seam along the ridge
/// where the field's far edge met the sky. One number now, and the seam is not avoided by
/// care at two call sites — there is only one call site.
///
/// The sun is written here because the sky is a function of it, and the light beside it
/// stands at a position rather than a bearing. Those are two decisions and they are
/// deliberately not made to agree: the sun's *bearing* is what the sky shows and the sun's
/// *place* is what lights the field, and a scene that derived one from the other would be a
/// scene that cannot light a field from behind it.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.13, 0.27, 0.56])
        .horizon([0.70, 0.66, 0.60])
        .ground([0.44, 0.40, 0.36])
        // The field is a plain and the air over it is the same all the way up, so the scale
        // height is long: a short one would put the haze on the ground and thin it away
        // above, which is a lake and not a desert.
        .density(0.0006)
        .scale_height(400.0)
}

/// Builds the dune field.
pub fn dunes() -> Value {
    let surface_shading = "";
    // One copy of the air, handed to the lighting stage and to the post stage. They describe
    // the same morning because they read the same constants, not because the scene was
    // careful twice.
    let helpers = format!("{}\n{}", HELPERS, atmosphere().wgsl());
    let grade = GRADE;

    Value::structure([
        ("name".to_owned(), Value::String("dunes".to_owned())),
        (
            "cameras".to_owned(),
            // The framing: the camera is above the field looking down across it, because a
            // camera inside the field compresses a heightfield into a band at the horizon.
            // It is cropped in — further out the whole field sits in shot with its edges just
            // inside, which reads as a slab floating in sky; cropped, what is left is a
            // landscape with no edge to it.
            //
            // The eye is **where the camera stands**, worked out against the field's own
            // extent and written down rather than left to be refitted. See `kit::camera`.
            Value::List(vec![camera(
                [70.865061882, 61.763768902, 142.827166405],
                [-15.0, 4.0, -25.0],
                std::f64::consts::FRAC_PI_3,
            )]),
        ),
        (
            // The sky as an **image**, which is what lets this scene render on a backend that
            // has no shader stage of its own. Latlong, because that is the one a path tracer
            // reads: an octahedral map handed to one is read as latlong and comes back as a
            // plausible frame with half the sky black and a hard vertical seam.
            //
            // Two to one, because `u` goes once round the world and `v` once over it; a square
            // latlong map is a distorted sky.
            //
            // The pixels are [`Atmosphere::environment_pixels`], and they are also what the
            // grade below reads its sky out of. The comment at `atmosphere` says the air is
            // handed to the lighting stage and the post stage, and this is the third hand it was
            // not being given: the two stages above are inside this renderer and the world is the
            // part another renderer reads.
            "images".to_owned(),
            Value::List(vec![image(
                "air",
                256,
                128,
                atmosphere().environment_pixels(256, 128),
                document::shaders::Projection::Latlong,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                // The sun, low and off to one side, so the crests catch it and the troughs are
                // in their own shadow. It stands well outside the field: a light inside the
                // volume it lights blows out everything near it. The shadow volume is wide
                // enough to hold the whole field, because a shadow map that stops short of the
                // scene's own edge draws a hard line across it.
                //
                // The radius is the distance to the far corner: 427 for the sun, 420 for the
                // fill, so each one's sphere holds the whole field. A light whose reach holds
                // everything the scene draws is promoted by the compiler to a **global**
                // light — in no cluster list, evaluated by every fragment — so neither number
                // is culling anything. Naming no radius at all would say the same thing and
                // say it more directly; what the number buys is that the scene states the
                // reach it means.
                // The power is the light's whole output over every direction, and the inverse square
                // law is what spreads it, so the number is a distance squared: it reads as
                // "as bright as the old brightness was at 217 units", which is where this
                // lamp stands from the middle of the field. See
                // `.agents/proposals/physical-light-units.md`.
                light_with_radius(
                    "sun",
                    [-150.0, 125.0, 95.0],
                    [1.0, 0.80, 0.55],
                    2_479_244.6,
                    175.0,
                    430.0,
                ),
                // The fill from the opposite side and from above was a lamp named `sky`, dim and
                // blue, **standing in for the sky** — and the reason it is written that way is the
                // honest one: *"one light with one falloff leaves every shadowed slope black, and
                // black is not what a shadow looks like outdoors."*
                //
                // It cannot be a sky, though, and the same sentence says why: a point light's
                // brightness is the square of its distance, and this field is 420 units to the far
                // corner. A lamp bright enough to lift the near troughs leaves the far plain black,
                // which is exactly the defect the lamp was added to cure. It is **replaced here,
                // not joined**: the environment light below reads this scene's own sky from the
                // image beside it, so the fill and the sky are one thing and cannot drift apart.
                //
                // Measured on the tracer, before this: the rasterised frame's top third carried
                // 155.8 of mean luma and the traced one 0.0, and the whole-frame correlation was
                // **-0.6585** — a picture whose bright part is the other picture's dark part.
                //
                // The radiance is **measured, not copied from `landscape`**. That scene's 0.55 was
                // never applied to it: the exporter read this quantity out of a field the schema
                // does not declare for an environment light, found nothing, and fell back to 1.0 —
                // so every world's brightness was unit until the field was removed. Sweeping the
                // attribute the tracer actually reads, against a raster sky of 155.7:
                //
                // | radiance | traced sky | gap |
                // |---|---|---|
                // | 0.55 | 79.6 | -76.1 |
                // | 1.00 | 146.1 | -9.7 |
                // | 1.05 | 153.4 | **-2.4** |
                // | 1.10 | 160.7 | +5.0 |
                // | 1.30 | 189.9 | +34.2 |
                //
                // A sky is a dome and a point light is a bulb, and this number is what tells the
                // difference: at 0.55 the field's shadowed slopes would go black again, which is
                // the fault the original `sky` lamp existed to cure.
                environment_light("air", "air", [1.0; 3], 1.06),
            ]),
        ),
        (
            "shaders".to_owned(),
            Value::List(
                mesh_shaders(surface_shading, &helpers)
                    .into_iter()
                    .chain(std::iter::once(post_shader(
                        grade,
                        &helpers,
                        Some(Projection::Latlong),
                    )))
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
                    Stage::Shadowing,
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
                    Stage::Depthing,
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
                    Stage::Lighting,
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

/// What the post stage does to the frame the scene produced.
///
/// # Invariants
///
/// **The air belongs here and not to a surface body.** `cycles_backend.md` decides that no
/// surface body may reach for `air_colour()` or `air_closure()`, because a surface that paints
/// its own fog is a surface a second backend cannot reproduce. `landscape` had its body removed
/// on that basis and the removal was measured; this scene was the last one still doing it.
///
/// A filmic curve rather than a clamp: the highlight rolls off towards one as it
/// approaches it, so a bright sky keeps its colour all the way to white instead of
/// turning into a flat white shape with an edge. A vignette is applied after the curve,
/// because a vignette applied before is itself curved and the two do not compose.
const GRADE: &str = r#"
    // The sky, for the pixels the scene never reached. A depth of one is the far plane,
    // which is where a depth buffer is cleared, so it is exactly the set of pixels no
    // geometry covered.
    //
    // Read out of the scene's own environment map at the environment light's own radiance,
    // which is what a path tracer draws its world from. Writing a sky law here instead gave
    // the two backends two different skies over one set of dunes, and no number could bring
    // them together: the map is the image, the radiance is the brightness, and this stage had
    // only been told about the ground.
    if depth >= 1.0 {
        let ray = ray_through(in.uv);
        color = environment_sky(ray);
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
