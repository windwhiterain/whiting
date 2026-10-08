//! A massif authored as a ridge and valley skeleton, standing on a plain.
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
use mountain::line::{Side, Station};
use mountain::tessellation::BeltRef;
use mountain::{
    End, FacetMesh, FootprintNode, LineRef, Massif, Ridge, RidgeId, RidgeNode, Skeleton,
    Tessellation, Valley, ValleyNode,
};
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

/// How finely the skeleton is sampled, in world units.
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
const STEP: f32 = 60.0;

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
const SUBJECT_HAZE: f64 = 0.18;

/// Where the snowline sits, as a fraction of the summit's height.
///
/// The line itself is wobbled in the shader, because a snowline drawn as a level is a level and
/// a level on a mountain is a thing that does not occur.
const SNOW_LINE: f64 = 0.50;

/// How far the snowline wanders either side of its own level, as a fraction of the summit.
const SNOW_WOBBLE: f64 = 0.10;

/// How far the snow fades in above the line, as a fraction of the summit.
const SNOW_FADE: f64 = 0.07;

/// The plain's height where it meets the massif, which is the rim's height.
const RIM: f64 = 0.0;

/// How wide the frame is, in radians.
///
/// **Wide, and that is the choice that reads as a valley.** A long lens from far back is a
/// photograph of a specimen: it stacks the ranges against their own far ridges and flattens the
/// air between them. A valley is a corridor, and a corridor is only readable through a lens that
/// takes in both walls at once and still leaves the ground between them running to the horizon —
/// which is why this is wide enough that the near wall of each range stands past the edge of the
/// frame, and the range is read as a wall rather than as a thing with a silhouette.
///
/// It is wide and the eye is **low**, which is the other half of the same reading: a wide lens
/// raised up looks down into the valley and turns it into a bowl.
const FIELD_OF_VIEW: f64 = 1.24;

/// How high off the plain the eye stands, as a multiple of the massif's radius.
///
/// Low. The eye is in the valley the massif stands in and not on a hill opposite it, because a
/// camera raised to half the subject's own height looks **down** on it, and a mountain seen from
/// above is a shape on a table. Every fraction of a radius above the ground the eye goes costs
/// the sense of scale the picture is trying to have.
const EYE_HEIGHT: f64 = 0.24;

/// How far along the valley the aim is taken, as a multiple of a range's radius.
///
/// **Along the axis and not across it.** Aiming across would put one wall in the middle of the
/// frame and the other off the edge, which is a mountain shot again; aiming along puts the two
/// walls at the edges and the corridor between them, which is the valley.
const AIM_ALONG: f64 = 2.60;

/// How far the aim is raised above the massif's own centre, as a multiple of its radius.
///
/// Above the centre, not below it: the eye is near the ground, so an aim at the middle of the
/// rock points the frame's centre into the massif's waist and leaves the summit in the top tenth.
const AIM_HEIGHT: f64 = 0.06;

/// How far round from the eye's own bearing the sun stands, in radians.
///
/// # Invariants
///
/// **Ahead of the eye, which is the whole reversal of what this scene used to do.** The eye looks
/// **along** the valley's axis, so a sun at an offset of a quarter turn and more was behind the
/// eye and lit every flank that faced the camera: the ranges came out as two bright brown cones,
/// which is a massif shot. A sun **near the axis** is a sun the camera is looking at, and a
/// camera looking at the sun sees the shadowed side of everything — the two walls go to silhouette
/// and the brightest thing left in the frame is the water between them, which is the reading the
/// valley is for.
///
/// It is not **zero**. A sun exactly down the axis makes the two walls identical in tone and
/// leaves the picture bilateral to a fault; a quarter of a turn off it puts the sun out of the
/// right of the frame, so one wall is a little darker than the other and the light has a side.
const SUN_OFFSET: f64 = 0.46;

/// How high the sun stands, in radians.
///
/// **Low, and lower than a mountain shot wants it.** Near the horizon the light rakes along the
/// flanks, so a facet turned a few degrees from it is markedly brighter than its neighbour, and
/// the boundary between two facets is a line of light rather than a line of paint. That is the
/// job this number used to do alone, and now it has a second: the sun has to be low enough that
/// it is **in** the sky the valley opens onto, because the glow around it is what the river
/// reflects and the rays that come through the cloud gaps are drawn from.
const SUN_ELEVATION: f64 = 0.34;

/// One of the valley's two ranges, and where it stands in the plan.
pub struct Range {
    /// What the content calls it, which is also the first segment of every belt path it owns.
    pub name: &'static str,
    /// Where its own centre stands, as multiples of a range's radius either side of the axis.
    pub centre: [f64; 2],
}

/// The two ranges the valley runs between, in the order their ids are numbered.
///
/// # Invariants
///
/// - **They are the same shape, stood in two places.** One massif is authored once and stood
///   twice, so the two sides of the valley cannot drift apart: the shape of the valley is the
///   shape of one range and where it was put. Two separately authored ranges would be two things
///   to keep in step, and the valley would stop being a valley the first time one of them moved.
///
/// - **They stand either side of `x = 0`, which is the valley's axis.** The eye, the aim and the
///   river all sit on that one line, and every layer of depth in this scene is read off it. A
///   valley is a corridor, and a corridor is defined by the line down it rather than by what
///   stands beside it.
///
/// - **They are close enough that the eye is between them.** Two ranges far enough apart to see
///   between would frame a gap of empty plain; the whole reading of the shot depends on the walls
///   being near enough to crop the frame, which is why the spacing is under two radii.
///
/// - **The order is the order their id runs follow one another**, so a caller numbering them walks
///   this list rather than inventing an order, and two tables cannot be numbered out of step with
///   the meshes they name.
pub const RANGES: [Range; 2] = [
    Range {
        name: "west",
        centre: [-1.20, 0.0],
    },
    Range {
        name: "east",
        centre: [1.20, 0.0],
    },
];

/// Returns the valley's ranges, unscaled.
///
/// # Invariants
///
/// **It is [`RANGES`] and nothing else**, and the spacing it carries is a fraction of a range
/// rather than a distance — so a caller scales it by whatever radius it is working at. Every
/// consumer — the id numbering, the mesh list, the placements, the reverse lookup — walks this one
/// list, and a range cannot be added to one of them and missed by another.
pub fn ranges() -> &'static [Range] {
    &RANGES
}

/// How far behind the ranges the eye stands, as a multiple of a range's radius.
const EYE_SET_BACK: f64 = 1.85;

/// Returns a massif stood with its own centre over a point in the plan.
///
/// # Invariants
///
/// - **It is a translation and nothing else.** The plan area, the winding and every topological
///   number are unchanged, which is the whole reason the same authored range can serve as both
///   walls of the valley: standing it somewhere is a fact about the picture, and a mesh that
///   changed shape when it moved would be a second mountain.
/// - **The height is not touched.** A valley floor is at one level on both sides of it, and a
///   range that rose as it moved would tilt its own foot out of the plain it stands in.
pub fn stood_at(mesh: &FacetMesh, at: [f64; 2]) -> FacetMesh {
    let (low, high) = mesh.bounds();
    let centre = [
        ((low[0] + high[0]) * 0.5) as f64,
        ((low[2] + high[2]) * 0.5) as f64,
    ];
    let mut moved = mesh.clone();
    for point in &mut moved.points {
        point[0] += (at[0] - centre[0]) as f32;
        point[2] += (at[1] - centre[1]) as f32;
    }
    moved
}

/// Returns where the eye stands, so a caller can measure the picture from it.
///
/// # Invariants
///
/// **It is the one eye [`landscape`] compiles**, read back rather than written twice. A probe that
/// placed its own eye to answer "how wide does this range stand in the frame" would be measuring
/// a camera nobody renders, and every framing number it reported would be a number about that one.
pub fn eye_at(radius: f64) -> [f64; 3] {
    [0.0, RIM + radius * EYE_HEIGHT, -radius * EYE_SET_BACK]
}

/// Returns how wide the frame is, in radians, for the same reason as [`eye_at`].
pub fn field_of_view() -> f64 {
    FIELD_OF_VIEW
}

/// Returns, for each range, the azimuth of its summit and of its nearest point, in degrees off
/// the view axis, read from the eye.
///
/// # Invariants
///
/// **Azimuth and not a position**, because the question this answers is whether a range fills the
/// frame or sits in it — and a point's distance from the eye says nothing about that, while its
/// angle off the view axis says exactly it. The half-frame is returned beside them, because a
/// bearing is a number without one.
///
/// The two come out **within a degree or two of each other**, and that is the finding rather than
/// a defect: a range's nearest point and its summit are both on the ray from the eye through the
/// range's own centre, so a range that is one blob in the plan subtends **one narrow slice of
/// azimuth** — two degrees here, against a half-frame of thirty-five. A two-degree object is a
/// thing standing in the frame, not a wall of it, and no relief and no focal length changes that:
/// to be a wall the range would have to reach from the axis out past the edge, which is the whole
/// width of the frame. A range asked to do both wants a plan that is not a disc. See
/// `.agents/proposals/valley-corridor.md`.
pub fn framing() -> Vec<(&'static str, f64, f64)> {
    let built = massif().build().expect("the authored massif expands");
    let (low, high) = built.bounds();
    let radius = (((high[0] - low[0]).powi(2) + (high[2] - low[2]).powi(2)) as f64).sqrt() * 0.5;
    let eye = eye_at(radius);
    let centre = [
        ((low[0] + high[0]) * 0.5) as f64,
        ((low[2] + high[2]) * 0.5) as f64,
    ];
    let square = |point: [f64; 3], from: [f64; 3]| {
        (point[0] - from[0]).powi(2) + (point[1] - from[1]).powi(2) + (point[2] - from[2]).powi(2)
    };
    let azimuth = |point: [f64; 3], from: [f64; 3]| {
        (point[0] - from[0]).atan2(point[2] - from[2]).abs() * 180.0 / std::f64::consts::PI
    };

    RANGES
        .iter()
        .map(|range| {
            // `stood_at` is a translation, so a point of the stood range and a point of the
            // authored one are the same point measured from two eyes: the eye is shifted back by
            // the same amount rather than the range being shifted forward.
            let shift = [
                range.centre[0] * radius - centre[0],
                range.centre[1] * radius - centre[1],
            ];
            let local_eye = [eye[0] - shift[0], eye[1], eye[2] - shift[1]];
            let summit = built
                .points
                .iter()
                .max_by(|left, right| left[1].total_cmp(&right[1]))
                .map(|point| [point[0] as f64, point[1] as f64, point[2] as f64])
                .unwrap_or([0.0, 0.0, 0.0]);
            let nearest = built
                .points
                .iter()
                .min_by(|left, right| {
                    let left = [left[0] as f64, left[1] as f64, left[2] as f64];
                    let right = [right[0] as f64, right[1] as f64, right[2] as f64];
                    square(left, local_eye).total_cmp(&square(right, local_eye))
                })
                .map(|point| [point[0] as f64, point[1] as f64, point[2] as f64])
                .unwrap_or([0.0, 0.0, 0.0]);
            (
                range.name,
                azimuth(summit, local_eye),
                azimuth(nearest, local_eye),
            )
        })
        .collect()
}

/// Builds the landscape.
pub fn landscape() -> Value {
    // One massif is authored once and **stood twice**, once for each side of the valley. That is
    // the whole of what keeps the two ranges from being two things that can drift apart: the
    // silhouette of the valley is the silhouette of one range and where it was put.
    let authored = massif();
    let built = authored.build().expect("the authored massif expands");
    let (low, high) = built.bounds();
    let radius = (((high[0] - low[0]).powi(2) + (high[2] - low[2]).powi(2)) as f64).sqrt() * 0.5;
    let summit = high[1] as f64;

    // One walk numbers every belt and records the row that names it, and **the runs follow one
    // another** rather than each beginning at one — two tables starting at one would hand the
    // same number to a flank on each side, and a pixel would resolve to whichever one a reader
    // asked about first.
    let mut next_id = 1;
    let mut ranges_built: Vec<(&'static str, FacetMesh, Identity)> =
        Vec::with_capacity(RANGES.len());
    for range in RANGES {
        let identity = Identity::of(&authored, next_id, |belt| belt_path(range.name, belt));
        next_id += identity.rows().len() as u32;
        ranges_built.push((
            range.name,
            stood_at(&built, [range.centre[0] * radius, range.centre[1] * radius]),
            identity,
        ));
    }

    // The eye stands **in the valley**, on its own axis, and looks along it rather than at
    // anything. That is the whole difference between a massif photographed and a valley walked
    // into: a subject in the middle of the frame reads as a specimen, and a corridor with its
    // walls at the edges reads as somewhere to be.
    let eye = [0.0, RIM + radius * EYE_HEIGHT, -radius * EYE_SET_BACK];
    let target = [0.0, RIM + radius * AIM_HEIGHT, radius * AIM_ALONG];
    let distance = ((eye[0] - target[0]).powi(2)
        + (eye[1] - target[1]).powi(2)
        + (eye[2] - target[2]).powi(2))
    .sqrt();

    let plain = radius * PLAIN_REACH;
    // Where the sun stands, as a bearing **off the camera's own**: the light is derived from the
    // eye rather than chosen beside it, so the two cannot be edited apart. A sun picked
    // independently reads well for exactly one camera position and is behind the mountain for the
    // next, and the failure looks like a scene that has gone dark for no reason.
    //
    // The eye is **on the valley's axis looking along it**, so its bearing is the axis rather than
    // a number of its own — and the sun is a little over a quarter turn off that, which puts it
    // across the valley and low: a sun behind the eye flattens the ranges, and one behind them
    // draws both walls as silhouettes with the river bright between them.
    let camera_bearing = std::f64::consts::FRAC_PI_2;
    let sun_bearing = camera_bearing + SUN_OFFSET;
    let sun = [
        sun_bearing.cos() * SUN_ELEVATION.cos(),
        SUN_ELEVATION.sin(),
        sun_bearing.sin() * SUN_ELEVATION.cos(),
    ];
    // The density that puts [`SUBJECT_HAZE`] of the air between the eye and the massif, and that
    // therefore closes it over the plain's far edge by a few times that. Derived and not chosen:
    // the two distances are the only numbers the answer depends on.
    let atmosphere = Atmosphere::new()
        .zenith([0.10, 0.22, 0.52])
        .horizon([0.62, 0.68, 0.78])
        .ground([0.36, 0.36, 0.38])
        .sun(sun, [1.0, 0.68, 0.36], 5.0, 0.60)
        .density(-(1.0 - SUBJECT_HAZE).ln() / distance)
        .scale_height(radius * 1.8);

    let helpers = format!(
        "{}\n{}\n{}",
        atmosphere.wgsl(),
        NOISE,
        constants(radius, summit, sun),
    );
    // The far ranges are the post stage's alone, so they are handed to the post stage alone.
    // They are a backdrop the air has already closed over, and a surface shader has no use for a
    // line painted into the sky.
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
        // The three images this scene's **own** shaders write, beside the frame. The massif's
        // read them from its points; the plain's write the reserved zero, because a plain has no
        // authored element of its own. Both have to write them: a stage that drew geometry and
        // did not would leave the image holding whatever an earlier fragment put there.
        (OUTPUTS.to_owned(), outputs_field()),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed(
                eye,
                target,
                FIELD_OF_VIEW,
                (radius * 0.02).max(0.5),
                plain * 1.2,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                // The sun is a **directional** light and not a place, and here that is a
                // correction rather than a preference. A point light's brightness falls off as the
                // square of its distance, and this scene has a subject six hundred units across
                // standing on a plain nine times that: a lamp bright enough for the massif leaves
                // the far plain black, and one bright enough for the far plain blows the massif
                // out. A light that reaches every distance equally is the only one that can draw
                // both, which is what the sun is.
                //
                // `direction` is the way the light **travels**, which is the negation of the
                // bearing the sky is drawn with. The two are the same decision and only one of
                // them is stated: [`sun`] is computed from the camera's own bearing above and this
                // reads it, so a scene that moved the eye moves the light with it.
                directional(
                    "sun",
                    [-sun[0], -sun[1], -sun[2]],
                    [1.00, 0.86, 0.66],
                    3.4,
                    radius * 1.6,
                ),
                // A fill from the opposite bearing and from above, dim and blue, standing in for
                // the sky. One light with one falloff leaves every shadowed face black, and black
                // is not what a shadow looks like outdoors.
                //
                // It is a **point** light because the pipeline carries exactly one directional
                // light: a light that stands nowhere is in no cluster list, so a second one has
                // nowhere to be read from and the compiler refuses the document rather than
                // ignoring half of it. A point light near the massif is the same fill to the
                // mountain, and the plain it lights weakly is already lit by the sun.
                //
                // **It stands ahead of the eye and over it, which is where this scene's sky
                // is.** With the sun down the valley the walls are all shadow side, and what
                // stands between a wall in shadow and a black rectangle is the light coming off
                // the bright sky the eye is looking into. Putting the fill behind the eye instead
                // — where a fill usually goes, because that is where a subject is — would light
                // the walls the camera cannot see and leave the ones it can see on the sun's own
                // terms, which is nothing.
                //
                // The reach holds the whole scene, which promotes it to a global light: it takes
                // no cluster slot and no fragment can be culled out of its reach. The number is
                // therefore not a distance it lights to but a statement that it lights
                // everything.
                //
                // It stands **over the valley rather than over the middle of the scene.** Its
                // brightness at a surface goes as the square of the distance to it, and the scene
                // is sixty times its own size, so where it stands is worth more to this picture
                // than how bright it is: hung over the ground between the two walls it is near
                // both of them, and set back over the plain's centre — which is where a fill
                // usually goes, because that is where the subject was — it is a quarter of the
                // way from the walls and reads as a lamp at the end of the valley.
                //
                // The power is the light's whole output, and this scene is thirty eight thousand
                // units across, so the number is large in a way a reader should be able to
                // check: the lamp stands about 23,000 units above the plain, and 23,000² is
                // 5.4 × 10⁸. A power of 4.5 × 10⁵ therefore arrives as the same radiance the
                // scene always had, which is what a lamp that far off has to be. See
                // `.agents/proposals/physical-light-units.md`.
                light_with_radius(
                    "sky",
                    [0.0, radius * 1.2, 0.0],
                    [0.40, 0.46, 0.64],
                    450_000.0,
                    radius * 1.6,
                    plain * 2.0,
                ),
            ]),
        ),
        ("shaders".to_owned(), Value::List(shaders)),
        (
            "meshes".to_owned(),
            // **One mesh per range, and they are told apart by name.** A placement names a mesh
            // by name, so two ranges drawn from one geometry still need two names — and the name
            // has to be the range's, because the belt paths a pixel resolves to start with it.
            Value::List({
                let mut entries: Vec<Value> = ranges_built
                    .iter()
                    .map(|(name, geometry, identity)| {
                        mesh(
                            &format!("range_{name}"),
                            "Triangles",
                            // The point schema is **the one the outputs' fields make**, so the
                            // vertex layout and the shader's parameter list are derived from one
                            // schema rather than two that have to agree.
                            massif_extras().point_schema(),
                            identity::faceted(geometry, identity),
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
                // One material per stage **per mesh**, because a material names one shader and
                // each mesh has its own four. The shadowing and depthing stages read no custom
                // data, so theirs is a placeholder those shaders ignore.
                //
                // The plain's lighting material is under the same name it always was, because a
                // scene's own naming is its own; what changed is that the shader behind it now
                // also writes the three outputs, all of them the reserved zero.
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
                // The stone, which the surface body replaces entirely: the tint is **white** and
                // the body writes the albedo, so the colour of the rock is decided in one place.
                // A tint with a colour in it would be multiplied by the body's albedo, which is
                // two decisions about one surface and the sort that drift apart.
                mesh_material(
                    "shade",
                    "massif_lighting",
                    surface([1.0, 1.0, 1.0], 0.0, 0.86, [0.0; 3]),
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
/// **Every range appears in all three buckets.** A material names one shader and a shader belongs
/// to one stage, so the three lists are the same placements under three materials and three sets
/// of labels — a label names one instance in the whole scene, not one per pass. A range left out
/// of one of them is geometry that never draws, which is a scene that validates and renders an
/// empty corridor.
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

/// How many nodes a crest is written with.
///
/// It is a node count and not a distance because a crest that wanders needs the nodes to wander
/// with: the wander is written into the node positions, so a crest sampled coarsely from a
/// straight bearing would lose the very thing that makes it not a radius.
const CREST_NODES: usize = 17;

/// How many crests this massif has.
const CREST_COUNT: usize = 10;

/// One hand-placed crest.
///
/// # Invariants
///
/// Every field is a number the author chose, and none of them is derivable from another. That is
/// the point of the table: the fields are deliberately **uncorrelated**, so the crests do not
/// sort into a family of similar ridges the way a generator's parameters do.
/// [`Crest::reach`] and the last entry of [`Crest::heights`] in particular are independent, which
/// is what puts a short crest high and a long one low — a shoulder beside the summit rather than
/// another skirt of it.
///
/// - **`sharpness` is read twice.** It is the stone the face is cut from, which travels to the
///   shader as the belt's ruggedness, and it sets [`Crest::sampling`]. One number, one decision:
///   the crests that are cut to be read as rock are the ones sampled finely enough to read.
/// - **`bend` wanders the crest in plan and is zero at both ends.** It is what keeps a crest from
///   reading as a radius, and it is bounded well inside the half-angle its own valley subtends,
///   because a crest that wandered further would put its valley on the far side of it — which
///   `validate` reports as a stretch of line that nothing faces.
#[derive(Debug, Clone, Copy)]
struct Crest {
    /// Where the crest runs, in degrees round the summit.
    bearing: f32,
    /// How far it reaches in the plan.
    reach: f32,
    /// How sharp the crest is, zero rounded and one a knife edge.
    sharpness: f32,
    /// Its flank width, as a fraction of its own reach.
    half_width: f32,
    /// How far it wanders off its own bearing at its middle, as a fraction of its reach.
    bend: f32,
    /// The elevation of every one of its nodes, first to last.
    ///
    /// # Invariants
    ///
    /// **One number per node, with no curve behind them.** A crest is [`CREST_NODES`] elevations
    /// from the summit to the rim and the mesh is sampled at exactly these values, so a bench an
    /// author writes is a bench in the geometry rather than a shape a formula happens to produce.
    /// A single curve from summit to foot cannot promise the two shapes a real crest is made of —
    /// a **bench**, where two of its numbers sit close together and the crest runs out nearly
    /// level for a stretch, and a **headwall**, where it holds its height and then goes down all
    /// at once — and it is that promise, not the silhouette, that separates a mountain from a
    /// tent.
    ///
    /// The column is **non-increasing**, which is what a line leaving a summit must be. Its first
    /// entry is [`SUMMIT_ELEVATION`] because every crest starts at the one apex, and its last is
    /// where the crest meets the rim.
    heights: [f32; CREST_NODES],
}

impl Crest {
    /// Returns the plan spacing multiplier this crest is sampled at.
    ///
    /// # Invariants
    ///
    /// It falls as [`Crest::sharpness`] rises, so the two are one decision rather than two that
    /// can be edited apart: a crest cut as a knife edge is also the one whose facets are small
    /// enough for the knife edge to be made of them.
    fn sampling(&self) -> f32 {
        1.15 - 0.55 * self.sharpness
    }

    /// Returns the authored elevation of one of this crest's nodes.
    ///
    /// # Invariants
    ///
    /// It is the table read and nothing else, because the mesh samples a crest at exactly these
    /// nodes. The read between two of them is [`Crest::height_at`]'s job and exists for the
    /// correspondence between a valley's station and a crest's parameter, not for the mesh.
    fn height(&self, node: usize) -> f32 {
        self.heights[node]
    }

    /// Returns the elevation this crest is at between two of its nodes.
    ///
    /// # Invariants
    ///
    /// **Piecewise linear in the authored numbers, and non-increasing throughout** because they
    /// are. A valley names its stations by *parameter* along each crest it runs beside rather
    /// than by node index, so the floor under such a station is this read and not
    /// [`Crest::height`] — which is what lets "the valley runs below the rock on both sides of it"
    /// be checked against the table instead of argued about.
    fn height_at(&self, fraction: f32) -> f32 {
        let position = fraction.clamp(0.0, 1.0) * (CREST_NODES - 1) as f32;
        let low = (position.floor() as usize).min(CREST_NODES - 2);
        let amount = position - low as f32;
        let (from, to) = (self.heights[low], self.heights[low + 1]);
        from + (to - from) * amount
    }

    /// Returns where this crest stands at a fraction of its run.
    fn position(&self, fraction: f32) -> [f32; 2] {
        let bearing = self.bearing.to_radians();
        let along = [bearing.cos(), bearing.sin()];
        let across = [-along[1], along[0]];
        let wander = self.bend * self.reach * (std::f32::consts::PI * fraction).sin();
        [
            SUMMIT[0] + along[0] * self.reach * fraction + across[0] * wander,
            SUMMIT[1] + along[1] * self.reach * fraction + across[1] * wander,
        ]
    }
}

/// The crests of this massif, in the order they were placed.
///
/// # Invariants
///
/// **The bearings are ascending and irregular.** They have to ascend, because a wedge is taken
/// between two angular neighbours and a bearing that reordered them would reorder the massif's
/// topology; they are irregular because ten crests at a uniform thirty-six degrees is a ten-pointed
/// star, and the eye reads a star as a generator's output before it reads anything else. The gaps
/// run from twenty-six to forty-four degrees.
///
/// **Neighbouring reaches differ by no more than about 1.6 to one, and that bound is the whole
/// reason for it.** A wedge's corner stands past the *further* of its two tips, so a long crest
/// beside a short one puts the corner most of a crest's length beyond the short one, and the
/// region between the two tips and that corner becomes a long thin sliver of facets — a fan of
/// triangles lying on the ground at the foot of the mountain. The overall spread is still nearly
/// three to one, because a tongue of rock running out to the plain and a buttress stopping short
/// of it are the two things a real foot is made of; what is forbidden is the jump *between*
/// neighbours, which is a fact about the walk rather than about the mountain.
///
/// **The reaches run from 295 to 820 and the last heights from 7 to 64**, and the two are placed
/// independently of each other. The reach sets where the foot is, and the last height sets how
/// much of the mountain is still standing there, so a short crest that ends high is a horn over a
/// shoulder and a long crest that ends low is a tongue running out to the plain. Those two
/// readings are what a fan cannot produce, because in a fan they are the same number.
///
/// **The three horns — 2, 5 and 6 — are the crests that make the skyline.** Each is short and
/// holds its height well past its middle, so each stands beside the summit as a crest of its own
/// with a saddle between, and the silhouette stops being a triangle. They are the reason the
/// picture reads as a mountain rather than as a cone with facets on it.
///
/// **Each column of heights is a different fall**, read across rather than down: crest 2 runs out
/// nearly level for its first half before turning down, crest 5 drops away early and carries on
/// as a tail, and crest 6 — the shortest of all — holds above five hundred until its last
/// third. A column of evenly spaced numbers is a ruled surface, and ten of those meeting at one
/// point is the tent this scene was rewritten to stop being, whatever else is done to them.
const CRESTS: [Crest; CREST_COUNT] = [
    Crest {
        bearing: 0.0,
        reach: 820.0,
        sharpness: 0.40,
        half_width: 0.030,
        bend: 0.075,
        heights: [
            1044.0, 1001.0, 950.0, 893.0, 832.0, 766.0, 693.0, 615.0, 539.0, 487.0, 444.0, 412.0,
            389.0, 363.0, 310.0, 203.0, 26.0,
        ],
    },
    Crest {
        bearing: 42.0,
        reach: 520.0,
        sharpness: 0.52,
        half_width: 0.038,
        bend: -0.065,
        heights: [
            1044.0, 980.0, 916.0, 856.0, 795.0, 731.0, 664.0, 592.0, 516.0, 464.0, 429.0, 415.0,
            403.0, 394.0, 363.0, 249.0, 16.0,
        ],
    },
    Crest {
        bearing: 73.0,
        reach: 330.0,
        sharpness: 0.94,
        half_width: 0.022,
        bend: 0.045,
        heights: [
            1044.0, 1038.0, 1032.0, 1025.0, 1015.0, 1001.0, 980.0, 954.0, 922.0, 882.0, 832.0,
            774.0, 708.0, 635.0, 574.0, 479.0, 84.0,
        ],
    },
    Crest {
        bearing: 111.0,
        reach: 530.0,
        sharpness: 0.66,
        half_width: 0.032,
        bend: -0.085,
        heights: [
            1044.0, 974.0, 905.0, 838.0, 777.0, 725.0, 684.0, 655.0, 635.0, 618.0, 597.0, 568.0,
            528.0, 479.0, 435.0, 316.0, 23.0,
        ],
    },
    Crest {
        bearing: 140.0,
        reach: 340.0,
        sharpness: 0.44,
        half_width: 0.040,
        bend: 0.070,
        heights: [
            1044.0, 951.0, 864.0, 783.0, 708.0, 638.0, 574.0, 516.0, 464.0, 418.0, 374.0, 334.0,
            296.0, 258.0, 220.0, 151.0, 10.0,
        ],
    },
    Crest {
        bearing: 184.0,
        reach: 470.0,
        sharpness: 0.84,
        half_width: 0.026,
        bend: -0.050,
        heights: [
            1044.0, 1024.0, 995.0, 957.0, 911.0, 856.0, 792.0, 722.0, 650.0, 580.0, 516.0, 458.0,
            406.0, 360.0, 310.0, 215.0, 62.0,
        ],
    },
    Crest {
        bearing: 217.0,
        reach: 295.0,
        sharpness: 0.90,
        half_width: 0.024,
        bend: 0.045,
        heights: [
            1044.0, 1035.0, 1024.0, 1006.0, 983.0, 954.0, 919.0, 879.0, 832.0, 783.0, 731.0, 679.0,
            624.0, 568.0, 516.0, 435.0, 93.0,
        ],
    },
    Crest {
        bearing: 253.0,
        reach: 460.0,
        sharpness: 0.60,
        half_width: 0.034,
        bend: -0.075,
        heights: [
            1044.0, 969.0, 896.0, 827.0, 760.0, 696.0, 635.0, 577.0, 522.0, 470.0, 421.0, 374.0,
            331.0, 287.0, 247.0, 171.0, 19.0,
        ],
    },
    Crest {
        bearing: 294.0,
        reach: 410.0,
        sharpness: 0.40,
        half_width: 0.042,
        bend: 0.080,
        heights: [
            1044.0, 940.0, 841.0, 748.0, 661.0, 580.0, 505.0, 435.0, 371.0, 313.0, 264.0, 226.0,
            197.0, 174.0, 148.0, 99.0, 10.0,
        ],
    },
    Crest {
        bearing: 328.0,
        reach: 640.0,
        sharpness: 0.74,
        half_width: 0.028,
        bend: -0.060,
        heights: [
            1044.0, 992.0, 937.0, 882.0, 827.0, 771.0, 716.0, 661.0, 606.0, 557.0, 516.0, 484.0,
            461.0, 444.0, 389.0, 258.0, 39.0,
        ],
    },
];

/// Where the summit stands in the plan.
///
/// It is the origin because every crest's numbers are written relative to it, and a summit
/// somewhere else would mean every bearing below was a bearing from a point nobody can see.
const SUMMIT: [f32; 2] = [0.0, 0.0];

/// How high the summit stands.
///
/// It is **low against the massif's width on purpose**, and the ratio is the whole of the shape.
/// A summit at four times the radius of its own foot is a spire: every crest leaves it at once,
/// every flank falls away from the same point, and the silhouette is a triangle no matter what the
/// crests are doing. Real relief is nearer a half again as wide as it is tall — the Matterhorn
/// carries four thousand metres over ten kilometres of mountain — and it is that proportion, more
/// than any detail of the skeleton, that makes a picture read as a massif rather than as an
/// object on a table.
///
/// It is still **low against the massif's width**, and that ratio is the constraint it is written
/// under: a summit at four times the radius of its own foot is a spire, and no arrangement of
/// crests about it stops the silhouette being a triangle. What changed here is not the ratio but
/// **what the number is for**. It used to be the height at which a single massif was photographed
/// from outside, and the eye was set to suit it. The massif is now one of two walls of a valley
/// seen from between them, and the number that governs the picture is the angle the summit makes
/// from the eye — which is the summit over the distance to it, and which a summit chosen to look
/// right as an object puts far too low in the frame. The tables above are written to this number
/// and the eye in [`landscape`] is set from the angle this one makes.
const SUMMIT_ELEVATION: f32 = 1044.0;

/// How far past the two tips a wedge's mouth stands on the rim.
///
/// It has to be past both of them, or the rim folds back on the chord between them and the
/// boundary is no longer a simple loop — and the thing an author would then be asked to fix is
/// the footprint rather than the mountain.
///
/// It is a good deal more than it has to be for that reason alone, because of what it does to
/// the **last** cell of each flank. That cell is bounded above by the rim arc from a crest's tip
/// to its valley's mouth — a hundred units or more of rim — and below by the valley's own run
/// from its corner to that same mouth, which is a tenth of the corner's distance. A rim arc
/// three times the length of the valley run under it zips into triangles three times longer than
/// they are wide, and ten flanks of those read as a fan of straw laid round the foot of the
/// mountain. Pushing the mouth out lengthens the valley run until the two are comparable, and
/// the lobed rim it produces — a bay between every pair of crest tips — is what a massif's foot
/// looks like anyway.
const MOUTH_BULGE: f32 = 0.28;

/// How many stations a valley places between its summit and its corner.
///
/// It is the number of ribs a flank is divided into, so it is the flank's resolution: too few and
/// the flank is one ruled plane, too many and the facets stop reading as faces.
const WEDGE_STEPS: usize = 11;

/// Returns the massif this scene is of: the crests above, and the valleys between them.
///
/// Public because the id a rendered pixel carries names something in *this* massif, and
/// answering that means building it again — see [crate::identity::locate]. A caller that wanted
/// only the path would still need the skeleton, so the two are published together.
///
/// # Why the crests are written out rather than generated
///
/// [`mountain::Starter`] would make a fan, and a fan is a cone: crests of one reach at one
/// profile, evenly
/// spaced, so every flank reaches the same height at the same distance and the silhouette is a
/// polygon meeting at one apex. Hand-editing a fan cannot fix that, because every edit that
/// would have to bend a flank also has to leave the ridge it belongs to in the right order around
/// the summit. The asymmetry has to be in the skeleton itself.
///
/// # Why every crest runs from the one summit
///
/// Elevation is non-increasing along every root-to-leaf path, so a connected skeleton has exactly
/// one summit — and a spur hung off a parent's flank at `t > 0` does not merely break that rule,
/// it **doubles the wedge between them on itself**. That wedge's fan of triangles around the
/// apex closes only when its first row collapses onto the apex, and the edge that collapse leaves
/// runs from the apex to the spur's junction, which is a segment of the parent ridge the same
/// wedge is already covering on its other side. The hierarchy is therefore written as
/// [`Ridge::child`] on a parent whose first station is the summit, which is what `Starter` does
/// too, and the asymmetry is carried entirely by the crests being different from each other.
///
/// [`mountain::derive_valleys`] cannot fill in the rest: its domain is a fan of **equal**
/// reaches, because
/// a wedge's corner is only closed-form when both ridges reach their tips at the same point on
/// the bisector. [`wedge`] is the same walk with that one assumption dropped.
pub fn massif() -> Massif {
    let wedges: Vec<Wedge> = (0..CREST_COUNT)
        .map(|index| wedge(index, (index + 1) % CREST_COUNT))
        .collect();
    let outline = footprint(&wedges);
    let mut massif = Massif::new(Skeleton::new(ridges(), bound(wedges, &outline), outline))
        .with_tessellation(Tessellation::default().with_step(STEP));

    stand_on_the_plain(&mut massif);

    // The heights in the two tables are numbers an author typed, so they are checked against each
    // other before anything is built from them. A valley floor at or above either crest it runs
    // between is not a valley: the flank beside it stops being a flank and the picture loses the
    // one relationship the whole massif is drawn from.
    debug_assert!(
        check_the_authored_heights().is_empty(),
        "the authored heights disagree with each other: {:#?}",
        check_the_authored_heights()
    );

    // The skeleton here is written by hand, so the correspondence it wrote is checked before it is
    // expanded. A rib that crosses a line, or a stretch of a line that no station faces, builds a
    // mesh that overlaps itself or has a hole in it — and both of those are perfectly ordinary
    // pictures of a mountain. This is the one place the scene can be told it is wrong, and it is
    // why the massif has an example of its own: `cargo run -p whiting-gallery --example
    // alpine_probe` reports every violation, draws the plan and the skyline, and measures the
    // facets, without a gpu anywhere in it.
    debug_assert!(
        massif.validate().is_empty(),
        "the authored massif is not valid: {:#?}",
        massif.validate()
    );
    massif
}

/// Checks that the two tables of heights can be drawn together.
///
/// # Invariants
///
/// Three things are checked, and all three are about the tables being edited against each other
/// rather than about the mesh:
///
/// - **A valley's first node is the summit.** Every valley begins at the one apex and the ten of
///   them are separate vertices that happen to stand on it, so a first column that disagreed
///   would tear the apex open rather than build a different mountain.
/// - **A valley floor is strictly below both crests it runs between, at every station.** A station
///   names a parameter rather than a node, so this compares against [`Crest::height_at`] and it is
///   the check that would otherwise be impossible to make by eye.
/// - **Neither table rises.** `validate` checks this for the skeleton, but only after it is built
///   and only for the lines it was handed; saying it here names the table as the thing that has to
///   hold.
fn check_the_authored_heights() -> Vec<String> {
    let mut problems = Vec::new();
    for wedge in 0..CREST_COUNT {
        let right = (wedge + 1) % CREST_COUNT;
        if VALLEY_HEIGHTS[wedge][0] != SUMMIT_ELEVATION {
            problems.push(format!(
                "valley {wedge} starts at {} rather than at the summit's {SUMMIT_ELEVATION}",
                VALLEY_HEIGHTS[wedge][0]
            ));
        }
        for node in 1..VALLEY_NODES {
            let floor = VALLEY_HEIGHTS[wedge][node];
            if node > 0 && floor > VALLEY_HEIGHTS[wedge][node - 1] {
                problems.push(format!(
                    "valley {wedge} rises from {} to {floor} at node {node}",
                    VALLEY_HEIGHTS[wedge][node - 1]
                ));
            }
            for (side, crest) in [(Side::Left, wedge), (Side::Right, right)] {
                let station = valley_station(node);
                let rock = CRESTS[crest].height_at(station);
                if floor >= rock {
                    problems.push(format!(
                        "valley {wedge} node {node} is at {floor}, which is not below crest \
                         {crest}'s {rock:.1} on its {side:?}"
                    ));
                }
            }
        }
    }
    problems
}

/// Returns the crest lines, the first one the root and the rest its children.
fn ridges() -> Vec<Ridge> {
    CRESTS
        .iter()
        .enumerate()
        .map(|(index, crest)| {
            let nodes: Vec<RidgeNode> = (0..CREST_NODES)
                .map(|step| {
                    let fraction = step as f32 / (CREST_NODES - 1) as f32;
                    RidgeNode::new(crest.position(fraction), crest.height(step))
                        .with_half_width(crest.half_width * crest.reach)
                        .with_sharpness(crest.sharpness)
                })
                .collect();
            let ridge = if index == 0 {
                Ridge::root(nodes)
            } else {
                Ridge::child(RidgeId(0), 0.0, nodes)
            };
            ridge.with_step_scale(crest.sampling())
        })
        .collect()
}

/// One wedge's plan, before its mouth can be bound to the rim.
struct Wedge {
    /// Where the mouth stands on the rim.
    mouth: [f32; 2],
    /// The nodes, each with the two crest stations it faces.
    nodes: Vec<([f32; 2], Station, Station)>,
}

/// Returns the wedge between two crests that are neighbours in bearing.
///
/// # Invariants
///
/// - **The corner stands past the further of the two tips.** The last station on each side is
///   `t = 1.0`, so the corner is where both ribs have reached their tips at once; a corner inside
///   the shorter crest's tip would send that rib backwards along it. Dividing the **larger** reach
///   by the cosine of the half angle is what puts the corner on the far side of both, and the
///   fan's version of this number — its own reach, because its reaches are equal — is the one
///   assumption [`mountain::derive_valleys`] is built on.
/// - **Both sides use the same parameter at every node.** A rib may run from a valley node to a
///   crest station of any length; it is a lateral edge of the belt and nothing is required of its
///   length. Walking both sides by one parameter is therefore the whole of the correspondence
///   here, and it is why unequal reaches cost nothing.
/// - **The stations tile each crest exactly once.** Each wedge names its two crests from `0.0` to
///   `1.0` and the wedges are taken between angular neighbours, so every crest has a wedge on each
///   of its two sides and no stretch of it is left facing nothing.
fn wedge(left: usize, right: usize) -> Wedge {
    let half = wrap(CRESTS[right].bearing - CRESTS[left].bearing) * 0.5;
    let middle = (CRESTS[left].bearing + half).to_radians();
    let direction = [middle.cos(), middle.sin()];
    let corner = CRESTS[left].reach.max(CRESTS[right].reach) / half.to_radians().cos().max(1.0e-3);
    let mouth = corner * (1.0 + MOUTH_BULGE);
    let swing = VALLEY_SWING * (left as f32 * 2.3).sin();

    let mut nodes = Vec::with_capacity(VALLEY_NODES);
    nodes.push((
        SUMMIT,
        Station::ridge(RidgeId(left), 0.0),
        Station::ridge(RidgeId(right), 0.0),
    ));
    for step in 1..=WEDGE_STEPS {
        let fraction = valley_station(step);
        let radius = valley_radius(fraction, left, right, corner);
        // The floor **wanders across its own wedge** rather than running down its bisector.
        //
        // Every rib in the massif joins a crest station to the valley node beside it, and a
        // valley on the bisector makes all of them point at the summit: the flanks then read as
        // folds of paper fanned from one point, which is the last of the tent left in the picture
        // once the silhouette is right. A floor that drifts across its wedge turns the ribs into
        // fall lines that fall in different directions, and a flank whose ribs disagree about
        // where down is a flank made of more than one plane.
        //
        // The amount is **signed per wedge from its own index**, so neighbouring floors drift
        // opposite ways and the massif's ribs cross over rather than running parallel, and it is
        // zero at both ends of the run so the summit stays one vertex and the mouth stays on the
        // rim.
        //
        // It is a fraction of the node's **own radius** and not of the wedge's corner, because a
        // wedge is narrow in absolute terms near the summit and wide at the rim: a drift fixed in
        // units is a large share of the wedge at one end and nothing at the other, and the share
        // is the thing that has to stay inside it.
        let lateral = swing * radius * (std::f32::consts::PI * fraction).sin();
        nodes.push((
            [
                SUMMIT[0] + direction[0] * radius - direction[1] * lateral,
                SUMMIT[1] + direction[1] * radius + direction[0] * lateral,
            ],
            Station::ridge(RidgeId(left), fraction),
            Station::ridge(RidgeId(right), fraction),
        ));
    }
    Wedge {
        mouth: [
            SUMMIT[0] + direction[0] * mouth,
            SUMMIT[1] + direction[1] * mouth,
        ],
        nodes,
    }
}

/// Returns the parameter along a valley's run that one of its nodes stands at.
///
/// # Invariants
///
/// The mouth **clamps to the run's end** rather than reaching past it. The mouth is on the rim
/// beyond the corner, so there is no fraction of this valley's own run that is also the fraction
/// of the crests beside it — and reading it as one puts a mouth's height under a crest that has
/// already ended, which is the only way a table row can pass the check in
/// [`check_the_authored_heights`] while still saying something untrue.
fn valley_station(node: usize) -> f32 {
    node.min(WEDGE_STEPS) as f32 / WEDGE_STEPS as f32
}

/// Returns where a valley's node stands at a fraction of its run.
///
/// # Invariants
///
/// **It runs with the crests and only opens out at the end.** The obvious radius for a valley
/// node is the wedge's corner times the fraction, which puts it at the same parameter as the two
/// crest stations beside it — and that reads well until the two reaches differ, because a corner
/// past the *further* tip is well outside the nearer one. The rib from such a node down to the
/// nearer crest's station then runs mostly along the flank instead of across it, and a flank
/// whose cross-edges are three times its along-edges zips into triangles three times longer than
/// they are wide. Half the massif is then slivers, and a massif of slivers reads as folded paper
/// however good its silhouette is.
///
/// So the node follows the **mean of the two reaches** for most of the run, which keeps both ribs
/// short and roughly perpendicular to the crest they meet, and only swings out to the corner over
/// the last third. That last third is where a real valley leaves its flank and opens into its
/// mouth anyway.
fn valley_radius(fraction: f32, left: usize, right: usize, corner: f32) -> f32 {
    let flank = 0.5 * (CRESTS[left].reach + CRESTS[right].reach) * fraction;
    flank + (corner - flank) * smoothstep(VALLEY_MOUTH_OPENING, 1.0, fraction as f64) as f32
}

/// The fraction of a valley's run over which it opens out from its flank to its corner.
const VALLEY_MOUTH_OPENING: f64 = 0.70;

/// How far a valley floor drifts across its own wedge, as a fraction of the wedge's corner.
///
/// It is small, and it has to be: the floor may wander as far as its wedge is wide and no
/// further, or it crosses the crest it is supposed to be the floor of. Seven percent is well
/// inside that and well outside invisible.
const VALLEY_SWING: f32 = 0.07;

/// Returns the authored elevation of one of a wedge's nodes.
///
/// # Invariants
///
/// It is the table read and nothing else. The floor is the author's own number rather than a
/// fraction of the two crests beside it, which is the change that stops the massif's valleys from
/// being one shape repeated ten times — under a formula, every floor was the lower of its two
/// crests times the same curve, and rows that shared a crest came out identical to the digit.
fn valley_elevation(node: usize, wedge: usize) -> f32 {
    VALLEY_HEIGHTS[wedge][node]
}

/// How many nodes a valley is written with: the summit, one per step, and the mouth.
///
/// The mouth is counted rather than left to the walk that binds the valleys to the rim, so that
/// [`VALLEY_HEIGHTS`] is indexed by the same numbers the finished [`Valley`] carries.
const VALLEY_NODES: usize = WEDGE_STEPS + 2;

/// The elevation of every node of every valley, wedge by wedge.
///
/// # Invariants
///
/// Row `i` is the valley between crest `i` and crest `i + 1`, so the last row runs back to the
/// first crest and the table has no gap in it. Each row is [`VALLEY_NODES`] numbers read from the
/// summit outwards, and the last two are the corner and the mouth standing beyond it.
///
/// - **The first column is the summit.** Every valley begins at the one apex, so this column is
///   [`SUMMIT_ELEVATION`] ten times over and [`check_the_authored_heights`] says so.
/// - **Every other column is strictly below both crests it runs between.** That is what makes a
///   floor a floor. The floor is also **never a fixed fraction of the lower of its two crests**,
///   because that is a plane: the same ratio under the same falling crest is a straight line from
///   end to end, and a flank with one across it is a sheet of card. Every row here separates from
///   its lower crest by a different amount at every node, which is what turns a belt into ground.
/// - **Each row is a different cut.** Rows 1 and 2 both run under crest 2's plateau and make a
///   cirque floor and a col out of it; rows 3 and 4 share crest 4 as their lower crest and share
///   nothing else, one being a gorge that goes down early and one a hanging valley that stops
///   over a lip; rows 7 and 8 share crest 8 the same way. A formula gave every row the same
///   curve, and a massif whose ten valleys are the same curve is a massif of ten copies of one
///   valley.
const VALLEY_HEIGHTS: [[f32; VALLEY_NODES]; CREST_COUNT] = [
    [
        1044.0, 590.0, 528.0, 462.0, 396.0, 322.0, 260.0, 223.0, 199.0, 172.0, 111.0, 5.0, 5.0,
    ],
    [
        1044.0, 620.0, 530.0, 455.0, 390.0, 330.0, 272.0, 216.0, 166.0, 120.0, 80.0, 4.0, 4.0,
    ],
    [
        1044.0, 600.0, 528.0, 462.0, 400.0, 344.0, 292.0, 244.0, 196.0, 148.0, 100.0, 6.0, 6.0,
    ],
    [
        1044.0, 514.0, 424.0, 338.0, 266.0, 207.0, 163.0, 126.0, 98.0, 73.0, 47.0, 3.0, 3.0,
    ],
    [
        1044.0, 570.0, 480.0, 400.0, 330.0, 268.0, 214.0, 168.0, 128.0, 94.0, 62.0, 3.0, 3.0,
    ],
    [
        1044.0, 557.0, 509.0, 454.0, 393.0, 329.0, 268.0, 216.0, 172.0, 137.0, 94.0, 20.0, 20.0,
    ],
    [
        1044.0, 586.0, 516.0, 451.0, 387.0, 325.0, 253.0, 179.0, 124.0, 84.0, 49.0, 4.0, 4.0,
    ],
    [
        1044.0, 520.0, 424.0, 340.0, 268.0, 208.0, 160.0, 122.0, 92.0, 68.0, 48.0, 3.0, 3.0,
    ],
    [
        1044.0, 518.0, 417.0, 331.0, 253.0, 190.0, 138.0, 101.0, 75.0, 58.0, 35.0, 3.0, 3.0,
    ],
    [
        1044.0, 572.0, 507.0, 444.0, 364.0, 286.0, 224.0, 181.0, 150.0, 123.0, 76.0, 7.0, 7.0,
    ],
];

/// Binds every wedge's mouth to the rim that was threaded through it.
///
/// # Invariants
///
/// The mouths are placed before the rim exists and the stations naming the rim cannot be written
/// until it does, so the rim is built first and its arc-length parameters are read back into the
/// valleys here. The rim alternates tip and mouth, so wedge `index`'s mouth is vertex
/// `2 * index + 1` of it.
fn bound(wedges: Vec<Wedge>, outline: &[FootprintNode]) -> Vec<Valley> {
    let mut spans = Vec::with_capacity(outline.len());
    let mut total = 0.0;
    for (index, node) in outline.iter().enumerate() {
        spans.push(total);
        total +=
            mountain::math::distance(node.position, outline[(index + 1) % outline.len()].position);
    }
    wedges
        .into_iter()
        .enumerate()
        .map(|(index, wedge)| {
            let mouth = Station::new(
                LineRef::Footprint,
                if total <= f32::EPSILON {
                    0.0
                } else {
                    spans[(2 * index + 1) % spans.len()] / total
                },
            );
            let nodes: Vec<ValleyNode> = wedge
                .nodes
                .into_iter()
                .enumerate()
                .map(|(node, (position, left, right))| {
                    ValleyNode::new(position, valley_elevation(node, index), left, right)
                })
                .chain(std::iter::once(ValleyNode::new(
                    wedge.mouth,
                    valley_elevation(VALLEY_NODES - 1, index),
                    mouth,
                    mouth,
                )))
                .collect();
            // The valley's **start names the summit's station** rather than standing at the same
            // coordinates: every valley begins at the one apex, and a valley that merely stood
            // there would put its own vertex a hair off the crests' — three vertices at a point
            // instead of one, which is the slit the whole `End` vocabulary exists to avoid.
            Valley::new(nodes)
                .with_step_scale(VALLEY_SAMPLING)
                .with_ends(End::On(Station::ridge(RidgeId(0), 0.0)), End::Free)
        })
        .collect()
}

/// How finely a valley floor is sampled, as a multiple of the massif's step.
///
/// A valley is sampled **finer** than a crest on purpose, and it is the opposite of what the
/// crests do. A crest is a long thin line whose samples become the rows of every belt facing it,
/// so fine sampling there buys smaller facets and nothing else. A valley floor is what the fan of
/// slivers converges on, so what it needs is enough samples that none of them is long enough to
/// draw.
const VALLEY_SAMPLING: f32 = 0.80;

/// Returns the rim, threaded through the tips and the mouths.
///
/// # Invariants
///
/// - **It alternates tip and mouth**, which is what lets [`bound`] name wedge `index`'s mouth as
///   vertex `2 * index + 1` without searching for it.
/// - **The mouths stand further out than the tips behind them**, which is what makes this a
///   simple loop. With reaches running from 170 to 545 a mouth drawn inside the chord between its
///   two tips would fold the boundary back on itself, and the thing an author would then be asked
///   to fix is the footprint rather than the mountain.
/// - **It is wound so the inside is on its left.** The expansion winds every face to look up, so a
///   rim wound the other way would put the mesh's plan area at the negation of the footprint's
///   and make the coverage check read a covered massif as inside out.
/// - **Every vertex's height is the last number of a table row**, a crest tip from
///   [`CRESTS`] and a mouth from [`VALLEY_HEIGHTS`]. The rim's vertices are the same points as the
///   crests' and the valleys' last nodes, so reading them from anywhere else would put two
///   elevations on one point and split the foot of the mountain along it.
fn footprint(wedges: &[Wedge]) -> Vec<FootprintNode> {
    wedges
        .iter()
        .enumerate()
        .flat_map(|(index, wedge)| {
            [
                FootprintNode::new(
                    CRESTS[index].position(1.0),
                    CRESTS[index].height(CREST_NODES - 1),
                ),
                FootprintNode::new(wedge.mouth, valley_elevation(VALLEY_NODES - 1, index)),
            ]
        })
        .collect()
}

/// Returns an angle wrapped into zero to a full turn.
///
/// The last crest's bearing and the first's differ by less than a turn but a comparison that does
/// not wrap would read that gap as negative and put the closing wedge on the far side of the
/// summit.
fn wrap(angle: f32) -> f32 {
    angle.rem_euclid(360.0)
}

/// Levels everything that reaches the rim down to the plain.
///
/// # Invariants
///
/// The fall is **tapered over the outer part of every line** rather than set at its last node.
/// Cutting only the last node would leave a cliff at the foot of every flank: the rim is at the
/// plain's height and the node before it is where the ridge's own profile left it, so the drop
/// happens across one segment. A shelf multiplied along the line takes the last fifth of every
/// flank down instead, and the product of two non-increasing non-negative functions is still
/// non-increasing, so the edit cannot make the massif invalid.
///
/// Every ridge tip and every valley mouth **is** a footprint vertex, so all three are set
/// together. Levelling only the footprint would leave each ridge ending above the ground it is
/// supposed to stand on, and the gap is at the one place in the frame the eye is trained on.
fn stand_on_the_plain(massif: &mut Massif) {
    for node in 0..massif.skeleton.footprint.len() {
        massif.set_elevation(LineRef::Footprint, node, RIM as f32);
    }
    for ridge in massif.skeleton.ridge_ids() {
        shelf(massif, LineRef::Ridge(ridge));
    }
    for valley in massif.skeleton.valley_ids() {
        shelf(massif, LineRef::Valley(valley));
    }
}

/// Takes the outer part of a line down to the rim's height.
fn shelf(massif: &mut Massif, line: LineRef) {
    let count = massif.skeleton.node_count(line);
    for node in 0..count {
        let fraction = node as f64 / (count - 1) as f64;
        let current = elevation(massif, line, node);
        let height = current as f64 * (1.0 - smoothstep(0.85, 1.0, fraction));
        massif.set_elevation(line, node, height as f32);
    }
}

/// Returns the elevation of one node of a line.
fn elevation(massif: &Massif, line: LineRef, node: usize) -> f32 {
    match line {
        LineRef::Ridge(ridge) => massif.skeleton.ridges[ridge.0].nodes[node].elevation,
        LineRef::Valley(valley) => massif.skeleton.valleys[valley.0].nodes[node].elevation,
        LineRef::Footprint => massif.skeleton.footprint[node].elevation,
    }
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
/// It is **exactly the rim's height** wherever the massif reaches, and the rolling is gated by
/// distance from the middle so that it is. A plain with a two-unit roll under the massif's rim
/// would cut through the flanks near the boundary and poke through the crests further in, and the
/// massif would look like it was sinking rather than standing.
///
/// The ground rises towards its own edge, so its silhouette is a low horizon instead of the
/// straight line a flat sheet ends on, and that rise is what the haze is sized against.
fn plain_height(x: f64, z: f64, radius: f64) -> f64 {
    let distance = (x * x + z * z).sqrt();
    // Both undulations are gated to begin **outside** the massif's own footprint, which is the
    // whole of the contract: the plain is exactly the rim's height wherever the massif reaches.
    // A two-unit roll inside the rim would cut through the foot of every flank and show as a
    // jagged seam where the mountain meets the ground.
    let beyond = smoothstep(radius * 0.95, radius * 1.25, distance);
    let away = smoothstep(radius * 1.30, radius * 3.2, distance);
    // The undulation is **coarse on purpose, and coarser than it wants to be.** The plain is a
    // grid of a few hundred units a cell, and anything the grid cannot carry comes back as the
    // grid: a hundred-unit ripple sampled every four hundred units is not a ripple, it is four
    // hundred unrelated cells, and near the eye — where one cell is a sixth of the frame — it is
    // a floor tiled in rectangles. The wavelength here is several cells at every octave, which
    // costs the floor its small relief and buys it back as a surface.
    //
    // It is also **off entirely across the floodplain**, which is a floodplain: the ground a
    // braided river runs over is the flattest ground in the picture, and the only relief on it
    // is the river's own bed and the benches cut beside it.
    let roll = fractal_noise([x * 0.00040, z * 0.00044], 3, 41) - 0.5;
    let horizon = smoothstep(radius * 2.2, radius * PLAIN_REACH, distance);
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
/// would reach the foot of both ranges and the corridor would be a plain with two mountains on it.
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
    let steep = smoothstep(0.25, 0.65, in.texture.y);
    let wood = 0.78 + 0.44 * in.texture.x;
    let patches = 0.86 + 0.28 * noise(in.world_position.xz * 0.02, 3);
    let bare = mix(vec3<f32>(0.46, 0.42, 0.37), vec3<f32>(0.19, 0.17, 0.16), steep);
    let stone = bare * wood * patches;

    // The snowline wanders with the ground it lies on rather than running level, and it is driven
    // by the same world position the rock is, so it stays on the mountain. It is held off the
    // steepest faces by the same steepness the stone is read from, because that is where it does
    // not stay.
    let wobble = SNOW_WOBBLE * (noise(in.world_position.xz * 0.004, 3) * 2.0 - 1.0);
    let above = smoothstep(SNOW_LINE + wobble - SNOW_FADE, SNOW_LINE + wobble + SNOW_FADE,
                           in.world_position.y);
    let snow = above * (1.0 - smoothstep(0.78, 0.98, in.texture.y));
    let snowed = mix(stone, vec3<f32>(0.93, 0.95, 1.00), snow);

    // The floor of a valley is soil and scrub rather than stone, and it is warmer for it. It is
    // what separates a valley from a narrower piece of the same slope.
    let floor_mix = 1.0 - smoothstep(0.0, SCENE_RADIUS * 0.45, in.world_position.y);
    color = color * mix(snowed, vec3<f32>(0.30, 0.29, 0.21), floor_mix * 0.75);

    // The **sky a face stands in**, and on this massif it is most of the light there is. The sun
    // in this scene is down the valley and low, so a wall that faces the eye is a face the sun
    // never reaches, and the scene's second light cannot carry that alone: a point light in the
    // middle of a plain sixty times its own size leaves behind it a number about how far away it
    // stands rather than about the sky. A face's own share of the sky is therefore decided here,
    // where the face's normal and the sun's bearing both are.
    //
    // **Upward-facing is what counts, and not merely sun-facing.** An overhang sees the same sky
    // as the ground beneath it and a vertical wall sees half of it, and that gradient is most of
    // what makes an unlit face read as a surface with a shape in it rather than as a hole cut in
    // the picture. Without it a backlit range is a black paper cut-out with one lit facet on it.
    let skyward = 0.35 + 0.65 * clamp(normal.y, 0.0, 1.0);
    let from_sun = clamp(dot(normal, SUN_AHEAD), 0.0, 1.0);
    color = color + air_colour() * skyward * (0.05 + 0.26 * (1.0 - from_sun));

    let eye = uniforms.projection.position;
    let view = normalize(in.world_position - eye);
    let facing = 1.0 - abs(dot(normal, view));
    let haze = air_closure(eye, in.world_position);
    color = mix(color, air_colour(), clamp(haze * (0.45 + 0.55 * facing), 0.0, 0.90));
"#;

/// What the plain's surface does after the light is applied.
///
/// # Invariants
///
/// The plain is drawn by a body of its own rather than by the massif's with a flag in it: it
/// carries none of the per-face facts the massif's body reads, so a shared body would have to be
/// told which mesh it was drawing, and the way to tell it is the material row — which is a branch
/// in a shader for a difference that is a different surface.
///
/// It hazes by the **same law** as the massif, from the same air. That is the whole reason the
/// atmosphere is one value handed to every stage: the plain meets the massif along the rim, and
/// two bodies washing towards two slightly different colours would draw that seam as a bright
/// line across the foot of the mountain.
///
/// **The floor carries the river, and it is the brightest thing in the frame.** A valley read as
/// a corridor is a shape with nothing in it, and a corridor with a silver thread running away
/// down it is a place — the eye follows the brightest thing in the picture to wherever it goes,
/// and a thread that recedes is the cheapest depth there is. Everything else on the floor is
/// graded *down* from it.
///
/// **The floodplain is graded by distance from the water**, green where it is wet and gold where
/// it is dry, because that is the only thing that distinguishes a floodplain from a lawn, and it
/// is what makes the river's course legible from above: the green is the river's history.
const GROUND: &str = r#"
    let patches = 0.86 + 0.28 * noise(in.world_position.xz * 0.004, 4);
    let dry = smoothstep(0.0, SCENE_RADIUS * 1.6, in.world_position.y);
    var albedo = mix(vec3<f32>(0.26, 0.29, 0.20), vec3<f32>(0.40, 0.36, 0.24), dry);

    // The eye and the angle the ground is seen at, taken **once and before anything is decided by
    // them**: a floor this flat is nearly all one thing seen at every angle from grazing at the
    // far end of the corridor to steep underfoot, and every pattern cut into it has to know which
    // of the two it is standing on.
    let eye = uniforms.projection.position;
    let view = normalize(in.world_position - eye);
    let facing = 1.0 - abs(dot(normal, view));

    // **The river**, running **down the valley's own axis** and nowhere else. The axis is `x = 0`,
    // which is where the eye and the aim already are, so the thread of water runs out of the
    // bottom of the frame and into the haze along the one line everything else is read against.
    //
    // It wanders by two octaves rather than one, because a single octave wanders at one scale and
    // a river that meanders at exactly one scale reads as a sine wave. The second octave is a
    // third of the first, which is the ratio that puts a bend inside a bend.
    let run = in.world_position.z;
    let wander = (noise(vec2<f32>(run * 0.00085, 4.2), 3) - 0.5) * RIVER_MEANDR
               + (noise(vec2<f32>(run * 0.0031, 9.1), 2) - 0.5) * RIVER_MEANDR * 0.45;
    let offset = (in.world_position.x - wander) / SCENE_RADIUS;
    // The **wet band** the river wanders inside: wide enough that the channels have somewhere to
    // be, and hard-edged enough that the floodplain ends rather than fading into the grass.
    let band = 1.0 - smoothstep(RIVER_HALF * 0.45, RIVER_HALF, abs(offset));
    // The braiding itself. A braided river is **one noise deciding water against gravel**, sampled
    // anisotropically — sampled finely across the band and coarsely along it, which is the ratio
    // that makes the features **channels and not blotches**. The gravel between them is what makes
    // it braided rather than a single wandering line: that is the whole difference between a river
    // and a stream, and it costs one threshold.
    //
    // The frequencies are in **world units and not in screen space**, which fixes their size on the
    // ground and leaves them free to go wrong in one place: a lattice cell is a fixed length of
    // river, so under the eye it is a cell across a third of the frame and at the far end of the
    // corridor it is a cell a pixel wide. `near` is how much of the eye's own ground is close
    // enough for that to be a lattice rather than texture, and the braiding is **folded back into
    // its own mean** as the ground recedes — the one thing a distance-faded pattern can do that a
    // pattern sampled per pixel cannot, and the reason the far end of the river is a silver line
    // rather than a field of crawling squares.
    let near = 1.0 - smoothstep(0.74, 0.99, facing);
    // The lattice is **warped before it is read**, at a scale between the channel's and the
    // bend's. Read straight, a value noise is a grid of axis-aligned cells, and a river cut by
    // a grid of axis-aligned cells is a set of rectangles; pushed around first, the same cells
    // become ribbons that lie along the run and break across it, which is what the channels
    // look like. The warp is the cheap half and the anisotropy is the other half.
    let warp = noise(vec2<f32>(run * 0.0022, offset * 6.0), 2) - 0.5;
    let threads = mix(0.53, noise(vec2<f32>(run * 0.0060 + offset * 1.1 + warp * 0.9, offset * 74.0), 4), near);
    let channel = smoothstep(0.40, 0.55, threads) * band;
    let bar = clamp(band - channel, 0.0, 1.0);
    // The **floodplain** is graded by how far the water is from where it stands, so the green is
    // the river's own history written on the ground: widest where the river has wandered, which is
    // widest where the noise says the threads are many.
    let bank = abs(offset) / RIVER_HALF;
    let wet = 1.0 - smoothstep(0.5, 5.0, bank);
    albedo = mix(albedo, vec3<f32>(0.17, 0.28, 0.12), wet * 0.85);
    // The **terrace risers**. A floodplain is not one sheet sloping to the water: it steps down to
    // it in two or three benches, and the bench is the only thing on a flat floor that gives it
    // the scale a person could stand in. It is keyed to how far the water is from where the
    // ground stands, so the benches follow the river's own wander instead of running level
    // across the valley — and it is **jittered by the ground's own noise**, because a bench whose
    // edge is a clean line across a plain is a stripe drawn on a lawn and not a step in a field.
    let jitter = (noise(in.world_position.xz * 0.0022, 3) - 0.5) * 0.9;
    let riser = smoothstep(0.80, 1.12, bank + jitter) * (1.0 - smoothstep(1.90, 2.30, bank + jitter))
        + smoothstep(2.90, 3.30, bank + jitter) * (1.0 - smoothstep(4.60, 5.10, bank + jitter));
    albedo = mix(albedo, vec3<f32>(0.46, 0.42, 0.27), clamp(riser, 0.0, 1.0) * 0.30 * near);
    // The **scrub and the bare ground** the benches are cut out of, at a scale of a couple of
    // hundred units: a floodplain is not one colour, and a floor that is one colour is a floor
    // with nothing on it for the eye to measure the river against.
    let scrub = smoothstep(0.42, 0.78, noise(in.world_position.xz * 0.0016, 4));
    albedo = mix(albedo, vec3<f32>(0.13, 0.19, 0.11), scrub * 0.55);
    // The gravel **bars**, flat and grey, which is the one thing in a braided river that is not
    // either water or grass. They are kept **darker than they look** on a bank: the water around
    // them is what is bright, and a bar at the value of dry sand is the brightest thing in the
    // frame, which puts the eye on the gravel instead of on the river running through it.
    albedo = mix(albedo, vec3<f32>(0.34, 0.32, 0.28), bar * 0.9);

    color = color * albedo * patches;

    // The floor's share of the **same sky the walls are lit by**, at a third of the walls' rate.
    // The plain is flat and so faces the sky as squarely as a wall's shoulder does, and it is the
    // ground the sun is behind: without this the corridor's floor is the one surface in the
    // frame that is lit by nothing but a second light standing far away from it.
    let skyward = 0.35 + 0.65 * clamp(normal.y, 0.0, 1.0);
    color = color + air_colour() * skyward * 0.08;

    // The **water** is drawn after the ground's own colour because it is not a colour: it is what
    // the sky does to a flat mirror, and a flat mirror seen at a grazing angle is nearly all sky.
    // That is the fresnel term, and on a plain seen from an eye near its own level it does the
    // work on its own — the water brightens with distance on its own, without being told to.
    //
    // It is **dark underneath and bright on top**, and the two are not the same mix. A river is
    // the one surface in the frame that carries a colour of its own — silt, and cold deep water —
    // and a term that returns sky *alone* paints the channel the colour of the sky, which is how
    // a river turns into a strip of white paper laid down the middle of a valley. The body under
    // the reflection is what lets the glint on top of it be a glint, and it is most of the water
    // under the eye, where the grazing term has not yet taken over.
    let reflected = mix(vec3<f32>(0.14, 0.19, 0.25), vec3<f32>(0.56, 0.61, 0.69), pow(facing, 2.2));
    let water = vec3<f32>(0.02, 0.04, 0.05) + reflected * (0.34 + 0.46 * threads);
    color = mix(color, water, channel);

    // The valley is **full of cloud at its far end and clear at the eye's**, and the boundary is
    // a distance down the valley rather than a height. A sea of cloud over the whole floor is a
    // lake with a mountain round it; the same cloud pushed to where the valley narrows is what a
    // valley looks like from inside one, and it leaves the floor free to carry the river.
    let swell = noise(in.world_position.xz * 0.0011, 4);
    let billow = noise(in.world_position.xz * 0.0045, 4);
    let crest = noise(in.world_position.xz * 0.0160, 3);
    let top = SCENE_RADIUS * CLOUD_TOP * (0.40 + 1.05 * swell + 0.42 * billow);
    let sea = 1.0 - smoothstep(top - SCENE_RADIUS * 0.050, top, in.world_position.y);
    // The sunlit tops and the shaded troughs between them. Cloud is nearly a lambertian white,
    // so almost all of its form comes from which way each little billow is facing — and here that
    // is decided by the swell rather than by a normal the ground does not have.
    let shade = 0.52 + 0.36 * billow + 0.28 * crest;
    let cloud = vec3<f32>(0.95, 0.96, 0.99) * shade;
    let far = smoothstep(CLOUD_FROM * SCENE_RADIUS, CLOUD_FROM * SCENE_RADIUS * 2.4, run);
    color = mix(color, cloud, sea * CLOUD_COVER * far);

    let haze = air_closure(eye, in.world_position);
    color = mix(color, air_colour(), clamp(haze * (0.45 + 0.55 * facing), 0.0, 0.98));
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
        // It is banded in **elevation alone**. Cirrus is streaks across the sky and not blotches
        // on it, and an azimuth term is what a person reaches for and what ruins it: `atan2` of a
        // ray sweeps a quarter turn across the frame and most of that in the last few columns, so
        // a noise lattice sampled on it steps whole cells between neighbouring pixels and the sky
        // comes out as a grid of rectangles. Elevation runs monotonically and slowly from the
        // bottom of the frame to the top, so the same lattice stays smooth everywhere.
        let streak = noise(vec2<f32>(ray.y * 46.0, 0.5), 4);
        let wisp = noise(vec2<f32>(ray.y * 21.0, 7.5), 3);
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
