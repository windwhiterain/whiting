//! A massif authored as a ridge **network**, standing on a plain.
//!
//! # What this scene is for
//!
//! It is the first caller of the `mountain` crate, and the point of it is that the geometry is
//! **authored rather than sampled**. Every other landscape in the gallery is a height function:
//! one call per grid point, and the shape of the mountain is whatever the arithmetic did. Here
//! the ridges and the valleys are lines that were placed, and the mesh is the expansion of those
//! lines — so the ridge lines in the picture are the lines that were written, and moving one
//! moves the mountain.
//!
//! # What the expansion gives the scene, and what the scene has to give back
//!
//! The crate hands over a [`FacetMesh`]: triangles, each one a planar block face, each carrying
//! the belt it came from and how rugged its crest is. The scene's job is the half the crate
//! deliberately does not do — turning that into a document the standard pipeline draws — and the
//! one decision in it is that **the normal is the face's own and is written per corner**.
//!
//! That is the whole of the block-face reading. A generator that averaged normals across the
//! shared vertices, as [`crate::geometry::heightfield`] does and must, would shade the massif as
//! one smooth surface with steps in it, and the facets the crate exists to produce would be
//! invisible.
//!
//! # The plain
//!
//! The massif's mesh ends at its footprint, so on its own it is a slab of rock floating over
//! nothing. It therefore stands on a plain: a heightfield whose height is **zero everywhere the
//! massif reaches**, which is why the rim is levelled before the mesh is built. The two surfaces
//! meet along the rim rather than one hovering over the other, and the seam between them is the
//! massif's own boundary rather than a crack between two pieces of arithmetic.

use definition::{Schema, Value};

mod massif;
use mountain::line::Side;
use mountain::tessellation::BeltRef;
use mountain::{FacetMesh, Massif};
use scene::standard::Stage;

use crate::air::Atmosphere;
use crate::geometry::{fractal_noise, heightfield, smoothstep};
use crate::identity::{self, Identity, massif_extras, outputs_field, plain_extras};
use crate::kit::{
    camera_placed, directional, instance, instances, light_with_radius, mesh, mesh_material,
    post_processes, screen_material, shaded_point_schema, surface,
};
use crate::shaders::{Block, Extras, lighting_shader_declaring, mesh_shaders_over, post_shader};

/// The scene document's `outputs` field, whose name the compiler's schema fixes.
const OUTPUTS: &str = "outputs";

/// Returns one mesh's four stage shaders, with the lighting one writing beside the frame.
///
/// # Invariants
///
/// - **A scene with two meshes needs two of these, and they cannot be merged.** The stages read
///   the *point layout of the mesh they draw*, so a plain drawn by a shader built over the
///   massif's wider point schema reads three attributes of somebody else's vertices. Nothing
///   checks a vertex layout against the mesh a pass draws, so the device catches it as a layout
///   disagreement — which is the good outcome, and not one to rely on.
///
/// - **The three other stages are the shared ones, not replacements.** They are kept rather than
///   rewritten because they have to stay on the same attribute numbering the mesh's layout has,
///   and they are generated from the same point schema this function is given.
///
/// - **`prefix` keeps the two sets' names apart**, because a shared label is one entry silently
///   overwriting the other. A material names a shader by label, so the label is how the two
///   halves of a scene find each other.
fn stage_set<'text>(
    prefix: &str,
    body: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
    point: &Schema,
    extras: &Extras,
) -> Vec<Value> {
    let body = body.into();
    let helpers = helpers.into();
    let mut shaders = mesh_shaders_over(prefix, "", body, helpers, point.clone());
    // Retained from **this set only** — the function returns one mesh's four, so a second
    // call's replacement cannot be removed by the first call's filter.
    shaders.retain(|shader| stage_of(shader) != Some(Stage::Lighting));
    let label = format!("{prefix}_lighting");
    shaders.push(lighting_shader_declaring(
        &label,
        "",
        body,
        helpers,
        crate::kit::gallery_custom_schema(),
        extras,
        crate::shaders::Projection::Latlong,
    ));
    shaders
}

/// Returns the stage a shader belongs to, read from its own variant tag.
///
/// # Invariants
///
/// **It is the tag, and the tag is [`Stage::tag`] of the stage itself.** A shader's label says
/// which mesh stage wrote it and a scene may reuse one across two meshes, so a filter keyed on
/// the label would drop or keep the wrong one. Going through `Stage::tag` rather than spelling
/// `"LightShader"` is what keeps this correct if a tag is ever renamed — the same reason the
/// emitter reads the tag through the same function.
fn stage_of(shader: &Value) -> Option<Stage> {
    match shader {
        Value::Enum { tag, .. } => [
            Stage::Shadowing,
            Stage::Depthing,
            Stage::Lighting,
            Stage::Emission,
        ]
        .into_iter()
        .find(|stage| stage.tag() == tag),
        _ => None,
    }
}

///
/// This is **deliberately coarse**, and it is the one number that decides whether the crate's
/// whole premise is visible. A triangle here is one planar block face, and a face is only a face
/// if it is big enough to read as one: sampled at a couple of units the facets are a pixel across
/// and the massif shades as the smooth cone the arithmetic would have produced anyway.
///
/// The unit it has to be measured against is the **mountain's own relief**, not its width. A
/// belt's cells are bounded across by a rib, and a rib is the whole drop from the crest to the
/// valley floor — a couple of hundred units here. Sampling the crests at a tenth of that gives
/// triangles ten times longer than they are wide, and a massif of those reads as crumpled paper
/// however good its silhouette is. Sixty units puts a facet at about a quarter of a rib, which is
/// a face with a shape rather than a splinter, and it leaves the crests sampled finely enough
/// for their own bends to survive.

/// How many cells the plain is divided into along each axis.
///
/// It is flat where the massif is, so the cells only have to be fine enough for the far ground's
/// rolling to read. A hundred cells across a plain nine times the massif's width is a cell of
/// sixty units, which over ground that is nearly level is a slope nobody can see the edges of.
const PLAIN_DIVISIONS: usize = 96;

/// How far the plain reaches, as a multiple of the massif's own radius.
///
/// It is a multiple and not a distance so that the two scale together. It is large because the
/// haze's only lever is distance: the air between the eye and the massif and the air between the
/// eye and the plain's far edge are in a fixed ratio, and a plain that stopped close enough to
/// see clearly would end on a visible line across the horizon.
const PLAIN_REACH: f64 = 18.0;

/// How much of the air stands between the eye and the massif.
///
/// # Invariants
///
/// This is stated as what the **subject** costs rather than as what the horizon costs, and the
/// density follows from it. The subject is what the picture is of, and its contrast is the thing
/// that gets destroyed by getting this wrong: a horizon that is not quite closed is a soft line
/// behind a mountain, while a massif washed half way into the air is a grey cone. The far edge
/// then follows, and it is checked by the note the compile prints rather than by hope.
const SUBJECT_HAZE: f64 = 0.10;

/// Where the snowline sits, as a fraction of the summit's height.
///
/// The line itself is wobbled in the shader, because a snowline drawn as a level is a level and
/// a level on a mountain is a thing that does not occur.
const SNOW_LINE: f64 = 0.88;

/// How far the snowline wanders either side of its own level, as a fraction of the summit.
const SNOW_WOBBLE: f64 = 0.10;

/// How far the snow fades in above the line, as a fraction of the summit.
const SNOW_FADE: f64 = 0.07;

/// The plain's height where it meets the massif, which is the rim's height.
const RIM: f64 = 0.0;

// --------------------------------------------------------------------------- the frame
//
// Everything below is derived from the massif's own measure rather than placed beside it: the eye,
// the aim, the sun, the air and the plain are all multiples of one radius, so a mountain that grew
// would carry its whole picture with it instead of leaving a camera inside it.

/// How wide the frame is, in radians.
///
/// **A long lens, and the reversal of what this scene used to use.** The old frame was 1.24 — a wide
/// angle, because the eye was standing in a corridor and had to take in both walls at once. The
/// consequence was the picture: a wide lens close to a mountain puts the near flanks past the edge of
/// the frame and the summit in the top tenth of it, and what is left in the middle is the ground.
/// A range read as a *range* is read from a distance, at an angle, with the whole of it in the frame
/// and its facets large enough to have a shape — which is what a long lens at a low elevation does.
const FIELD_OF_VIEW: f64 = 0.72;

/// Where the eye stands, as multiples of the massif's own radius.
///
/// `azimuth` is the bearing it looks along, measured in the plan from the massif's centre; `back`
/// how far beyond the radius it stands; `height` how far above the plain, as a fraction of the
/// summit rather than of the width, because it is the angle the rock subtends that decides the
/// picture and the rock's height is the thing being subtended.
///
/// **Low and to one side.** Low, because a mountain seen from above is a shape on a table and one
/// seen from below is a place somebody could be standing. To one side, because a range seen end-on is
/// one peak, and a range seen across is a *ridge* — a line of them, which is the reading the spine
/// and its spurs were authored to give.
struct Eye {
    azimuth: f64,
    back: f64,
    height: f64,
}

const EYE: Eye = Eye {
    azimuth: -1.42,
    back: 2.55,
    height: 0.30,
};

/// Where the aim lands, as multiples of the massif's radius and height.
///
/// Not at the summit. A frame centred on the highest point hangs the mountain on the middle of the
/// sky and wastes the half of it that has the detail; the aim is set below the summit and **along the
/// spine**, so the ridge runs across the frame and its far end recedes into the air.
const AIM_ON_SPINE: f64 = 0.06;
const AIM_HEIGHT: f64 = 0.46;

/// Where the sun stands, as a bearing off the eye's own and an elevation.
///
/// **Across the view and low, which is the whole of the block-face reading.** A facet is visible only
/// as a difference between it and its neighbour, and the difference is largest where the light runs
/// nearly along the surface: a sun high overhead lights every flank of a cone alike and the massif
/// comes out as one smooth shade, which is what the old frame did to the old geometry. A sun a
/// little past three quarters off the view axis rakes across the spine, so the walls that face it
/// burn and the ones turned a few degrees away go dark, and the boundary between two of them is a
/// line of light rather than a line of paint.
const SUN_OFFSET: f64 = -1.95;
const SUN_ELEVATION: f64 = 0.30;

/// The ranges drawn in this frame: the massif, and its own measure.
pub struct Range {
    /// What the content calls it, which is also the first segment of every belt path it owns.
    pub name: &'static str,
    /// Where its centre stands, as multiples of its own radius.
    pub centre: [f64; 2],
    /// How large it is, as a multiple of the authored massif.
    ///
    /// **The second scale of the fractal.** The crate builds one mountain; a *range* is that mountain
    /// and a smaller one standing beyond it, and the eye reads the pair as one system because the
    /// shapes are of one kind at two sizes. It is a scale rather than a second skeleton because the
    /// alternative — authoring a second massif — is authoring the same topology twice.
    pub scale: f64,
    /// How far it is turned about the vertical, in radians.
    ///
    /// A range is asymmetric — a spine with its spurs on both sides, longer one way than the other —
    /// so a turn gives a different silhouette rather than the same one mirrored, and that is what
    /// stops the second range reading as a copy of the first.
    pub turn: f64,
}

/// The ranges, in the order their ids are numbered.
///
/// **The second is behind the first along the view axis, and turned.** Two ranges standing side by
/// side are the same object twice and the eye says so; one beyond the other, at a different bearing
/// and a smaller size, is the second and third scale of the same hierarchy and the eye reads the
/// picture as a country rather than as a model. The turn is not cosmetic: the near range is seen
/// across its spine, and a far range seen the same way would show the same silhouette, which is the
/// twin again. Turned end-on it shows the profile the near one hides, and the two agree about the
/// system without agreeing about the shape.
pub const RANGES: [Range; 2] = [
    Range {
        name: "near",
        centre: [0.0, 0.0],
        scale: 1.0,
        turn: 0.0,
    },
    Range {
        name: "far",
        centre: [1.05, 1.45],
        scale: 0.80,
        turn: 1.15,
    },
];

/// Returns the ranges, unscaled.
pub fn ranges() -> &'static [Range] {
    &RANGES
}

/// Returns a massif stood with its own centre over a point in the plan, at a scale.
///
/// # Invariants
///
/// - **It moves and sizes the points and nothing else.** The belt a face came from, its block, its
///   stations and its ruggedness all survive, so the id a pixel carries still names the authored
///   element it was authored as.
/// - **The scale is uniform.** A massif stretched along one axis is a massif whose facets are
///   stretched with it, and a block face that is twice as long as it is wide is not a block face; the
///   relief scale that matters is the ratio of a fall to a rib, and a uniform scale leaves it alone.
/// - **The turn is about the vertical only.** Tilting a range would tilt its rim out of the plain it
///   stands on, and the gap that opens is at the one place in the frame the eye is trained on.
pub fn stood_at(mesh: &FacetMesh, at: [f64; 2], scale: f64, turn: f64) -> FacetMesh {
    let (low, high) = mesh.bounds();
    let centre = [
        ((low[0] + high[0]) * 0.5) as f64,
        ((low[2] + high[2]) * 0.5) as f64,
    ];
    let (cosine, sine) = (turn.cos() as f32, turn.sin() as f32);
    let mut moved = mesh.clone();
    for point in &mut moved.points {
        let east = (point[0] as f64 - centre[0]) as f32;
        let north = (point[2] as f64 - centre[1]) as f32;
        point[0] = (east * cosine - north * sine) * scale as f32 + at[0] as f32;
        point[2] = (east * sine + north * cosine) * scale as f32 + at[1] as f32;
        point[1] *= scale as f32;
    }
    moved
}

/// Returns where the eye stands, so a caller can measure the picture from it.
///
/// **It is the one eye [`landscape`] compiles**, read back rather than written twice. A probe that
/// placed its own eye to answer "how wide does this range stand in the frame" would be measuring a
/// camera nobody renders.
pub fn eye_at(radius: f64) -> [f64; 3] {
    [
        EYE.azimuth.cos() * EYE.back * radius,
        RIM + radius * EYE.height,
        EYE.azimuth.sin() * EYE.back * radius,
    ]
}

/// Returns how wide the frame is, in radians, for the same reason as [`eye_at`].
pub fn field_of_view() -> f64 {
    FIELD_OF_VIEW
}

/// Returns, for each range, the azimuth of its summit and of its nearest point, in degrees off the
/// view axis, read from the eye.
///
/// # Invariants
///
/// **Azimuth and not a position**, because the question is whether a range fills the frame or sits in
/// it, and a point's distance from the eye says nothing about that.
///
/// The two numbers are read from the **authored** mesh and the eye shifted back by the range's own
/// placement, because [`stood_at`] is a translation and a scale: a point of the stood range and a
/// point of the authored one are the same point measured from two eyes, and rebuilding the mesh per
/// range to ask the question would be answering it with a second copy of the geometry.
pub fn framing() -> Vec<(&'static str, f64, f64)> {
    let built = massif().build().expect("the authored massif expands");
    let (low, high) = built.bounds();
    let radius = (((high[0] - low[0]).powi(2) + (high[2] - low[2]).powi(2)) as f64).sqrt() * 0.5;
    let eye = eye_at(radius);
    let square = |point: [f64; 3], from: [f64; 3]| {
        (point[0] - from[0]).powi(2) + (point[1] - from[1]).powi(2) + (point[2] - from[2]).powi(2)
    };
    // The angle off the axis the eye looks along, which is the ray from the eye to the scene's aim
    // rather than to the origin: a frame aimed along the spine has its axis along the spine, and
    // measuring off the bearing to the middle of the plan would report the same range as off-centre
    // for having its summit at one end of it — which is the arrangement, not the error.
    let aim = target_at(&built, radius, high[1] as f64, &RANGES[0]);
    let axis = [aim[0] - eye[0], aim[2] - eye[2]];
    let axis = axis[1].atan2(axis[0]);
    let azimuth = |point: [f64; 3], from: [f64; 3]| {
        let bearing = (point[2] - from[2]).atan2(point[0] - from[0]);
        (bearing - axis).abs().rem_euclid(std::f64::consts::PI) * 180.0 / std::f64::consts::PI
    };
    RANGES
        .iter()
        .map(|range| {
            let shift = [range.centre[0] * radius, range.centre[1] * radius];
            let local_eye = [eye[0] - shift[0], eye[1], eye[2] - shift[1]];
            let summit = built
                .points
                .iter()
                .max_by(|left, right| left[1].total_cmp(&right[1]))
                .map(|point| {
                    [
                        point[0] as f64 * range.scale,
                        point[1] as f64 * range.scale,
                        point[2] as f64 * range.scale,
                    ]
                })
                .unwrap_or([0.0, 0.0, 0.0]);
            let nearest = built
                .points
                .iter()
                .min_by(|left, right| {
                    let left = [left[0] as f64, left[1] as f64, left[2] as f64];
                    let right = [right[0] as f64, right[1] as f64, right[2] as f64];
                    square(left, local_eye).total_cmp(&square(right, local_eye))
                })
                .map(|point| {
                    [
                        point[0] as f64 * range.scale,
                        point[1] as f64 * range.scale,
                        point[2] as f64 * range.scale,
                    ]
                })
                .unwrap_or([0.0, 0.0, 0.0]);
            (
                range.name,
                azimuth(summit, local_eye),
                azimuth(nearest, local_eye),
            )
        })
        .collect()
}

/// Where the aim stands, so [`framing`] and [`landscape`] measure the same axis.
///
/// The aim is set on the **subject range's own placement**, not on the authored mesh's bounds centre.
/// The two are different points and mixing them is the bug this guards: [`stood_at`] moves the
/// authored mesh's centre to wherever the range was placed, so an aim read off the authored bounds
/// looks at empty ground by exactly that much and hangs the mountain off one edge of the frame.
///
/// A little of the aim is then carried **along the spine** rather than sitting on the middle of it,
/// because a range framed on its centre is symmetric and a symmetric range has no direction to travel
/// in. The spine's direction is a fact about the authored shape — the long axis of its plan bounds —
/// so it is turned by the range's own turn before it is added.
fn target_at(mesh: &FacetMesh, radius: f64, summit: f64, range: &Range) -> [f64; 3] {
    let (low, high) = mesh.bounds();
    let along = if (high[0] - low[0]) >= (high[2] - low[2]) {
        [1.0, 0.0]
    } else {
        [0.0, 1.0]
    };
    let (cosine, sine) = (range.turn.cos(), range.turn.sin());
    let turned = [
        along[0] * cosine - along[1] * sine,
        along[0] * sine + along[1] * cosine,
    ];
    let reach = (high[0] - low[0]).max(high[2] - low[2]) as f64 * 0.5 * range.scale;
    [
        range.centre[0] * radius + turned[0] * reach * AIM_ON_SPINE,
        RIM + summit * AIM_HEIGHT,
        range.centre[1] * radius + turned[1] * reach * AIM_ON_SPINE,
    ]
}

/// Returns the massif this scene is of.
pub fn massif() -> Massif {
    massif::massif()
}

/// Returns how many crests and how many basins the massif is built from.
///
/// A probe prints them beside the violations, because the two numbers that decide whether a
/// hierarchy is worth authoring are whether it validates and how much of it the eye can see.
pub fn crest_count() -> usize {
    massif().skeleton.ridges.len()
}

/// Returns the number of nodes each valley floor is written with, by valley.
pub fn valley_nodes() -> Vec<usize> {
    massif()
        .skeleton
        .valleys
        .iter()
        .map(|valley| valley.nodes.len())
        .collect()
}

/// Builds the landscape.
pub fn landscape() -> Value {
    let authored = massif();
    let built = authored.build().expect("the authored massif expands");
    let (low, high) = built.bounds();
    let radius = (((high[0] - low[0]).powi(2) + (high[2] - low[2]).powi(2)) as f64).sqrt() * 0.5;
    let summit = high[1] as f64;

    // One walk numbers every belt and records the row that names it, and **the runs follow one
    // another** rather than each beginning at one — two tables starting at one would hand the same
    // number to a flank on each range, and a pixel would resolve to whichever one a reader asked
    // about first.
    let mut next_id = 1;
    let mut ranges_built: Vec<(&'static str, FacetMesh, Identity)> =
        Vec::with_capacity(RANGES.len());
    for range in RANGES {
        let identity = Identity::of(&authored, next_id, |belt| belt_path(range.name, belt));
        next_id += identity.rows().len() as u32;
        ranges_built.push((
            range.name,
            stood_at(
                &built,
                [range.centre[0] * radius, range.centre[1] * radius],
                range.scale,
                range.turn,
            ),
            identity,
        ));
    }

    // The eye looks **back along the bearing it stands at**, so the massif's centre is what it aims
    // at and the spine runs across the frame. The aim is set down the spine rather than at the
    // middle: the near end of a range is the part with the facets big enough to read, and centring on
    // the middle of the plan puts the summit at the top of the sky and the detail behind the eye.
    let eye = eye_at(radius);
    let target = target_at(&built, radius, summit, &RANGES[0]);
    let distance = ((eye[0] - target[0]).powi(2)
        + (eye[1] - target[1]).powi(2)
        + (eye[2] - target[2]).powi(2))
    .sqrt();

    let plain = radius * PLAIN_REACH;
    // Where the sun stands, as a bearing **off the camera's own**: the light is derived from the eye
    // rather than chosen beside it, so the two cannot be edited apart. A sun picked independently
    // reads well for exactly one camera position and is behind the mountain for the next, and the
    // failure looks like a scene that has gone dark for no reason.
    let sun_bearing = EYE.azimuth + std::f64::consts::PI + SUN_OFFSET;
    let sun = [
        sun_bearing.cos() * SUN_ELEVATION.cos(),
        SUN_ELEVATION.sin(),
        sun_bearing.sin() * SUN_ELEVATION.cos(),
    ];
    // The density that puts [`SUBJECT_HAZE`] of the air between the eye and the massif, and that
    // therefore closes it over the plain's far edge by a few times that. Derived and not chosen: the
    // two distances are the only numbers the answer depends on.
    let atmosphere = Atmosphere::new()
        .zenith([0.09, 0.21, 0.50])
        .horizon([0.66, 0.70, 0.79])
        .ground([0.34, 0.34, 0.36])
        .sun(sun, [1.0, 0.72, 0.44], 5.0, 0.60)
        .density(-(1.0 - SUBJECT_HAZE).ln() / distance)
        .scale_height(radius * 1.8);

    let helpers = format!(
        "{}\n{}\n{}",
        atmosphere.wgsl(),
        NOISE,
        constants(radius, summit, sun),
    );
    // The far ranges are the post stage's alone, so they are handed to the post stage alone. They
    // are a backdrop the air has already closed over, and a surface shader has no use for a line
    // painted into the sky.
    let post_helpers = format!("{helpers}\n{}", crate::ridges::RIDGES);

    let mut shaders: Vec<Value> = Vec::new();
    shaders.extend(stage_set(
        "massif",
        MASSIF,
        &helpers,
        &massif_extras().point_schema(),
        &massif_extras(),
    ));
    shaders.extend(stage_set(
        "plain",
        GROUND,
        &helpers,
        &shaded_point_schema(),
        &plain_extras(),
    ));
    shaders.push(post_shader(GRADE, &post_helpers));

    Value::structure([
        ("name".to_owned(), Value::String("landscape".to_owned())),
        // The three images this scene's **own** shaders write, beside the frame. The massif reads
        // them from its points; the plain writes the reserved zero, because a plain has no authored
        // element of its own. Both have to write them: a stage that drew geometry and did not would
        // leave the image holding whatever an earlier fragment put there.
        (OUTPUTS.to_owned(), outputs_field()),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed(
                eye,
                target,
                FIELD_OF_VIEW,
                (radius * 0.05).max(0.5),
                plain * 1.2,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                // The sun is a **directional** light and not a place, and here that is a correction
                // rather than a preference. A point light's brightness falls off as the square of its
                // distance, and this scene has a subject two thousand metres across standing on a
                // plain nine times that: a lamp bright enough for the massif leaves the far plain
                // black, and one bright enough for the far plain blows the massif out. A light that
                // reaches every distance equally is the only one that can draw both, which is what
                // the sun is.
                //
                // `direction` is the way the light **travels**, which is the negation of the bearing
                // the sky is drawn with. The two are the same decision and only one of them is
                // stated: `sun` is computed from the eye above and this reads it, so a scene that
                // moved the eye moves the light with it.
                directional(
                    "sun",
                    [-sun[0], -sun[1], -sun[2]],
                    [1.00, 0.84, 0.62],
                    9.0,
                    radius * 4.0,
                ),
                // A fill from the sky, dim and blue, standing in for the whole dome. One light with
                // one falloff leaves every shadowed facet black, and black is not what a shadow looks
                // like outdoors — but it is also not what this scene wants, because a fill strong
                // enough to see is a fill that lifts the dark facets to the brightness of the lit ones
                // and erases the very difference the raking sun was placed to make. It is a **tenth**
                // of what the corridor needed, and the corridor needed it because the walls were all
                // shadow side.
                light_with_radius(
                    "sky",
                    [0.0, radius * 2.0, 0.0],
                    [0.36, 0.44, 0.62],
                    9_000_000.0,
                    radius * 3.0,
                    plain * 2.0,
                ),
            ]),
        ),
        ("shaders".to_owned(), Value::List(shaders)),
        (
            "meshes".to_owned(),
            // **One mesh per range, and they are told apart by name.** A placement names a mesh by
            // name, so two ranges drawn from one geometry still need two names — and the name has to
            // be the range's, because the belt paths a pixel resolves to start with it.
            Value::List({
                let mut entries: Vec<Value> = ranges_built
                    .iter()
                    .map(|(name, geometry, identity)| {
                        mesh(
                            &format!("range_{name}"),
                            "Triangles",
                            // The point schema is **the one the outputs' fields make**, so the vertex
                            // layout and the shader's parameter list are derived from one schema
                            // rather than two that have to agree.
                            massif_extras().point_schema(),
                            // The block, and not the flank: a belt is one block face and a rib is a
                            // joint in the rock, so this is the grouping that lets the massif read as
                            // cut rather than as draped. See `across_the_belt`.
                            identity::faceted(geometry, identity, identity::across_the_belt),
                        )
                    })
                    .collect();
                entries.push(mesh(
                    "plain",
                    "Triangles",
                    shaded_point_schema(),
                    heightfield(PLAIN_DIVISIONS, plain, |x, z| plain_height(x, z, radius)),
                ));
                entries
            }),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "mesh_materials".to_owned(),
            Value::List(vec![
                // One material per stage **per mesh**, because a material names one shader and each
                // mesh has its own four. The shadowing and depthing stages read no custom data, so
                // theirs is a placeholder those shaders ignore.
                mesh_material(
                    "cast",
                    "massif_shadowing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                mesh_material(
                    "depth",
                    "massif_depthing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                // The stone, which the surface body replaces entirely: the tint is **white** and the
                // body writes the albedo, so the colour of the rock is decided in one place. A tint
                // with a colour in it would be multiplied by the body's albedo, which is two
                // decisions about one surface and the sort that drift apart.
                mesh_material(
                    "shade",
                    "massif_lighting",
                    surface([1.0, 1.0, 1.0], 0.0, 0.90, [0.0; 3]),
                ),
                mesh_material(
                    "ground_depth",
                    "plain_depthing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                mesh_material(
                    "ground_shade",
                    "plain_lighting",
                    surface([1.0, 1.0, 1.0], 0.0, 0.95, [0.0; 3]),
                ),
            ]),
        ),
        ("instances".to_owned(), instances(instance_buckets())),
    ])
}

/// Returns one placement per pass, with the ranges walked rather than written out.
///
/// # Invariants
///
/// **Every range appears in all three buckets.** A material names one shader and a shader belongs to
/// one stage, so the three lists are the same placements under three materials and three sets of
/// labels — a label names one instance in the whole scene, not one per pass. A range left out of one
/// of them is geometry that never draws, which is a scene that validates and renders an empty frame.
///
/// **The plain is in two of the three.** A flat sheet cannot occlude itself, and putting it in the
/// shadow map would spend the map's extent on ground that has nothing to say.
fn instance_buckets() -> Vec<(Stage, Vec<Value>)> {
    let placed = |suffix: &str, material: &str| -> Vec<Value> {
        RANGES
            .iter()
            .map(|range| {
                instance(
                    format!("{}_{suffix}", range.name),
                    &format!("range_{}", range.name),
                    [0.0, 0.0, 0.0],
                    [0.0; 3],
                    [1.0; 3],
                    material,
                )
            })
            .collect()
    };
    let mut depth = placed("depth", "depth");
    depth.push(plain_instance("plain_depth", "ground_depth"));
    let mut shade = placed("shade", "shade");
    shade.push(plain_instance("plain_shade", "ground_shade"));
    vec![
        (Stage::Shadowing, placed("cast", "cast")),
        (Stage::Depthing, depth),
        (Stage::Lighting, shade),
    ]
}

/// Returns the plain's placement in one pass.
fn plain_instance(label: &str, material: &str) -> Value {
    instance(
        label,
        "plain",
        [0.0, 0.0, 0.0],
        [0.0; 3],
        [1.0; 3],
        material,
    )
}

/// Returns the path a belt is named by, in this scene's own hierarchy.
///
/// # Invariants
///
/// - **The path is the content's and not the crate's.** `Massif` has no naming and no grouping
///   and is not asked for any; a path is a fact about the code that built the massif, which is
///   the only place a hierarchy exists. `SPEC/mountain.md` is human-editable only, and a naming
///   tree inside the crate would be a requirement change rather than an implementation.
/// - **The path is a function of the range and the `BeltRef` alone**, so the same belt is named
///   the same way in every build of the same skeleton, and an id means the same thing on either
///   side of an edit. A path built from anything that varies between builds would be a path that
///   answers a different question each time it is asked.
/// - **The range is the first segment, and it has to be.** This scene draws two massifs, each
///   numbering its belts from its own walk, so a path leaving the range out would name two
///   different flanks the same thing — and the id space is exactly what has to stay separable
///   from the geometry. A reader resolving a pixel would be handed the far side's path while
///   looking at the near one.
///
/// The hierarchy is this scene's, and it is deliberately shallow until there is a third range to
/// nest: a range, the valley that carries its flank, and which side of it. Growing it is a matter
/// of changing this one function, which is the property that keeps the id space separable from
/// the geometry.
pub fn belt_path(range: &str, belt: BeltRef) -> String {
    format!(
        "landscape/{}/valley_{}/{}",
        range,
        belt.valley.0,
        match belt.side {
            Side::Left => "left_flank",
            Side::Right => "right_flank",
        }
    )
}

/// Returns the plain's height at a point.
///
/// # Invariants
///
/// How far a range's footprint reaches beyond its own centre, as a multiple of its radius.
///
/// **The half-diagonal of the plan bounds, not the radius.** A range is elongated — a spine with its
/// spurs, longer one way than the other — and `radius` is the root-sum-square half-width, which is
/// smaller than the long axis. Gating the plain's rise on the radius alone leaves the ends of the
/// spine standing in ground that has already begun to lift, and the underside of the surface shows
/// through as a black slit at exactly the foot the eye is looking at.
const FOOT_REACH: f64 = 1.15;

/// It is **exactly the rim's height** wherever a range reaches, and the rolling is gated by distance
/// from the *nearest range's own centre* so that it is. Measuring from the world origin is the
/// mistake this guards: with a second range stood off to one side, the origin's radius is the wrong
/// circle around it, the ground rises through its foot, and the massif's own underside shows as a
/// black slit where the two disagree — a surface, not a solid, and the plain is what hides the
/// difference.
///
/// The ground rises towards its own edge, so its silhouette is a low horizon instead of the straight
/// line a flat sheet ends on, and that rise is what the haze is sized against.
fn plain_height(x: f64, z: f64, radius: f64) -> f64 {
    // The nearest range's own measure of how far out this is. A range's footprint is not a circle
    // and its plan bounds are not centred on the origin, so the conservative radius is the one that
    // covers the whole of the authored massif wherever it was stood.
    let distance = RANGES
        .iter()
        .map(|range| {
            let centre = [range.centre[0] * radius, range.centre[1] * radius];
            let foot = radius * range.scale;
            let (east, north) = (x - centre[0], z - centre[1]);
            (east * east + north * north).sqrt() - foot * FOOT_REACH
        })
        .fold(f64::INFINITY, f64::min);
    // Both undulations are gated to begin **outside** every range's own footprint, which is the
    // whole of the contract. A two-unit roll inside a rim would cut through the foot of every flank
    // and show as a jagged seam where the mountain meets the ground.
    let beyond = smoothstep(0.0, radius * 0.55, distance);
    let away = smoothstep(radius * 0.60, radius * 2.6, distance);
    // The undulation is **coarse on purpose, and coarser than it wants to be.** The plain is a
    // grid of a few hundred units a cell, and anything the grid cannot carry comes back as the
    // grid: a hundred-unit ripple sampled every four hundred units is not a ripple, it is four
    // hundred unrelated cells, and near the eye — where one cell is a sixth of the frame — it is
    // a floor tiled in rectangles. The wavelength here is several cells at every octave, which
    // costs the floor its small relief and buys it back as a surface.
    let roll = fractal_noise([x * 0.00040, z * 0.00044], 3, 41) - 0.5;
    let horizon = smoothstep(radius * 1.4, radius * PLAIN_REACH, distance);
    RIM + beyond * away * (roll * radius * 0.16 + horizon * radius * 0.22)
}

/// The constants the shader bodies are written against.
fn constants(radius: f64, summit: f64, sun: [f64; 3]) -> String {
    format!(
        "const SNOW_LINE: f32 = {:.3};\n\
         const SNOW_WOBBLE: f32 = {:.3};\n\
         const SNOW_FADE: f32 = {:.3};\n\
         const SCENE_RADIUS: f32 = {radius:.3};\n\
         const CLOUD_TOP: f32 = {CLOUD_TOP:.4};\n\
         const CLOUD_COVER: f32 = {CLOUD_COVER:.4};\n\
         const CLOUD_FROM: f32 = {CLOUD_FROM:.4};\n\
         const RIVER_HALF: f32 = {RIVER_HALF:.4};\n\
         const RIVER_MEANDR: f32 = {RIVER_MEANDR:.4};\n\
         const RIDGE_SCALE: f32 = {RIDGE_SCALE:.4};\n\
         const RIDGE_LAYERS: i32 = {RIDGE_LAYERS};\n\
         const CIRRUS_COVER: f32 = {CIRRUS_COVER:.4};\n\
         const SUN_AHEAD: vec3<f32> = vec3<f32>({:.4}, {:.4}, {:.4});\n",
        summit * SNOW_LINE,
        summit * SNOW_WOBBLE,
        (summit * SNOW_FADE).max(1.0),
        sun[0],
        sun[1],
        sun[2],
    )
}

/// How high the cloud in the valley stands, as a fraction of the massif's own radius.
///
/// **Just under the eye.** The eye stands a little over a tenth of the radius up, so a sea at
/// this height is seen almost edge on: it runs out to the horizon as a surface rather than as a
/// patch of ground, which is the reading that gives the shot its depth. A sea much lower would be
/// looked down on and would read as a lake; much higher and the massif's own foot would be buried.
const CLOUD_TOP: f64 = 0.150;

/// How completely the cloud covers the ground beneath it.
const CLOUD_COVER: f64 = 0.94;

/// How much of the high sky the cirrus is allowed to take.
const CIRRUS_COVER: f64 = 0.55;

/// How far down the valley the cloud begins, as a multiple of a range's radius.
///
/// **A distance down the valley and not a height**, which is the whole of what moved the cloud
/// out of the foreground. The cloud used to be a sea the massif came out of; here it is what
/// closes the far end of the corridor, and the floor between the eye and it is left free to carry
/// the river. A height could not do this job: the floor is flat, so every part of it is at the
/// same height and a cloud keyed on height either covers all of it or none.
const CLOUD_FROM: f64 = 5.2;

/// How wide the wet ground either side of the river's own line is, as a multiple of a radius.
///
/// It is wide because a braided river is a **wet corridor rather than a channel**, and narrow
/// because it has to stay a fraction of the valley's own width — at twice this the floodplain
/// would from the foot of both ranges and the corridor would be a plain with two mountains on it.
///
/// It is also **narrow enough to survive being walked on**. A band of a fixed width in world
/// space is not a fixed width on the screen: the eye stands on its own near edge, so the metre
/// under the eye is the metre that fills the bottom of the frame, and a band wide enough to look
/// right at the far end of the corridor is a lake at the near end. Half this is the number that
/// leaves a green floodplain either side of the thread all the way down.
const RIVER_HALF: f64 = 0.085;

/// How far the river's line wanders off the valley's axis, as a multiple of a radius.
///
/// A river on the axis exactly would be a stripe, and a stripe is the one shape that cannot read
/// as water: it says where the water is without ever saying the water went. This is more than the
/// half-width, so the line wanders **off the middle of its own wet ground** — which is what puts
/// the wider side of the floodplain on the outside of each bend, where a real river puts it.
const RIVER_MEANDR: f64 = 0.30;

/// What the two surface bodies are written against.
///
/// # Invariants
///
/// The noise is sampled in **world space along the ground**, so a pattern stays put on the
/// mountain when the camera moves. Sampled in screen space it would slide across the picture as
/// the eye moves, which reads as a smear on the lens rather than as stone.
const NOISE: &str = r#"
/// Returns a value in zero to one from a two dimensional coordinate.
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

/// Returns noise summed over `octaves`, in roughly zero to one.
fn noise(point: vec2<f32>, octaves: i32) -> f32 {
    var total = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var octave = 0; octave < octaves; octave = octave + 1) {
        total = total + amplitude * value_noise(point * frequency);
        frequency = frequency * 2.1;
        amplitude = amplitude * 0.5;
    }
    return total * 1.14;
}
"#;

/// What the massif's surface does after the light is applied.
///
/// # Invariants
///
/// The bands are functions of **height and steepness**, and the two are not the same thing. Snow
/// lies on what is high and level; bare rock shows on what is steep at any height; and scree
/// gathers below. A mountain banded by height alone has snow on its vertical faces, which is the
/// one place snow does not stay.
///
/// The ruggedness in `in.texture.x` is the crest's authored sharpness and it **varies the stone
/// between neighbouring faces**. Two facets of one flank differ only by their normals otherwise,
/// and facets lit by normals alone read as one smooth surface with steps in it; what makes a
/// facet fan read as rock is that the faces are not all the same stone.
const MASSIF: &str = r#"
    // The stone a facet is cut from, and it is *the* albedo rather than a multiplier on one: the
    // material's tint is white and this is the surface's colour. A body that scaled a tint would
    // be deciding the colour in two places, and the second place is the one that drifts.
    // **Steepness is read off the normal and not off the face's own `dip_angle`.** The two say the
    // same thing — a face's dip is the angle its own normal makes with the horizon — and the normal
    // is the one of them the rasteriser interpolates. `dip_angle` is a property of a face, written
    // the same at all three of its corners, so it is constant across each triangle and steps at
    // every triangle edge: a massif of two thousand of those reads as a fan of straw rather than
    // as a face of rock, and it does so at exactly the scale of its own tessellation.
    let steep = smoothstep(0.25, 0.65, 1.0 - clamp(normal.y, 0.0, 1.0));
    let patches = 0.86 + 0.28 * noise(in.world_position.xz * 0.02, 3);
    let bare = mix(vec3<f32>(0.46, 0.42, 0.37), vec3<f32>(0.19, 0.17, 0.16), steep);
    // The stone varies **per block**, and the number that says which block a facet belongs to is the
    // authored id — a flat per-face value, written the same at all three corners of every triangle of
    // its belt, so it is constant across a block and steps only at a block's own edge. That is what
    // makes it the right key and what makes `dip_angle` the wrong one: both are per-face, but the
    // joint the variation is describing *is* the block boundary.
    //
    // It was read off `in.texture.x`, which is the belt's own across-parameter: a ramp along the
    // direction a flank is walked, so every block in a flank carried the same gradient and the flank
    // came out striped like straw. The hash is deliberately crude and its amplitude deliberately
    // small — stone differs block to block by a little, not by a colour.
    let block = fract(in.authored_id * 0.6180339887);
    let stone = bare * (0.88 + 0.24 * block) * patches;

    // The snowline wanders with the ground it lies on rather than running level, and it is driven
    // by the same world position the rock is, so it stays on the mountain. It is held off the
    // steepest faces by the same steepness the stone is read from, because that is where it does
    // not stay.
    let wobble = SNOW_WOBBLE * (noise(in.world_position.xz * 0.004, 3) * 2.0 - 1.0);
    let above = smoothstep(SNOW_LINE + wobble - SNOW_FADE, SNOW_LINE + wobble + SNOW_FADE,
                           in.world_position.y);
    let snow = above * (1.0 - smoothstep(0.78, 0.98, 1.0 - clamp(normal.y, 0.0, 1.0)));
    let snowed = mix(stone, vec3<f32>(0.93, 0.95, 1.00), snow);

    // The floor of a valley is soil and scrub rather than stone, and it is warmer for it. It is
    // what separates a valley from a narrower piece of the same slope.
    let floor_mix = 1.0 - smoothstep(0.0, SCENE_RADIUS * 0.45, in.world_position.y);
    // The surface's own colour, named once because two terms below multiply by it and a second copy
    // of the expression is a second copy that can drift from the first.
    let albedo = mix(snowed, vec3<f32>(0.30, 0.29, 0.21), floor_mix * 0.75);
    color = color * albedo;

    // The **sky a face stands in**, which on a range this dark is most of what separates a shadow
    // from a hole in the picture. The sun rakes across the spine, so the flanks turned away from it
    // are a large share of the frame, and the scene's second light cannot carry them alone: a point
    // light in the middle of a plain eighteen times its own size leaves behind it a number about how
    // far away it stands rather than about the sky. A face's own share of the sky is therefore
    // decided here, where the face's normal and the sun's bearing both are.
    //
    // **Upward-facing is what counts, and not merely sun-facing.** An overhang sees the same sky
    // as the ground beneath it and a vertical wall sees half of it, and that gradient is most of
    // what makes an unlit face read as a surface with a shape in it rather than as a black cut-out.
    // The **sky a face stands in**, and the reason a shadowed rock is not black.
    //
    // It is **multiplied by the stone's own colour, not added to it**. Skylight is reflected like
    // any other light, and a term that is added raw is not reflected light at all — it is fog sitting
    // in front of the surface, which lifts a shadowed slab of basalt to the same value as a shadowed
    // slab of limestone and flattens every hue in the frame towards the colour of the air. That was
    // most of what read as "the whole picture is one pale colour".
    let skyward = 0.35 + 0.65 * clamp(normal.y, 0.0, 1.0);
    let from_sun = clamp(dot(normal, SUN_AHEAD), 0.0, 1.0);
    color = color + air_colour() * albedo * skyward * (0.05 + 0.26 * (1.0 - from_sun));

    let eye = uniforms.projection.position;
    let view = normalize(in.world_position - eye);
    let facing = 1.0 - abs(dot(normal, view));
    let haze = air_closure(eye, in.world_position);
    color = mix(color, air_colour(), clamp(haze * (0.45 + 0.25 * facing), 0.0, 0.72));
"#;

/// What the plain's surface does after the light is applied.
///
/// # Invariants
///
/// The plain is drawn by a body of its own rather than by the massif's with a flag in it: it carries
/// none of the per-face facts the massif's body reads, so a shared body would have to be told which
/// mesh it was drawing, and the way to tell it is the material row — which is a branch in a shader
/// for a difference that is a different surface.
///
/// It hazes by the **same law** as the massif, from the same air. That is the whole reason the
/// atmosphere is one value handed to every stage: the plain meets the massif along the rim, and two
/// bodies washing towards two slightly different colours would draw that seam as a bright line
/// across the foot of the mountain.
///
/// # What the ground is, and why the old ground was wrong
///
/// The first version of this scene stood its eye in a corridor and drew the floor as a **floodplain
/// with a river down its axis**: green graded from the water, terrace benches, braided channels, all
/// keyed to the line `x = 0`. That premise died with the corridor. The eye now stands out on the
/// floor of the foreland looking at a range, and what the old body still painted was a lawn with a
/// stream across it — a bright green field filling the bottom half of a picture that is about stone.
///
/// What is actually there is an **alluvial apron**: everything the range has shed, sorted by how far
/// the water carried it. Coarse grey gravel at the foot where the slopes end, sand and silt beyond,
/// and only then whatever will grow. It is keyed to the one fact the ground has that the mountain
/// also knows — the distance from the range's own centre — because that distance *is* the sorting.
///
/// **The channels radiate, they do not run.** A braid that leaves a range at many mouths spreads as
/// it loses its load, so the pattern is keyed to the bearing round the centre and not to one axis.
/// Drawing a single river down the middle of a picture of a mountain is drawing a road.
const GROUND: &str = r#"
    let patches = 0.90 + 0.16 * noise(in.world_position.xz * 0.004, 4);
    // How far this point stands from the range, in the range's own measure. Every decision the apron
    // makes is made against this one number, because it is the only one the ground has that the
    // mountain also knows.
    let reach = length(in.world_position.xz) / SCENE_RADIUS;

    // **Gravel at the foot, sand beyond it, scrub where the fines have settled.** Not three textures
    // blended by noise but three distances, because the boundaries are what make the sorting
    // legible. Kept low in value: the mountain is the subject, and a bright floor competes with it
    // for the eye's first look.
    var albedo = mix(
        vec3<f32>(0.34, 0.32, 0.29),
        vec3<f32>(0.44, 0.39, 0.29),
        smoothstep(0.85, 1.55, reach));
    albedo = mix(albedo, vec3<f32>(0.30, 0.32, 0.21), smoothstep(1.9, 3.6, reach));

    // The **fan lobes**. Where a basin's mouth meets the plain it dumps a shallow cone, and a row of
    // those cones reads from above as lobes of pale sorted gravel between darker, coarser ground.
    // Keyed to the bearing round the range, so there is one lobe per direction the rock came from.
    let bearing = atan2(in.world_position.z, in.world_position.x);
    let lobes = noise(vec2<f32>(bearing * 3.4, reach * 1.7), 3);
    let fan = smoothstep(0.52, 0.78, lobes) * (1.0 - smoothstep(0.95, 1.9, reach));
    albedo = mix(albedo, vec3<f32>(0.47, 0.44, 0.38), fan * 0.55);

    // The **channels**, radiating as they spread: a noise read finely across the bearing and coarsely
    // along the run, which is the ratio that makes fingers rather than blotches, and faded out
    // entirely beyond the apron where the fines have covered them.
    let threads = noise(vec2<f32>(bearing * 34.0 + reach * 2.2, reach * 5.4), 2);
    let wet = (1.0 - smoothstep(0.92, 1.35, reach)) * smoothstep(0.60, 0.72, threads);

    // The **scattered stone** at the foot, where the talus has not been sorted into anything yet.
    // This is the part of the ground that agrees with the rock above it, and it is what keeps the
    // near foreground from reading as a lawn meeting a wall.
    let rubble = smoothstep(0.55, 0.86, noise(in.world_position.xz * 0.020, 3))
        * (1.0 - smoothstep(0.70, 1.25, reach));
    albedo = mix(albedo, vec3<f32>(0.26, 0.25, 0.24), rubble * 0.5);

    color = color * albedo * patches;

    // The floor's share of the **same sky the walls are lit by, at the same rate**. The plain is
    // flat, so it faces the sky as squarely as a wall's shoulder does — and it is the ground the sun
    // is behind, so a long wedge of it lies in the range's own cast shadow. A sky term of a few
    // percent is what made that wedge read as a hole cut in the picture rather than as shade: the
    // massif's body has always let an unlit face reach a third of the sky, and a floor that is
    // *more* open to the dome than a wall is cannot be allowed to see less of it.
    let eye = uniforms.projection.position;
    let view = normalize(in.world_position - eye);
    let facing = 1.0 - abs(dot(normal, view));
    let skyward = 0.35 + 0.65 * clamp(normal.y, 0.0, 1.0);
    let from_sun = clamp(dot(normal, SUN_AHEAD), 0.0, 1.0);
    color = color + air_colour() * albedo * skyward * (0.10 + 0.30 * (1.0 - from_sun));

    // The **air between the eye and the ground**, which on this floor is doing more work than it
    // looks: the sun rakes across the range, so a long wedge of plain behind it is in the mountain's
    // own shadow, and a shadowed ground with only a sky term and a distant fill light in its
    // neighbourhood comes back as a hole cut in the picture rather than as shade. The massif's body
    // has always had this line; the first rewrite of the ground body dropped it and the range's cast
    // shadow turned black.
    let haze = air_closure(eye, in.world_position);
    color = mix(color, air_colour(), clamp(haze * (0.45 + 0.25 * facing), 0.0, 0.72));

    // The **water**, drawn after the ground's own colour because it is not a colour: it is what the
    // sky does to a flat mirror, and a mirror seen at a grazing angle is nearly all sky. Dark
    // underneath and bright on top, because a channel that returns sky *alone* is a strip of white
    // paper laid on the ground, which is how the first version of this floor read.
    let reflected = mix(
        vec3<f32>(0.30, 0.38, 0.47),
        vec3<f32>(0.72, 0.76, 0.82),
        pow(facing, 2.2));
    let water = vec3<f32>(0.06, 0.11, 0.16) + reflected * (0.55 + 0.30 * threads);
    color = mix(color, water, wet * 0.7);

    // The seam where the rock meets the ground it was shed onto. The massif's own foot is levelled
    // to the rim, and a hard line there is the one edge in the picture that says "two objects"; this
    // is where the talus of the lowest belt and the gravel of the apron are made to overlap.
    color = mix(color, vec3<f32>(0.33, 0.31, 0.28), (1.0 - smoothstep(0.78, 0.98, reach)) * 0.18);
"#;

/// What the post stage does to the frame.
///
/// # Invariants
///
/// The sky is drawn where nothing was, and it is drawn from the **ray the pixel looks along**, so
/// it moves with the camera. A sky read off the screen coordinate is a backdrop: it stays put
/// when the camera tilts, and a landscape in front of a backdrop is a photograph with the
/// mountain pasted on it.
///
/// The curve is applied before the vignette, because a vignette applied first is itself curved
/// and the two do not compose.
const GRADE: &str = r#"
    if depth >= 1.0 {
        let ray = ray_through(in.uv);
        // Three ranges behind the one the plain is drawn out to. They are painted rather than
        // modelled because the air closes over modelled ground long before the eye can see it,
        // and a horizon made of one flat line is the thing that gives a landscape away as a
        // diorama.
        color = ridges(air_sky(ray), ray, RIDGE_SCALE, RIDGE_LAYERS);

        // The **shafts**, which are what makes a low sun read as a low sun rather than as a
        // bright patch of sky. A crepuscular ray is a gap in the cloud letting one beam of light
        // through, so it is drawn as a streak radiating from the sun's own bearing — and the
        // bearing is the only coordinate it is given, which is what makes the streak radial by
        // construction rather than by a noise lattice that happens to line up.
        //
        // They go on **after** the ranges, because a shaft is airlight between the eye and
        // whatever is behind it: a shaft drawn under a distant ridge is a ridge with a bright
        // edge on it, and one drawn over the near ground would be a beam with nothing to come
        // through.
        let side = normalize(cross(SUN_AHEAD, vec3<f32>(0.0, 1.0, 0.0)));
        let rise = cross(side, SUN_AHEAD);
        let sweep = atan2(dot(ray, side), dot(ray, rise));
        let towards = clamp(dot(ray, SUN_AHEAD), 0.0, 1.0);
        let shaft = smoothstep(0.50, 0.88, noise(vec2<f32>(sweep * 2.4, 0.5), 3))
            * pow(towards, 2.6);
        color = color + vec3<f32>(1.00, 0.88, 0.70) * shaft * 0.18;

        // High cirrus above them, on the ray rather than on the pixel for the same reason the
        // ranges are: a sky drawn in screen space is a backdrop, and a backdrop behind a mountain
        // is a photograph with a mountain on it.
        //
        // It is sampled on a **cloud deck** — the point where the ray meets a flat plane overhead —
        // and not on the ray's elevation and not on its azimuth. Elevation alone is a ring: the set
        // of rays at one `y` is a circle round the eye, so any function of it draws concentric hoops
        // across the whole sky, which is the one thing cirrus never is. Azimuth alone is worse: the
        // unit horizontal direction changes fastest at the sides of the frame and not at all
        // overhead, so a noise lattice read on it smears into great flat wedges. A deck is a plane,
        // the lattice is isotropic on it, and the cells stretch towards the horizon on their own —
        // which is the perspective a person actually sees when they look up at a sky that goes on.
        let deck = ray.xz / max(abs(ray.y), 0.16);
        let streak = noise(deck * 0.55 + vec2<f32>(3.1, -1.7), 4);
        let wisp = noise(deck * 1.35 + vec2<f32>(-4.2, 2.6), 3);
        let cirrus = smoothstep(0.56, 0.82, streak * 0.62 + wisp * 0.38)
            * smoothstep(0.03, 0.30, ray.y)
            * (1.0 - smoothstep(0.60, 0.95, ray.y));
        color = mix(color, vec3<f32>(0.99, 0.99, 1.00), cirrus * CIRRUS_COVER);
    }

    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.18), vec3<f32>(0.0), vec3<f32>(1.0));
    color = color * color * (3.0 - 2.0 * color) * 0.26 + color * 0.74;

    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.30, 0.0, 1.0);
"#;

/// How high the far ranges stand in this scene's frame.
///
/// Above one, not below. [`crate::ridges::RIDGES`] scales the painted ranges for a camera that
/// looks level or down, and this one looks **up** — the eye is near the plain and the massif fills
/// the middle of the frame — so the ranges are lifted to sit behind the massif's own shoulders
/// instead of below its foot, which is where a level camera's skyline would put them.
const RIDGE_SCALE: f32 = 1.35;

/// How many painted ranges close the far end of the corridor, which is **five**.
///
/// More than the other two scenes take, and for the same reason this scene is not the other two
/// scenes: the eye is looking **down a valley**, and what a valley does at its far end is recede.
/// A painted horizon is a stack of steps of air, so a corridor that recedes through four ranges
/// has four steps in it and one that recedes through two has two — and two, at a notch the
/// distance the air has already closed, is a line with a shadow on it rather than a valley. The
/// fifth is the layer the air has almost taken back.
const RIDGE_LAYERS: i32 = 5;
