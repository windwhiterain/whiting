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
    End, FootprintNode, LineRef, Massif, Ridge, RidgeId, RidgeNode, Skeleton, Tessellation, Valley,
    ValleyNode,
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
/// **Long, and that is the choice that reads as distance.** A wide lens near a mountain makes it
/// lean away from the eye and shows more ground than sky; a long lens from further back flattens
/// the air between the two and stacks the massif against its own far ridges. Forty-five degrees
/// is the point where the massif still fills the frame from far enough back that the plain it
/// stands on is visible all the way to the horizon.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_4;

/// How high off the plain the eye stands, as a multiple of the massif's radius.
///
/// Low. The eye is in the valley the massif stands in and not on a hill opposite it, because a
/// camera raised to half the subject's own height looks **down** on it, and a mountain seen from
/// above is a shape on a table. Every fraction of a radius above the ground the eye goes costs
/// the sense of scale the picture is trying to have.
const EYE_HEIGHT: f64 = 0.14;

/// How far the aim is pushed off the massif's own centre, sideways, as a multiple of its radius.
///
/// It is what puts the subject off the middle of the frame. A third of the way out is enough to
/// break the symmetry without putting the summit at the edge, and the space it leaves is where
/// the far ranges and the air go.
const AIM_OFFSET: f64 = 0.22;

/// How far the aim is raised above the massif's own centre, as a multiple of its radius.
///
/// Above the centre, not below it: the eye is near the ground, so an aim at the middle of the
/// rock points the frame's centre into the massif's waist and leaves the summit in the top tenth.
const AIM_HEIGHT: f64 = 0.16;

/// How far round from the eye's own bearing the sun stands, in radians.
///
/// # Invariants
///
/// It is a little over a quarter turn. A sun **behind the eye** lights every face of a flank
/// within a few percent of every other, so the massif shades as the smooth cone its arithmetic
/// would have produced and the facets the crate exists to make are invisible. A sun **behind the
/// massif** draws a silhouette with the flanks on the fill light alone. A little over a quarter
/// turn is the angle in between, where one flank is lit, the next is not, and the boundary between
/// them is a crest.
const SUN_OFFSET: f64 = 1.62;

/// How high the sun stands, in radians.
///
/// **Low**, and low is the number that makes the facets legible rather than the one that makes
/// the picture bright. A high sun is overhead, and an overhead sun lights the tops of things and
/// leaves their sides at the fill's mercy — which flattens a massif into a plan view of itself.
/// Near the horizon the light rakes along the flanks, so a facet turned a few degrees from it is
/// markedly brighter than its neighbour, and the boundary between two facets is a line of light
/// rather than a line of paint.
const SUN_ELEVATION: f64 = 0.26;

/// Builds the landscape.
pub fn landscape() -> Value {
    let massif = massif();
    let raised = massif.build().expect("the authored massif expands");
    // One walk numbers every belt and records the row that names it. It happens before the mesh
    // is read so that the number a fragment writes and the row a reader looks up come from the
    // same pass — see `identity`.
    let identity = Identity::of(&massif, belt_path);
    let (low, high) = raised.bounds();
    let radius = (((high[0] - low[0]).powi(2) + (high[2] - low[2]).powi(2)) as f64).sqrt() * 0.5;
    let summit = high[1] as f64;

    let centre = [
        ((low[0] + high[0]) * 0.5) as f64,
        ((low[1] + high[1]) * 0.5) as f64,
        ((low[2] + high[2]) * 0.5) as f64,
    ];
    // The eye stands back and **low**, near the plain, and looks up at the massif. It is off to
    // one side by more than a quarter turn, so the crests are seen obliquely: looked at down their
    // own axis a fan of ridges reads as two faces and a fold, which is a tent.
    //
    // Every distance in it is a multiple of the massif's own radius, so a skeleton edited to a
    // different size is framed the same way and the haze below stays the same fraction of the
    // subject.
    let eye = [
        centre[0] + radius * 1.78,
        RIM + radius * EYE_HEIGHT,
        centre[2] + radius * 0.68,
    ];
    // The aim is pushed **sideways** rather than only up, which is what puts the massif off the
    // middle of the frame. Centred, a mountain of this width is symmetrical about the vertical and
    // reads as a specimen; a third off, the same mountain reads as something the eye is standing
    // in front of, and the space left on one side is where the air and the far ranges have to go.
    let camera_bearing = (eye[2] - centre[2]).atan2(eye[0] - centre[0]);
    let sideways = [-camera_bearing.sin(), camera_bearing.cos()];
    let target = [
        centre[0] + sideways[0] * radius * AIM_OFFSET,
        centre[1] + radius * AIM_HEIGHT,
        centre[2] + sideways[1] * radius * AIM_OFFSET,
    ];
    let distance = ((eye[0] - centre[0]).powi(2)
        + (eye[1] - centre[1]).powi(2)
        + (eye[2] - centre[2]).powi(2))
    .sqrt();

    let plain = radius * PLAIN_REACH;
    // Where the sun stands, as a bearing **off the camera's own**: the light is derived from the
    // eye rather than chosen beside it, so the two cannot be edited apart. A sun picked
    // independently reads well for exactly one camera position and is behind the mountain for the
    // next, and the failure looks like a scene that has gone dark for no reason.
    //
    // The offset is a little over a quarter turn, which is the angle that rakes across the flanks:
    // a sun behind the eye flattens the facets, and one behind the massif draws a silhouette.
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
        constants(radius, summit),
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
                    [1.0, 0.92, 0.80],
                    2.1,
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
                // The reach holds the whole scene, which promotes it to a global light: it takes
                // no cluster slot and no fragment can be culled out of its reach. The number is
                // therefore not a distance it lights to but a statement that it lights
                // everything.
                light_with_radius(
                    "sky",
                    [radius * 0.6, radius * 1.5, -radius * 0.7],
                    [0.34, 0.44, 0.70],
                    2.4,
                    radius * 1.6,
                    plain * 2.0,
                ),
            ]),
        ),
        ("shaders".to_owned(), Value::List(shaders)),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh(
                    "massif",
                    "Triangles",
                    // The point schema is **the one the outputs' fields make**, so the vertex
                    // layout and the shader's parameter list are derived from one schema rather
                    // than two that have to agree.
                    massif_extras().point_schema(),
                    identity::faceted(&raised, &identity),
                ),
                mesh(
                    "plain",
                    "Triangles",
                    shaded_point_schema(),
                    heightfield(PLAIN_DIVISIONS, plain, |x, z| plain_height(x, z, radius)),
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
        (
            "instances".to_owned(),
            // One placement per pass. The massif casts a shadow and the plain does not: a flat
            // sheet cannot occlude itself, and putting it in the shadow map would spend the map's
            // extent on ground that has nothing to say.
            instances(vec![
                (
                    Stage::Shadowing,
                    vec![instance(
                        "massif_cast",
                        "massif",
                        [0.0, 0.0, 0.0],
                        [0.0; 3],
                        [1.0; 3],
                        "cast",
                    )],
                ),
                (
                    Stage::Depthing,
                    vec![
                        instance(
                            "massif_depth",
                            "massif",
                            [0.0, 0.0, 0.0],
                            [0.0; 3],
                            [1.0; 3],
                            "depth",
                        ),
                        instance(
                            "plain_depth",
                            "plain",
                            [0.0, 0.0, 0.0],
                            [0.0; 3],
                            [1.0; 3],
                            "ground_depth",
                        ),
                    ],
                ),
                (
                    Stage::Lighting,
                    vec![
                        instance(
                            "massif_shade",
                            "massif",
                            [0.0, 0.0, 0.0],
                            [0.0; 3],
                            [1.0; 3],
                            "shade",
                        ),
                        instance(
                            "plain_shade",
                            "plain",
                            [0.0, 0.0, 0.0],
                            [0.0; 3],
                            [1.0; 3],
                            "ground_shade",
                        ),
                    ],
                ),
            ]),
        ),
    ])
}

/// One hand-placed crest.
///
/// # Invariants
///
/// Every field is a number the author chose, and none of them is derivable from another. That is
/// the point of the table: the fields are deliberately **uncorrelated**, so the crests do not
/// sort into a family of similar ridges the way a generator's parameters do.
/// [`Crest::reach`] and [`Crest::tip`] in particular are independent, which is what puts a short
/// crest high and a long one low — a shoulder beside the summit rather than another skirt of it.
///
/// - **`profile` shapes the fall.** Elevation at a fraction `f` is
///   `tip + (summit - tip) * (1 - f)^profile`, so a profile above one gives most of the height
///   away in the first third and runs out nearly level, which is a shoulder; below one and the
///   crest carries its height out and drops at the end, which is a cliff above the foot. Both
///   ends are exact whatever the exponent, so a crest still leaves the summit at the summit's
///   height and still reaches the rim at the rim's.
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
    /// Where it meets the rim, and how high it is there.
    tip: f32,
    /// The elevations it is at a quarter, a half and three quarters along.
    ///
    /// # Invariants
    ///
    /// **Written out rather than shaped by an exponent**, and that is the difference between a
    /// mountain and a tent. One smooth curve from summit to foot is a ruled surface: a crest and
    /// a valley that both descend evenly are nearly parallel, the surface between them is nearly
    /// planar, and ten nearly planar surfaces meeting at one point are folded paper however good
    /// the silhouette is. Three heights are enough to put a **bench** — two of them close
    /// together, so the crest runs out nearly level for a stretch — or a **headwall**, where the
    /// crest holds its height and then goes down all at once. Both are the shapes a real crest is
    /// made of, and both break a flank into faces that are not one plane.
    ///
    /// The three are **strictly decreasing**, so the line stays non-increasing whatever is
    /// interpolated between them. The fourth height is [`Crest::tip`], so a crest still arrives at
    /// the rim at exactly the rim's elevation.
    heights: [f32; 3],
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

    /// Returns the elevation this crest has at a fraction of its run.
    ///
    /// # Invariants
    ///
    /// **Piecewise linear between the authored heights, and non-increasing throughout.** The
    /// knots are the summit followed by [`Crest::heights`], and the last of them is the tip, so
    /// the line still leaves at the summit's height and still arrives at the rim's — an exponent
    /// cannot promise both and a table of heights can.
    fn elevation(&self, fraction: f32) -> f32 {
        let knots = [
            SUMMIT_ELEVATION,
            self.heights[0],
            self.heights[1],
            self.heights[2],
            self.tip,
        ];
        let position = fraction.clamp(0.0, 1.0) * (knots.len() - 1) as f32;
        let low = (position.floor() as usize).min(knots.len() - 2);
        let amount = position - low as f32;
        knots[low] + (knots[low + 1] - knots[low]) * amount
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
/// **The reaches run from 295 to 820 and the tips from 7 to 64**, and the two are placed
/// independently of each other. The reach sets where the foot is, and the tip sets how much of
/// the mountain is still standing there, so a short crest with a high tip is a horn over a
/// shoulder and a long crest with a low tip is a tongue running out to the plain. Those two
/// readings are what a fan cannot produce, because in a fan they are the same number.
///
/// **The three horns — 2, 5 and 6 — are the crests that make the skyline.** Each is short and
/// holds its height well past its middle, so each stands beside the summit as a crest of its own
/// with a saddle between, and the silhouette stops being a triangle. They are the reason the
/// picture reads as a mountain rather than as a cone with facets on it.
const CRESTS: [Crest; 10] = [
    Crest {
        bearing: 0.0,
        reach: 820.0,
        sharpness: 0.40,
        half_width: 0.030,
        bend: 0.075,
        tip: 18.0,
        heights: [655.0, 400.0, 300.0],
    },
    Crest {
        bearing: 42.0,
        reach: 520.0,
        sharpness: 0.52,
        half_width: 0.038,
        bend: -0.065,
        tip: 11.0,
        heights: [600.0, 330.0, 190.0],
    },
    Crest {
        bearing: 73.0,
        reach: 330.0,
        sharpness: 0.94,
        half_width: 0.022,
        bend: 0.045,
        tip: 58.0,
        heights: [700.0, 620.0, 300.0],
    },
    Crest {
        bearing: 111.0,
        reach: 530.0,
        sharpness: 0.66,
        half_width: 0.032,
        bend: -0.085,
        tip: 16.0,
        heights: [500.0, 470.0, 180.0],
    },
    Crest {
        bearing: 140.0,
        reach: 340.0,
        sharpness: 0.44,
        half_width: 0.040,
        bend: 0.070,
        tip: 7.0,
        heights: [450.0, 430.0, 110.0],
    },
    Crest {
        bearing: 184.0,
        reach: 470.0,
        sharpness: 0.84,
        half_width: 0.026,
        bend: -0.050,
        tip: 43.0,
        heights: [640.0, 560.0, 240.0],
    },
    Crest {
        bearing: 217.0,
        reach: 295.0,
        sharpness: 0.90,
        half_width: 0.024,
        bend: 0.045,
        tip: 64.0,
        heights: [660.0, 430.0, 310.0],
    },
    Crest {
        bearing: 253.0,
        reach: 460.0,
        sharpness: 0.60,
        half_width: 0.034,
        bend: -0.075,
        tip: 13.0,
        heights: [520.0, 480.0, 160.0],
    },
    Crest {
        bearing: 294.0,
        reach: 410.0,
        sharpness: 0.40,
        half_width: 0.042,
        bend: 0.080,
        tip: 7.0,
        heights: [420.0, 400.0, 95.0],
    },
    Crest {
        bearing: 328.0,
        reach: 640.0,
        sharpness: 0.74,
        half_width: 0.028,
        bend: -0.060,
        tip: 27.0,
        heights: [600.0, 540.0, 240.0],
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
const SUMMIT_ELEVATION: f32 = 720.0;

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
    let wedges: Vec<Wedge> = (0..CRESTS.len())
        .map(|index| wedge(index, (index + 1) % CRESTS.len()))
        .collect();
    let outline = footprint(&wedges);
    let mut massif = Massif::new(Skeleton::new(ridges(), bound(wedges, &outline), outline))
        .with_tessellation(Tessellation::default().with_step(STEP));

    stand_on_the_plain(&mut massif);

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

/// Returns the crest lines, the first one the root and the rest its children.
fn ridges() -> Vec<Ridge> {
    CRESTS
        .iter()
        .enumerate()
        .map(|(index, crest)| {
            let nodes: Vec<RidgeNode> = (0..CREST_NODES)
                .map(|step| {
                    let fraction = step as f32 / (CREST_NODES - 1) as f32;
                    RidgeNode::new(crest.position(fraction), crest.elevation(fraction))
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

/// How many nodes a crest is written with.
///
/// It is a node count and not a distance because a crest that wanders needs the nodes to wander
/// with: the wander is written into the node positions, so a crest sampled coarsely from a
/// straight bearing would lose the very thing that makes it not a radius.
const CREST_NODES: usize = 17;

/// One wedge's plan, before its mouth can be bound to the rim.
struct Wedge {
    /// Where the mouth stands on the rim.
    mouth: [f32; 2],
    /// The elevation at the mouth.
    mouth_elevation: f32,
    /// The nodes, each with the two crest stations it faces.
    nodes: Vec<([f32; 2], f32, Station, Station)>,
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

    let mut nodes = Vec::with_capacity(WEDGE_STEPS + 2);
    nodes.push((
        SUMMIT,
        SUMMIT_ELEVATION,
        Station::ridge(RidgeId(left), 0.0),
        Station::ridge(RidgeId(right), 0.0),
    ));
    for step in 1..=WEDGE_STEPS {
        let fraction = step as f32 / WEDGE_STEPS as f32;
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
            valley_elevation(fraction, left, right),
            Station::ridge(RidgeId(left), fraction),
            Station::ridge(RidgeId(right), fraction),
        ));
    }
    Wedge {
        mouth: [
            SUMMIT[0] + direction[0] * mouth,
            SUMMIT[1] + direction[1] * mouth,
        ],
        mouth_elevation: valley_elevation(1.0, left, right),
        nodes,
    }
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

/// Returns the elevation the valley between two crests has at a fraction of its run.
///
/// # Invariants
///
/// **It is a fraction of the lower of the two crests it runs between**, so the valley floor is
/// below the rock on both sides of it at every station — which is what makes it a valley and not
/// a line drawn across a flank.
///
/// **The fraction holds high and then cuts in, and that is the whole of the massif's shape.** It
/// is near its maximum through the first fifth of the run, so the summit and everything under it
/// is one connected mass with shallow ribs in it, and it only closes hard over the second half,
/// where the real valleys are. A fraction that fell evenly instead — high at the summit, low at
/// the rim, cutting all the way — would put a deep notch under the summit on every one of the
/// massif's ten sides, and ten notches under one apex is a bundle of blades rather than a
/// mountain.
///
/// Both the factor and the two crest profiles are non-increasing, so the product is too: an edit
/// here cannot make the massif invalid.
fn valley_elevation(fraction: f32, left: usize, right: usize) -> f32 {
    let floor = CRESTS[left]
        .elevation(fraction)
        .min(CRESTS[right].elevation(fraction));
    floor * (VALLEY_HOLD - VALLEY_CUT * smoothstep(0.18, 0.90, fraction as f64) as f32)
}

/// How close under its crests a valley runs at the summit, as a fraction of them.
const VALLEY_HOLD: f32 = 0.85;

/// How much of that closeness the valley gives back over the lower part of its run.
const VALLEY_CUT: f32 = 0.80;

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
                .map(|(position, elevation, left, right)| {
                    ValleyNode::new(position, elevation, left, right)
                })
                .chain(std::iter::once(ValleyNode::new(
                    wedge.mouth,
                    wedge.mouth_elevation,
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
fn footprint(wedges: &[Wedge]) -> Vec<FootprintNode> {
    wedges
        .iter()
        .enumerate()
        .flat_map(|(index, wedge)| {
            [
                FootprintNode::new(CRESTS[index].position(1.0), CRESTS[index].tip),
                FootprintNode::new(wedge.mouth, wedge.mouth_elevation),
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
/// - **The path is a function of the `BeltRef` alone**, so the same belt is named the same way
///   in every build of the same skeleton, and an id means the same thing on either side of an
///   edit. A path built from anything that varies between builds would be a path that answers a
///   different question each time it is asked.
///
/// The hierarchy is this scene's, and it is deliberately shallow until there is a second
/// massif to nest: a massif, the valley that carries the flank, and which side of it. Growing it
/// is a matter of changing this one function, which is the property that keeps the id space
/// separable from the geometry.
pub fn belt_path(belt: BeltRef) -> String {
    format!(
        "landscape/valley_{}/{}",
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
    let away = smoothstep(radius * 1.05, radius * 2.2, distance);
    let roll = fractal_noise([x * 0.0016, z * 0.0018], 4, 41) - 0.5;
    let near = (fractal_noise([x * 0.006, z * 0.0064], 3, 17) - 0.5) * radius * 0.02;
    let horizon = smoothstep(radius * 2.2, radius * PLAIN_REACH, distance);
    RIM + beyond * (near + away * (roll * radius * 0.15 + horizon * radius * 0.22))
}

/// The constants the shader bodies are written against.
fn constants(radius: f64, summit: f64) -> String {
    format!(
        "const SNOW_LINE: f32 = {:.3};\n\
         const SNOW_WOBBLE: f32 = {:.3};\n\
         const SNOW_FADE: f32 = {:.3};\n\
         const SCENE_RADIUS: f32 = {radius:.3};\n\
         const CLOUD_TOP: f32 = {CLOUD_TOP:.4};\n\
         const CLOUD_COVER: f32 = {CLOUD_COVER:.4};\n\
         const RIDGE_SCALE: f32 = {RIDGE_SCALE:.4};\n\
         const CIRRUS_COVER: f32 = {CIRRUS_COVER:.4};\n",
        summit * SNOW_LINE,
        summit * SNOW_WOBBLE,
        (summit * SNOW_FADE).max(1.0),
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
const GROUND: &str = r#"
    let patches = 0.86 + 0.28 * noise(in.world_position.xz * 0.004, 4);
    let dry = smoothstep(0.0, SCENE_RADIUS * 1.6, in.world_position.y);
    let albedo = mix(vec3<f32>(0.26, 0.29, 0.20), vec3<f32>(0.40, 0.36, 0.24), dry);
    color = color * albedo * patches;

    // The valley this massif stands in is **full of cloud**, and the massif comes out of it.
    //
    // It is drawn on the ground rather than into the air because the air is not a surface
    // anything can stand in front of: there is no second mesh to put a cloud on, and a cloud
    // painted over the sky would be in front of the ranges and behind the mountain at the same
    // time. What is drawn here is the ground below a level, and the level is ragged — two
    // octaves of noise on the world's own ground coordinate, so the top edge of the cloud is a
    // coastline rather than a contour — which is the whole of what makes it read as a sea.
    //
    // It also does the composition's work: without it the massif stands on a flat green field
    // that takes up a third of the frame, and with it the ground between the eye and the horizon
    // is a surface that is going somewhere.
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
    color = mix(color, cloud, sea * CLOUD_COVER);

    let eye = uniforms.projection.position;
    let view = normalize(in.world_position - eye);
    let facing = 1.0 - abs(dot(normal, view));
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
        color = ridges(air_sky(ray), ray, RIDGE_SCALE);

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
