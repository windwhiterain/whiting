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
//! the belt it came from and how rugged its crest is. That mesh is what the scene **reads**, and
//! it reads two things from it.
//!
//! - **The authored shape**, which is the point: the ridge lines in the picture are the lines
//!   that were written, and moving one moves the mountain.
//! - **The identity**, face by face, so that a pixel still resolves to the crest it was
//!   authored as — see [`crate::identity`], and [`Erosion`] for how a lattice keeps it.
//!
//! # What it draws, and why it is not the mesh the crate built
//!
//! The scene draws the massif **eroded**: read onto a lattice, solved, and drawn on the
//! lattice's own vertices ([`Erosion`]). That is a change of what this scene is a demonstration
//! of, and it is worth being plain about: the block-face reading — a flank as a set of planar
//! facets meeting at sharp edges, normals written per corner — **is gone from the picture**. A
//! height field shaded by its own gradient is a smooth surface, which is what an eroded mountain
//! is, and the two cannot both be shown by the same geometry. The facet mesh still exists and is
//! still what the lattice is read from; what changed is that it is no longer what is drawn.
//!
//! # The plain
//!
//! The massif's mesh ends at its footprint, so on its own it is a slab of rock floating over
//! nothing. It therefore stands on a plain: a heightfield whose height is **zero everywhere the
//! massif reaches**, which is why the rim is levelled before the mesh is built. The two surfaces
//! meet along the rim rather than one hovering over the other, and the seam between them is the
//! massif's own boundary rather than a crack between two pieces of arithmetic — which is also
//! why the solve's output is faded back to the massif's own at the rim.

use definition::{PackedList, Schema, Value};

mod eroded;
mod massif;
pub use eroded::{CELL as EROSION_CELL, Erosion};
use mountain::line::Side;
use mountain::tessellation::BeltRef;
use mountain::{FacetMesh, Massif};
use scene::standard::Stage;
use terrain::HeightField;

use document::air::Atmosphere;
use document::geometry::{fractal_noise, heightfield, smoothstep};
use document::identity::{Identity, massif_extras, outputs_field, plain_extras};
use document::kit::{
    camera_placed_up, directional, environment_light, image, instance, instances, mesh,
    mesh_material, post_processes, screen_material, shaded_point_schema, surface,
};
use document::shaders::{
    Block, Extras, Projection, lighting_shader_declaring, mesh_shaders_over, post_shader,
};

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
        document::kit::gallery_custom_schema(),
        extras,
        document::shaders::Projection::Latlong,
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
///
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

/// The ground's height where it meets the massif, which is the rim's height.
///
/// **A massif is a range on a plateau, not a tower on a plain.** This is the elevation the land
/// already stands at before anything rises from it, and it is the largest single number in the
/// picture: the summit is authored at 2400 and this is where the country around it sits, so the massif
/// is the difference between the two — a range of a few hundred metres carried on high ground, which
/// is what a real mountain group is. Set at zero, the same authoring is a 2400-metre spire standing on
/// nothing, and that is what every version of this scene before this one drew.
///
/// It also fixes the *proportion of relief to width*, because the camera and the aim are offsets from
/// it: raising it moves the eye up with the land and the range gets shorter in the frame without the
/// frame moving, which is the same statement as the paragraph above.
///
/// Nothing here is a physical unit; the massif's own measure is what everything is a multiple of.
const RIM: f64 = 1450.0;

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
/// How high the sun stands, in radians.
///
/// High, near fifty degrees: this scene's body is a plain grey diffuse and the plan is meant to
/// read as the reference's hillshade, where the light comes from above and the flanks differ by
/// tone rather than by being one bright face and one black one.
const SUN_ELEVATION: f64 = 0.85;

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

/// Which camera the scene compiles.
///
/// # Why there are two
///
/// The across-the-spine view is the picture. It is also **useless for judging the ridge network**: seen
/// from the ground, three ridges one behind another are one ridge, and a range that has been grown
/// badly looks exactly like a range that has been grown well until you walk around it. The question
/// "does this branch and rejoin the way a real massif does" is a question about the **plan**, and the
/// only camera that answers it is one looking down.
///
/// So there is a second, steep view for looking at the shape rather than at the picture, chosen by
/// `LANDSCAPE_VIEW=overhead`. It is a debug camera and it is not the scene: the sun, the aim and the
/// haze all follow the eye they are given, so what the overhead view shows is the same geometry under
/// the same light and not a second lighting setup that has to be kept in step.
fn view() -> View {
    match std::env::var("LANDSCAPE_VIEW").as_deref() {
        Ok("overhead") => View::Overhead,
        _ => View::Across,
    }
}

/// The two cameras, as the eye offsets that differ between them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    /// Along the ground, across the spine: the picture.
    Across,
    /// Almost straight down: the plan, for judging the network.
    Overhead,
}

/// Returns where the eye stands, so a caller can measure the picture from it.
///
/// **It is the one eye [`landscape`] compiles**, read back rather than written twice. A probe that
/// placed its own eye to answer "how wide does this range stand in the frame" would be measuring a
/// camera nobody renders.
pub fn eye_at(radius: f64) -> [f64; 3] {
    match view() {
        View::Across => [
            EYE.azimuth.cos() * EYE.back * radius,
            RIM + radius * EYE.height,
            EYE.azimuth.sin() * EYE.back * radius,
        ],
        // Steep but not vertical: a camera looking exactly down its own up axis has no orientation,
        // and the massif's own long axis is what the frame should be square to.
        View::Overhead => [
            EYE.azimuth.cos() * OVERHEAD_BACK * radius,
            RIM + radius * OVERHEAD_HEIGHT,
            EYE.azimuth.sin() * OVERHEAD_BACK * radius,
        ],
    }
}

/// How far back an overhead eye stands, as a fraction of the massif's radius.
///
/// Nearly none: the plan this view exists for is the reference's own — straight down, the ridge
/// network read as a pattern — and any more back than this turns the pattern into a skyline.
const OVERHEAD_BACK: f64 = 0.12;

/// How high an overhead eye stands, as a fraction of the massif's radius.
const OVERHEAD_HEIGHT: f64 = 2.60;

/// Returns the direction the frame's top edge runs along, in the world.
///
/// # Invariants
///
/// The ground view's up is world-up: a horizon is level and that is the whole decision. The plan
/// looks nearly straight down, where world-up is the view direction and a look-at built on it has
/// no orientation at all — so the plan's up is a **horizontal** direction instead, the one from
/// the massif out towards where the eye stands, which keeps the plan's left and right the same
/// left and right the ground view shows.
fn frame_up() -> [f64; 3] {
    match view() {
        View::Across => [0.0, 1.0, 0.0],
        View::Overhead => [EYE.azimuth.cos(), 0.0, EYE.azimuth.sin()],
    }
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
    // Straight down at the subject's own centre. Carrying the aim along the spine is what gives the
    // ground view a direction to travel in; from above it would only push the network off the middle
    // of the frame, which is the one thing an overhead view is for.
    if view() == View::Overhead {
        return [
            range.centre[0] * radius,
            RIM + summit * AIM_HEIGHT,
            range.centre[1] * radius,
        ];
    }
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
    let mut solved: Vec<(&'static str, FacetMesh, Erosion, Identity)> =
        Vec::with_capacity(RANGES.len());
    for range in RANGES {
        let identity = Identity::of(&authored, next_id, |belt| belt_path(range.name, belt));
        next_id += identity.rows().len() as u32;
        // Eroded **after** it is stood at, and not before: erosion acts in world units, so a
        // solve run on the authored mesh and then scaled would carve the near range at the
        // far range's resolution.
        let stood = stood_at(
            &built,
            [range.centre[0] * radius, range.centre[1] * radius],
            range.scale,
            range.turn,
        );
        let erosion = Erosion::of(&stood);
        solved.push((range.name, stood, erosion, identity));
    }
    // Every range's lattice is drawn **after every range is solved**, because a lattice drops
    // the cells that stand under another range's rock — see [`Erosion::lattice`] — and that ask
    // needs the other range's surface, which its own solve is.
    // Cloned rather than borrowed, so the solves can be consumed by the draw loop below: a
    // surface is a few thousand floats, against a lattice draw that reads it per cell.
    let belows: Vec<Vec<HeightField>> = (0..solved.len())
        .map(|index| {
            solved
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, (_, _, erosion, _))| erosion.surface.clone())
                .collect()
        })
        .collect();
    let mut ranges_built: Vec<(&'static str, PackedList, Identity)> =
        Vec::with_capacity(solved.len());
    for ((name, stood, erosion, identity), below) in solved.into_iter().zip(belows) {
        let below: Vec<&HeightField> = below.iter().collect();
        // Drawn on the **lattice's own vertices**, not on the authored mesh's. The erosion
        // is a field of a few thousand cells and the massif it fell on has a hundred and
        // seventy four points, so displacing those throws the whole solve away before the
        // frame sees it — see [`Erosion::lattice`].
        ranges_built.push((name, erosion.lattice(&stood, &identity, &below), identity));
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

    let helpers = format!("{}\n{}\n{}", atmosphere.wgsl(), NOISE, constants(sun),);
    // The far ranges are the post stage's alone, so they are handed to the post stage alone. They
    // are a backdrop the air has already closed over, and a surface shader has no use for a line
    // painted into the sky.
    let post_helpers = format!("{helpers}\n{}", crate::ridges::RIDGES);

    let mut shaders: Vec<Value> = Vec::new();
    // Both meshes are drawn with **no surface body at all**. What a massif looks like is a
    // question for a later stage and a baked texture, not for a per-fragment shader here: the
    // geometry is what this pipeline is for, and a surface the path tracer cannot reproduce is a
    // surface only one backend draws. The colour each mesh wears is its material's own tint.
    shaders.extend(stage_set(
        "massif",
        "",
        &helpers,
        &massif_extras().point_schema(),
        &massif_extras(),
    ));
    shaders.extend(stage_set(
        "plain",
        "",
        &helpers,
        &shaded_point_schema(),
        &plain_extras(),
    ));
    shaders.push(post_shader(GRADE, &post_helpers, Some(Projection::Latlong)));

    Value::structure([
        ("name".to_owned(), Value::String("landscape".to_owned())),
        // The three images this scene's **own** shaders write, beside the frame. The massif reads
        // them from its points; the plain writes the reserved zero, because a plain has no authored
        // element of its own. Both have to write them: a stage that drew geometry and did not would
        // leave the image holding whatever an earlier fragment put there.
        (OUTPUTS.to_owned(), outputs_field()),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed_up(
                eye,
                target,
                frame_up(),
                FIELD_OF_VIEW,
                (radius * 0.05).max(0.5),
                plain * 1.2,
            )]),
        ),
        (
            // The sky as an **image**, which is what makes this scene render on a backend that has
            // no shader stage of its own. Latlong, because that is the one a path tracer reads: an
            // octahedral map handed to one is read as latlong and comes back as a plausible frame
            // with half the sky black and a hard vertical seam.
            //
            // Two to one, because `u` goes once round the world and `v` once over it; a square
            // latlong map is a distorted sky.
            "images".to_owned(),
            Value::List(vec![image(
                "air",
                256,
                128,
                atmosphere.environment_pixels(256, 128),
                document::shaders::Projection::Latlong,
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
                // The ambient is an **environment light** reading this scene's own sky, and not a
                // lamp standing in for it. One point light with one falloff leaves every shadowed
                // facet black, which is why there was a fill here at all — but a fill strong enough
                // to see lifts the dark facets to the brightness of the lit ones and erases the very
                // difference the raking sun was placed to make. It is not a tuning problem: a point
                // light cannot be a dome, because its brightness is the square of its distance and
                // this scene's subject is two thousand metres across.
                //
                // The map is [`Atmosphere::environment_pixels`] — the same sky the post stage draws,
                // term for term. An ambient from a different afternoon than the sky above it is a
                // frame where a shadowed facet is one colour and the sky is another, and nothing in
                // the picture says which of the two the author meant.
                environment_light("air", "air", [1.0; 3], 0.55),
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
                    .map(|(name, points, _)| {
                        mesh(
                            &format!("range_{name}"),
                            "Triangles",
                            // The point schema is **the one the outputs' fields make**, so the vertex
                            // layout and the shader's parameter list are derived from one schema
                            // rather than two that have to agree.
                            massif_extras().point_schema(),
                            points.clone(),
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
                // The stone. The colour of the rock is **the material's own data**, because
                // there is no longer a body deciding it per fragment: one surface, one decision.
                // A plain grey, so the two backends read one albedo rather than each deriving
                // its own stone out of a shader only the rasteriser is asked to run.
                mesh_material(
                    "shade",
                    "massif_lighting",
                    surface([0.58, 0.58, 0.58], 0.0, 0.90, [0.0; 3]),
                ),
                mesh_material(
                    "ground_depth",
                    "plain_depthing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                mesh_material(
                    "ground_shade",
                    "plain_lighting",
                    surface([0.34, 0.32, 0.29], 0.0, 0.95, [0.0; 3]),
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
///
/// Only what something still reads. `SNOW_LINE`, `SNOW_WOBBLE`, `SNOW_FADE` and `SCENE_RADIUS`
/// went with the massif's surface body; `CLOUD_TOP`, `CLOUD_COVER`, `CLOUD_FROM`, `RIVER_HALF`
/// and `RIVER_MEANDR` were already unread, the last two left over from the river this plain used
/// to have. A constant emitted for nobody is a number in the document that looks as load-bearing
/// as one that is.
fn constants(sun: [f64; 3]) -> String {
    format!(
        "const RIDGE_SCALE: f32 = {RIDGE_SCALE:.4};\n\
         const RIDGE_LAYERS: i32 = {RIDGE_LAYERS};\n\
         const CIRRUS_COVER: f32 = {CIRRUS_COVER:.4};\n\
         const SUN_AHEAD: vec3<f32> = vec3<f32>({:.4}, {:.4}, {:.4});\n",
        sun[0], sun[1], sun[2],
    )
}

/// How much of the high sky the cirrus is allowed to take.
const CIRRUS_COVER: f64 = 0.55;

/// The noise the post stage draws its weather from.
///
/// # Invariants
///
/// The only body left that reads this is the grade, and it uses it for the valley cloud and the
/// cirrus. Those are sampled in **world space**, so a bank of cloud stays put on the landscape
/// when the eye moves; sampled in screen space it would slide across the picture, which reads as a
/// smear on the lens rather than as weather.
///
/// The field is **gradient noise faded with the quintic**, and that is not a preference between
/// two noises that look alike. Value noise interpolates corner *values* bilinearly: it is
/// continuous across a cell boundary and nothing else is, so its slope jumps there, and the cirrus
/// window — which thresholds the field — turns that jump into a hard edge along the lattice. The
/// sky's own projection then stretches those lattice cells until each one spans a hundred pixels,
/// and the picture is a grid of axis-aligned plates. Gradient noise is twice differentiable across
/// the boundary, so the same window draws a cloud edge instead of a seam.
const NOISE: &str = r#"
/// Returns a scrambled integer.
///
/// # Invariants
///
/// **It is integer arithmetic, not `fract(sin(x) * k)`.** A sine hash needs its argument small
/// enough that an `f32` still carries its fractional part. A cloud deck carries coordinates of
/// tens, and a cell coordinate is that deck multiplied by a frequency and a per-octave turn, so the
/// dot products reach tens of thousands; past about `1e4` the argument's leading bits are all an
/// `f32` keeps and the hash becomes a function of them, which is a set of plates rather than a
/// hash. Multiplying an integer is exact at every magnitude the sky reaches.
fn scramble(value: u32) -> u32 {
    var word = value * 747796405u + 2891336453u;
    word = ((word >> ((word >> 28u) + 4u)) ^ word) * 277803737u;
    return (word >> 22u) ^ word;
}

/// Returns the seed of the lattice cell a coordinate falls in.
///
/// # Invariants
///
/// **Both coordinates are folded into one word before it is scrambled.** A cell is named by its two
/// halves together, and scrambling the halves separately lets one cancel the other, which is a
/// whole family of cells sharing a seed and therefore a family of corners steering the same way.
fn cell_seed(cell: vec2<i32>) -> u32 {
    return scramble(bitcast<u32>(cell.x) * 1597334677u ^ bitcast<u32>(cell.y) * 2654435761u);
}

/// Returns the direction one lattice corner steers its cell's noise towards.
///
/// # Invariants
///
/// **Every corner steers somewhere.** The draw is from the whole circle rather than from a small
/// fixed set of axes, because a set with a preference for the axes puts that preference into the
/// field, and an axis-aligned preference is the one thing this sky must not have. The nudge keeps
/// the single seed whose bits are all zero from returning the zero vector, which would leave a flat
/// plate exactly where the corner stood.
fn corner_gradient(seed: u32) -> vec2<f32> {
    let draw = vec2<f32>(f32(seed & 65535u), f32((seed >> 16u) & 65535u)) * (2.0 / 65535.0) - 1.0;
    return normalize(draw + vec2<f32>(1.0e-4, 1.0e-4));
}

/// Returns gradient noise, in roughly minus one to one.
///
/// # Invariants
///
/// **The fade is the quintic.** `6t^5 - 15t^4 + 10t^3` arrives at a cell boundary with its first
/// *and* its second derivative at zero, so nothing about the seam between two cells is left to
/// see. The cubic the value noise used leaves the slope itself jumping there instead.
fn gradient_noise(point: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor(point));
    let within = point - floor(point);
    let fade = within * within * within * (within * (within * 6.0 - 15.0) + 10.0);
    let east = vec2<f32>(1.0, 0.0);
    let north = vec2<f32>(0.0, 1.0);
    let low_west = dot(corner_gradient(cell_seed(cell)), within);
    let low_east = dot(corner_gradient(cell_seed(cell + vec2<i32>(1, 0))), within - east);
    let high_west = dot(corner_gradient(cell_seed(cell + vec2<i32>(0, 1))), within - north);
    let high_east = dot(corner_gradient(cell_seed(cell + vec2<i32>(1, 1))), within - east - north);
    return mix(
        mix(low_west, low_east, fade.x),
        mix(high_west, high_east, fade.x),
        fade.y
    ) * 1.4142;
}

/// The turn each octave takes before the next one raises its frequency.
///
/// # Invariants
///
/// **It is a turn and not a scale.** Octaves at whole-number frequencies share their cell
/// boundaries: an octave at 1 and the next at 2.1 put every seam of the finer one very near a seam
/// of the coarser one, and two near-coincident lattices reinforce each other into the grid the
/// gradient noise was taken to remove.
const OCTAVE_TURN: mat2x2<f32> = mat2x2<f32>(0.766, -0.643, 0.643, 0.766);

/// Returns noise summed over `octaves`, in roughly zero to one.
///
/// # Invariants
///
/// **The sum is centred and scaled here**, because the grade thresholds this field and the
/// thresholds are numbers the picture is read against: an octave sum that is only accidentally in
/// zero to one leaves every window written over it meaning whatever the octaves happened to do.
/// The turn between octaves is part of the sum for the same reason — it is what keeps the finer
/// octaves off the coarser ones' lattice, and a field with a visible lattice is what this whole
/// body exists to stop drawing.
fn noise(point: vec2<f32>, octaves: i32) -> f32 {
    var total = 0.0;
    var amplitude = 0.5;
    var sample = point;
    for (var octave = 0; octave < octaves; octave = octave + 1) {
        total = total + amplitude * gradient_noise(sample);
        sample = OCTAVE_TURN * sample * 2.1;
        amplitude = amplitude * 0.5;
    }
    return total * 1.14 + 0.5;
}
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
        color = ridges(environment_sky(ray), ray, RIDGE_SCALE, RIDGE_LAYERS);

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
