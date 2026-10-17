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

use definition::Value;
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
use crate::building_identity::{building_extras, not_authored_extras, outputs_field, stage_set};
use crate::geometry::plane_facing;
use crate::kit::{
    camera_placed, environment_light, flux_for_irradiance, image, instance, instances, light,
    light_with_radius, mesh, mesh_material, offset_by, post_processes, screen_material, surface,
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
        let air = clamp(1.0 - exp(-along_ray * 0.012), 0.0, 0.45);
        color = mix(color, environment_sky(vec3<f32>(ray_through(in.uv).x, 0.02, ray_through(in.uv).z)), air);
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
"#;

/// The plant this scene draws, expanded at the crate's own default dial.
///
/// The dial is [`Detail::DEFAULT`] and not a number written here, because the point of the picture is the
/// plant and not a resolution: a scene that picked its own dial would be a second author of the geometry.
fn expansion(species: &Species) -> Expansion {
    expand(&species.plant, Detail::DEFAULT, &species.fields)
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
}

impl Planting {
    /// A planting standing where it grew.
    pub fn new(species: Species) -> Self {
        Self {
            species,
            placement: Placement::IDENTITY,
            seats: Vec::new(),
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
}

/// How far apart two plantings' id bases are: a plant of more than four thousand nodes is not one a picture draws.
pub const PLANTING_STRIDE: u32 = 4096;

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
fn planted_at(position: [f32; 3], turn: f32) -> Placement {
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
    if id == crate::identity::NOT_AUTHORED {
        return None;
    }
    let index = (id / PLANTING_STRIDE) as usize;
    let plantings = grove_plantings();
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
    plant_scene(&grove_plantings(), &GROVE_FRAMING, GROVE)
}

/// Builds the bamboo scene: one camera, a key and a fill, a floor, the plant's wood and its blade.
pub fn bamboo_picture() -> Value {
    plant_scene(&[Planting::new(bamboo(SEED))], &STILL, SCENE)
}

/// The same plant under a lateral wind, drawn beside it: a field bends spines, and a picture is the only place
/// the bend is visible as a shape rather than as a number.
pub fn windy_picture() -> Value {
    let species = bamboo(SEED).with_fields(vec![Field::light([1.0, 0.0, 0.0], 0.12)]);
    plant_scene(&[Planting::new(species)], &STILL, WINDY)
}

/// The same plant fitted to a crown: `trim` writes length nudges until each tip is on the envelope, so the frame
/// shows one tree with shorter boughs rather than a different tree.
pub fn crowned_picture() -> Value {
    let mut species = bamboo(SEED);
    let crown = Envelope::radial(&[(0.0, 0.55), (0.5, 0.45), (1.0, 0.35)]);
    trim(&mut species.plant, &crown);
    plant_scene(&[Planting::new(species)], &STILL, CROWNED)
}

/// The same plant with a knot at the base of every bough: a decorator is a template and three numbers, and the
/// frame shows the three numbers on the bark rather than in a table.
pub fn knots_picture() -> Value {
    let species = bamboo(SEED);
    let seats = knots(&species);
    plant_scene(&[Planting::new(species).with_seats(seats)], &STILL, KNOTTED)
}

/// The name the hand's work is drawn under: one plant, six edits, and nothing authored twice.
pub const SCULPTED: &str = "sculpted";

/// The plant the hand worked on, drawn: a bake, a thickness, a cut, a slide, a graft and a knot.
pub fn sculpted_picture() -> Value {
    let sculpted = vegetation::derive::sculpted(SEED);
    plant_scene(
        &[Planting::new(sculpted.species).with_seats(sculpted.seats)],
        &STILL,
        SCULPTED,
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
fn plant_scene(plantings: &[Planting], framing: &Framing, name: &str) -> Value {
    // **Two point schemas, and they are the two identities**: every authored mesh carries `authored_id`,
    // the ground carries none and writes the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let ground = not_authored_extras();

    // The blade meshes: one per distinct template, so two plantings of one species share one mesh.
    let mut blades: Vec<&LeafTemplate> = Vec::new();
    let mut blade_of: Vec<Option<usize>> = Vec::new();
    for planting in plantings {
        // **A planting may have no template at all**: a tuft of grass has no leaves, because its blades are its
        // wood. It draws nothing from a blade mesh, and the leaf loops below are empty for it.
        let Some(template) = planting.species.templates.first() else {
            blade_of.push(None);
            continue;
        };
        let index = match blades.iter().position(|held| *held == template) {
            Some(index) => index,
            None => {
                blades.push(template);
                blades.len() - 1
            }
        };
        blade_of.push(Some(index));
    }
    let one_planting = plantings.len() == 1;
    let tagged = |index: usize, label: &str| -> String {
        if one_planting {
            label.to_owned()
        } else {
            format!("{label}-{index}")
        }
    };
    let blade_label = |index: usize| -> String {
        let which = blade_of[index].expect("a planting with leaves has a template");
        if blades.len() == 1 {
            "leaf".to_owned()
        } else {
            format!("leaf-{which}")
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
            expansion: expansion(&planting.species),
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
    // rasterised sky and the traced one cannot drift apart.
    let helpers = atmosphere().wgsl();

    let mut materials = vec![
        mesh_material(
            "culm-cast",
            "plant_shadowing",
            surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
        ),
        mesh_material(
            "culm-depth",
            "plant_depthing",
            surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
        ),
        mesh_material(
            "culm-shade",
            "plant_lighting",
            surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
        ),
        // The blade's two caster rows share one material: a caster carries no id, so a material per group
        // would be six rows that differ only in a field no stage of theirs reads.
        mesh_material(
            "leaf-cast",
            "plant_shadowing",
            surface(LEAF_TINT, 0.0, 0.62, [0.0; 3]),
        ),
        mesh_material(
            "leaf-depth",
            "plant_depthing",
            surface(LEAF_TINT, 0.0, 0.62, [0.0; 3]),
        ),
    ];
    let leaf_body = surface(LEAF_TINT, 0.0, 0.62, [0.0; 3]);
    // **One material per leaf group, and the only thing that differs is the offset.** The blade is one mesh
    // drawn by every leaf, so a leaf's own corners carry the blade's base id and the node it hangs from is
    // its row's offset: `authored_offset = node - LEAF_BASE_ID`. The groups still draw in **one** pass,
    // because the emitter groups by shader, mesh and images and not by material.
    for (index, planting) in drawn.iter().enumerate() {
        for (group, leaves) in planting.expansion.leaves.iter().enumerate() {
            let node = planting.ids[&leaves.node];
            materials.push(mesh_material(
                &tagged(index, &format!("leaf-shade-{group}")),
                "plant_lighting",
                offset_by(leaf_body.clone(), f64::from(node - LEAF_BASE_ID)),
            ));
        }
    }
    // A knot is drawn by every seat of its template, so its corners carry the blade's base id and each seat's row
    // carries the node it sits on: one mesh, one material per seat, and the seats still draw in one pass.
    for (index, planting) in drawn.iter().enumerate() {
        if planting.decorators.is_empty() {
            continue;
        }
        for (stage, shader) in [("cast", "plant_shadowing"), ("depth", "plant_depthing")] {
            materials.push(mesh_material(
                &tagged(index, &format!("knot-{stage}")),
                shader,
                surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
            ));
        }
        for (seat, instance) in planting.decorators.iter().enumerate() {
            let node = planting.ids[&instance.node];
            materials.push(mesh_material(
                &tagged(index, &format!("knot-shade-{seat}")),
                "plant_lighting",
                offset_by(
                    surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
                    f64::from(node - LEAF_BASE_ID),
                ),
            ));
        }
    }
    let earth = surface(FLOOR_TINT, 0.0, 0.94, [0.0; 3]);
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
        let mut placements = vec![
            fixed("culm", format!("culm-{stage}")),
            fixed("floor", format!("floor-{stage}")),
        ];
        for (index, planting) in drawn.iter().enumerate() {
            for (group, leaves) in planting.expansion.leaves.iter().enumerate() {
                let material = if stage == "shade" {
                    tagged(index, &format!("leaf-shade-{group}"))
                } else {
                    format!("leaf-{stage}")
                };
                for leaf in &leaves.leaves {
                    let world = planting.placement.then(&leaf.placement);
                    let (translation, rotation, scale) =
                        crate::vegetation_instancing::standing_at(&world, leaf.scale);
                    placements.push(instance(
                        format!("{}-{stage}-{group}-{}", tagged(index, "leaf"), leaf.index),
                        &blade_label(index),
                        translation,
                        rotation,
                        scale,
                        &material,
                    ));
                }
            }
        }
        // **One placement per decorator, and its frame is the seat's**: a knot has no orientation of its own to
        // choose, which is why the same conversion a blade uses converts it.
        for (index, planting) in drawn.iter().enumerate() {
            for (seat, decorator) in planting.decorators.iter().enumerate() {
                let material = if stage == "shade" {
                    tagged(index, &format!("knot-shade-{seat}"))
                } else {
                    tagged(index, &format!("knot-{stage}"))
                };
                let (translation, rotation, scale) =
                    crate::vegetation_instancing::seated(decorator);
                placements.push(instance(
                    format!("{}-{stage}-{seat}", tagged(index, "knot")),
                    "knot",
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
                atmosphere().environment_pixels(512, 256),
                Projection::Latlong,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "key",
                    KEY_POSITION,
                    KEY_COLOUR,
                    flux_for_irradiance(KEY_IRRADIANCE, KEY_REACH),
                    KEY_EXTENT,
                    KEY_REACH,
                ),
                light(
                    "fill",
                    FILL_POSITION,
                    FILL_COLOUR,
                    flux_for_irradiance(0.7, 3.0),
                    KEY_EXTENT,
                ),
                // The day itself: the ambient every shadowed leaf is lit by *and* the sky the frame's
                // background is drawn from, which is one number read by both backends.
                environment_light("air", "air", [1.0; 3], SKY_RADIANCE),
            ]),
        ),
        (
            "shaders".to_owned(),
            Value::List({
                let mut shaders = stage_set("plant", "", "", &point, &extras);
                shaders.extend(stage_set("ground", "", "", &ground.point_schema(), &ground));
                shaders.push(post_shader(GRADE, &helpers, Some(Projection::Latlong)));
                shaders
            }),
        ),
        // The one image this scene writes beside the frame: which node each pixel shows.
        ("outputs".to_owned(), outputs_field()),
        ("meshes".to_owned(), {
            // **One wood mesh for the whole scene.** A mesh per plant would be a pass per plant, because the emitter
            // groups by mesh; the ids are in the corners, so merging the geometry does not merge the names — a pixel
            // still answers which planting and which node.
            let mut meshes = vec![mesh(
                "culm",
                "Triangles",
                point.clone(),
                crate::vegetation_mesh::wood_points_of(drawn.iter().map(|planting| {
                    (
                        planting.expansion.wood.as_slice(),
                        &planting.ids,
                        &planting.placement,
                    )
                })),
            )];
            for (index, template) in blades.iter().enumerate() {
                let label = if blades.len() == 1 {
                    "leaf".to_owned()
                } else {
                    format!("leaf-{index}")
                };
                meshes.push(mesh(
                    &label,
                    "Triangles",
                    point.clone(),
                    template_points(template),
                ));
            }
            // **A decorator mesh only when a seat asks for one**: a scene with no seats is the same document
            // it was before decorators existed, down to the frame it writes.
            if drawn.iter().any(|planting| !planting.decorators.is_empty()) {
                meshes.push(mesh(
                    "knot",
                    "Triangles",
                    point.clone(),
                    crate::vegetation_mesh::decorator_points(&knot()),
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
