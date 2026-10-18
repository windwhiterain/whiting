//! One bamboo, drawn: its wood as a single mesh and its leaves as one blade instanced once per leaf.
//!
//! # What this scene is for
//!
//! `vegetation` measures itself in [`plant_probe`](../../../vegetation/examples/plant_probe.rs): a plant
//! walks, a field counts, a blade is a solid. **Those are numbers about geometry, and geometry can satisfy
//! all of them and still look like nothing** — a leaf standing in the air, a crown that is a hedgehog of
//! triangles, a culm whose normals face inward. This is the scene that shows the plant reaching a frame,
//! and it lives here rather than in an example because a gallery scene is this tree's one picture format:
//! [`render`] draws it, `--census` reports what it holds, `--locate` answers what a pixel is.
//!
//! ```text
//! cargo run -p whiting-device-rasterize --example render -- bamboo out/bamboo
//! ```
//!
//! **It is in [`crate::Gallery::build`] and not in [`crate::Gallery::names`].** Those are two lists and the
//! second is not a shorter way of writing the first: `names` is the set the tree's suites hold themselves
//! to, and `scene`'s `clustered_lights` carries a table of expected pass counts per scene.
//! [`crate::sweep_picture`] is unlisted for the same reason, and this one is rendered by name on request.
//!
//! # The claim the frame makes
//!
//! **A leaf field is a distribution and not a generator**: one blade, `n` rigid placements, one mesh.
//! The document therefore carries three meshes — the wood, the blade, the ground — and the lighting bucket
//! carries `n + 2` placements of which the `n` leaves are one **instanced draw**: the emitter groups by
//! shader, mesh and images, so every leaf of every twig lands in one pass over one instance buffer. That is
//! a claim about a call site rather than about a table, which is why it is here and not in a probe, and
//! `plant_instancing_probe` measures the one part of it that could be quietly wrong — the transform.
//!
//! # What is not built here
//!
//! **No second mesh for a second node.** The plant is one object standing on a ground plane, so a material
//! per branch would be ten materials and ten passes over one tree. What differs between a culm and a twig
//! is geometry, which is already in the points. And the identity a pixel carries is the node's own, written
//! per corner; the leaf template is the one mesh whose ids cannot be baked, because it is one mesh drawn by
//! many rows — see [`crate::vegetation_mesh`].
//!
//! [`render`]: ../../../device-rasterize/examples/render.rs

use definition::{PackedList, Value};
use scene::standard::Stage;
use vegetation::decorator::{Placed, Seat, Template, placed};
use vegetation::derive::{Species, bamboo};
use vegetation::envelope::{Envelope, trim};
use vegetation::expand::{Detail, Expansion, expand};
use vegetation::field::Field;
use vegetation::handle::Handle;
use vegetation::leaf::LeafTemplate;
use vegetation::plant::Placement;

use crate::air::Atmosphere;
use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, stage_set, stage_set_over_schema,
};
use crate::geometry::plane_facing;
use crate::kit::{
    camera_placed, environment_light, flux_for_irradiance, image, instance, instances, light,
    light_with_radius, mesh, mesh_material, offset_by, plant_custom_schema, post_processes,
    screen_material, shadow_bias_scale, surface,
};

use crate::shaders::{Projection, post_shader};
use crate::vegetation_mesh::{LEAF_BASE_ID, node_ids, node_ids_from, template_points};

/// The name the still scene is built and rendered under.
pub const SCENE: &str = "bamboo";

/// The name the same plant is built and rendered under **with a wind on it**, so a picture can show what a
/// field does beside the plant it does it to.
pub const WINDY: &str = "bamboo_wind";
/// The name the same plant is drawn under **fitted to a crown**, so a picture can show what the envelope took.
pub const CROWNED: &str = "bamboo_crown";
/// The name the same plant is drawn under **with a knot at the base of every bough**.
pub const KNOTTED: &str = "bamboo_knots";

/// The seed the plant's leaf field is derived from.
///
/// **The number is the scene's, not the crate's.** [`vegetation::derive::bamboo`] takes a seed for the leaf
/// field's own phase and jitter and uses it for nothing else, so this one number is the whole difference
/// between this plant's leaves and any other drawing of the same tree.
const SEED: u64 = 7;

/// The frame, which is **taller than it is wide**: the subject is 2.2 m up and about 2.7 m across, so a
/// landscape frame either crops the crown or stands so far back that a bamboo becomes a line.
const WIDTH: i64 = 1200;
const HEIGHT: i64 = 1400;

/// Where the camera stands, and what it looks at.
///
/// The eye is **five metres out and half a metre above the middle of the culm**, which is the whole of the
/// framing: the crown reaches 1.35 m either side of a culm that stands 2.2 m tall, and the frame is
/// narrower than it is tall, so the half width has to clear 1.35 m at the crown or the frame crops the one
/// part of a bamboo that is a bamboo. Standing at half the culm's height rather than above its crown puts
/// the crown against the sky and the floor's horizon behind the plant, so the two metres of culm are read
/// as height instead of as a plan seen from a roof.
const EYE: [f64; 3] = [3.40, 1.55, 3.80];
const TARGET: [f64; 3] = [0.16, 1.10, 0.05];
const FIELD_OF_VIEW: f64 = 0.66;
const NEAR: f64 = 0.05;
const FAR: f64 = 24.0;

/// The ground the plant stands on, and how far it reaches.
///
/// Six metres, which is further than the frame's own near corner, so the floor does not end inside the
/// picture: a floor edge at 3 m under a camera 5.4 m back is a horizon line half way up the frame, and the
/// whole of this scene is one plant and the ground it stands on.
const FLOOR_EXTENT: f64 = 6.0;
const FLOOR_TINT: [f64; 3] = [0.40, 0.38, 0.33];

/// The key, which is the light that makes a culm read as a round thing rather than a stripe.
///
/// It stands **above and to the lens's right**, so the culm has a lit side and a shadowed side at once and
/// every twig's shadow falls across the floor to the left of its caster. The reach is past the far corner
/// of the floor, so nothing in frame falls out of the cluster grid at its boundary.
const KEY_POSITION: [f64; 3] = [2.6, 3.9, 2.0];
const KEY_COLOUR: [f64; 3] = [1.0, 0.96, 0.86];
const KEY_IRRADIANCE: f64 = 3.0;
const KEY_REACH: f64 = 7.0;
const KEY_EXTENT: f64 = 3.0;

/// The fill, which names no reach and so is **global**: it is a second light on the shadowed side, it is
/// culled by nothing, and no shadow of it falls on the ground.
const FILL_POSITION: [f64; 3] = [-2.6, 1.5, -1.8];
const FILL_COLOUR: [f64; 3] = [0.44, 0.54, 0.76];
const FILL_IRRADIANCE: f64 = 0.7;
/// The distance the fill's irradiance was stated at before lightings existed, kept as the default so
/// that every scene drawn before this type is the scene it was.
const FILL_DISTANCE: f64 = 3.0;

/// The two greens of a bamboo, which are not the same green.
///
/// A culm is waxy and pale and a leaf is dark and matte, and one tint for both makes a bamboo look like a
/// tree drawn in felt pen: the leaf is the thing this scene exists to draw, so it is the one that carries
/// the colour.
const CULM_TINT: [f64; 3] = [0.46, 0.50, 0.24];
const LEAF_TINT: [f64; 3] = [0.20, 0.38, 0.15];

/// The sky this scene draws where nothing else is, written once so the haze and the horizon agree.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.16, 0.28, 0.50])
        .horizon([0.62, 0.68, 0.72])
        .ground([0.38, 0.37, 0.34])
        // A thin day over a garden: the air is a haze on the far wood and nothing more, so the
        // scale height is short and the density is low.
        .density(0.0012)
        .scale_height(60.0)
}

/// How bright the sky is, as the one number both backends read.
///
/// # Invariants
///
/// It is the environment light's `radiance`, which the rasteriser reads as the ambient the shadows
/// are lit by and as the sky behind the plant, and which the exporter writes as Cycles'
/// `background` strength — so the two frames cannot disagree about how bright the day is. The map
/// is the sky's colour and this is its brightness, which is why changing the sky's look does not
/// mean touching this number.
const SKY_RADIANCE: f64 = 1.0;

/// What the post stage does to the frame.
///
/// The air is an exponential over the geometry only, for the reason the shed's grade gives: the sky is
/// already the far field, and a haze that reached it as well would put two different greys on one horizon.
/// **The haze's own coefficient is [`Lighting::haze`]**, spliced in where the builder formats this: a plant
/// five metres from the lens and a tree thirty metres from it are not in the same air, and one coefficient
/// for both made every distant tree a ghost.
const GRADE: &str = r#"
    // **The sky is the scene's own environment map**, which is what a path tracer draws its world
    // from: writing a sky law here instead gave the two backends two different skies over one
    // plant, and no number could bring them together. The map is the sky's colour and the
    // environment light's radiance is its brightness.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * {HAZE}), 0.0, 0.45);
        color = mix(color, environment_sky(vec3<f32>(ray_through(in.uv).x, 0.02, ray_through(in.uv).z)), air);
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
"#;

/// The plant this scene draws, expanded at the dial its planting carries.
///
/// The dial is the planting's and not the scene's, because a hero in the foreground and a companion on
/// the far ridge are the same species at two resolutions: a scene that picked one dial for every
/// planting would draw the ridge at the hero's cost.
fn expansion(species: &Species, detail: Detail) -> Expansion {
    expand(&species.plant, detail, &species.fields)
}

/// A knot: a short tube that bulges, seated on a bough's base so the culm shows where a limb leaves it.
fn knot() -> Template {
    let spine = shape::Curve::new(vec![
        shape::Jet::new([0.0, 0.0, 0.0], [0.012, 0.0, 0.0]),
        shape::Jet::new([0.014, 0.0, 0.0], [0.010, 0.0, 0.0]),
        shape::Jet::new([0.026, 0.0, 0.0], [0.004, 0.0, 0.0]),
    ])
    .expect("the knot's knots are distinct");
    let mut sweep = shape::Sweep::new(spine, shape::Section::circle());
    sweep.scale = shape::Scalar::new(&[(0.0, 0.010), (0.5, 0.026), (1.0, 0.004)])
        .expect("the knot's law is a law");
    Template {
        sweep,
        stations: 7,
        around: 12,
    }
}

/// One knot at the base of every bough: the seats a scene places.
fn knots(species: &Species) -> Vec<Seat> {
    species
        .plant
        .live()
        .filter(|(handle, _)| handle.depth() == 1)
        .map(|(handle, node)| {
            Seat::new(handle, 0, 0.1, node.attachment().slot)
                .sunk(0.012)
                .scaled(1.8)
        })
        .collect()
}

/// Resolves an `authored_id` this scene's frame writes into the handle that names the node.
///
/// # Invariants
///
/// - **The table is built by the same walk the mesh's ids were numbered by**, so a pixel and a handle are
///   the same number by construction rather than by two orderings that have not diverged yet.
/// - **An id this plant does not have answers `None`**, and so does the reserved zero, because *"no such
///   node"* and *"not an authored element"* are different answers.
pub fn handle_of(id: u32) -> Option<String> {
    let species = bamboo(SEED);
    node_ids(&species.plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Returns the sidecar table a frame of this scene is read beside: every id and the handle it names.
///
/// One line per node, in id order, because the table is read beside a picture and a reader looking for
/// *"which twig is that"* wants the ordered list rather than a lookup they have to write.
pub fn sidecar() -> String {
    let species = bamboo(SEED);
    let mut ids: Vec<(u32, String)> = node_ids(&species.plant)
        .into_iter()
        .map(|(handle, id)| (id, handle.to_string()))
        .collect();
    ids.sort_unstable();
    let mut table = String::from(
        "authored_id  handle                 a leaf of this node is drawn by the blade mesh with this offset\n",
    );
    for (id, handle) in ids {
        table.push_str(&format!(
            "{id:>10}  {handle:<22} {}\n",
            id as f64 - f64::from(LEAF_BASE_ID)
        ));
    }
    table
}

/// A scene of plants is framed by its own camera and its own ground: the same document, a different eye.
///
/// # Invariants
///
/// A framing carries **numbers only**, so two scenes framed differently are one builder's two arguments rather than
/// two builders: the camera, the frame and the floor's reach are the three things a picture of plants differs in
/// from another one, and everything the plants themselves are made of is the plantings'.
#[derive(Debug, Clone, Copy)]
pub struct Framing {
    /// The frame's width in pixels.
    pub width: i64,
    /// The frame's height in pixels.
    pub height: i64,
    /// Where the camera stands.
    pub eye: [f64; 3],
    /// What it looks at.
    pub target: [f64; 3],
    /// The vertical field of view, in radians.
    pub field_of_view: f64,
    /// The near plane, which must be closer than the nearest leaf.
    pub near: f64,
    /// The far plane, which must reach past the floor's own corner.
    pub far: f64,
    /// How far the floor reaches from the origin.
    pub floor: f64,
    /// The ground the scene stands on: a plane at zero, or a heightfield's own law.
    pub ground: Ground,
}

/// The ground a scene stands on.
///
/// # Invariants
///
/// **A ground is a function and not a type**, so a caller with a `terrain::HeightField` passes its sampler and this
/// crate stays terrain-independent — the same rule [`vegetation::ground`] states for a plant's base, kept by the
/// scene that draws where the base sits. A plane is [`Ground::Plane`] and is the floor every still scene stands on.
#[derive(Debug, Clone, Copy)]
pub enum Ground {
    /// A plane at zero.
    Plane,
    /// A heightfield: a swell, a dune, a courtyard's camber.
    Fielded {
        /// The height at a plan position, in metres.
        height: fn(f64, f64) -> f64,
        /// How many cells across the field is drawn with.
        divisions: usize,
    },
}

/// One surface a plant is drawn in: a tint and a roughness, which is all of a bark or a leaf that this
/// builder's surface law reads.
///
/// # Invariants
///
/// **A skin is two numbers and not a material**, because a material is a document's row — a label, a shader
/// and a surface — and the label and the shader are this builder's to choose. Two plantings that agree on a
/// skin share a mesh, which is what makes a grove of one species one draw and a frame of ten species ten.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Skin {
    /// The diffuse tint.
    pub tint: [f64; 3],
    /// The roughness.
    pub roughness: f64,
}

impl Skin {
    /// A skin of one tint and one roughness.
    pub const fn new(tint: [f64; 3], roughness: f64) -> Self {
        Self { tint, roughness }
    }

    /// The surface row this skin is drawn by.
    fn value(self) -> Value {
        surface(self.tint, 0.0, self.roughness, [0.0; 3])
    }
}

/// What a plant's own shadow test's receiver push and depth bias are scaled by.
///
/// # Invariants
///
/// **A leaf and not a plant.** The two numbers the shadow test biases by are sized for a surface
/// whose casters stand metres away: on a page fitted to a whole tree one texel is a sixth of a
/// metre, and the receiver is pushed a whole texel off itself before the test even runs. A leaf's
/// casters are centimetres away, so a caster a millimetre in front of a blade is *inside* that push
/// and the test cannot fire — which is why a crown's interior came out exactly as bright as its
/// rim.
///
/// **The bole keeps the default, and the reason is the same fact read the other way round.** A
/// trunk's casters are metres away, which is the case the numbers were written for; scaled to a
/// fiftieth they no longer cover a trunk's own rasterisation error, and the bole comes back
/// striped with acne. The scale is a statement about how close this surface's casters are, so it
/// belongs on the surfaces whose casters are close.
///
/// See `.agents/investigate/leaf-self-shadowing.md`.
pub const PLANT_SHADOW_BIAS_SCALE: f64 = 0.05;

/// Returns one plant material's row, with that row's own shadow-bias scale.
///
/// # Invariants
///
/// **`None` leaves the row exactly as it was, and it is the caster and depther rows that pass it.**
/// The plant's shading stage declares [`crate::kit::plant_custom_schema`]; its shadowing and
/// depthing stages come from [`crate::shaders::mesh_shaders_over`] and declare the gallery's own. A
/// caster row naming a field its shader does not declare is a row the compiler refuses by name, so
/// the field goes on the shaded rows and on nothing else.
fn plant_row(row: Value, scale: Option<f64>) -> Value {
    match scale {
        Some(scale) => shadow_bias_scale(row, scale),
        None => row,
    }
}

/// The three surfaces a planting is drawn in: its wood, one skin per leaf template and one per decorator
/// template, in the order the species' own indices name them.
///
/// # Invariants
///
/// **Colour is a scene's fact and travels with the planting**, not with the species: the `vegetation` crate
/// stays colour-free, which is the same rule that keeps a light direction out of it. The default is the two
/// greens every vegetation scene was drawn in before palettes existed, so a species that never names a
/// palette is drawn exactly as it was.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    /// The skin every node's wood is drawn in.
    pub wood: Skin,
    /// One skin per leaf template, in template order.
    pub leaves: Vec<Skin>,
    /// One skin per decorator template, in template order.
    pub seats: Vec<Skin>,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            wood: Skin::new(CULM_TINT, 0.72),
            leaves: vec![Skin::new(LEAF_TINT, 0.62)],
            seats: vec![Skin::new(CULM_TINT, 0.72)],
        }
    }
}

impl Palette {
    /// The skin a leaf template is drawn in: its own, or the palette's first leaf skin when the palette
    /// holds none for it, so a species that adds a template and says nothing about colour is still drawn
    /// whole rather than failing in the middle of a render.
    fn leaf(&self, template: usize) -> Skin {
        self.leaves
            .get(template)
            .or(self.leaves.first())
            .copied()
            .unwrap_or(Skin::new(LEAF_TINT, 0.62))
    }

    /// The skin a decorator template is drawn in, by the same rule as [`Palette::leaf`].
    fn seat(&self, template: usize) -> Skin {
        self.seats
            .get(template)
            .or(self.seats.first())
            .copied()
            .unwrap_or(Skin::new(CULM_TINT, 0.72))
    }
}

/// The light a vegetation scene stands in: one key, one fill, the day itself, and the ground's own colour.
///
/// # Invariants
///
/// **The lights are numbers a scene states, not constants of this module.** A kapok in a dry-hot valley at
/// noon and a fir on a misty ridge at dawn are two pictures, and before this type they were one: the key,
/// the fill, the sky and the floor's tint were module constants and changing one changed every vegetation
/// scene. The default is those constants, so every existing scene is the scene it was.
#[derive(Debug, Clone, Copy)]
pub struct Lighting {
    /// Where the key stands.
    pub key_position: [f64; 3],
    /// The key's colour.
    pub key_colour: [f64; 3],
    /// The key's irradiance at the subject.
    pub key_irradiance: f64,
    /// The distance from the key at which `key_irradiance` is stated, which is normally the length of
    /// `key_position`.
    ///
    /// It is a separate number from the reach because the two answer different questions: the distance
    /// says where the stated irradiance holds, and the reach says where the light stops. A tree framed at
    /// thirty metres wants its cutoff far past the floor's edge and its irradiance stated at the tree,
    /// and one number cannot be both.
    pub key_distance: f64,
    /// How far the key's shadow reach extends.
    pub key_reach: f64,
    /// The key's own extent, which is its softness.
    pub key_extent: f64,
    /// Where the fill stands.
    pub fill_position: [f64; 3],
    /// The fill's colour.
    pub fill_colour: [f64; 3],
    /// The fill's irradiance.
    pub fill_irradiance: f64,
    /// The distance from the fill at which `fill_irradiance` is stated.
    pub fill_distance: f64,
    /// The ground's skin.
    pub floor: Skin,
    /// The day: the sky the frame's background is drawn from and the haze on the far wood.
    pub sky: Atmosphere,
    /// How bright the sky is, the one number both backends read.
    pub sky_radiance: f64,
    /// The post stage's aerial-perspective coefficient, per metre of ray.
    ///
    /// It is the grade's own exponential and not the atmosphere's density: the atmosphere is what a path
    /// tracer scatters through and this is what the rasteriser's post stage fakes over the geometry, and
    /// the two agree by sharing a sky rather than by sharing a number. A garden's five metres and a
    /// ridge's thirty are not in the same air.
    pub haze: f64,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            key_position: KEY_POSITION,
            key_colour: KEY_COLOUR,
            key_irradiance: KEY_IRRADIANCE,
            key_distance: KEY_REACH,
            key_reach: KEY_REACH,
            key_extent: KEY_EXTENT,
            fill_position: FILL_POSITION,
            fill_colour: FILL_COLOUR,
            fill_irradiance: FILL_IRRADIANCE,
            fill_distance: FILL_DISTANCE,
            floor: Skin::new(FLOOR_TINT, 0.94),
            sky: atmosphere(),
            sky_radiance: SKY_RADIANCE,
            haze: 0.012,
        }
    }
}

/// One plant in a scene: a species, where it stands, and any decorators it carries.
///
/// # Invariants
///
/// A planting's placement moves it into the world and **does not change its ids**: an id names a plant through the
/// base its planting gave it ([`node_ids_from`]) and a node through its offset, so a frame drawn from five plantings
/// answers *which plant and which node* with one number.
pub struct Planting {
    /// The plant and its templates.
    pub species: Species,
    /// Where the plant stands: the frame its own coordinates are in.
    pub placement: Placement,
    /// The decorators its nodes carry, in its own coordinates.
    pub seats: Vec<Seat>,
    /// The surfaces it is drawn in.
    pub palette: Palette,
    /// How finely its wood is expanded: a hero in the foreground and a companion on the far ridge are
    /// the same species at two dials, which is the level of detail a grove needs and a still does not.
    pub detail: Detail,
}

impl Planting {
    /// A planting standing where it grew, in the default palette and at the crate's own dial.
    pub fn new(species: Species) -> Self {
        Self {
            species,
            placement: Placement::IDENTITY,
            seats: Vec::new(),
            palette: Palette::default(),
            detail: Detail::DEFAULT,
        }
    }

    /// The same planting somewhere else.
    pub fn at(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// The same planting carrying decorators.
    pub fn with_seats(mut self, seats: Vec<Seat>) -> Self {
        self.seats = seats;
        self
    }

    /// The same planting drawn in its own surfaces.
    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.palette = palette;
        self
    }

    /// The same planting expanded at its own dial.
    pub fn with_detail(mut self, detail: Detail) -> Self {
        self.detail = detail;
        self
    }
}

/// How far apart two plantings' id bases are: a plant of more than sixty-five thousand nodes is not one
/// a picture draws.
///
/// The stride was four thousand while the gallery's plants were culms and shrubs; a broadleaf crown
/// dense enough to be a crown is tens of thousands of nodes, and four thousand capped the dove tree's
/// crown at nine metres against its note's eleven to fifteen. The ids are `f32` in the point schema and
/// every id below 2^24 is exact in one, so the wider stride costs nothing.
pub const PLANTING_STRIDE: u32 = 65_536;

/// The framing of the four scenes that draw one plant.
const STILL: Framing = Framing {
    width: WIDTH,
    height: HEIGHT,
    eye: EYE,
    target: TARGET,
    field_of_view: FIELD_OF_VIEW,
    near: NEAR,
    far: FAR,
    floor: FLOOR_EXTENT,
    ground: Ground::Plane,
};

/// The framing of a grove: back far enough for three culms, and no further.
const GROVE_FRAMING: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [2.55, 1.85, 3.35],
    target: [0.05, 1.00, 0.00],
    field_of_view: 0.75,
    near: 0.05,
    far: 32.0,
    floor: 7.0,
    ground: Ground::Fielded {
        height: swell,
        divisions: 56,
    },
};

/// The name a grove of three culms and three shrubs is built and rendered under.
pub const GROVE: &str = "grove";

/// The three seeds the culms are sown from. A bamboo's rule leaves nothing to chance but its leaves, so three
/// culms differ in their foliage and in where they stand — which is what a clump of bamboo is.
const GROVE_CULMS: [u64; 3] = [7, 8, 9];

/// The three seeds the undergrowth is sown from. A shrub's rule draws its own skeleton, so three seeds are three
/// different shrubs rather than three drawings of one.
const GROVE_SHRUBS: [u64; 3] = [31, 47, 59];

/// Where each culm stands and which way it faces.
const GROVE_CULM_PLACES: [([f32; 3], f32); 3] = [
    ([-0.62, 0.0, 0.30], 0.3),
    ([0.04, 0.0, -0.25], 1.9),
    ([0.66, 0.0, 0.34], 3.4),
];

/// Where each shrub stands and which way it faces: between the culms, so the grove reads as one planting.
const GROVE_SHRUB_PLACES: [([f32; 3], f32); 3] = [
    ([-0.98, 0.0, -0.22], 0.9),
    ([0.34, 0.0, 0.42], 2.6),
    ([0.95, 0.0, -0.30], 4.7),
];

/// A frame standing at a place and turned about the world's `y`: the only two numbers a planting needs to stand
/// somewhere, and the reason a turn is a placement rather than a decorator's forbidden angle — a *plant* may be
/// turned, a decorator may not, because a decorator has to sit on a surface that already has a frame.
pub(crate) fn planted_at(position: [f32; 3], turn: f32) -> Placement {
    let (sin, cos) = turn.sin_cos();
    Placement {
        origin: position,
        x: [cos, 0.0, -sin],
        y: [0.0, 1.0, 0.0],
        z: [sin, 0.0, cos],
    }
}

/// The swell a grove stands on: a long slow roll across the frame and a shorter one across it.
///
/// # Invariants
///
/// It is the **one** ground of the scene: the same function draws the floor's heightfield and seats every planting, so
/// a plant cannot stand at a height the floor does not have. A second function would be a second ground.
fn swell(x: f64, z: f64) -> f64 {
    0.34 * (1.3 * x + 0.4).sin() * (1.1 * z).cos()
}

/// The same swell as a seating function, which takes a plan position in the plant's own units.
fn seated_swell(plan: [f32; 2]) -> f32 {
    swell(f64::from(plan[0]), f64::from(plan[1])) as f32
}
/// The ground cover's grid: how many tufts across, and how many deep.
const GROVE_TUFTS: (usize, usize) = (14, 10);

/// How far the ground cover reaches, in metres, across and deep: wider than the plantings and inside the floor.
const GROVE_TUFT_SPAN: (f32, f32) = (3.4, 2.4);

/// The ground cover: a jittered grid of tufts, each from its own seed, so no two cells are the same plant.
///
/// # Invariants
///
/// **The scatter is drawn from the cell's own index**, so the meadow is the same meadow every time it is built and a
/// picture of it can be compared with the next one: a hash of the index seeds the jitter, the tuft's own seed and the
/// turn, in that order.
fn grove_tufts() -> Vec<Planting> {
    let mut plantings = Vec::new();
    for column in 0..GROVE_TUFTS.0 {
        for row in 0..GROVE_TUFTS.1 {
            let index = column * GROVE_TUFTS.1 + row;
            let mut rng = vegetation::rng::Rng::new(7_000 + index as u64);
            let across = GROVE_TUFT_SPAN.0 / GROVE_TUFTS.0 as f32;
            let deep = GROVE_TUFT_SPAN.1 / GROVE_TUFTS.1 as f32;
            let x = -0.5 * GROVE_TUFT_SPAN.0 + across * (column as f32 + 0.2 + 0.6 * rng.unit());
            let z = -0.5 * GROVE_TUFT_SPAN.1 + deep * (row as f32 + 0.2 + 0.6 * rng.unit());
            let turn = std::f32::consts::TAU * rng.unit();
            plantings.push(
                Planting::new(vegetation::derive::tuft(9_000 + index as u64)).at(
                    vegetation::ground::seat(planted_at([x, 0.0, z], turn), seated_swell),
                ),
            );
        }
    }
    plantings
}
/// The six plantings a grove is, in the order their ids are numbered: the culms first, then the undergrowth.
fn grove_plantings() -> Vec<Planting> {
    // **Every planting is seated on the scene's own ground**, so the grove is on its swell rather than at zero; the
    // ground is an authored function (`vegetation::ground`), and it is the one the floor is drawn from.
    let place = |position: [f32; 3], turn: f32| {
        vegetation::ground::seat(planted_at(position, turn), seated_swell)
    };
    let mut plantings = Vec::new();
    for (seed, (position, turn)) in GROVE_CULMS.iter().zip(GROVE_CULM_PLACES.iter()) {
        plantings.push(Planting::new(bamboo(*seed)).at(place(*position, *turn)));
    }
    for (seed, (position, turn)) in GROVE_SHRUBS.iter().zip(GROVE_SHRUB_PLACES.iter()) {
        plantings.push(Planting::new(vegetation::derive::shrub(*seed)).at(place(*position, *turn)));
    }
    // The ground cover last, so the ids an author names stay the small ones.
    plantings.extend(grove_tufts());
    plantings
}

/// Resolves a grove frame's `authored_id` into the planting and the handle it names.
///
/// # Invariants
///
/// **The id is split the way it was written**: the planting index is the id's quotient by [`PLANTING_STRIDE`] and
/// the node is its remainder, so a frame answers *which plant* and *which node* with one number and no table. An id
/// whose planting or node does not exist answers `None` rather than a neighbouring plant's node.
pub fn grove_handle_of(id: u32) -> Option<String> {
    planting_handle(&grove_plantings(), id)
}

/// Resolves an id in a scene of plantings: the quotient names the planting and the remainder the node.
///
/// # Invariants
///
/// **One split for every scene of plantings**, which is what makes a pair comparable: the same division answers the
/// grove's six plants and a demo's two, so a pixel's answer and a rectangle's agree across every scene that draws more
/// than one plant. A planting with **no rows at all** still costs its stride, so removing a plant never renumbers the
/// ones after it.
pub(crate) fn planting_handle(plantings: &[Planting], id: u32) -> Option<String> {
    if id == crate::identity::NOT_AUTHORED {
        return None;
    }
    let index = (id / PLANTING_STRIDE) as usize;
    let planting = plantings.get(index)?;
    let ids = node_ids_from(
        &planting.species.plant,
        LEAF_BASE_ID + index as u32 * PLANTING_STRIDE,
    );
    ids.iter()
        .find(|(_, held)| **held == id)
        .map(|(handle, _)| format!("planting {index}, {handle}"))
}

/// Builds the grove scene: five shrubs from five seeds, each turned to its own place.
pub fn grove_picture() -> Value {
    plant_scene(&grove_plantings(), &GROVE_FRAMING, GROVE, &[])
}

/// Builds the bamboo scene: one camera, a key and a fill, a floor, the plant's wood and its blade.
pub fn bamboo_picture() -> Value {
    plant_scene(&[Planting::new(bamboo(SEED))], &STILL, SCENE, &[])
}

/// The same plant under a lateral wind, drawn beside it: a field bends spines, and a picture is the only place
/// the bend is visible as a shape rather than as a number.
pub fn windy_picture() -> Value {
    let species = bamboo(SEED).with_fields(vec![Field::light([1.0, 0.0, 0.0], 0.12)]);
    plant_scene(&[Planting::new(species)], &STILL, WINDY, &[])
}

/// The same plant fitted to a crown: `trim` writes length nudges until each tip is on the envelope, so the frame
/// shows one tree with shorter boughs rather than a different tree.
pub fn crowned_picture() -> Value {
    let mut species = bamboo(SEED);
    let crown = Envelope::radial(&[(0.0, 0.55), (0.5, 0.45), (1.0, 0.35)]);
    trim(&mut species.plant, &crown);
    plant_scene(&[Planting::new(species)], &STILL, CROWNED, &[])
}

/// The same plant with a knot at the base of every bough: a decorator is a template and three numbers, and the
/// frame shows the three numbers on the bark rather than in a table.
pub fn knots_picture() -> Value {
    let mut species = bamboo(SEED);
    // The knot is the species' own ornament: a seat names it by index, so the template travels with the
    // plant that carries the seats.
    species.decorators = vec![knot()];
    let seats = knots(&species);
    plant_scene(
        &[Planting::new(species).with_seats(seats)],
        &STILL,
        KNOTTED,
        &[],
    )
}

/// The name the segmentation view is built and rendered under.
pub const SEGMENTATION: &str = "segmentation";

/// The one function a segmentation view needs: a colour per element id.
///
/// # Invariants
///
/// **The colour is a function of the id and of nothing else**, so two elements that share an id share a colour — a
/// leaf and the twig it grew from are one element — and neighbouring ids are a golden angle apart, which is what
/// makes them distinguishable without a table that would be a second place the ids are numbered.
const SEGMENTATION_HELPERS: &str = r#"
fn element_colour(id: f32) -> vec3<f32> {
    if id < 0.5 {
        return vec3<f32>(0.10);
    }
    let hue = fract(id * 0.6180339887) * 6.0;
    let sector = i32(floor(hue)) % 6;
    let fraction = hue - floor(hue);
    let low = 0.22;
    let rising = mix(low, 1.0, fraction);
    let falling = mix(1.0, low, fraction);
    var colour = vec3<f32>(1.0, rising, low);
    if sector == 1 {
        colour = vec3<f32>(falling, 1.0, low);
    } else if sector == 2 {
        colour = vec3<f32>(low, 1.0, rising);
    } else if sector == 3 {
        colour = vec3<f32>(low, falling, 1.0);
    } else if sector == 4 {
        colour = vec3<f32>(rising, low, 1.0);
    } else if sector == 5 {
        colour = vec3<f32>(1.0, low, falling);
    }
    return colour;
}
"#;

/// What a segmentation view's plant surface writes: the element's colour, whatever the light did.
const SEGMENTATION_BODY: &str = r#"
    color = element_colour(in.authored_id);
"#;

/// What a segmentation view's ground writes: the reserved zero's own grey, unlit, so the subject is the subject.
const SEGMENTATION_GROUND: &str = r#"
    color = vec3<f32>(0.10);
"#;

/// Builds the segmentation view: the same plant, every element in a colour of its own.
///
/// # What this frame is for
///
/// Every other plant scene answers *"what does it look like"*; this one answers *"what is this"*. Its surface body
/// throws the lighting away and writes a colour derived from the point's own `authored_id`, so one node is one
/// colour, a leaf wears the colour of the twig it grew from, and [`crate::locate`]'s two answers can be checked
/// against what the frame shows rather than imagined from a number.
pub fn segmentation_picture() -> Value {
    plant_scene_with(
        &[Planting::new(bamboo(SEED))],
        &STILL,
        SEGMENTATION,
        &View {
            plant: SEGMENTATION_BODY,
            ground: SEGMENTATION_GROUND,
            helpers: SEGMENTATION_HELPERS,
            grade: SEGMENTATION_GRADE,
        },
        &Lighting::default(),
        &[],
    )
}

/// What a segmentation view's post stage does: an encode and nothing else.
///
/// # Invariants
///
/// **A segmentation frame is not tone-mapped**, because its colours are ids: a Reinhard curve and an S-curve would
/// make two neighbouring elements look alike, and the one thing the frame exists to answer is which element a pixel
/// is. The sky is still the scene's own map, so the frame shows where the plant meets its ground.
const SEGMENTATION_GRADE: &str = r#"
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    color = pow(color, vec3<f32>(0.4545));
"#;
/// The name the hand's work is drawn under: one plant, six edits, and nothing authored twice.
pub const SCULPTED: &str = "sculpted";

/// The plant the hand worked on, drawn: a bake, a thickness, a cut, a slide, a graft and a knot.
pub fn sculpted_picture() -> Value {
    let mut sculpted = vegetation::derive::sculpted(SEED);
    // The seat the hand placed names template 0, so the species carries the knot it is.
    sculpted.species.decorators = vec![knot()];
    plant_scene(
        &[Planting::new(sculpted.species).with_seats(sculpted.seats)],
        &STILL,
        SCULPTED,
        &[],
    )
}

/// Resolves a sculpted frame's `authored_id` into the handle that names the node.
///
/// # Invariants
///
/// The table is **the sculpted plant's own walk**, not the plain bamboo's: the graft added a node, so every id after
/// the second bough's children is one higher, and a table built from `bamboo(SEED)` would answer with the wrong node
/// from there on. That is the reason this resolver exists rather than reusing [`handle_of`].
pub fn sculpted_handle_of(id: u32) -> Option<String> {
    let sculpted = vegetation::derive::sculpted(SEED);
    node_ids(&sculpted.species.plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}
/// Builds one scene of plants: the camera, the lights, the floor, every planting's wood and the one blade mesh
/// every leaf of every planting instances.
///
/// # Invariants
///
/// - **One blade mesh, and one material per leaf group of every planting.** The emitter groups by shader, mesh and
///   images and not by material, so five shrubs' leaves are **one** pass over one instance buffer: a planting is
///   not instanced (its wood is its own) and its *leaves* are, which is the whole of the design's instancing claim
///   and the reason the grove costs one blade mesh and no more.
/// - **A planting's wood is one mesh, baked in the world**: a plant of five hundred solids is one draw, and its
///   corners carry the ids its own planting numbered ([`node_ids_from`]), so two plantings never write one id.
/// - **A second planting's labels carry its index**, because a label names one instance in the whole scene.
pub fn plant_scene(plantings: &[Planting], framing: &Framing, name: &str, props: &[Prop]) -> Value {
    plant_scene_with(
        plantings,
        framing,
        name,
        &View::PHOTOGRAPH,
        &Lighting::default(),
        props,
    )
}

/// The same builder under a lighting of the scene's own: a species' epic frame states its hour and its air
/// here, and a frame that does not care says nothing and gets the day every vegetation scene has had.
pub fn plant_scene_lit(
    plantings: &[Planting],
    framing: &Framing,
    name: &str,
    lighting: &Lighting,
    props: &[Prop],
) -> Value {
    plant_scene_with(plantings, framing, name, &View::PHOTOGRAPH, lighting, props)
}

/// Something in a scene that is not a plant: a wall, a kerb, a bench.
///
/// # Invariants
///
/// A prop is **world geometry with no identity**: it carries no `authored_id`, so its pixels are the reserved zero and
/// `--region` calls them the ground's own — which is what a wall is to a plant, the world it grows against rather than
/// a thing whose node an author nudges. It draws on the ground's three stages, so a prop cannot be a plant by accident
/// and a plant cannot be a prop.
#[derive(Debug, Clone)]
pub struct Prop {
    /// Its name, which its three materials and its mesh are tagged with.
    pub name: String,
    /// Its points, in its own coordinates, on the shaded layout every gallery generator writes.
    pub points: PackedList,
    /// The colour it is drawn in.
    pub tint: [f64; 3],
    /// Its roughness.
    pub roughness: f64,
    /// Where it stands: a prop is not turned and not scaled, because a wall that needs to be is a wall whose own
    /// generator should have been asked for the other one.
    pub at: [f64; 3],
}
/// The bodies a view writes for itself: where a frame that is not a photograph departs from one.
///
/// # Invariants
///
/// **Every body is spliced, not composed**: an empty body is a splice that declares nothing, so the photograph and a
/// view that writes all four run the same builder and differ only in four strings — a view cannot change the
/// geometry, the lights or the ids, which is what makes two views of one plant comparable.
#[derive(Debug, Clone, Copy)]
struct View {
    /// The plant's surface body, spliced after the lighting has been added and before the fragment writes.
    plant: &'static str,
    /// The ground's surface body, in the same place.
    ground: &'static str,
    /// Helpers both bodies may call, appended to the shader's own helpers.
    helpers: &'static str,
    /// The post stage's body; empty means the scene's own grade.
    grade: &'static str,
}

impl View {
    /// The photograph: the default lighting, and the grade the plant scenes have always had.
    const PHOTOGRAPH: View = View {
        plant: "",
        ground: "",
        helpers: "",
        grade: "",
    };
}

/// The same builder with the bodies of a view open, which is what a frame other than a photograph needs.
///
/// # Invariants
///
/// **Every body is spliced where the shader puts it, and an empty body declares nothing**, so the photograph and a
/// view that writes all of them run one builder and differ in four strings — a view cannot change the geometry, the
/// lights or the ids, which is what makes two views of one plant comparable. `plant` and `ground` are spliced after
/// the lighting has been added and before the fragment writes its outputs, where `color` is the accumulation and
/// `in` is the varying struct; `helpers` is appended to the shader's own; `grade` replaces the post stage's body.
fn plant_scene_with(
    plantings: &[Planting],
    framing: &Framing,
    name: &str,
    view: &View,
    lighting: &Lighting,
    props: &[Prop],
) -> Value {
    // **Two point schemas, and they are the two identities**: every authored mesh carries `authored_id`,
    // the ground carries none and writes the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let ground = not_authored_extras();

    // **One mesh per distinct (geometry, skin) pair, and one rule for all three kinds.** The wood of every
    // planting, every blade template and every decorator template are keyed the same way, so two plantings
    // of one species share one blade mesh and one wood mesh, and two species in one frame cannot share a
    // tint by accident — the skin is part of the key. A scene whose plantings agree on every skin is one
    // mesh of each kind, which is the document this builder wrote before palettes existed.
    let mut wood_skins: Vec<Skin> = Vec::new();
    let mut wood_of: Vec<usize> = Vec::new();
    for planting in plantings {
        let index = match wood_skins
            .iter()
            .position(|held| *held == planting.palette.wood)
        {
            Some(index) => index,
            None => {
                wood_skins.push(planting.palette.wood);
                wood_skins.len() - 1
            }
        };
        wood_of.push(index);
    }
    let mut blades: Vec<(&LeafTemplate, Skin)> = Vec::new();
    let mut blade_of: Vec<Vec<usize>> = Vec::new();
    for planting in plantings {
        // **A planting may have no template at all**: a tuft of grass has no leaves, because its blades are
        // its wood. It draws nothing from a blade mesh, and the leaf loops below are empty for it.
        let row = planting
            .species
            .templates
            .iter()
            .enumerate()
            .map(|(which, template)| {
                let key = (template, planting.palette.leaf(which));
                match blades.iter().position(|held| *held == key) {
                    Some(index) => index,
                    None => {
                        blades.push(key);
                        blades.len() - 1
                    }
                }
            })
            .collect();
        blade_of.push(row);
    }
    // A decorator mesh exists only when a seat asks for one: a scene with no seats is the same document
    // it was before decorators existed, down to the frame it writes.
    let mut knots: Vec<(&Template, Skin)> = Vec::new();
    let mut knot_of: Vec<Vec<Option<usize>>> = plantings
        .iter()
        .map(|planting| vec![None; planting.species.decorators.len()])
        .collect();
    for (which_planting, planting) in plantings.iter().enumerate() {
        for seat in &planting.seats {
            if knot_of[which_planting]
                .get(seat.template)
                .is_some_and(Option::is_some)
            {
                continue;
            }
            let Some(template) = planting.species.decorators.get(seat.template) else {
                continue;
            };
            let key = (template, planting.palette.seat(seat.template));
            let index = match knots.iter().position(|held| *held == key) {
                Some(index) => index,
                None => {
                    knots.push(key);
                    knots.len() - 1
                }
            };
            knot_of[which_planting][seat.template] = Some(index);
        }
    }
    let one_planting = plantings.len() == 1;
    let tagged = |index: usize, label: &str| -> String {
        if one_planting {
            label.to_owned()
        } else {
            format!("{label}-{index}")
        }
    };
    let wood_label = |index: usize| -> String {
        if wood_skins.len() == 1 {
            "culm".to_owned()
        } else {
            format!("culm-{index}")
        }
    };
    let blade_label = |index: usize| -> String {
        if blades.len() == 1 {
            "leaf".to_owned()
        } else {
            format!("leaf-{index}")
        }
    };
    let knot_label = |index: usize| -> String {
        if knots.len() == 1 {
            "knot".to_owned()
        } else {
            format!("knot-{index}")
        }
    };
    // The label a knot mesh's caster and depther rows carry: the mesh's own when there is one of them, which
    // is the label this builder wrote before a species could carry two ornaments.
    let knot_stage_label = |index: usize, mesh: usize| -> String {
        if knots.len() == 1 {
            tagged(index, "knot")
        } else {
            tagged(index, &format!("knot-{mesh}"))
        }
    };

    // Everything a planting contributes, read once: its expansion, its ids, and its decorators in the world.
    struct Drawn {
        expansion: Expansion,
        ids: std::collections::BTreeMap<Handle, u32>,
        decorators: Vec<Placed>,
        placement: Placement,
    }
    let drawn: Vec<Drawn> = plantings
        .iter()
        .enumerate()
        .map(|(index, planting)| Drawn {
            expansion: expansion(&planting.species, planting.detail),
            ids: node_ids_from(
                &planting.species.plant,
                LEAF_BASE_ID + index as u32 * PLANTING_STRIDE,
            ),
            decorators: placed(&planting.species.plant, &planting.seats)
                .into_iter()
                .map(|instance| Placed {
                    placement: planting.placement.then(&instance.placement),
                    ..instance
                })
                .collect(),
            placement: planting.placement,
        })
        .collect();

    // **One copy of the air**: the shader's helpers and the environment map are the same struct's numbers, so the
    // rasterised sky and the traced one cannot drift apart. A view with a body of its own appends its helpers here,
    // so the lighting stage and the post stage still read one string.
    let helpers = if view.helpers.is_empty() {
        lighting.sky.wgsl()
    } else {
        format!("{}\n{}", lighting.sky.wgsl(), view.helpers)
    };

    let mut materials: Vec<Value> = Vec::new();
    // **One wood material set per wood mesh**, in the order the meshes are drawn: a caster, a depther and a
    // shader over one skin.
    for (index, skin) in wood_skins.iter().enumerate() {
        let label = wood_label(index);
        for (stage, shader) in [
            ("cast", "plant_shadowing"),
            ("depth", "plant_depthing"),
            ("shade", "plant_lighting"),
        ] {
            materials.push(mesh_material(
                &format!("{label}-{stage}"),
                shader,
                // **The bole keeps the gallery's default**, because its casters are metres away and
                // the numbers the shadow test biases by are written for exactly that distance.
                plant_row(
                    skin.value(),
                    (stage == "shade").then_some(crate::kit::DEFAULT_SHADOW_BIAS_SCALE),
                ),
            ));
        }
    }
    // The blade's two caster rows share one material: a caster carries no id, so a material per group
    // would be six rows that differ only in a field no stage of theirs reads.
    let caster = Skin::new(LEAF_TINT, 0.62);
    materials.push(mesh_material(
        "leaf-cast",
        "plant_shadowing",
        caster.value(),
    ));
    materials.push(mesh_material(
        "leaf-depth",
        "plant_depthing",
        caster.value(),
    ));
    // **One material per leaf group, and the only things that differ are the offset and the skin.** The
    // blade is one mesh drawn by every leaf, so a leaf's own corners carry the blade's base id and the node
    // it hangs from is its row's offset: `authored_offset = node - LEAF_BASE_ID`. The groups still draw in
    // **one** pass, because the emitter groups by shader, mesh and images and not by material.
    for (index, planting) in drawn.iter().enumerate() {
        for (group, leaves) in planting.expansion.leaves.iter().enumerate() {
            let node = planting.ids[&leaves.node];
            materials.push(mesh_material(
                &tagged(index, &format!("leaf-shade-{group}")),
                "plant_lighting",
                plant_row(
                    offset_by(
                        plantings[index].palette.leaf(leaves.template).value(),
                        f64::from(node - LEAF_BASE_ID),
                    ),
                    Some(PLANT_SHADOW_BIAS_SCALE),
                ),
            ));
        }
    }
    // A knot is drawn by every seat of its template, so its corners carry the blade's base id and each seat's
    // row carries the node it sits on: one mesh, one material per seat, and the seats still draw in one pass.
    for (index, planting) in drawn.iter().enumerate() {
        if planting.decorators.is_empty() {
            continue;
        }
        for mesh in knot_of[index].iter().flatten() {
            let label = knot_stage_label(index, *mesh);
            for (stage, shader) in [("cast", "plant_shadowing"), ("depth", "plant_depthing")] {
                materials.push(mesh_material(
                    &format!("{label}-{stage}"),
                    shader,
                    knots[*mesh].1.value(),
                ));
            }
        }
        for (seat, instance) in planting.decorators.iter().enumerate() {
            let node = planting.ids[&instance.node];
            materials.push(mesh_material(
                &tagged(index, &format!("knot-shade-{seat}")),
                "plant_lighting",
                plant_row(
                    offset_by(
                        plantings[index].palette.seat(instance.template).value(),
                        f64::from(node - LEAF_BASE_ID),
                    ),
                    Some(PLANT_SHADOW_BIAS_SCALE),
                ),
            ));
        }
    }
    let earth = lighting.floor.value();
    for (stage, shader) in [
        ("cast", "ground_shadowing"),
        ("depth", "ground_depthing"),
        ("shade", "ground_lighting"),
    ] {
        materials.push(mesh_material(
            &format!("floor-{stage}"),
            shader,
            earth.clone(),
        ));
        // **A prop draws on the ground's stages**, so it carries no identity and cannot be mistaken for a plant.
        for prop in props {
            materials.push(mesh_material(
                &format!("{}-{stage}", prop.name),
                shader,
                surface(prop.tint, 0.0, prop.roughness, [0.0; 3]),
            ));
        }
    }

    // **One leaf placement per leaf instance, and the label says which group and which rank**: a label
    // names one instance in the *whole* scene, so the same leaf is drawn in three passes under three
    // labels.
    let placed_rows = |stage: &str| {
        let fixed = |label: &str, material: String| {
            instance(
                format!("{label}-{stage}"),
                label,
                [0.0; 3],
                [0.0; 3],
                [1.0; 3],
                &material,
            )
        };
        // The wood is baked in world coordinates and the floor is a plane, so neither needs a transform of its
        // own. The blade has none — every leaf places it, and a placement at the origin would be a leaf growing
        // out of the ground.
        let mut placements: Vec<Value> = wood_skins
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let label = wood_label(index);
                fixed(&label, format!("{label}-{stage}"))
            })
            .collect();
        placements.push(fixed("floor", format!("floor-{stage}")));
        for prop in props {
            placements.push(instance(
                format!("{}-{stage}", prop.name),
                &prop.name,
                prop.at,
                [0.0; 3],
                [1.0; 3],
                &format!("{}-{stage}", prop.name),
            ));
        }
        for (index, planting) in drawn.iter().enumerate() {
            for (group, leaves) in planting.expansion.leaves.iter().enumerate() {
                let material = if stage == "shade" {
                    tagged(index, &format!("leaf-shade-{group}"))
                } else {
                    format!("leaf-{stage}")
                };
                let mesh = blade_label(blade_of[index][leaves.template]);
                for leaf in &leaves.leaves {
                    let world = planting.placement.then(&leaf.placement);
                    let (translation, rotation, scale) =
                        crate::vegetation_instancing::standing_at(&world, leaf.scale);
                    placements.push(instance(
                        format!("{}-{stage}-{group}-{}", tagged(index, "leaf"), leaf.index),
                        &mesh,
                        translation,
                        rotation,
                        scale,
                        &material,
                    ));
                }
            }
        }
        // **One placement per decorator, and its frame is the seat's**: a knot has no orientation of its own to
        // choose, which is why the same conversion a blade uses converts it. A seat whose template the species
        // does not carry draws nothing, which is the same refusal [`placed`] makes of a pruned node.
        for (index, planting) in drawn.iter().enumerate() {
            for (seat, decorator) in planting.decorators.iter().enumerate() {
                let Some(mesh) = knot_of[index].get(decorator.template).copied().flatten() else {
                    continue;
                };
                let material = if stage == "shade" {
                    tagged(index, &format!("knot-shade-{seat}"))
                } else {
                    format!("{}-{stage}", knot_stage_label(index, mesh))
                };
                let (translation, rotation, scale) =
                    crate::vegetation_instancing::seated(decorator);
                placements.push(instance(
                    format!("{}-{stage}-{seat}", tagged(index, "knot")),
                    &knot_label(mesh),
                    translation,
                    rotation,
                    scale,
                    &material,
                ));
            }
        }
        placements
    };

    Value::structure([
        ("name".to_owned(), Value::String(name.to_owned())),
        (
            "frame".to_owned(),
            Value::structure([
                ("width".to_owned(), Value::Int(framing.width)),
                ("height".to_owned(), Value::Int(framing.height)),
            ]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed(
                framing.eye,
                framing.target,
                framing.field_of_view,
                framing.near,
                framing.far,
            )]),
        ),
        (
            // The sky as an **image**, which is what lets this scene render on a backend that has no
            // shader stage of its own: a path tracer draws its world from the map, and the grade
            // reads the same map for the pixels no geometry covered. Latlong, because that is the
            // projection a path tracer reads an environment in.
            "images".to_owned(),
            Value::List(vec![image(
                "air",
                512,
                256,
                lighting.sky.environment_pixels(512, 256),
                Projection::Latlong,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "key",
                    lighting.key_position,
                    lighting.key_colour,
                    flux_for_irradiance(lighting.key_irradiance, lighting.key_distance),
                    lighting.key_extent,
                    lighting.key_reach,
                ),
                light(
                    "fill",
                    lighting.fill_position,
                    lighting.fill_colour,
                    flux_for_irradiance(lighting.fill_irradiance, lighting.fill_distance),
                    lighting.key_extent,
                ),
                // The day itself: the ambient every shadowed leaf is lit by *and* the sky the frame's
                // background is drawn from, which is one number read by both backends.
                environment_light("air", "air", [1.0; 3], lighting.sky_radiance),
            ]),
        ),
        (
            "shaders".to_owned(),
            Value::List({
                let mut shaders = stage_set_over_schema(
                    "plant",
                    view.plant,
                    &helpers,
                    &point,
                    &extras,
                    &plant_custom_schema(),
                );
                shaders.extend(stage_set(
                    "ground",
                    view.ground,
                    &helpers,
                    &ground.point_schema(),
                    &ground,
                ));
                let built = GRADE.replace("{HAZE}", &format!("{:.6}", lighting.haze));
                let grade = if view.grade.is_empty() {
                    built.as_str()
                } else {
                    view.grade
                };
                shaders.push(post_shader(grade, &helpers, Some(Projection::Latlong)));
                shaders
            }),
        ),
        // The one image this scene writes beside the frame: which node each pixel shows.
        ("outputs".to_owned(), outputs_field()),
        ("meshes".to_owned(), {
            // **One wood mesh per wood skin, and one mesh for the whole scene when the plantings agree on
            // one.** A mesh per plant would be a pass per plant, because the emitter groups by mesh; the ids
            // are in the corners, so merging the geometry does not merge the names — a pixel still answers
            // which planting and which node.
            let mut meshes: Vec<Value> = wood_skins
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    mesh(
                        &wood_label(index),
                        "Triangles",
                        point.clone(),
                        crate::vegetation_mesh::wood_points_of(
                            drawn
                                .iter()
                                .zip(wood_of.iter())
                                .filter(|(_, held)| **held == index)
                                .map(|(planting, _)| {
                                    (
                                        planting.expansion.wood.as_slice(),
                                        &planting.ids,
                                        &planting.placement,
                                    )
                                }),
                        ),
                    )
                })
                .collect();
            for (index, (template, _)) in blades.iter().enumerate() {
                meshes.push(mesh(
                    &blade_label(index),
                    "Triangles",
                    point.clone(),
                    template_points(template),
                ));
            }
            // **A decorator mesh only when a seat asks for one**: a scene with no seats is the same document
            // it was before decorators existed, down to the frame it writes.
            for (index, (template, _)) in knots.iter().enumerate() {
                meshes.push(mesh(
                    &knot_label(index),
                    "Triangles",
                    point.clone(),
                    crate::vegetation_mesh::decorator_points(template),
                ));
            }
            // **The floor is the ground the plantings were seated on**: a plane, or the same function as a
            // heightfield. A second function would be a second ground, and a plant could stand where the floor is not.
            let floor = match framing.ground {
                Ground::Plane => plane_facing(framing.floor, 0.0, true),
                Ground::Fielded { height, divisions } => {
                    crate::geometry::heightfield(divisions, framing.floor, height)
                }
            };
            meshes.push(mesh("floor", "Triangles", ground.point_schema(), floor));
            for prop in props {
                meshes.push(mesh(
                    &prop.name,
                    "Triangles",
                    ground.point_schema(),
                    prop.points.clone(),
                ));
            }
            Value::List(meshes)
        }),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "instances".to_owned(),
            instances(vec![
                (Stage::Shadowing, placed_rows("cast")),
                (Stage::Depthing, placed_rows("depth")),
                (Stage::Lighting, placed_rows("shade")),
            ]),
        ),
    ])
}
