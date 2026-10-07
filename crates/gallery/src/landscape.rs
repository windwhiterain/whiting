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

use definition::{PackedList, Value};
use mountain::{FacetMesh, LineRef, Massif, RidgeId, Starter, Tessellation};
use scene::standard::Stage;

use crate::air::Atmosphere;
use crate::geometry::{Shaded, fractal_noise, heightfield, smoothstep};
use crate::kit::{
    camera_placed, directional, instance, instances, light_with_radius, mesh, mesh_material,
    post_processes, screen_material, shaded_point_schema, surface,
};
use crate::shaders::{lighting_shader_with_schema, mesh_shaders, post_shader};

/// How finely the skeleton is sampled, in world units.
///
/// This is **deliberately coarse**, and it is the one number that decides whether the crate's
/// whole premise is visible. A triangle here is one planar block face, and a face is only a face
/// if it is big enough to read as one: sampled at a couple of units the facets are a pixel across
/// and the massif shades as the smooth cone the arithmetic would have produced anyway. Eighteen
/// units over a massif a few hundred across is a facet of thirty-odd pixels, which is the scale a
/// rock face has when it is seen from a valley.
const STEP: f32 = 18.0;

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
const SUN_OFFSET: f64 = 1.55;

/// How high the sun stands, in radians.
///
/// Low enough that a block face turned a little away from it is markedly darker than its
/// neighbour, high enough that the whole massif is not one long shadow.
const SUN_ELEVATION: f64 = 0.50;

/// Builds the landscape.
pub fn landscape() -> Value {
    let massif = massif();
    let raised = massif.build().expect("the authored massif expands");
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
        centre[0] + radius * 2.25,
        RIM + radius * 0.30,
        centre[2] + radius * 0.85,
    ];
    let target = [centre[0], centre[1] + radius * 0.30, centre[2]];
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
    let camera_bearing = (eye[2] - centre[2]).atan2(eye[0] - centre[0]);
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

    Value::structure([
        ("name".to_owned(), Value::String("landscape".to_owned())),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed(
                eye,
                target,
                std::f64::consts::FRAC_PI_3,
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
                    [1.0, 0.84, 0.62],
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
        (
            "shaders".to_owned(),
            Value::List(
                mesh_shaders(MASSIF, &helpers)
                    .into_iter()
                    // The plain is drawn by the same pipeline and a different law: it holds no
                    // snow and no rock, and the body above reads per face facts the plain's
                    // heightfield does not carry. A second lighting shader under its own label is
                    // one shader more and one branch fewer.
                    .chain(std::iter::once(lighting_shader_with_schema(
                        "ground_lighting",
                        "",
                        GROUND,
                        &helpers,
                        crate::kit::gallery_custom_schema(),
                    )))
                    .chain(std::iter::once(post_shader(GRADE, &helpers)))
                    .collect(),
            ),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh(
                    "massif",
                    "Triangles",
                    shaded_point_schema(),
                    faceted(&raised),
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
                // One material per stage, because a material names one shader and a shader belongs
                // to one pass. The shadowing and depthing stages read no custom data, so theirs is
                // a placeholder those shaders ignore.
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
                // The stone, which the surface body replaces entirely: the tint is **white** and
                // the body writes the albedo, so the colour of the rock is decided in one place.
                // A tint with a colour in it would be multiplied by the body's albedo, which is
                // two decisions about one surface and the sort that drift apart.
                mesh_material(
                    "shade",
                    "lighting",
                    surface([1.0, 1.0, 1.0], 0.0, 0.86, [0.0; 3]),
                ),
                mesh_material(
                    "ground_depth",
                    "depthing",
                    surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
                ),
                mesh_material(
                    "ground_shade",
                    "ground_lighting",
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

/// Returns the massif this scene is of: a generated fan, then edited by hand.
///
/// # Why the generated massif is edited and not used as it comes
///
/// [`Starter`] is a starting point and says so. What it cannot know is what the scene is *of*:
/// that the massif has to stand on a plain, that one crest should be sharp and the rest rounded,
/// that a flank should be sampled finer than the rest. Those are the author's decisions, and they
/// are made here with the crate's own ops rather than baked into the generator.
fn massif() -> Massif {
    let params = Starter {
        ridges: 8,
        nodes: 15,
        reach: 200.0,
        summit: 400.0,
        tip: 30.0,
        bulge: 0.08,
        bearing: -2.30,
        spread: std::f32::consts::TAU,
        irregularity: 0.42,
        relief: 0.75,
        seed: 23,
    };
    let mut massif = Massif::starter(params.seed, params)
        .with_tessellation(Tessellation::default().with_step(STEP));

    shape_the_crests(&mut massif);
    bend_the_ridges(&mut massif);
    stand_on_the_plain(&mut massif);
    sharpen_the_main_ridge(&mut massif);
    deepen_a_valley(&mut massif);

    // The skeleton here is written by hand, so the correspondence it wrote is checked before it is
    // expanded. A rib that crosses a line, or a stretch of a line that no station faces, builds a
    // mesh that overlaps itself or has a hole in it — and both of those are perfectly ordinary
    // pictures of a mountain. This is the one place the scene can be told it is wrong.
    debug_assert!(
        massif.validate().is_empty(),
        "the authored massif is not valid: {:#?}",
        massif.validate()
    );
    massif
}

/// Gives each crest a different profile, so the massif has summits and shoulders rather than one
/// apex with eight equal falls.
///
/// # Invariants
///
/// The exponent is read from the ridge's own authored **sharpness**, which the generator already
/// varies per ridge, so the shape of a crest and the stone it is made of are one decision rather
/// than two that can be edited apart. Every profile still begins at the summit and falls, so the
/// line stays non-increasing and the edit cannot make the massif invalid.
fn shape_the_crests(massif: &mut Massif) {
    for ridge in massif.skeleton.ridge_ids() {
        let count = massif.skeleton.ridges[ridge.0].nodes.len();
        let sharpness = massif.skeleton.ridges[ridge.0].nodes[0].sharpness;
        // A sharp crest carries its height far out; a blunt one gives most of it up in the first
        // third. Both endpoints are untouched, so a crest still leaves the summit at the summit's
        // height and still reaches the rim at the rim's.
        let exponent = 1.70 - 1.10 * sharpness;
        let summit = massif.skeleton.ridges[ridge.0].nodes[0].elevation;
        let tip = massif.skeleton.ridges[ridge.0].nodes[count - 1].elevation;
        for node in 1..count - 1 {
            let fraction = node as f32 / (count - 1) as f32;
            let shaped = summit + (tip - summit) * fraction.powf(exponent);
            massif.set_elevation(LineRef::Ridge(ridge), node, shaped);
        }
    }
}

/// Bends each ridge in plan, so a crest wanders down its flank instead of running straight out
/// from the summit.
///
/// # Invariants
///
/// The bend is **zero at both ends** of every ridge, and that is what keeps the edit legal. A
/// ridge's first node is the summit that eight ridges share and its last node is a footprint
/// vertex; moving either would tear the summit apart or pull the rim off the plain. What moves is
/// the middle, which is the part that reads as a ridge rather than as a radius.
///
/// The correspondence is untouched: a station names a **parameter** on the ridge, so bending the
/// line moves what the parameter resolves to and moves the valley's ribs with it. Nothing needs
/// re-binding.
fn bend_the_ridges(massif: &mut Massif) {
    for ridge in massif.skeleton.ridge_ids() {
        let count = massif.skeleton.ridges[ridge.0].nodes.len();
        let start = massif.skeleton.ridges[ridge.0].nodes[0].position;
        let end = massif.skeleton.ridges[ridge.0].nodes[count - 1].position;
        let along = crate::geometry::normalized(crate::geometry::subtract(
            [end[0] as f64, 0.0, end[1] as f64],
            [start[0] as f64, 0.0, start[1] as f64],
        ));
        // The perpendicular of the ridge's own direction, in the plan.
        let across = [-along[2], along[0]];
        // The bend is small enough to stay **inside the wedge the ridge owns**: a crest bent
        // further than the half-angle it subtends at its own valley would put the valley on the
        // other side of it, and `validate` reports exactly that as a line with a stretch nothing
        // faces. Six percent of a ridge's length is a fifth of the smallest half-wedge here.
        let seed = (ridge.0 as f64) * 1.7;
        let amount = 0.040 + 0.020 * (seed * 1.3).sin();
        let sense = if (seed * 2.1).sin() >= 0.0 { 1.0 } else { -1.0 };
        let reach = (end[0] as f64 - start[0] as f64).hypot(end[1] as f64 - start[1] as f64);
        for node in 1..count - 1 {
            let fraction = node as f64 / (count - 1) as f64;
            let offset =
                sense * amount * (std::f64::consts::PI * fraction).sin() * (1.9 * fraction).cos();
            let current = massif.skeleton.ridges[ridge.0].nodes[node].position;
            massif.move_node(
                LineRef::Ridge(ridge),
                node,
                [
                    (current[0] as f64 + across[0] * offset * reach) as f32,
                    (current[1] as f64 + across[1] * offset * reach) as f32,
                ],
            );
        }
    }
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

/// Gives the highest ridge a crest worth the name, and samples it finer than the rest.
///
/// # Invariants
///
/// The ridge chosen is the one whose tip stands highest, which is the one that stands highest
/// along its whole length: every ridge starts at the same summit elevation and falls by the same
/// profile, so their order never changes. Reading the summit instead would find eight ridges that
/// agree.
fn sharpen_the_main_ridge(massif: &mut Massif) {
    let highest = massif
        .skeleton
        .ridge_ids()
        .into_iter()
        .max_by(|left, right| {
            let tip = |ridge: RidgeId| {
                massif.skeleton.ridges[ridge.0]
                    .nodes
                    .last()
                    .map(|node| node.elevation)
                    .unwrap_or(f32::MIN)
            };
            tip(*left).total_cmp(&tip(*right))
        });
    let Some(highest) = highest else {
        return;
    };
    for node in 0..massif.skeleton.ridges[highest.0].nodes.len() {
        let mut value = massif.skeleton.ridges[highest.0].nodes[node];
        // A knife edge near the summit and a broader shoulder below it, which is what a crest
        // that carries a spire looks like from a distance.
        let fraction = node as f32 / (massif.skeleton.ridges[highest.0].nodes.len() - 1) as f32;
        value.sharpness = 0.95 - 0.45 * fraction;
        value.half_width *= 0.55;
        massif.set_ridge_node(highest, node, value);
    }
    // The crest is where the facets are smallest and the eye spends the most time, so it is the
    // one line worth sampling twice as finely as the rest.
    massif.set_step_scale(LineRef::Ridge(highest), 0.5);
}

/// Deepens the valley beside the main ridge, so the two flanks read as separate mountains.
///
/// # Invariants
///
/// Only the elevations move, and they move **down**, so the valley stays a valley: an edit that
/// raised a valley node above its neighbour would be reported by `validate` rather than quietly
/// built, and a scene that shipped one would have a ridge running through its own floor.
fn deepen_a_valley(massif: &mut Massif) {
    let Some(first) = massif.skeleton.valley_ids().first().copied() else {
        return;
    };
    let count = massif.skeleton.valleys[first.0].nodes.len();
    for node in 1..count.saturating_sub(1) {
        let fraction = node as f32 / (count - 1) as f32;
        let current = massif.skeleton.valleys[first.0].nodes[node].elevation;
        let drop = current * 0.22 * (1.0 - fraction);
        massif.set_elevation(LineRef::Valley(first), node, current - drop);
    }
}

/// Turns the massif's facets into the points the standard pipeline draws.
///
/// # Invariants
///
/// - **The normal is the face's own, written at all three corners.** A face is one planar block
///   face, and giving its corners a shared averaged normal would shade it as part of a smooth
///   surface — which is the one thing the expansion produced it not to be.
/// - **A point is written per corner and never shared between faces.** Sharing a vertex between
///   two faces that carry different normals is exactly the averaging this refuses, and the
///   pipeline's meshes are not indexed, so a shared vertex is not expressible anyway.
/// - The texture coordinate carries the face's **ruggedness** and its **steepness**. This mesh
///   samples no texture, and those two are the numbers the crate knows about a face that a shader
///   could not recover from its position and its normal: how sharp the crest above it was
///   authored, and how far over it leans. The steepness is the dip angle as a fraction of a right
///   angle, so one is a vertical face, and it is written that way rather than in radians because
///   the slot is a pair of unitless parameters and a reader should not have to know pi.
fn faceted(mesh: &FacetMesh) -> PackedList {
    let mut points = Shaded::with_capacity(mesh.faces.len() * 3);
    for face in &mesh.faces {
        let corners = mesh.triangle(face);
        let normal = mesh.normal(face);
        let texture = [
            face.attributes.ruggedness as f64,
            (face.attributes.dip_angle as f64 / std::f64::consts::FRAC_PI_2).clamp(0.0, 1.0),
        ];
        for corner in corners {
            points.push(
                [corner[0] as f64, corner[1] as f64, corner[2] as f64],
                [normal[0] as f64, normal[1] as f64, normal[2] as f64],
                texture,
            );
        }
    }
    points.finish()
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
         const SCENE_RADIUS: f32 = {radius:.3};\n",
        summit * SNOW_LINE,
        summit * SNOW_WOBBLE,
        (summit * SNOW_FADE).max(1.0),
    )
}

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
    let snowed = mix(stone, vec3<f32>(0.86, 0.90, 0.97), snow);

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
        color = air_sky(ray);
    }

    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.18), vec3<f32>(0.0), vec3<f32>(1.0));
    color = color * color * (3.0 - 2.0 * color) * 0.26 + color * 0.74;

    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.30, 0.0, 1.0);
"#;
