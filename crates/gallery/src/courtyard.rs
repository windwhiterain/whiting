
//!
//! One place, built once: a terraced hillside in the lee of a ridge, a rammed earth farmhouse
//! with a grey tiled roof, a paved yard, a stone pond, a clay-tiled wall with a gate in it,
//! and a bamboo grove standing over all of it. The six moments are that place under six
//! airs — a misty dawn, a bright clouded midday, a wet afternoon, a golden hour, a night with
//! one lamp lit, and a snow — and each of them is a camera as well, because a moment is a
//! fact about the light and a frame is a fact about where the photographer stood.
//!
//! # What makes it read as a place
//!
//! - **The yard is the ground of every frame.** Nothing is placed for one view: the pond sits
//!   where the yard drains, the jar stands where water is drawn, the racks face the sun, the
//!   gate opens onto the path down to the valley. A photograph of a courtyard is a photograph
//!   of someone's morning, and the way to get one is to build the morning rather than the
//!   photograph.
//! - **The bamboo is a grove and not a row.** Culms rise in clusters where the root mat is —
//!   a stand of bamboo spreads by runners and comes up in clumps — and the clumps are the
//!   whole composition: a dark mass in one corner, a lighter one behind the wall, and single
//!   culms leaning over the roof.
//! - **The mountain is air, not geometry.** The far ranges are drawn by the post stage as
//!   ridge lines held in a direction, so they hold still when the camera moves, and the haze
//!   that carries them is the same haze the hills fade into. A range built as geometry would
//!   be a model of a valley; a range built as weather is a valley.
//! - **One air, one sun, told once.** The sky, the haze, the water's mirror and the fog are
//!   written from one [`Moment`], formatted into the WGSL, so the sun in the pond and the sun
//!   in the air cannot drift apart.
//!
//! # A note on the bamboo, which is the hard one
//!
//! A culm is [`geometry::culm`]: a taper with a ringed joint at every node. Its leaves are
//! [`geometry::leaf_spray`]: a ring of tapered blades about a twig end. The grove is those
//! two meshes placed a few hundred times, and what makes it read as bamboo rather than as a
//! field of green sticks is that **the leaves all hang off the top third** and that the culms
//! lean. A bamboo stand is a mass of leaves with light coming through it, and everything the
//! scene does is in service of that one silhouette.

use definition::Value;
use scene::standard::Stage;

use crate::geometry::{
    blade, cuboid, culm, double_blade, fractal_noise, hash01, heightfield, lathe, leaf_spray, lerp,
    masked_heightfield, plane_facing, prism, ridged_noise, sphere, tuft, value_noise,
};
use crate::kit::{
    camera_placed_on_lens, directional, gallery_custom_schema, instance,
    instances, light_with_radius, mesh, mesh_material, post_processes, screen_material,
    shaded_point_schema, surface_reflecting,
};
use crate::shaders::{post_shader};

/// The six moments the place is drawn in.
pub const MOMENTS: [&str; 6] = [
    "courtyard-dawn",
    "courtyard-cloud",
    "courtyard-rain",
    "courtyard-dusk",
    "courtyard-night",
    "courtyard-snow",
];

/// How far the inner terrain reaches from the origin, and how finely it is divided.
///
/// The far hills are the frame's own horizon and the fog eats most of them, so the grid is
/// spent where the eye is: a cell here is three and a third units, which is a hand's width of
/// ground at ten paces and a terrace riser at two hundred. The ranges beyond this are drawn by
/// the post stage rather than modelled, because a mountain 1800 units away that has to survive
/// being resolved at four units a cell is a mountain nobody can afford.
const EXTENT: f64 = 320.0;
const DIVISIONS: usize = 192;

/// The yard: how far the paving reaches, and where the ground it was laid on stops falling in.
///
/// The carve is the whole trick of putting a flat courtyard into a hillside: the land is
/// levelled inside `YARD` and blended back into its own fall over the half width beyond it, so
/// the yard is flat to the eye and has a bank on every side rather than a cliff. A hard edge
/// would be a plane intersecting a heightfield, which draws as a jagged seam and reads as a
/// mistake in the geometry rather than as a bank.
const YARD: [f64; 2] = [13.0, 11.5];
const BANK: f64 = 9.0;

/// Where the house stands: the centre of its footprint, its height, and its half depth.
///
/// It faces south — `+z`, where the gate is — which is the way a house in the northern
/// hemisphere is built: the sun comes round the south side all day, and the north wall is the
/// cold one that gets no window.
const HOUSE: [f64; 3] = [0.0, 0.0, -14.5];
const HOUSE_WIDTH: f64 = 14.4;
const HOUSE_DEPTH: f64 = 8.0;
const WALL_HEIGHT: f64 = 3.4;
const PLINTH: f64 = 0.42;

/// The roof: how far each slope runs from the ridge to the eave, and how far it rises.
///
/// # Why the mesh is built at half these numbers
///
/// A roof is two numbers wide and one number deep, and a heightfield builds a **square** run
/// whose local coordinates are world units. The mesh is therefore one slope's own square, and the
/// instance scales it out to the roof's real width, run and rise.
///
/// **The scale is one number**, and that is the whole of why the mesh is half size: `z` and `y`
/// must be scaled by the same factor or the slope is a different angle from the one the mesh was
/// built at — and the tile body recovers how far down the slope a fragment is by dividing the
/// height it has fallen by `rise / run`, which is only the slope the mesh actually has if the two
/// are equal. The first version of this scaled the width and left the slope alone, and the roof
/// came out a **warped surface** with its eave in the air: a test of three points showed the
/// surface was not planar, which is what a mesh whose two axes are scaled apart is.
const ROOF_SLOPE: f64 = 2.2;
const ROOF_RUN: f64 = 1.95;
const ROOF_RISE: f64 = 1.35;
const ROOF_OVERHANG: f64 = 0.90;
const ROOF_HALF_WIDTH: f64 = HOUSE_WIDTH * 0.5 + ROOF_OVERHANG;

/// The pond in the south-east corner: its centre, how far it reaches, and how deep it is.
const POND: [f64; 2] = [7.2, 4.4];
const POND_REACH: [f64; 2] = [2.7, 2.1];
const POND_DEPTH: f64 = 0.30;

/// Where the gate stands in the south wall.
const GATE: [f64; 2] = [-3.4, 10.6];
const WALL_REACH: [f64; 2] = [13.4, 11.0];
const WALL_HIGH: f64 = 1.95;

/// Where the bamboo stands: the centre of a clump, and the radius it comes up inside.
struct Clump {
    x: f64,
    z: f64,
    radius: f64,
    /// How many culms come up in it.
    culms: usize,
    /// How tall the tallest is.
    height: f64,
    /// How much of the clump's height carries leaves.
    leafy: f64,
}

/// The grove, as clumps rather than as culms.
///
/// # Invariants
///
/// A clump is where a runner stopped, and every culm in it is a culm from that node: the
/// positions are hashes of the clump's index and the culm's, so an unchanged document grows
/// the same grove, and a culm's own height leans on the clump's.
///
/// Nothing stands in the yard or in the house, and the clumps are placed to do three jobs at
/// once: the dark mass in the north-west corner that every frame is composed against, the
/// stand behind the wall on the east that gives the yard its far side, and the few culms on
/// the bank south of the wall that lean into the light.
fn grove() -> Vec<Clump> {
    let mut clumps = Vec::new();
    let seeded: [(f64, f64, f64, usize, f64, f64); 22] = [
        // The north-west corner: the big one, and the one the dawn frame is built on.
        (-17.5, -10.5, 4.6, 15, 12.5, 0.52),
        (-22.0, -4.0, 3.8, 10, 11.0, 0.50),
        (-14.0, -18.0, 3.4, 9, 12.0, 0.46),
        (-24.5, -13.5, 3.0, 7, 10.0, 0.42),
        // Behind the house, over the ridge of its roof.
        (6.0, -20.0, 3.2, 8, 12.0, 0.44),
        (12.5, -16.5, 2.8, 6, 10.5, 0.40),
        (0.0, -23.0, 3.0, 7, 11.5, 0.42),
        (-6.5, -21.0, 2.6, 6, 10.0, 0.40),
        // The east stand, behind the wall: the yard's far side.
        (18.5, -2.0, 3.6, 11, 11.5, 0.50),
        (21.0, 4.5, 3.0, 8, 10.0, 0.46),
        (17.5, -9.0, 2.6, 6, 9.5, 0.40),
        (23.0, -1.0, 2.4, 5, 9.0, 0.38),
        // South of the wall, on the bank: the culms that lean into the low light.
        (-13.0, 16.0, 3.2, 9, 9.5, 0.44),
        (-18.5, 12.0, 2.6, 6, 8.5, 0.40),
        (-8.0, 19.5, 2.4, 5, 8.0, 0.38),
        (12.0, 17.5, 3.0, 8, 9.0, 0.42),
        (18.0, 14.0, 2.6, 6, 8.5, 0.38),
        (-1.0, 21.0, 2.2, 4, 7.5, 0.36),
        // The west bank, below the yard, where the ground falls away.
        (-19.0, 6.0, 2.8, 7, 9.0, 0.40),
        (-22.5, 10.5, 2.2, 4, 8.0, 0.34),
        // Two stands further out, for the frames that look along the valley.
        (-30.0, -20.0, 4.0, 8, 11.0, 0.44),
        (30.0, -14.0, 3.6, 7, 10.5, 0.42),
    ];
    for (x, z, radius, culms, height, leafy) in seeded {
        clumps.push(Clump {
            x,
            z,
            radius,
            culms,
            height,
            leafy,
        });
    }
    clumps
}

/// The time of day one frame is drawn at: every number that makes one moment different.
///
/// # Invariants
///
/// This is the **only** place a colour of the air is written. The sky, the ridge backdrop,
/// the fog, the water's reflection and the surface bodies all read it through [`Tone::wgsl`],
/// so a moment is one decision rather than six that have to agree.
struct Tone {
    /// The sun's direction **from** the scene — the way the light comes from.
    sun: [f64; 3],
    sun_colour: [f64; 3],
    sun_intensity: f64,
    /// The disc and the glow around it, for the sky.
    disc: [f64; 3],
    glow: [f64; 3],
    /// The sky's own gradient.
    zenith: [f64; 3],
    horizon: [f64; 3],
    ground: [f64; 3],
    /// The fill from the other quarter, and how far it carries.
    fill: [f64; 3],
    fill_intensity: f64,
    fill_position: [f64; 3],
    fill_reach: f64,
    fill_shadow: f64,
    /// What the haze scatters towards, at the sun's quarter and away from it.
    haze: [f64; 3],
    haze_sunward: [f64; 3],
    /// How far the ridge lines stand above the horizon, and how many layers.
    ridge_base: f64,
    ridge_layers: i64,
    /// The exposure the grade is written at, and how much grain it carries.
    exposure: f64,
    saturation: f64,
    grain: f64,
    vignette: f64,
    /// A flat wash added to everything, for a wet or a snowy air.
    wash: [f64; 3],
    wash_amount: f64,
    /// How much snow lies on the ground.
    snow: f64,
    /// How much of the sky a **body of water** is allowed to show. One step further than
    /// [`Tone::graze`] for the same reason and with the same failure: a mirror at ten degrees to
    /// the eye is the sky, and a whole courtyard of water at ten degrees is a white frame.
    mirror: f64,
    /// How much of the sky a surface is allowed to show at a grazing angle.
    ///
    /// # Why this is a moment's own number and not a constant
    ///
    /// A grazing reflection is the brightest thing in a photograph and the wrongest thing in a
    /// renderer: Schlick's term reaches one at ninety degrees whether the surface is glass or mud,
    /// so a frame whose surfaces are all seen at ten degrees comes out as one flat sheet of the
    /// sky's own colour. What decides how much of it is real is how much sky there is to reflect,
    /// and that is what a moment is.
    graze: f64,
}

impl Tone {
    /// Returns the WGSL every stage of this moment is written against.
    fn wgsl(&self, lamp: f64, wetness: f64, mist: (f64, f64, f64)) -> String {
        let v = |c: [f64; 3]| {
            format!(
                "vec3<f32>({:.5}, {:.5}, {:.5})",
                c[0], c[1], c[2]
            )
        };
        format!(
            "const SKY_HORIZON: vec3<f32> = {};\n\
             const SKY_ZENITH: vec3<f32> = {};\n\
             const SUN_DIRECTION: vec3<f32> = {};\n\
             const SKY_GLOW: vec3<f32> = {};\n\
             const SKY_DISC: f32 = {:.6};\n\
             const SUN_COLOUR: vec3<f32> = {};\n\
             const HAZE_AWAY: vec3<f32> = {};\n\
             const HAZE_TOWARD: vec3<f32> = {};\n\
             const RIDGE_BASE: f32 = {:.5};\n\
             const RIDGE_LAYERS: i32 = {};\n\
             const GROUND_TINT: vec3<f32> = {};\n\
             const WASH: vec3<f32> = {};\n\
             const WASH_AMOUNT: f32 = {:.4};\n\
             const EXPOSURE: f32 = {:.4};\n\
             const SATURATION: f32 = {:.4};\n\
             const GRAIN: f32 = {:.5};\n\
             const VIGNETTE: f32 = {:.4};\n\
             const AIR_CEILING: f32 = {:.3};\n\
             const AIR_DENSITY: f32 = {:.6};\n\
             const AIR_SCALE_HEIGHT: f32 = {:.3};\n\
             const SNOW: f32 = {:.4};\n\
             const WETNESS: f32 = {:.4};
             const GRAZE: f32 = {:.3};\n             const MIRROR: f32 = {:.3};\n\
             const TILE_LAMBERT: f32 = {:.4};\n\
             const LAMP_POSITION: vec3<f32> = {};\n\
             const LAMP_COLOUR: vec3<f32> = vec3<f32>(1.0, 0.63, 0.28);\n\
             const LAMP_STRENGTH: f32 = {};\n\
             {}\
             {SHARED}\n",
            v(self.horizon),
            v(self.zenith),
            v(self.sun),
            v(self.glow),
            self.disc[0],
            v(self.sun_colour),
            v(self.haze),
            v(self.haze_sunward),
            self.ridge_base,
            self.ridge_layers,
            v(self.ground),
            v(self.wash),
            self.wash_amount,
            self.exposure,
            self.saturation,
            self.grain,
            self.vignette,
            mist.0,
            mist.1,
            mist.2,
            self.snow,
            wetness,
            self.graze,
            self.mirror,
            // A slab seen down a slope of this pitch loses about a third of the sun's own
            // strength. Written here rather than computed in the shader because it is a fact
            // about the roof the scene builds, and the mesh and the body cannot disagree about
            // a number only one of them has.
            1.0 / (1.0 + (ROOF_RISE / ROOF_SLOPE) * (ROOF_RISE / ROOF_SLOPE)).sqrt(),
            v(LAMP),
            lamp,
            roof_constants(),
        )
    }
}

/// The six moments.
fn tone(moment: &str) -> Tone {
    base_tone(moment)
}

/// Returns how bright the lamp is in a moment.
fn lamp_of(moment: &str) -> f64 {
    match moment {
        "courtyard-dawn" | "courtyard-dusk" => 0.30,
        "courtyard-night" => 1.9,
        _ => 0.0,
    }
}

/// Returns how wet the yard is in a moment: a number in zero to one every surface reads.
///
/// # Why it is derived from the sun's own height
///
/// A courtyard is wet all over or it is not wet: the stone, the tiles, the bark and the leaves
/// are one fact about the hour, and a scene where they were six facts is a scene where five of
/// them are wrong. The height of the sun is what actually decides it — a yard wet at dawn has
/// dried by ten — so this is the one number the moment already carries, read as the thing it
/// means.
fn wetness_of(moment: &str) -> f64 {
    let sun = base_tone(moment).sun;
    let rise =
        sun[1] / (sun[0] * sun[0] + sun[1] * sun[1] + sun[2] * sun[2]).sqrt().max(1e-6);
    match moment {
        // A wet afternoon: the rain has just stopped and nothing has dried.
        "courtyard-rain" => 1.0,
        _ => ((0.30 - rise) / 0.36).clamp(0.0, 1.0),
    }
}

/// The mist's own numbers, per moment: the height its ceiling stands at, how thick it is at that
/// ceiling, and how fast it thins above it.
///
/// # Why the ceiling and the density are the two numbers that matter
///
/// A garden mist is not weak fog — it is **strong fog that is shallow**. The optical depth along
/// a near level ray is the density times the whole distance, so a mist dense enough to be seen
/// at all over a kilometre drowns a yard at ten metres: the first attempt here put the ceiling
/// below the eye and reduced the whole world, near and far alike, to one flat wash of the air's
/// own colour. What makes a valley read as a valley is that **the near ground is below the mist
/// and the far range is above it**, and that is a fact about where the layer's top is.
///
/// The slab is entered through an exponentiating density rather than a step, so a ray that
/// skims the ceiling picks up the mist the way a photograph of one shows it: thinned at the edge,
/// solid in the middle.
fn mist_of(moment: &str) -> (f64, f64, f64) {
    match moment {
        // Before sunrise: the layer lies in the low ground and the terrace tops stand out of it.
        "courtyard-dawn" => (6.5, 0.00090, 5.5),
        // A bright day: the air is clear and what is left of it is the far range's own haze.
        "courtyard-cloud" => (70.0, 0.00040, 55.0),
        // Wet: a low even wash with no ceiling to speak of, which is what a raining afternoon is.
        "courtyard-rain" => (26.0, 0.00110, 22.0),
        // Evening: the day's warmth has gone off the ground and the mist is coming down again.
        "courtyard-dusk" => (14.0, 0.00085, 10.0),
        // Night: the air is still and the layer stands in the valley.
        "courtyard-night" => (10.0, 0.00080, 8.0),
        // Snow: a low ceiling that never lifts, and a white ground under it.
        "courtyard-snow" => (30.0, 0.00105, 26.0),
        other => unimplemented!("`{other}` is not a courtyard moment"),
    }
}

/// The stated numbers of one moment.
fn base_tone(moment: &str) -> Tone {
    match moment {
        "courtyard-dawn" => Tone {
            // The sun is down behind the north-west ridge and coming over it: the grove at the
            // yard's north-west corner is between the light and the camera, which is what turns
            // it into a mass of backlit leaves rather than a green wall.
            sun: [-0.46, 0.19, -0.87],
            sun_colour: [1.0, 0.70, 0.46],
            sun_intensity: 2.5,
            disc: [1.55, 1.10, 0.66],
            glow: [0.85, 0.44, 0.22],
            zenith: [0.075, 0.135, 0.260],
            horizon: [0.420, 0.330, 0.255],
            ground: [0.115, 0.125, 0.140],
            fill: [0.50, 0.63, 0.85],
            fill_intensity: 0.60,
            fill_position: [420.0, 340.0, 260.0],
            fill_reach: 1500.0,
            fill_shadow: 0.0,
            haze: [0.330, 0.330, 0.360],
            haze_sunward: [0.98, 0.62, 0.32],
            ridge_base: 0.012,
            ridge_layers: 4,
            exposure: 0.48,
            saturation: 1.22,
            grain: 0.016,
            vignette: 0.30,
            wash: [0.0; 3],
            wash_amount: 0.0,
            snow: 0.0,
            graze: 0.30,
            mirror: 0.80,
        },
        "courtyard-cloud" => Tone {
            // High and away to the south-east: the yard is in the sun, the wall's north face is
            // in its own shade, and the sky is a bright deck of cloud rather than a blue.
            sun: [0.52, 0.62, 0.59],
            sun_colour: [1.0, 0.96, 0.88],
            sun_intensity: 2.35,
            disc: [0.0; 3],
            glow: [1.0, 0.96, 0.88],
            zenith: [0.115, 0.210, 0.430],
            horizon: [0.475, 0.495, 0.530],
            ground: [0.300, 0.300, 0.290],
            fill: [0.62, 0.70, 0.82],
            fill_intensity: 0.75,
            fill_position: [-520.0, 420.0, -640.0],
            fill_reach: 1500.0,
            fill_shadow: 0.0,
            haze: [0.470, 0.495, 0.535],
            haze_sunward: [0.66, 0.64, 0.62],
            ridge_base: 0.024,
            ridge_layers: 4,
            exposure: 0.52,
            saturation: 1.16,
            grain: 0.013,
            vignette: 0.26,
            wash: [0.0; 3],
            wash_amount: 0.0,
            snow: 0.0,
            graze: 0.55,
            mirror: 1.00,
        },
        "courtyard-rain" => Tone {
            // No sun at all: a wet afternoon under a low uniform sky, which is the light a
            // whitewashed wall and a wet stone are actually best at. The sun's own strength is
            // nearly nothing and the fill from overhead is everything, so a surface's own
            // colour is what the frame is about.
            sun: [-0.22, -0.94, 0.26],
            sun_colour: [0.62, 0.66, 0.72],
            sun_intensity: 0.75,
            disc: [0.0; 3],
            glow: [0.72, 0.76, 0.82],
            zenith: [0.300, 0.320, 0.350],
            horizon: [0.560, 0.575, 0.590],
            ground: [0.300, 0.300, 0.300],
            fill: [0.70, 0.73, 0.78],
            fill_intensity: 1.05,
            fill_position: [80.0, 620.0, 60.0],
            fill_reach: 1500.0,
            fill_shadow: 0.0,
            haze: [0.520, 0.540, 0.560],
            haze_sunward: [0.570, 0.585, 0.600],
            ridge_base: 0.010,
            ridge_layers: 5,
            exposure: 0.58,
            saturation: 1.05,
            grain: 0.015,
            vignette: 0.30,
            wash: [0.045, 0.050, 0.055],
            wash_amount: 0.55,
            snow: 0.0,
            graze: 0.35,
            mirror: 0.85,
        },
        "courtyard-dusk" => Tone {
            // The sun has gone off the yard and is lighting the underside of the cloud, so the
            // light that reaches the place is warm, broad and coming from behind the camera at
            // an angle, and the sky it is measured against is the brightest thing in the frame.
            sun: [-0.74, 0.13, 0.66],
            sun_colour: [1.0, 0.58, 0.30],
            sun_intensity: 2.6,
            disc: [1.85, 0.92, 0.42],
            glow: [1.0, 0.42, 0.16],
            zenith: [0.090, 0.110, 0.230],
            horizon: [0.620, 0.330, 0.170],
            ground: [0.095, 0.085, 0.090],
            fill: [0.44, 0.47, 0.70],
            fill_intensity: 0.45,
            fill_position: [700.0, 300.0, -600.0],
            fill_reach: 1500.0,
            fill_shadow: 0.0,
            haze: [0.470, 0.380, 0.330],
            haze_sunward: [1.00, 0.52, 0.26],
            ridge_base: 0.014,
            ridge_layers: 4,
            exposure: 0.50,
            saturation: 1.26,
            grain: 0.016,
            vignette: 0.34,
            wash: [0.0; 3],
            wash_amount: 0.0,
            snow: 0.0,
            graze: 0.22,
            mirror: 0.30,
        },
        "courtyard-night" => Tone {
            // A moon high and to the south-west, so the yard has a light and the bamboo has
            // shadows; the sky's own light is small, and the lamp does the rest.
            sun: [-0.40, 0.58, 0.71],
            sun_colour: [0.42, 0.52, 0.82],
            sun_intensity: 0.16,
            disc: [0.0; 3],
            glow: [0.62, 0.72, 0.95],
            zenith: [0.020, 0.030, 0.070],
            horizon: [0.055, 0.070, 0.115],
            ground: [0.012, 0.014, 0.020],
            fill: [0.10, 0.14, 0.26],
            fill_intensity: 0.05,
            fill_position: [420.0, 520.0, -430.0],
            fill_reach: 1500.0,
            fill_shadow: 0.0,
            haze: [0.070, 0.085, 0.130],
            haze_sunward: [0.150, 0.170, 0.240],
            ridge_base: 0.010,
            ridge_layers: 4,
            exposure: 0.17,
            saturation: 1.14,
            grain: 0.030,
            vignette: 0.42,
            wash: [0.0; 3],
            wash_amount: 0.0,
            snow: 0.0,
            graze: 0.22,
            mirror: 0.30,
        },
        "courtyard-snow" => Tone {
            // Snow light: an overcast sky that is the brightest thing there is, a sun behind it,
            // and a ground that throws most of it back up. The place is the same place and the
            // air has taken everything but the value structure away, which is the whole reason
            // the sixth frame exists.
            sun: [0.16, 0.74, 0.65],
            sun_colour: [0.92, 0.94, 1.0],
            sun_intensity: 1.35,
            disc: [0.0; 3],
            glow: [1.0, 0.99, 0.98],
            zenith: [0.430, 0.470, 0.550],
            horizon: [0.720, 0.740, 0.775],
            ground: [0.620, 0.650, 0.700],
            fill: [0.78, 0.82, 0.90],
            fill_intensity: 0.70,
            fill_position: [-160.0, 700.0, -520.0],
            fill_reach: 1500.0,
            fill_shadow: 0.0,
            haze: [0.700, 0.725, 0.765],
            haze_sunward: [0.800, 0.810, 0.830],
            ridge_base: 0.010,
            ridge_layers: 4,
            exposure: 0.46,
            saturation: 0.82,
            grain: 0.014,
            vignette: 0.28,
            wash: [0.055, 0.058, 0.065],
            wash_amount: 0.35,
            snow: 1.0,
            graze: 0.45,
            mirror: 0.70,
        },
        other => unimplemented!("`{other}` is not a courtyard moment"),
    }
}

/// The camera one moment is photographed from.
///
/// Every one names a lens rather than an angle: a 28 mm on a full frame is the whole of a small
/// yard, a 45 mm is the house, an 85 mm is a plant against a wall. The eye is at a person's
/// height in every one of them, because a courtyard is a place someone stands in, and the shots
/// are the six a person would actually take: the whole place from the gate, the yard across its
/// length, the facade square on, the pond close, a plant against the wall, and out over the
/// valley the terrace falls to.
struct Eye {
    at: [f64; 3],
    look: [f64; 3],
    focal: f64,
    sensor: f64,
    roll: f64,
}

fn eye(moment: &str) -> Eye {
    match moment {
        // From beside the gate, at the yard's south-east corner: the whole place at once, the
        // house across it, the grove behind the house and the bank's bamboo over the wall. It is
        // the frame that says what the place is, and every other one is a detail of it.
        "courtyard-dawn" => Eye {
            at: [10.2, 1.66, 9.8],
            look: [-0.6, 3.2, -12.0],
            focal: 32.0,
            sensor: 36.0,
            roll: 0.006,
        },
        // Across the yard along its length, from the north-east corner by the mill: the pond in
        // the middle distance, the wall and the gate beyond it, the bank and the valley.
        "courtyard-cloud" => Eye {
            at: [9.8, 1.68, -7.4],
            look: [-5.5, 1.30, 8.0],
            focal: 34.0,
            sensor: 36.0,
            roll: -0.016,
        },
        // The house square on, from the middle of the yard: the door, the veranda, the roof's
        // curve and the grove standing over it. The one frame with no perspective trick in it.
        "courtyard-rain" => Eye {
            at: [-1.6, 1.64, 4.2],
            look: [0.6, 3.0, -12.5],
            focal: 34.0,
            sensor: 36.0,
            roll: 0.010,
        },
        // Three-quarter on the house's south-east corner, close in, seen along the veranda with
        // the gate's roof behind. The warm hour, and the frame it is for.
        "courtyard-dusk" => Eye {
            at: [10.2, 1.70, -2.0],
            look: [-3.5, 2.7, -13.5],
            focal: 30.0,
            sensor: 36.0,
            roll: 0.0,
        },
        // The pond at night, from the north-west corner of the yard: the near culms dark against
        // the sky, the lamp burning on the bank, and the water in the middle of the frame.
        "courtyard-night" => Eye {
            at: [-1.6, 1.10, 0.6],
            look: [6.6, 1.35, 7.0],
            focal: 34.0,
            sensor: 36.0,
            roll: 0.014,
        },
        // Over the wall to the north-east, at the slope, the grove's tops and the ridge behind.
        // A long lens, because snow is a texture and a texture wants to be near enough to see.
        "courtyard-snow" => Eye {
            at: [7.2, 1.78, 6.6],
            look: [-7.0, 6.4, -13.5],
            focal: 62.0,
            sensor: 36.0,
            roll: -0.008,
        },
        other => unimplemented!("`{other}` is not a courtyard moment"),
    }
}

/// The exposure and the grade every moment's post stage runs.
///
/// # Invariants
///
/// The order is the order light takes through a camera, and it is the whole of why a frame
/// reads as a photograph rather than as a render with a filter on it: the air is closed
/// **before** the film curve, because a fog applied after the curve is a fog painted on the
/// picture; the curve is applied once; the saturation and the S are the film's own; the
/// vignette and the grain are the camera's and the emulsion's, and they come last.
const GRADE: &str = r#"
    let ray = ray_through(in.uv);
    let far = depth >= 1.0;

    // What the pixel shows in the world, from the depth the prepass wrote. A pixel nothing was
    // drawn on is answered by the ray at the far plane, which is what the air is closed over
    // for the sky: the sky is not a surface, it is air all the way out, and closing the air
    // over a finite distance would leave the horizon clearer than the range behind it.
    let world = world_position(in.uv, min(depth, 0.9999));
    let reach = select(distance(uniforms.camera.position, world), FAR_AIR, far);

    if far {
        color = air_sky(ray);
        color = ridges(color, ray);
    }
    color = courtyard_air(color, ray, world, reach);

    // The lamp of a night, and the one place a moment adds light to a frame rather than to a
    // scene: a single warm source in the air, falling off with distance, so the dark of a
    // courtyard at night has one anchor in it and something for the eye to walk towards.
    if LAMP_STRENGTH > 0.001 {
        let to_lamp = LAMP_POSITION - uniforms.camera.position;
        let along = max(dot(ray, normalize(to_lamp)), 0.0);
        let toward = pow(along, 220.0);
        let halo = pow(along, 6.0);
        color = color + LAMP_COLOUR * (toward * 3.2 + halo * 0.10) * LAMP_STRENGTH;
    }

    // The wet air of a rain, and the flat light of a snow: a wash towards the air's own colour.
    color = mix(color, WASH, WASH_AMOUNT * clamp(dot(color, vec3<f32>(0.33)) * 1.4, 0.0, 1.0));

    // The exposure and the film's curve, once.
    var developed = max(color, vec3<f32>(0.0)) * EXPOSURE;
    developed = (developed * (2.51 * developed + vec3<f32>(0.03)))
        / (developed * (2.43 * developed + vec3<f32>(0.59)) + vec3<f32>(0.14));
    developed = clamp(developed, vec3<f32>(0.0), vec3<f32>(1.0));
    developed = pow(developed, vec3<f32>(0.4545));

    // The film's saturation, and then a gentle S about the middle so a lit face and a shaded
    // one are separated rather than sitting at one value.
    let luma = dot(developed, vec3<f32>(0.2126, 0.7152, 0.0722));
    developed = clamp(mix(vec3<f32>(luma), developed, SATURATION), vec3<f32>(0.0), vec3<f32>(1.0));
    developed = developed * developed * (3.0 - 2.0 * developed) * 0.24 + developed * 0.76;

    // The lens: a vignette that is a circle in the space the frame is actually in.
    let centred = (in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, size.y / max(size.x, 1.0));
    developed = developed * clamp(1.0 - dot(centred, centred) * VIGNETTE, 0.0, 1.0);

    // And the grain, scaled by what it lands on, so it shows in the half tones and the deep
    // shadow stays clean, on the pixel lattice so that it does not crawl.
    let lattice = floor(in.uv * size);
    let noise = hash21(lattice) - 0.5;
    color = developed + vec3<f32>(noise * GRAIN * smoothstep(0.02, 0.30, dot(developed, vec3<f32>(0.33))));
"#;

/// The WGSL every stage of every moment is written against: noise, the yard's own ground
/// colour, the sky's ridges, and the air.
///
/// # Invariants
///
/// The constants this is written **against** are not here — they are in [`Tone::wgsl`] and
/// they are per moment. What is here is the arithmetic, which is the same in all six because
/// the place is.
const SHARED: &str = r#"
/// The colour of the air itself.
///
/// This is the number a distant surface tends towards and the number the sky reaches at its
/// horizon, written once here rather than at the two call sites — because the two call sites
/// are what disagree, and a comment saying they should agree is not a constraint.
fn air_colour() -> vec3<f32> {
    return SKY_HORIZON;
}

/// The sky in a direction, as a function of the ray rather than of the pixel.
///
/// # Invariants
///
/// The ray is the **post stage's**, rebuilt from the camera's own basis, and this is what draws
/// the sky where no geometry reached. The glow and the disc are added only when the moment named
/// them, and the moment's own numbers decide: an overcast sky has no disc at all, which is the
/// difference between a cloudy day and a sunny one and is a scene's decision rather than the
/// renderer's.
///
/// Below the horizon there is no sky, only more air. Stopping the ramp at the horizon would
/// leave a hard line down the middle of a frame whose ground is mist.
fn air_sky(ray: vec3<f32>) -> vec3<f32> {
    let above = pow(clamp(ray.y, 0.0, 1.0), 0.62);
    var colour = mix(air_colour(), SKY_ZENITH, above);
    if SKY_DISC > 0.0 {
        // The warm band along the sun's bearing, hugging the horizon: the brightest thing in most
        // outdoor frames, and it is only here, which is what makes it a direction rather than a
        // bright patch.
        //
        // The falloff is **steep**, and that is the whole of what makes it a bearing rather than a
        // wash. A glow wide enough to be seen a quarter of the sky away lifts the horizon grey a
        // hundred and thirty degrees off the sun, and a sky that is bright everywhere is a sky with
        // no light in it — which is exactly what the air module's own default reach produced here.
        let toward = clamp(dot(ray, SUN_DIRECTION), 0.0, 1.0);
        colour = colour + SKY_GLOW * pow(toward, 34.0);
        // The disc itself, added inside the glow rather than after it, so that widening the glow
        // does not widen the sun.
        colour = colour + SKY_DISC * smoothstep(0.9996, 0.99962, dot(ray, SUN_DIRECTION));
    }
    return select(
        mix(air_colour(), GROUND_TINT, clamp(-ray.y * 4.0, 0.0, 1.0)),
        colour,
        ray.y > 0.0,
    );
}

/// Returns a value in zero to one from a lattice point.
fn hash21(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

/// Returns smooth value noise in zero to one.
fn value_noise21(point: vec2<f32>) -> f32 {
    let cell = floor(point);
    let within = fract(point);
    let blend = within * within * (3.0 - 2.0 * within);
    return mix(
        mix(hash21(cell), hash21(cell + vec2<f32>(1.0, 0.0)), blend.x),
        mix(hash21(cell + vec2<f32>(0.0, 1.0)), hash21(cell + vec2<f32>(1.0, 1.0)), blend.x),
        blend.y,
    );
}

/// Returns noise summed over four octaves, in roughly zero to one.
fn fractal21(point: vec2<f32>) -> f32 {
    var total = 0.0;
    var amplitude = 0.5;
    var at = point;
    for (var octave = 0; octave < 4; octave = octave + 1) {
        total = total + amplitude * value_noise21(at);
        at = at * 2.07;
        amplitude = amplitude * 0.5;
    }
    return total * 1.05;
}

/// Returns creased noise in zero to one: crests and gullies rather than swells.
fn creased21(point: vec2<f32>) -> f32 {
    var total = 0.0;
    var amplitude = 0.5;
    var at = point;
    for (var octave = 0; octave < 4; octave = octave + 1) {
        let folded = 1.0 - abs(value_noise21(at) * 2.0 - 1.0);
        total = total + amplitude * folded;
        at = at * 2.11;
        amplitude = amplitude * 0.5;
    }
    return total * 1.10;
}

/// Returns one dimensional value noise in zero to one, for the ridge lines.
fn ridge_noise(value: f32) -> f32 {
    let cell = floor(value);
    let within = fract(value);
    let blend = within * within * (3.0 - 2.0 * within);
    let near = fract(sin(cell * 127.1) * 43758.5453);
    let far = fract(sin((cell + 1.0) * 127.1) * 43758.5453);
    return mix(near, far, blend);
}

/// The far ranges, drawn where no geometry reached.
///
/// # Invariants
///
/// A ridge line is a function of the ray's **azimuth**, not of the pixel: a silhouette in
/// screen space is a backdrop, and a backdrop does not hold still when the camera moves. Each
/// layer is inked a step deeper than the one behind it, because what separates two ranges is
/// the air between them and nothing else, and the foot of every one of them is left to dissolve
/// into the haze the near hills are already fading into. The layers are counted from the
/// farther, so a nearer crest overdraws a farther one where they cross.
fn ridges(sky: vec3<f32>, ray: vec3<f32>) -> vec3<f32> {
    let azimuth = atan2(ray.x, -ray.z);
    var result = sky;
    for (var layer = f32(RIDGE_LAYERS) - 1.0; layer >= 0.0; layer = layer - 1.0) {
        // Two creases at incommensurate frequencies, multiplied: a single crease saturates at
        // its summit and a saturated crease is a plateau, which reads as a wall.
        let swell = ridge_noise(azimuth * (1.5 + layer * 0.9) + layer * 17.0);
        let crest = 1.0 - abs(swell * 2.0 - 1.0);
        let second = 1.0 - abs(ridge_noise(azimuth * (3.7 + layer * 2.1) + layer * 41.0) * 2.0 - 1.0);
        let spur = ridge_noise(azimuth * (6.1 + layer * 2.3) + layer * 29.0);
        let elevation = RIDGE_BASE + layer * RIDGE_BASE * 1.05
            + crest * second * (RIDGE_BASE * 1.5 + layer * RIDGE_BASE * 0.85)
            + spur * RIDGE_BASE * 0.55;
        if ray.y < elevation && ray.y > -0.10 {
            let depth = 0.94 - layer * 0.05;
            let top = 1.0 - smoothstep(elevation - 0.0022, elevation, ray.y);
            let foot = smoothstep(-0.10, -0.010, ray.y);
            result = mix(result, mix(result, air_colour(), depth), top * foot);
        }
    }
    return result;
}
"#;

/// The helpers the **post** stage is written against, and only the post stage.
///
/// # Why these are not in [`SHARED`]
///
/// Both of them rebuild a ray through a pixel from the camera's own basis, and the basis is a
/// field of the post stage's uniform and of no other stage's: `ray_through` is emitted by
/// `shaders::post` and by nothing else. A helper set that is handed to every stage therefore
/// cannot mention it — a lighting shader handed this text does not parse, and the failure is
/// `no definition in scope for identifier: ray_through` pointing at a shader that looks
/// perfectly well formed.
const POST_HELPERS: &str = r#"
/// How far the air is closed over for a pixel that shows only sky.
const FAR_AIR: f32 = 3400.0;

/// Returns the world position a pixel shows, given the depth buffer's value.
///
/// # Invariants
///
/// The recovery is written against rows two and three of the whole view-projection, so it
/// holds for any projection the camera can emit: `z_clip = row_z · world` and
/// `w = row_w · world`, and `world = eye + ray * t` makes the depth one linear equation in
/// `t`. Reading the bare perspective's coefficients instead reads every distance as the far
/// plane the moment the view's rotation is composed into them.
fn world_position(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ray = ray_through(uv);
    let eye = uniforms.camera.position;
    let matrix = uniforms.projection.view_projection;
    let row_z = vec4<f32>(matrix[0][2], matrix[1][2], matrix[2][2], matrix[3][2]);
    let row_w = vec4<f32>(matrix[0][3], matrix[1][3], matrix[2][3], matrix[3][3]);
    let at_eye_z = dot(row_z.xyz, eye) + row_z.w;
    let at_eye_w = dot(row_w.xyz, eye) + row_w.w;
    let per_depth = dot(row_z.xyz - depth * row_w.xyz, ray);
    let t = (depth * at_eye_w - at_eye_z) / max(per_depth, 1e-9);
    return eye + ray * clamp(t, 0.0, 4000.0);
}

/// Closes the air between the eye and a point, and warms it towards the sun.
///
/// # Invariants
///
/// The optical depth is the **closed form** of an exponential height fog along a ray, taken
/// piecewise: a ray that dips below the ceiling integrates down to the ceiling and then at the
/// full density. Stepping the fog would band where it is thin, and applying the single
/// exponential form to a dipping ray extrapolates the density's growth without bound, which a
/// few hundred units of turns into a white frame.
///
/// The ceiling wanders with the ground so the mist pools in the low ground and tears along the
/// banks, and the scatter is warmed only **towards** the sun, which is why a misty valley is
/// brightest when it is looked into.
fn courtyard_air(color: vec3<f32>, ray: vec3<f32>, world: vec3<f32>, reach: f32) -> vec3<f32> {
    let eye = uniforms.camera.position;
    let ceiling = AIR_CEILING
        + 5.0 * fractal21(world.xz * 0.011 + vec2<f32>(uniforms.time * 0.010, 0.0));
    let above = max(eye.y - ceiling, 0.0);
    let falloff = 0.085;

    var optical = 0.0;
    if eye.y <= ceiling {
        optical = AIR_DENSITY * reach;
    } else {
        let dip = falloff * ray.y;
        let cross = (ceiling - eye.y) / min(ray.y, -1e-6);
        if ray.y < -0.0005 && cross < reach {
            optical = AIR_DENSITY * exp(-falloff * above) * (1.0 - exp(-dip * cross)) / dip
                + AIR_DENSITY * (reach - cross);
        } else if abs(dip) > 0.0005 {
            optical = AIR_DENSITY * exp(-falloff * above) * (1.0 - exp(-dip * reach)) / dip;
        } else {
            optical = AIR_DENSITY * exp(-falloff * above) * reach;
        }
    }
    // Aerial perspective proper: even above the mist the air is not nothing.
    // Aerial perspective proper: even above the mist the air is not nothing, and at four
    // parts in a hundred thousand over a kilometre it is what carries a far range.
    optical = optical + reach * 0.00016;

    let sunward = pow(max(dot(ray, SUN_DIRECTION), 0.0), 4.0);
    let haze = mix(HAZE_AWAY, HAZE_TOWARD, sunward);
    return mix(color, haze, clamp(1.0 - exp(-optical), 0.0, 1.0));
}
"#;

/// Where the lamp stands: on the bank between the pond and the gate, at a person's height.
const LAMP: [f64; 3] = [5.1, 1.30, 8.3];

/// The height of the land, before the yard was cut into it.
///
/// # Invariants
///
/// The place is a **shelf**: the ground falls away to the south and west, a bank rises behind
/// the house, and the mass of a ridge stands off in the north-west. Every height in the scene
/// is decided against this one function, so a clump of bamboo and the ridge it is seen against
/// are the same landscape.
fn land(x: f64, z: f64) -> f64 {
    // The fall to the valley in the south and the west, gentle enough that a terrace holds it.
    let south = smoothstep(-8.0, 74.0, z) * 15.0;
    let west = smoothstep(-14.0, -76.0, x) * 8.0;

    // The bank behind the house, and the rise it sits on. This is the ground the yard was cut
    // out of; without it the house would stand on a plain.
    let bank = smoothstep(6.0, 30.0, z.abs() * 0.35 - z * 0.72) * 2.4;
    let rise = 6.5 * smoothstep(8.0, 62.0, -z);

    // The ridge: a mass that gathers from a hundred and fifty units out and crests at four
    // hundred and thirty, built from creased noise rather than a ramp, because a ramp's knee seen
    // from below is a dome against the sky and a dome is a volcano.
    //
    // **How far out it stands is the whole of what makes it a range rather than a wall.** A mass
    // rising from fifty units away fills half the frame above the roofline at forty degrees of
    // elevation, and forty degrees is a cliff three hundred metres high with a house at its foot.
    // From a hundred and fifty it subtends twenty, which is a mountain in a valley.
    let mass = smoothstep(150.0, 430.0, -z) * (30.0 + 30.0 * creased_1d(x));
    let shoulder = smoothstep(210.0, 470.0, -z) * 20.0 * creased_1d(x * 0.55 + 11.0);

    // Side walls, low and creased, so the valley closes rather than opens onto nothing.
    let wall = smoothstep(110.0, 320.0, x.abs()) * (10.0 + 22.0 * ridged_noise([z * 0.02, x * 0.015], 4, 47));

    // The hills that carry the terraces, and the ground's own slow wrinkle.
    let roll = 5.5 * fractal_noise([x * 0.010, z * 0.010], 3, 23);

    south + west + bank + rise + mass + shoulder + wall + roll
}

/// A one dimensional crease, for the ridge's own crest line.
fn creased_1d(value: f64) -> f64 {
    let folded = 1.0 - (fract_signed(value * 0.055) * 2.0 - 1.0).abs();
    let second = 1.0 - (fract_signed(value * 0.107 + 0.31) * 2.0 - 1.0).abs();
    0.45 + 0.75 * folded * second
}

/// Returns a number in zero to one from a coordinate, smoothly.
fn fract_signed(value: f64) -> f64 {
    let wrapped = value - value.floor();
    // Smooth the sawtooth into a wander, so the crest line is a curve rather than a zigzag.
    wrapped * wrapped * (3.0 - 2.0 * wrapped)
}

/// The signed distance to the yard's rectangle: negative inside, positive outside.
fn yard_distance(x: f64, z: f64) -> f64 {
    let dx = x.abs() - YARD[0];
    let dz = z.abs() - YARD[1];
    let outside = (dx.max(0.0).powi(2) + dz.max(0.0).powi(2)).sqrt();
    outside + dx.max(dz).min(0.0)
}

/// The height of the ground the scene is built on: the land, with the yard levelled into it.
fn ground(x: f64, z: f64) -> f64 {
    let distance = yard_distance(x, z);
    // The blend is a smoothstep of the distance rather than a mask on the rectangle, so the
    // bank rises out of the paving as a continuous surface and its normal is true to it.
    let levelled = smoothstep(0.0, BANK, distance);
    land(x, z) * levelled
}

/// The slope of the ground, by central difference, from the same function the mesh is built
/// from.
fn slope(x: f64, z: f64) -> f64 {
    let step = 2.0 * EXTENT / DIVISIONS as f64;
    let dx = ground(x + step, z) - ground(x - step, z);
    let dz = ground(x, z + step) - ground(x, z - step);
    (dx * dx + dz * dz).sqrt() / (2.0 * step)
}

/// A smoothstep, matching the WGSL builtin's shape.
fn smoothstep(low: f64, high: f64, value: f64) -> f64 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Quantises a height to steps, with the riser a smooth turn over the top of each step.
///
/// A `floor` alone tears the surface at every level, and the heightfield's normals are taken
/// from the height function itself, so a torn riser shades as a stripe of noise rather than as
/// a bank.
fn quantise(value: f64, step: f64, flat: f64) -> f64 {
    let q = value / step;
    let whole = q.floor();
    let part = q - whole;
    (whole + smoothstep(flat, 1.0, part)) * step
}

/// The height a terrace stands at, on the ground that is farmed.
fn terraced(x: f64, z: f64) -> f64 {
    let base = ground(x, z);
    // The paddies: only below the yard and holding no great slope, because nobody terraces a
    // cliff and a contour stripe on one reads as snow.
    let band = smoothstep(11.5, 34.0, z)
        * (1.0 - smoothstep(0.42, 0.78, slope(x, z)))
        * (1.0 - smoothstep(30.0, 78.0, base));
    if band <= 0.0 {
        return base;
    }
    let step = 2.4 * (0.85 + 0.30 * value_noise([x * 0.05, z * 0.05], 91));
    let phase = 1.5 * (value_noise([x * 0.03, z * 0.03], 57) - 0.5);
    let cut = quantise(base + phase, step, 0.68) - phase;
    base + (cut - base) * band
}

/// The mesh height of the whole world: the terraced ground, under the sky.
fn terrain(x: f64, z: f64) -> f64 {
    terraced(x, z)
}

/// How much of the yard's paving a point of the ground is, from one at the middle of the yard
/// to zero on the bank.
///
/// # Invariants
///
/// This is the **same** mask the surface body branches on, and it is written once here because
/// the two are one fact: what the ground is made of. It fades over the same `BANK` the height is
/// blended over, so the paving ends where the ground stops being flat — a paving mask that ran
/// past the level ground would paint slabs up a slope, and one that stopped short of it would
/// leave a ring of bare earth in the middle of a courtyard.
fn yard_mask(x: f64, z: f64) -> f64 {
    1.0 - smoothstep(0.0, BANK, yard_distance(x, z))
}

/// The roof, as one mesh: both slopes of a curved tile roof in a single heightfield.
///
/// # Invariants
///
/// The profile is a **tent**: the ridge at the local origin, and a straight fall to either
/// side, with the last third of a unit of each slope turned **up** into an eave. A Chinese roof's
/// eave is the one line that says which tradition it is built in — a straight eave is a
/// European roof — and the turn is what a photograph of one actually shows.
///
/// The tile channels are **not** geometry. A tile is a quarter of a metre and the mesh's own cell
/// is a twentieth of the run, so a ripple modelled in the surface would be sampled twice a cycle
/// exactly at the distances a roof is looked at from, and would alias into a moiré. The channels
/// are drawn by the surface body instead, from the world position, where they can be faded out as
/// a tile falls below a pixel — the same argument `stilllife` makes for its linen weave.
fn roof_tile(length: f64) -> impl Fn(f64, f64) -> f64 {
    move |x: f64, z: f64| {
        let run = z.abs();
        let fall = ROOF_RISE * (run / length).min(1.0);
        let eave_start = length - 0.45;
        let turn = if run > eave_start {
            smoothstep(eave_start, length, run) * 0.17
        } else {
            0.0
        };
        let ridge = 0.10 * (1.0 - smoothstep(0.0, 0.62, run));
        let saddle = -0.03 * smoothstep(length - 0.30, length, run);
        // A slight sag along the ridge, because a roof that has stood for fifty years has one.
        let settle = 0.030 * (1.0 - smoothstep(0.0, 1.5, run)) * ((x * 0.44).cos() * 0.5 + 0.5);
        fall + turn + ridge + saddle - settle
    }
}

/// The profile of a water jar: a wide belly, a narrow neck and a rolled lip.
///
/// The inside is part of the same closed profile, so the water in it is a second surface and
/// the rim is a lip rather than an edge.
const JAR: [[f64; 2]; 15] = [
    [0.000, 0.000],
    [0.115, 0.000],
    [0.205, 0.035],
    [0.270, 0.115],
    [0.298, 0.230],
    [0.290, 0.350],
    [0.255, 0.455],
    [0.205, 0.530],
    [0.185, 0.575],
    [0.200, 0.610],
    [0.196, 0.628],
    [0.172, 0.600],
    [0.165, 0.520],
    [0.100, 0.420],
    [0.000, 0.360],
];

/// The profile of a carrying basket: an open cone with a heavier foot.
const BASKET: [[f64; 2]; 11] = [
    [0.000, 0.000],
    [0.105, 0.000],
    [0.135, 0.018],
    [0.150, 0.050],
    [0.190, 0.150],
    [0.235, 0.270],
    [0.268, 0.372],
    [0.282, 0.400],
    [0.268, 0.392],
    [0.250, 0.360],
    [0.240, 0.330],
];

/// The profile of a stone mill: a thick disc with a raised rim.
const MILL: [[f64; 2]; 9] = [
    [0.000, 0.000],
    [0.360, 0.000],
    [0.410, 0.030],
    [0.420, 0.120],
    [0.408, 0.150],
    [0.360, 0.140],
    [0.250, 0.132],
    [0.120, 0.118],
    [0.000, 0.108],
];

/// One thing the scene draws, accumulated with the material each pass needs.
///
/// # Invariants
///
/// A material is declared once and named by every instance drawn with it, and an instance is
/// listed under each pass that draws it, because the pass is the bucket and a material names
/// one shader, which belongs to one stage. This is the arrangement the pipeline is built for:
/// what differs between two surfaces is a shader and a row, not a pipeline.
struct Place {
    materials: Vec<Value>,
    shadowing: Vec<Value>,
    depthing: Vec<Value>,
    lighting: Vec<Value>,
    emission: Vec<Value>,
}

impl Place {
    fn new() -> Self {
        Self {
            materials: Vec::new(),
            shadowing: Vec::new(),
            depthing: Vec::new(),
            lighting: Vec::new(),
            emission: Vec::new(),
        }
    }

    fn declare(&mut self, label: &str, shader: &str, custom: Value) -> String {
        self.materials.push(mesh_material(label, shader, custom));
        label.to_owned()
    }

    /// Draws one placement in the three stages a lit solid is drawn in.
    ///
    /// `kind` is which surface this is, and it is carried by **all three** rows rather than by
    /// the shading one alone. The two depth stages read none of it — their vertex stage places
    /// a point and their fragment stage writes depth — but a row is verified against the
    /// material's own custom schema whichever pass it is drawn in, so a row that omitted the
    /// field would be a document the schema refuses rather than a cheaper depth pass.
    ///
    /// `cast` says whether the thing is put in the shadow map. It is one flag per object for
    /// one reason: a shadow map holds what is put in it, and a hundred and ninety grass tufts
    /// casting their own shadows is a hundred and ninety ways for the map to grow acne, for a
    /// shadow nothing in the frame can resolve. Ground cover does not cast.
    #[allow(clippy::too_many_arguments)]
    fn solid(
        &mut self,
        label: &str,
        mesh: &str,
        at: [f64; 3],
        turn: f64,
        scaled: [f64; 3],
        tint: [f64; 3],
        roughness: f64,
        reflect: f64,
        kind: f64,
        cast: bool,
    ) {
        let rotation = [0.0, turn, 0.0];
        let row = material(tint, roughness, reflect, kind);
        let named = |suffix: &str| -> String {
            let mut name = String::with_capacity(label.len() + suffix.len() + 1);
            name.push_str(label);
            name.push(' ');
            name.push_str(suffix);
            name
        };
        if cast {
            let material = self.declare(&named("cast"), "courtyard_shadowing", row.clone());
            self.shadowing
                .push(instance(named("c"), mesh, at, rotation, scaled, &material));
        }
        let material = self.declare(&named("depth"), "courtyard_depthing", row.clone());
        self.depthing
            .push(instance(named("d"), mesh, at, rotation, scaled, &material));
        let material = self.declare(&named("shade"), "lighting", row);
        self.lighting
            .push(instance(named("s"), mesh, at, rotation, scaled, &material));
    }

    /// Draws one placement **only** in the depth and shading stages: ground cover, which casts
    /// no shadow, and a leaf, which is one sided and casts a shadow a page cannot resolve.
    #[allow(clippy::too_many_arguments)]
    fn plant(
        &mut self,
        label: &str,
        mesh: &str,
        at: [f64; 3],
        turn: f64,
        scaled: [f64; 3],
        tint: [f64; 3],
        roughness: f64,
        kind: f64,
    ) {
        let rotation = [0.0, turn, 0.0];
        let row = material(tint, roughness, 0.0, kind);
        let named = |suffix: &str| -> String {
            let mut name = String::with_capacity(label.len() + suffix.len() + 1);
            name.push_str(label);
            name.push(' ');
            name.push_str(suffix);
            name
        };
        let material = self.declare(&named("depth"), "courtyard_depthing", row.clone());
        self.depthing
            .push(instance(named("d"), mesh, at, rotation, scaled, &material));
        let material = self.declare(&named("shade"), "lighting", row);
        self.lighting
            .push(instance(named("s"), mesh, at, rotation, scaled, &material));
    }

    fn build(self) -> (Vec<Value>, Value) {
        (
            self.materials,
            instances(vec![
                (Stage::Shadowing, self.shadowing),
                (Stage::Depthing, self.depthing),
                (Stage::Lighting, self.lighting),
                (Stage::Emission, self.emission),
            ]),
        )
    }
}

/// The material a surface is, as a number the lighting body branches on.
///
/// # Why this is a number and not a shader a material
///
/// One shader a material would be the framework's own answer — a material names a shader and
/// the shader's stage is the pass it is drawn in — and it is the wrong one here for a reason
/// that is a fact about bamboo: a grove is six hundred placements of two meshes, and six
/// hundred materials is six hundred passes, because a pass draws one instance. What tells a
/// stone from a whitewashed wall is not the *arithmetic* either; both are a noise and a
/// modulate. So the arithmetic is one body, the choice is one number in the row, and the
/// passes are as few as the meshes.
const KIND: &str = "kind";

/// The material kinds the surface body branches on.
const STONE: f64 = 0.0;
const EARTH: f64 = 1.0;
const PLASTER: f64 = 2.0;
const TILE: f64 = 3.0;
const TIMBER: f64 = 4.0;
const BAMBOO: f64 = 5.0;
const LEAF: f64 = 6.0;
const GRASS: f64 = 7.0;
const WATER: f64 = 8.0;
const METAL: f64 = 9.0;
const CLOTH: f64 = 10.0;

/// The custom schema a courtyard material carries: the gallery's row and the kind beside it.
fn custom_schema() -> definition::Schema {
    use definition::{Schema, SchemaKind};
    let base = gallery_custom_schema();
    let SchemaKind::Struct { fields } = base.kind else {
        unreachable!("it is not a struct")
    };
    let mut fields = fields;
    fields.insert(
        KIND.to_owned(),
        Schema::new(SchemaKind::Float).with_documentation(
            "Which surface this row is, as a number the lighting body branches on.",
        ),
    );
    Schema::new(SchemaKind::structure(fields))
}

/// Builds one material's custom data.
fn material(tint: [f64; 3], roughness: f64, reflect: f64, kind: f64) -> Value {
    let Value::Struct(mut fields) = surface_reflecting(tint, 0.0, roughness, [0.0; 3], reflect)
    else {
        unreachable!("it is not a struct")
    };
    fields.insert(KIND.to_owned(), Value::Float(kind));
    Value::Struct(fields)
}

/// Where the roof's ridge stands, and how high.
///
/// The ridge is the roof mesh's own origin, so this is the one point the placement and the tile
/// body both measure from: the body recovers how far down a slope a fragment is from the height
/// it has fallen below `ROOF_RIDGE[1]`, which is why the two cannot disagree about where the
/// tiles run.
const ROOF_RIDGE: [f64; 3] = [HOUSE[0], PLINTH + WALL_HEIGHT + ROOF_RISE, HOUSE[2]];

/// How the roof mesh is scaled to the roof: one entry per axis, as a ratio of the mesh.
///
/// `z` and `y` are the same, because a roof's slope is one angle: scaling the run and the rise
/// by one number leans the surface while keeping it a surface. Scaling them apart — which is
/// what happens when a mesh built in local units is placed with a rise in world ones — puts the
/// eave in the sky and the ridge underground.
fn roof_scale() -> [f64; 3] {
    let lean = ROOF_RUN / ROOF_SLOPE;
    [2.0 * ROOF_HALF_WIDTH / (2.0 * ROOF_SLOPE), lean, lean]
}

/// The two numbers a roof tile body is written against.
fn roof_constants() -> String {
    format!(
        "const ROOF_RIDGE_X: f32 = {:.4};\nconst ROOF_RIDGE_Y: f32 = {:.4};\n\
         const ROOF_LOCAL_Z: f32 = {:.5};\n",
        ROOF_RIDGE[0],
        ROOF_RIDGE[1],
        ROOF_RUN / ROOF_RISE,
    )
}

/// The surface WGSL every lighting body is written against: the noise, the sky, and the
/// per-material arithmetic.
///
/// # Invariants
///
/// Every function here is named with a `c_` prefix, because a scene's helpers are declared at
/// module level beside the pipeline's own and `SHARED` already owns `hash21`, `value_noise21`
/// and `fractal21`. Two functions with one name is a shader that will not compile.
const SURFACES: &str = r#"
/// Returns the colour of the air along a direction, at the **level** a surface reflects.
///
/// # Why this is not `air_sky`
///
/// The post stage rebuilds a ray through a pixel from the camera's own basis and hands it to
/// `air_sky`; a lighting stage has no basis to rebuild one from, and `air_sky` needs the
/// function that does. What a surface reflecting a direction actually wants is the air **at
/// that direction's own elevation**, and the elevation is the only half of a ray a sky made of
/// a zenith, a horizon and a ground is a function of. So this is the same gradient the sky is
/// built from, without the glow and the disc — which are what the post stage adds where no
/// geometry reached, which is exactly the pixels a reflection is not.
fn c_sky_at(direction: vec3<f32>) -> vec3<f32> {
    let rise = clamp(direction.y, -1.0, 1.0);
    let above = pow(clamp(rise, 0.0, 1.0), 0.62);
    return select(
        mix(SKY_HORIZON, GROUND_TINT, clamp(-rise * 4.0, 0.0, 1.0)),
        mix(SKY_HORIZON, SKY_ZENITH, above),
        rise > 0.0,
    );
}

/// Returns how much of the sun a sky of this moment has in it, zero to one.
///
/// Every surface that is lit partly by the sky and partly by the sun reads this, so the
/// proportion between the two is one number rather than one per surface.
fn c_sun_share() -> f32 {
    return clamp(EXPOSURE * dot(SUN_COLOUR, vec3<f32>(0.35)), 0.0, 1.0);
}

/// Returns the lane pattern of a bamboo stem, in zero to one.
///
/// A culm's own vertical striation, which is what separates it from a painted pole: the fibres
/// run the length of the stem and there is a waxy bloom in the grooves.
fn c_culm_grain(position: vec3<f32>, along: f32) -> f32 {
    let angle = atan2(position.z, position.x);
    let lane = 0.5 + 0.5 * sin(angle * 11.0 + along * 0.6 + fractal21(position.xz * 0.9) * 2.0);
    return lane;
}

/// Returns how bare this part of a plant is, zero to one: the tips are the leaves and the
/// bottom is the stem.
///
/// A grove drawn with leaves down its whole height is a hedge; bamboo carries its leaves on the
/// top third of the culm, and the body is where that is said.
fn c_bareness(height: f32, base: f32, top: f32, leafy: f32) -> f32 {
    let up = clamp((height - base) / max(top - base, 0.001), 0.0, 1.0);
    return clamp((1.0 - up / max(leafy, 0.05)) * 2.2, 0.0, 1.0);
}

/// Returns the vertical mottle of weathered stone, in zero to one.
fn c_stone_mottle(position: vec3<f32>) -> f32 {
    let coarse = fractal21(position.xz * 0.35 + vec2<f32>(position.y * 0.22, 0.0));
    let fine = fractal21(position.xz * 2.6);
    let damp = smoothstep(0.0, 0.35, -position.y + 0.2);
    return clamp(coarse * 0.66 + fine * 0.22 + damp * 0.12, 0.0, 1.0);
}

/// Returns the channels of a tiled roof, in zero to one: one at a ridge of tile, zero in a
/// channel.
///
/// # Invariants
///
/// The channels are read off the roof's **world** position and the roof's own ridge, so the
/// pattern is fixed by the same two constants the mesh was placed with rather than by a
/// texture coordinate nobody carries. The channels fade out where a tile falls below a pixel,
/// which is the whole reason this is arithmetic and not geometry: a tile is a quarter of a
/// metre and the mesh's own cell is the same size, so a modelled ripple would be sampled twice
/// a cycle and would alias exactly at the distances a roof is looked at from.
fn c_roof_tiles(position: vec3<f32>) -> f32 {
    // The roof's own two axes, recovered from the world position: the ridge runs along `x` and
    // the slope runs down in `y` away from it, so how far down the slope a point is comes back
    // from the height it has fallen.
    let along = position.x - ROOF_RIDGE_X;
    let run = max(ROOF_RIDGE_Y - position.y, 0.0) / ROOF_LOCAL_Z;
    let channels = 0.5 + 0.5 * cos(along * 20.5);
    let courses = 0.5 + 0.5 * cos(run * 7.4 + along * 0.4);
    let width = length(fwidth(position.xz));
    let drawn = smoothstep(0.18, 0.60, 0.28 / max(width, 0.0005));
    return mix(0.5, channels * 0.70 + courses * 0.30, drawn);
}
"#;

/// Builds the lighting body: what every surface of the courtyard does with the light it got.
///
/// # Invariants
///
/// The body is spliced into the lighting stage's fragment function, so it reads the names that
/// template has in scope — `color`, `normal`, `ambient`, `diffuse_color`, `fresnel`,
/// `in.world_position`, `uniforms` — and declares none of them twice. It ends by writing
/// `color` and nothing else: the pipeline owns the light, and a body that rewrote the shading
/// would get the shadow term, the falloff or the Fresnel subtly wrong.
///
/// # Where the material's kind is read from, and why it is not a field of the shader's schema
///
/// The pipeline's own template binds the three material numbers it knows about at fixed names —
/// `metallic`, `roughness`, `reflect`, in the order the row schema sorts them — and a scene that
/// wants a fourth has no name to reach it by. What it does have is **laziness the template
/// already performs**: a surface body runs at the very end of the fragment, after every read of
/// those three, so the slot the *scene* put the kind in is read here and nowhere else.
///
/// The slot is `reflect`, and the reason is that `reflect` is the one number the template reads
/// **exactly once, before the body**, into `in.material.z`. `metallic` is read into two terms
/// and `roughness` into three, and a value that has already stood in for one of those is a
/// value that has already been used as the thing it is not.
fn surface_body() -> &'static str {
    r#"    let kind = in.material.z;
    let at = in.world_position;
    let to_eye = normalize(uniforms.projection.position - at);
    let view = normalize(at - uniforms.projection.position);
    let facing = clamp(dot(normal, -view), 0.0, 1.0);
    let sky_share = c_sun_share();

    if (kind < 0.5) {
        // Rough stone: a wall's base course, the pond's rim, the mill, the step. The mottle is
        // damp low down, because a stone in a wet yard is wet at its foot and dry at its head.
        let mottle = c_stone_mottle(at);
        color = color * mix(0.70, 1.16, mottle);
        let wet = smoothstep(0.9, 0.0, at.y) * WETNESS;
        color = color * mix(1.0, 0.52, wet);
        color = color + fresnel * c_sky_at(reflect(view, normal)) * (0.030 + 0.18 * wet) * GRAZE;
    } else if (kind < 1.5) {
        // The ground, which is one mesh and five materials: paving, the joints between its
        // slabs, the earth of a bank, the water of a flooded terrace and the snow that lies on
        // all of it. Which of them a fragment is, is the mask the mesh carries in the one
        // component of a normal it has free — see [`crate::geometry::masked_heightfield`] — and
        // nothing here recomputes it. A mask worked out in the shader and a mask worked out in
        // the generator are the same arithmetic written twice, and they are free to disagree at
        // exactly the seam a courtyard's paving ends at.
        //
        // The normal is reassigned from that mask, because the generator writes a placeholder
        // there: a yard is flat, a joint is a shallow groove, and a bank is the slope it is.
        let mask = normal.z;
        let flat = smoothstep(0.60, 0.84, mask);
        normal = normalize(mix(vec3<f32>(0.0, 0.86, 0.51), vec3<f32>(0.0, 1.0, 0.0), flat));
        let graze = clamp(dot(normal, -view), 0.0, 1.0);
        // The paving's own pattern. A courtyard is laid in courses and every course is a
        // different slab, so the joints run both ways and a slab's colour is its own hash.
        let course = at.xz / 0.94;
        let cell = floor(course);
        let within = abs(fract(course) - vec2<f32>(0.5, 0.5)) * 2.0;
        let joint = smoothstep(0.80, 0.98, max(within.x, within.y));
        let slab = hash21(cell);
        let grain = fractal21(at.xz * 5.2);
        var ground = mix(
            vec3<f32>(0.46, 0.455, 0.425),
            vec3<f32>(0.68, 0.665, 0.620),
            slab * 0.55 + grain * 0.45,
        );
        ground = mix(ground, vec3<f32>(0.26, 0.26, 0.245), joint * 0.78);
        // The bank: earth, its own clods, and what grows on it.
        let clod = fractal21(at.xz * 1.9);
        let tufted = smoothstep(0.42, 0.78, value_noise21(at.xz * 0.22));
        let bank = mix(
            vec3<f32>(0.150, 0.130, 0.088),
            vec3<f32>(0.100, 0.128, 0.060),
            tufted * 0.80,
        ) * (0.62 + 0.66 * clod);
        ground = mix(ground, bank, 1.0 - smoothstep(0.20, 0.46, mask));
        // The terraces below the yard, which are water: a flooded paddy mirrors the sky, and
        // what it holds under the water is a little rice.
        let flooded = smoothstep(30.0, 48.0, at.z) * flat;
        let mirror = (0.030 + 0.970 * pow(1.0 - graze, 5.0)) * MIRROR;
        ground = mix(
            ground,
            ground * 0.22 + c_sky_at(reflect(view, normal)) * mirror,
            flooded * 0.92,
        );
        // Snow lies on whatever faces up, and every surface here does.
        ground = mix(ground, vec3<f32>(0.60, 0.64, 0.72), SNOW * flat * 0.80);
        color = color * ground;
        // A pool of standing water on the paving, where the yard drains: the same paving mask
        // and a noise for where the low spots are.
        let pool = smoothstep(0.58, 0.74, value_noise21(at.xz * 0.16 + vec2<f32>(3.1, 7.7)))
            * flat * WETNESS;
        color = mix(
            color,
            color * 0.18 + c_sky_at(reflect(view, normal)) * mirror * 0.90,
            pool * 0.55,
        );
        // The mottle of a stone that has been walked on for a hundred years, under all of it.
        color = color * mix(0.86, 1.10, c_stone_mottle(at * 1.4));
        color = color + fresnel * c_sky_at(reflect(view, normal))
            * (0.010 + 0.09 * WETNESS * flat) * (1.0 - flooded) * (1.0 - pool) * GRAZE;
    } else if (kind < 2.5) {
        // Whitewash: lime over rammed earth. It is the brightest thing in every frame and it is
        // not flat — a hand-applied wash has trowel bands and it has run at the base.
        let trowel = fractal21(vec2<f32>(at.x * 0.22 + at.z * 0.10, at.y * 0.30));
        let band = value_noise21(vec2<f32>((at.x + at.z) * 0.06, at.y * 0.9));
        var wash = 0.86 + 0.30 * trowel * band;
        // The splash line: the yard throws water up the first half metre of every wall, and
        // that line is what tells a real wall from a white box.
        let splash = (1.0 - smoothstep(0.15, 0.85, at.y)) * WETNESS;
        wash = mix(wash, 0.56, splash * 0.85);
        // And the foot of a wall in a village is greener than its head.
        wash = wash * mix(1.0, 0.86, 1.0 - smoothstep(0.0, 1.8, at.y));
        color = color * wash;
        color = color + fresnel * c_sky_at(reflect(view, normal)) * 0.030 * GRAZE;
    } else if (kind < 3.5) {
        // A tiled roof: grey clay, fired dark and weathered pale, wet through when it rains.
        let tile = c_roof_tiles(at);
        let weathered = fractal21(at.xz * 0.55 + vec2<f32>(at.y * 0.3, 0.0));
        var shade = 0.66 + 0.46 * tile;
        shade = shade * (0.86 + 0.28 * weathered);
        color = color * shade;
        // The sheen along a wet tile is the one specular line a roof has, and it is what makes
        // a roof read as tiled rather than as a slab when there is no sun to model it.
        let sheen = pow(facing, 2.0) * WETNESS * 0.22;
        color = color + fresnel * c_sky_at(reflect(view, normal)) * (0.045 + sheen) * GRAZE;
    } else if (kind < 4.5) {
        // Timber: a post, a door frame, the rack. Open grain along the length, greyed outside
        // and dark where the rain has run.
        let across = at.x * 8.0 + at.z * 6.0;
        let grain = value_noise21(vec2<f32>(across, at.y * 0.35));
        let check = 0.5 + 0.5 * sin(across * 2.4 + grain * 3.0);
        color = color * mix(0.72, 1.10, check * 0.55 + grain * 0.45);
        let rain = smoothstep(0.9, 0.0, at.y) * WETNESS;
        color = color * mix(1.0, 0.55, rain);
        color = color + fresnel * c_sky_at(reflect(view, normal)) * 0.03 * WETNESS;
    } else if (kind < 5.5) {
        // Bamboo. A culm is a cylinder of almost nothing: the whole of what it looks like is
        // the light on it, so the body is a rim, a lane of wax, and no albedo at all to speak
        // of.
        let lane = c_culm_grain(at, at.y);
        let sheen = pow(1.0 - facing, 2.4);
        color = color * mix(0.74, 1.30, lane * 0.55 + sheen * 0.45);
        // A culm's own translucency: backlit, a green stem glows, and a grove against the sun
        // is the one place that matters. It is added on the **far** side of the stem from the
        // eye, which is where a thin wall of green shows its light.
        let through = pow(clamp(dot(-view, SUN_DIRECTION), 0.0, 1.0), 2.2) * (1.0 - sky_share);
        color = color + SUN_COLOUR * through * 0.55;
        color = color + fresnel * c_sky_at(reflect(view, normal)) * (0.10 + 0.18 * WETNESS) * GRAZE;
    } else if (kind < 6.5) {
        // A leaf. Leaves are the one surface in an outdoor frame that light comes *through*,
        // and a grove lit without it is a grove of green wire. The transmission is strongest
        // when the sun is behind the leaf, which is what `dot(-view, SUN_DIRECTION)` measures.
        let through = pow(clamp(dot(-view, SUN_DIRECTION), 0.0, 1.0), 1.6);
        color = color * mix(0.80, 1.16, value_noise21(at.xz * 3.1));
        color = color + SUN_COLOUR * through * (1.0 - sky_share) * 1.30;
        color = color + air_colour() * 0.14 * (1.0 - sky_share);
        let wet = fresnel * c_sky_at(reflect(view, normal)) * WETNESS * 0.40;
        color = color + wet * GRAZE;
        // And the snow that lies along a leaf's own upper side, which is where snow lies.
        if (SNOW > 0.001) {
            let gather = clamp(normal.y, 0.0, 1.0);
            color = mix(color, vec3<f32>(0.82, 0.86, 0.94), SNOW * gather * 0.55);
        }
    } else if (kind < 7.5) {
        // Grass: the same transmission as a leaf, a fifth of the size and a tenth of the light.
        let through = pow(clamp(dot(-view, SUN_DIRECTION), 0.0, 1.0), 1.4);
        color = color + SUN_COLOUR * through * (1.0 - sky_share) * 0.42;
        let frost = SNOW * clamp(normal.y, 0.0, 1.0);
        color = mix(color, vec3<f32>(0.78, 0.82, 0.90), frost * 0.7);
    } else if (kind < 8.5) {
        // Water. The pond is a mirror and the only thing in the yard that is one, so this is
        // where the sky comes down to the ground.
        let time = uniforms.time;
        let ripple = (value_noise21(at.xz * 1.7 + vec2<f32>(time * 0.30, time * 0.13)) - 0.5)
            + 0.5 * (value_noise21(at.xz * 4.1 - vec2<f32>(time * 0.17, time * 0.26)) - 0.5);
        let still = 0.35 + 0.65 * value_noise21(at.xz * 0.30 + vec2<f32>(time * 0.02, 0.0));
        let water_normal = normalize(vec3<f32>(
            normal.x + ripple * 0.055 * still,
            normal.y,
            normal.z + ripple * 0.048 * still,
        ));
        let facing_water = clamp(dot(water_normal, -view), 0.0, 1.0);
        let mirror = 0.030 + 0.970 * pow(1.0 - facing_water, 5.0);
        // What is under the water shows through where the reflection is weak, and what is under
        // it is the yard's own stone seen through a hand's depth of it.
        color = color * mix(1.0, 0.22, clamp(mirror, 0.0, 1.0));
        color = color + c_sky_at(reflect(view, water_normal)) * mirror * 0.96;
    } else if (kind < 9.5) {
        // Metal: the lamp's hood, the hoop of a bucket. It has colours only in what it
        // reflects, and what it reflects here is one sky.
        color = color * 0.35;
        color = color + c_sky_at(reflect(view, normal)) * in.material.z * 0.30;
    } else {
        // Cloth: the awning, the mat. A woven thing scatters, and it is the only surface here
        // that is warm and matte at once.
        let weave = value_noise21(vec2<f32>(at.x * 6.0, at.z * 6.0));
        color = color * mix(0.84, 1.08, weave);
        color = color + SUN_COLOUR * pow(clamp(dot(-view, SUN_DIRECTION), 0.0, 1.0), 1.8) * (1.0 - sky_share) * 0.30;
    }
"#
}

/// The emission body, which is what is lit rather than what is lit by something: the lamp.
const EMISSION_BODY: &str = r#"
    let kind = in.material.z;
    if (kind > 11.5) {
        // A lamp: a flame with a halo. The halo is what a camera actually records of one — a
        // bright core a few pixels across and a falloff around it — and it is written as a
        // function of distance from the flame's own position rather than as the sphere's own
        // surface, so a flame mesh of any size draws the same picture.
        let halo = clamp(1.0 - distance(in.world_position, LAMP_POSITION) / 0.55, 0.0, 1.0);
        emitted = emitted + vec3<f32>(1.55, 0.82, 0.34) * (0.25 + pow(halo, 1.4) * 1.9);
    }
"#;

/// The profile of a water jar, in the scene's units, already turned into a mesh.
fn jar_profile() -> Vec<[f64; 2]> {
    JAR.iter().map(|pair| [pair[0], pair[1]]).collect()
}

/// The profile of a carrying basket.
fn basket_profile() -> Vec<[f64; 2]> {
    BASKET.iter().map(|pair| [pair[0], pair[1]]).collect()
}

/// The profile of a stone mill.
fn mill_profile() -> Vec<[f64; 2]> {
    MILL.iter().map(|pair| [pair[0], pair[1]]).collect()
}

/// The profile of a bamboo bench: a plank on two legs, which is a lathe only in the sense that
/// a bench is not, so this is a cuboid and the profile below belongs to the post cap.
fn bench_mesh() -> definition::PackedList {
    cuboid(2.6, 0.10, 0.46)
}

/// Builds every mesh the place draws.
///
/// # Invariants
///
/// A mesh here is drawn by every placement that names it, and a placement is one entry per
/// pass, so the count that costs anything is the number of **meshes**: the grove is two, the
/// yard is three, the house is nine.
fn meshes() -> Vec<Value> {
    let point = shaded_point_schema();
    let mut list = vec![
        mesh(
            "ground",
            "Triangles",
            point.clone(),
            masked_heightfield(DIVISIONS, EXTENT, terrain, yard_mask),
        ),
        mesh("roof", "Triangles", point.clone(), heightfield(56, ROOF_SLOPE, {
            let profile = roof_tile(ROOF_SLOPE);
            move |x, z| profile(x, z)
        })),
        mesh("wall", "Triangles", point.clone(), cuboid(1.0, 1.0, 1.0)),
        mesh("tile_cap", "Triangles", point.clone(), prism(1.0, 1.0, 1.0)),
        mesh("gable", "Triangles", point.clone(), prism(1.0, 1.0, 1.0)),
        mesh(
            "ridge",
            "Triangles",
            point.clone(),
            cuboid(1.0, 1.0, 1.0),
        ),
        // A bamboo culm: five internodes, a hand's width across, twelve units tall. Every
        // culm in the grove is this one mesh, scaled and turned.
        mesh("culm", "Triangles", point.clone(), culm(5, 0.105, 12.0)),
        // The spray a twig carries: nine leaves about a hand long. A bamboo leaf is a hand's
        // width and a hand's length, and at twenty units a frame pixel is a centimetre and a
        // half, so a leaf narrower than about twenty centimetres is a line rather than a leaf.
        mesh(
            "spray",
            "Triangles",
            point.clone(),
            leaf_spray(11, 1.30, 0.30, 0.22),
        ),
        // A single leaf, for the few that stand clear of the mass.
        mesh("leaf", "Triangles", point.clone(), blade(1.40, 0.32, 0.20)),
        // A clump of grass: nine blades rising from one root.
        mesh("tuft", "Triangles", point.clone(), tuft(9, 0.52, 0.42)),
        // A broadleaf, for the plants along the walls and the taro by the pond.
        mesh(
            "broadleaf",
            "Triangles",
            point.clone(),
            double_blade(0.72, 0.30, 0.16),
        ),
        mesh(
            "stone",
            "Triangles",
            point.clone(),
            sphere(7, 0.5),
        ),
        mesh("peg", "Triangles", point.clone(), cuboid(1.0, 1.0, 1.0)),
        mesh("jar", "Triangles", point.clone(), lathe(&jar_profile(), 20)),
        mesh(
            "basket",
            "Triangles",
            point.clone(),
            lathe(&basket_profile(), 16),
        ),
        mesh(
            "mill",
            "Triangles",
            point.clone(),
            lathe(&mill_profile(), 20),
        ),
        mesh("bench", "Triangles", point.clone(), bench_mesh()),
        mesh(
            "water",
            "Triangles",
            point.clone(),
            plane_facing(1.0, 0.0, true),
        ),
        mesh("flame", "Triangles", point.clone(), sphere(6, 0.035)),
        mesh("awning", "Triangles", point.clone(), cuboid(1.0, 0.02, 1.0)),
    ];
    list.push(mesh("pot", "Triangles", point, lathe(&POT, 14)));
    list
}

/// The profile of a flower pot on the veranda's end.
const POT: [[f64; 2]; 7] = [
    [0.000, 0.000],
    [0.100, 0.000],
    [0.108, 0.030],
    [0.150, 0.210],
    [0.168, 0.250],
    [0.152, 0.256],
    [0.140, 0.230],
];

/// The interior of the house, the veranda, and the yard: every placement that is not bamboo.
///
/// The bamboo is placed apart, in [`grove_instances`], because a grove is generated and this is
/// built.
fn built_instances(place: &mut Place) {
    let eave = PLINTH + WALL_HEIGHT;

    // The ground. It casts its own shadow, and that shadow is most of what the sun does here:
    // the bank behind the house lays the yard's north side into shade and the grove does the
    // rest. The row's own colour is nearly white, because the ground's colour is decided per
    // fragment by the surface — bank, paving, terrace, grass and snow are one mesh and five
    // materials' worth of arithmetic.
    let ground_kind = EARTH;
    place.solid(
        "ground",
        "ground",
        [0.0; 3],
        0.0,
        [1.0; 3],
        [1.0, 0.98, 0.94],
        0.88,
        0.02,
        ground_kind,
        true,
    );

    // The house: its plinth, four walls, two gable ends, the roof, and the ridge over it.
    place.solid(
        "plinth",
        "wall",
        [HOUSE[0], 0.0, HOUSE[2]],
        0.0,
        [HOUSE_WIDTH + 0.5, PLINTH, HOUSE_DEPTH + 0.5],
        [0.34, 0.33, 0.31],
        0.88,
        0.03,
        STONE,
        true,
    );
    for (side, at, scaled) in [
        (
            "front",
            [HOUSE[0], PLINTH, HOUSE[2] + HOUSE_DEPTH * 0.5],
            [HOUSE_WIDTH, WALL_HEIGHT, 0.55],
        ),
        (
            "back",
            [HOUSE[0], PLINTH, HOUSE[2] - HOUSE_DEPTH * 0.5],
            [HOUSE_WIDTH, WALL_HEIGHT, 0.55],
        ),
        (
            "east",
            [HOUSE[0] + HOUSE_WIDTH * 0.5, PLINTH, HOUSE[2]],
            [0.55, WALL_HEIGHT, HOUSE_DEPTH],
        ),
        (
            "west",
            [HOUSE[0] - HOUSE_WIDTH * 0.5, PLINTH, HOUSE[2]],
            [0.55, WALL_HEIGHT, HOUSE_DEPTH],
        ),
    ] {
        place.solid(
            &format!("house-{side}"),
            "wall",
            at,
            0.0,
            scaled,
            [0.60, 0.585, 0.545],
            0.90,
            0.02,
            PLASTER,
            true,
        );
    }
    // The gable ends: the triangle under each slope, in the same whitewash as the walls. They
    // stand at the wall's own line rather than out at the eave, because the roof overhangs
    // them — which is what an eave is.
    for side in [-1.0, 1.0] {
        place.solid(
            &format!("gable-{}", if side < 0.0 { "west" } else { "east" }),
            "gable",
            [HOUSE[0] + side * HOUSE_WIDTH * 0.5, eave - 0.06, HOUSE[2]],
            0.0,
            [ROOF_RISE - 0.06, HOUSE_DEPTH, 0.52],
            [0.60, 0.585, 0.545],
            0.90,
            0.02,
            PLASTER,
            true,
        );
    }
    // The roof: both slopes at once, because the mesh is a tent. Its square is scaled out to the
    // roof's width, its run and its rise — see [`roof_scale`] — and its tile channels are
    // arithmetic in the world's own space, so the stretch cannot reach them.
    place.solid(
        "roof",
        "roof",
        ROOF_RIDGE,
        0.0,
        roof_scale(),
        [0.30, 0.305, 0.315],
        0.72,
        0.05,
        TILE,
        true,
    );
    // The ridge tile: a course of half-round caps along the crest, and the slight lift at each
    // end that every roof of this kind has.
    place.solid(
        "ridge-tile",
        "ridge",
        [ROOF_RIDGE[0], ROOF_RIDGE[1] + 0.10, ROOF_RIDGE[2]],
        0.0,
        [2.0 * ROOF_HALF_WIDTH + 0.7, 0.26, 0.34],
        [0.26, 0.27, 0.28],
        0.66,
        0.06,
        TILE,
        true,
    );

    // The veranda: a stone platform under the eave, and the posts that hold the eave up.
    place.solid(
        "veranda",
        "wall",
        [HOUSE[0], 0.0, HOUSE[2] + HOUSE_DEPTH * 0.5 + 1.05],
        0.0,
        [HOUSE_WIDTH + 0.6, 0.34, 2.1],
        [0.36, 0.35, 0.33],
        0.90,
        0.03,
        STONE,
        true,
    );
    for index in 0..4 {
        let across = -5.2 + index as f64 * 3.45;
        place.solid(
            &format!("post-{index}"),
            "peg",
            [HOUSE[0] + across, 0.34, HOUSE[2] + HOUSE_DEPTH * 0.5 + 1.85],
            0.0,
            [0.17, eave - ROOF_RISE * 0.42, 0.17],
            [0.115, 0.088, 0.060],
            0.64,
            0.03,
            TIMBER,
            true,
        );
    }

    // The door: a dark timber panel in the front wall, with a frame and a threshold.
    place.solid(
        "door",
        "peg",
        [
            HOUSE[0] - 1.4,
            PLINTH + 0.02,
            HOUSE[2] + HOUSE_DEPTH * 0.5 + 0.20,
        ],
        0.0,
        [2.0, 2.5, 0.14],
        [0.085, 0.058, 0.036],
        0.60,
        0.03,
        TIMBER,
        true,
    );
    for side in [-1.0, 1.0] {
        place.solid(
            &format!("door-frame-{}", if side < 0.0 { "a" } else { "b" }),
            "peg",
            [
                HOUSE[0] - 1.4 + side * 1.12,
                PLINTH + 0.02,
                HOUSE[2] + HOUSE_DEPTH * 0.5 + 0.22,
            ],
            0.0,
            [0.18, 2.62, 0.20],
            [0.145, 0.105, 0.068],
            0.62,
            0.03,
            TIMBER,
            true,
        );
    }
    place.solid(
        "door-lintel",
        "peg",
        [
            HOUSE[0] - 1.4,
            PLINTH + 2.62,
            HOUSE[2] + HOUSE_DEPTH * 0.5 + 0.22,
        ],
        0.0,
        [2.44, 0.20, 0.22],
        [0.145, 0.105, 0.068],
        0.62,
        0.03,
        TIMBER,
        true,
    );
    // Two windows, shuttered: a dark recess, which is most of what a window is at this
    // distance, and the sill under it.
    for (index, across) in [3.3, 5.4].into_iter().enumerate() {
        place.solid(
            &format!("window-{index}"),
            "peg",
            [
                HOUSE[0] + across,
                PLINTH + 1.30,
                HOUSE[2] + HOUSE_DEPTH * 0.5 + 0.18,
            ],
            0.0,
            [1.35, 1.25, 0.12],
            [0.075, 0.055, 0.038],
            0.58,
            0.04,
            TIMBER,
            true,
        );
        place.solid(
            &format!("window-sill-{index}"),
            "peg",
            [
                HOUSE[0] + across,
                PLINTH + 0.62,
                HOUSE[2] + HOUSE_DEPTH * 0.5 + 0.24,
            ],
            0.0,
            [1.60, 0.12, 0.26],
            [0.330, 0.325, 0.305],
            0.90,
            0.03,
            STONE,
            true,
        );
    }

    // The yard wall, in panels, with the gate's opening left in the south run.
    for (index, (at, across, depth)) in wall_panels().into_iter().enumerate() {
        place.solid(
            &format!("wall-{index}"),
            "wall",
            at,
            0.0,
            [across, WALL_HIGH, depth],
            [0.545, 0.530, 0.495],
            0.88,
            0.03,
            PLASTER,
            true,
        );
        place.solid(
            &format!("wall-cap-{index}"),
            "tile_cap",
            [at[0], at[1] + WALL_HIGH, at[2]],
            0.0,
            [across + 0.34, 0.24, depth + 0.34],
            [0.255, 0.265, 0.275],
            0.70,
            0.05,
            TILE,
            true,
        );
    }

    // The gate: a step through, two posts, a lintel, and a roof over the whole of it.
    place.solid(
        "gate-step",
        "wall",
        [GATE[0], 0.0, GATE[1]],
        0.0,
        [2.9, 0.16, 1.6],
        [0.38, 0.37, 0.35],
        0.90,
        0.03,
        STONE,
        true,
    );
    for side in [-1.0, 1.0] {
        place.solid(
            &format!("gate-post-{}", if side < 0.0 { "a" } else { "b" }),
            "peg",
            [GATE[0] + side * 1.30, 0.16, GATE[1]],
            0.0,
            [0.26, 2.50, 0.26],
            [0.115, 0.085, 0.058],
            0.62,
            0.03,
            TIMBER,
            true,
        );
    }
    place.solid(
        "gate-lintel",
        "peg",
        [GATE[0], 2.48, GATE[1]],
        0.0,
        [3.2, 0.24, 0.30],
        [0.115, 0.085, 0.058],
        0.62,
        0.03,
        TIMBER,
        true,
    );
    place.solid(
        "gate-roof",
        "gable",
        [GATE[0], 2.72, GATE[1]],
        std::f64::consts::FRAC_PI_2,
        [0.95, 3.5, 0.9],
        [0.235, 0.245, 0.255],
        0.70,
        0.05,
        TILE,
        true,
    );

    // The pond: a rim of rough stones, the water inside it, and what grows at its edge.
    for index in 0..18 {
        let angle = index as f64 / 18.0 * std::f64::consts::TAU;
        let x = POND[0] + angle.cos() * POND_REACH[0] * 0.94;
        let z = POND[1] + angle.sin() * POND_REACH[1] * 0.94;
        let variety = hash01(index, 3, 211);
        place.solid(
            &format!("rim-{index}"),
            "stone",
            [x, 0.20 + variety * 0.06, z],
            variety * 3.0,
            [
                0.42 + variety * 0.16,
                0.36 + variety * 0.12,
                0.42 + hash01(index, 7, 213) * 0.16,
            ],
            [
                0.395 + variety * 0.06,
                0.385 + variety * 0.06,
                0.360 + variety * 0.05,
            ],
            0.92,
            0.02,
            STONE,
            true,
        );
    }
    // The water: an ellipse a finger's width below the rim's top. It casts nothing — a mirror
    // is not an occluder — and its rows are white, because every colour the water has is the
    // sky it is reflecting.
    place.solid(
        "pond",
        "water",
        [POND[0], POND_DEPTH, POND[1]],
        0.0,
        [POND_REACH[0] * 0.92, 1.0, POND_REACH[1] * 0.92],
        [0.055, 0.075, 0.070],
        0.12,
        1.0,
        WATER,
        false,
    );
    for index in 0..5 {
        let angle = hash01(index, 11, 217) * std::f64::consts::TAU;
        place.solid(
            &format!("pond-stone-{index}"),
            "stone",
            [
                POND[0] + angle.cos() * POND_REACH[0] * 0.5,
                0.24,
                POND[1] + angle.sin() * POND_REACH[1] * 0.5,
            ],
            angle,
            [0.30, 0.26, 0.30],
            [0.33, 0.33, 0.31],
            0.94,
            0.02,
            STONE,
            false,
        );
    }
    for index in 0..11 {
        let angle = hash01(index, 13, 219) * std::f64::consts::TAU;
        place.plant(
            &format!("taro-{index}"),
            "broadleaf",
            [
                POND[0] + angle.cos() * POND_REACH[0] * 1.08,
                0.20,
                POND[1] + angle.sin() * POND_REACH[1] * 1.08,
            ],
            hash01(index, 17, 221) * std::f64::consts::TAU,
            [1.0 + hash01(index, 19, 223) * 0.5; 3],
            [0.135, 0.215, 0.090],
            0.72,
            LEAF,
        );
    }

    // The jar the yard draws its water from, on the veranda's east end.
    place.solid(
        "jar",
        "jar",
        [8.4, 0.34, -6.2],
        0.4,
        [1.05, 1.05, 1.05],
        [0.235, 0.175, 0.120],
        0.52,
        0.06,
        TILE,
        true,
    );
    // Two baskets by the door, one stacked on the other.
    place.solid(
        "basket-a",
        "basket",
        [-6.6, 0.34, -6.0],
        0.2,
        [1.0, 1.0, 1.0],
        [0.520, 0.430, 0.255],
        0.80,
        0.02,
        CLOTH,
        true,
    );
    place.solid(
        "basket-b",
        "basket",
        [-6.35, 0.74, -5.75],
        1.1,
        [0.86, 0.86, 0.86],
        [0.470, 0.385, 0.225],
        0.80,
        0.02,
        CLOTH,
        true,
    );
    // The mill, in the yard's north-east corner, where the grain is ground.
    place.solid(
        "mill",
        "mill",
        [9.6, 0.0, -8.4],
        0.0,
        [1.0, 1.0, 1.0],
        [0.430, 0.420, 0.400],
        0.90,
        0.02,
        STONE,
        true,
    );
    // The bench against the west wall, and the rack the washing dries on.
    place.solid(
        "bench",
        "bench",
        [-11.2, 0.42, -3.0],
        std::f64::consts::FRAC_PI_2,
        [1.0; 3],
        [0.255, 0.180, 0.105],
        0.70,
        0.03,
        TIMBER,
        true,
    );
    for (label, z) in [("a", -4.05), ("b", -1.95)] {
        place.solid(
            &format!("bench-leg-{label}"),
            "peg",
            [-11.2, 0.0, z],
            0.0,
            [0.13, 0.42, 0.13],
            [0.150, 0.105, 0.060],
            0.68,
            0.03,
            TIMBER,
            true,
        );
    }
    for index in 0..2 {
        place.solid(
            &format!("rack-post-{index}"),
            "peg",
            [-2.0 + index as f64 * 5.6, 0.0, 7.6],
            0.0,
            [0.16, 2.30, 0.16],
            [0.130, 0.092, 0.058],
            0.66,
            0.03,
            TIMBER,
            true,
        );
    }
    place.solid(
        "rack-bar",
        "peg",
        [0.8, 2.24, 7.6],
        0.0,
        [6.2, 0.10, 0.10],
        [0.130, 0.092, 0.058],
        0.66,
        0.03,
        TIMBER,
        true,
    );
    // The washing on the rack: three panels, hanging, which is the one thing in the yard that
    // moves and the one thing here that is white for the light to be read off.
    for index in 0..3 {
        place.solid(
            &format!("cloth-{index}"),
            "awning",
            [-1.4 + index as f64 * 2.2, 1.15, 7.6],
            0.05 * (index as f64 - 1.0),
            [1.5, 2.1, 0.05],
            [0.560, 0.545, 0.500],
            0.94,
            0.0,
            CLOTH,
            true,
        );
    }
    // The lamp: a post, an arm, a hood, and the flame under the hood.
    place.solid(
        "lamp-post",
        "peg",
        [LAMP[0] - 0.55, 0.0, LAMP[2] - 0.35],
        0.0,
        [0.14, LAMP[1] + 0.45, 0.14],
        [0.140, 0.100, 0.062],
        0.66,
        0.03,
        TIMBER,
        true,
    );
    place.solid(
        "lamp-arm",
        "peg",
        [LAMP[0] - 0.28, LAMP[1] + 0.30, LAMP[2] - 0.18],
        0.7,
        [0.66, 0.08, 0.08],
        [0.140, 0.100, 0.062],
        0.66,
        0.03,
        TIMBER,
        true,
    );
    place.solid(
        "lamp-hood",
        "awning",
        [LAMP[0], LAMP[1] + 0.15, LAMP[2]],
        0.7,
        [0.44, 0.26, 0.44],
        [0.245, 0.190, 0.110],
        0.44,
        0.14,
        METAL,
        true,
    );
    // The flame: the one thing in the place that is not lit by anything.
    let flame_material = place.declare("flame", "emission", material([1.0; 3], 0.5, 0.0, 12.0));
    place.emission.push(instance(
        "flame",
        "flame",
        LAMP,
        [0.0; 3],
        [1.0, 1.35, 1.0],
        &flame_material,
    ));

    // A pot of something on the veranda's west end: the one thing in the yard that is only
    // there because a person put it there, which is what a photograph of a courtyard is of.
    place.solid(
        "pot",
        "pot",
        [-8.9, 0.34, -6.1],
        0.9,
        [1.0; 3],
        [0.290, 0.170, 0.115],
        0.62,
        0.04,
        TILE,
        true,
    );
    for index in 0..5 {
        let angle = index as f64 * 1.257 + 0.3;
        place.plant(
            &format!("pot-leaf-{index}"),
            "broadleaf",
            [
                -8.9 + angle.cos() * 0.10,
                0.58 + hash01(index, 23, 227) * 0.16,
                -6.1 + angle.sin() * 0.10,
            ],
            angle * 2.0,
            [1.2, 1.4, 1.2],
            [0.165, 0.235, 0.105],
            0.70,
            LEAF,
        );
    }

    // Ground cover inside the yard: tufts along the walls, in the corners, and along the
    // pond's bank — everywhere the paving stops. None of it in the middle of the yard, because
    // a yard that is swept is swept, and that is the one thing that says a person lives here.
    let mut tuft = 0;
    for index in 0..240i64 {
        let x = (hash01(index, 3, 301) - 0.5) * 2.0 * (WALL_REACH[0] + 6.0);
        let z = (hash01(index, 7, 303) - 0.5) * 2.0 * (WALL_REACH[1] + 6.0);
        let edge = yard_distance(x, z);
        if edge < 1.8 {
            continue;
        }
        if edge > 17.0 {
            continue;
        }
        let pond = ((x - POND[0]) / (POND_REACH[0] * 1.05)).powi(2)
            + ((z - POND[1]) / (POND_REACH[1] * 1.05)).powi(2);
        if pond < 1.0 {
            continue;
        }
        if slope(x, z) > 1.25 {
            continue;
        }
        tuft += 1;
        let scale = 0.7 + hash01(index, 13, 311) * 0.9;
        place.plant(
            &format!("tuft-{tuft}"),
            "tuft",
            [x, terrain(x, z) - 0.07, z],
            hash01(index, 17, 313) * std::f64::consts::TAU,
            [scale, scale * (0.8 + hash01(index, 19, 317) * 0.7), scale],
            [
                0.100 + 0.05 * hash01(index, 23, 319),
                0.200 + 0.10 * hash01(index, 29, 323),
                0.075 + 0.04 * hash01(index, 31, 327),
            ],
            0.82,
            GRASS,
        );
    }
}

/// Where the yard wall's panels stand: one entry per panel, with the gate's opening left out.
///
/// The wall is the yard's own rectangle, a shade outside the paving, and the south run is split
/// into two panels with the gate's own width between them.
fn wall_panels() -> Vec<([f64; 3], f64, f64)> {
    let (x, z) = (WALL_REACH[0], WALL_REACH[1]);
    let gate_near = GATE[0] - 1.55;
    let gate_far = GATE[0] + 1.55;
    vec![
        // North.
        ([0.0, 0.0, -z], 2.0 * x + 0.5, 0.42),
        // East.
        ([x, 0.0, 0.0], 0.42, 2.0 * z),
        // West.
        ([-x, 0.0, 0.0], 0.42, 2.0 * z),
        // South, west of the gate.
        (
            [(-x + gate_near) * 0.5, 0.0, z],
            gate_near + x,
            0.42,
        ),
        // South, east of the gate.
        (
            [(x + gate_far) * 0.5, 0.0, z],
            x - gate_far,
            0.42,
        ),
    ]
}

/// Places the grove: every culm, every branch and every spray of leaves.
///
/// # Invariants
///
/// A culm's base is the ground's own height where it stands, and its own scale is the clump's
/// height times a hash, so a clump is a stand of stems of different ages — which is what bamboo
/// is, because a runner puts up a new stem every year and they do not all reach the same
/// height.
///
/// The leaves hang in the **top third** of each culm, at the height the stem's own bareness
/// runs out. Everything above the ground and below that is stem, and a grove drawn without
/// that is a hedge.
fn grove_instances(place: &mut Place) {
    let mut index = 0i64;
    for (clump_index, clump) in grove().into_iter().enumerate() {
        // The two clumps that are furthest from every camera: the light they are drawn with is
        // a fifth of the grove's cost and none of the frame's, so they are drawn by the
        // shading stage and cast nothing.
        let far = clump_index >= 20;
        for _culm_index in 0..clump.culms {
            index += 1;
            let angle = hash01(index, 3, 401) * std::f64::consts::TAU;
            let reach = clump.radius * hash01(index, 7, 403).sqrt();
            // A culm leans outwards from the clump's own centre as it rises, which is what a
            // stand of them does as it grows towards the light.
            let x = clump.x + angle.cos() * reach;
            let z = clump.z + angle.sin() * reach;
            let ground = terrain(x, z);
            let height = clump.height
                * (0.52 + 0.55 * hash01(index, 11, 407))
                * if far { 0.85 } else { 1.0 };
            let lean = (0.02 + 0.055 * hash01(index, 13, 409)) * clump.radius.sqrt();
            let turn = hash01(index, 17, 411) * std::f64::consts::TAU;
            let stem = [x, ground - 0.12, z];
            // The mesh is a culm of twelve units, so the scale is how tall this one is over
            // twelve, and the lean is two rotations about the foot.
            let scale = height / 12.0;
            let rotation = [lean * turn.sin(), turn, lean * turn.cos()];
            let tint = [
                0.175 + 0.075 * hash01(index, 19, 413),
                0.205 + 0.070 * hash01(index, 23, 417),
                0.085 + 0.040 * hash01(index, 29, 419),
            ];
            place.solid(
                &format!("culm-{index}"),
                "culm",
                stem,
                rotation[1],
                [scale, scale, scale],
                tint,
                0.42,
                0.16,
                BAMBOO,
                !far,
            );
            // The top of the culm, in the world, for the leaves: the lean moves it.
            let top = [
                x + lean * turn.sin() * height * 0.5,
                ground - 0.12 + height * 0.965,
                z + lean * turn.cos() * height * 0.5,
            ];
            // Two to four sprays per culm, in its top third.
            let sprays = 2 + (hash01(index, 31, 421) * 3.0) as usize;
            for spray in 0..sprays {
                let fraction = 1.0 - (spray as f64 + 0.4) / (sprays as f64 + 0.2);
                let share = clump.leafy + (1.0 - clump.leafy) * fraction * 0.85;
                let at = [
                    lerp(stem[0], top[0], share),
                    lerp(stem[1], top[1], share),
                    lerp(stem[2], top[2], share),
                ];
                let side = hash01(index * 7 + spray as i64, 37, 423) * std::f64::consts::TAU;
                let out = 0.10 + 0.35 * hash01(index * 7 + spray as i64, 41, 427);
                let size = scale * (0.80 + 0.45 * hash01(index * 7 + spray as i64, 43, 429));
                place.plant(
                    &format!("spray-{index}-{spray}"),
                    "spray",
                    [at[0] + side.cos() * out, at[1], at[2] + side.sin() * out],
                    side,
                    [size, size, size],
                    [
                        0.115 + 0.055 * hash01(index * 7 + spray as i64, 47, 431),
                        0.180 + 0.075 * hash01(index * 7 + spray as i64, 53, 433),
                        0.070 + 0.035 * hash01(index * 7 + spray as i64, 59, 437),
                    ],
                    0.74,
                    LEAF,
                );
            }
            // And a few single leaves standing clear of the mass, which is what the sun comes
            // through a grove as.
            for leaf in 0..2 {
                let seed = index * 13 + leaf as i64;
                let share = clump.leafy + 0.10 + 0.22 * hash01(seed, 61, 439);
                let side = hash01(seed, 67, 443) * std::f64::consts::TAU;
                place.plant(
                    &format!("leaf-{index}-{leaf}"),
                    "leaf",
                    [
                        lerp(stem[0], top[0], share) + side.cos() * 0.30,
                        lerp(stem[1], top[1], share),
                        lerp(stem[2], top[2], share) + side.sin() * 0.30,
                    ],
                    side,
                    [scale * 1.1; 3],
                    [
                        0.135 + 0.060 * hash01(seed, 71, 449),
                        0.205 + 0.080 * hash01(seed, 73, 457),
                        0.080 + 0.040 * hash01(seed, 79, 461),
                    ],
                    0.74,
                    LEAF,
                );
            }
        }
    }
}

/// The shaders the place declares: one of each stage, over the scene's own row.
///
/// # Why one shader and not one per surface
///
/// A pass draws one instance, so one shader per material is one pass per material, and a grove
/// of six hundred placements is six hundred passes. What tells a stone from a whitewashed wall
/// is not a different *programme* either — the arithmetic is a noise and a modulate for both —
/// so the choice is a number in the row and the bodies branch on it. It is the same trade
/// `tiles` makes with its image and `stilllife` makes with its six surface bodies; the
/// difference is only how many surfaces there are.
///
/// # Why the post stage is handed more than the others
///
/// [`helpers`] is what every stage of this moment is written against. The post stage is handed
/// [`POST_HELPERS`] beside it, because both of those helpers rebuild a ray through a pixel from
/// the camera's basis and only a post stage has a basis.
fn build_shaders(helpers: &str) -> Vec<Value> {
    let row = custom_schema();
    let mut list = crate::shaders::placement_shaders_with_schema("courtyard", row.clone());
    list.push(crate::shaders::lighting_shader_with_schema(
        "lighting",
        "",
        surface_body(),
        helpers,
        row.clone(),
    ));
    list.push(crate::shaders::emission_shader_with_schema(
        "emission",
        EMISSION_BODY,
        helpers,
        row,
    ));
    list.push(post_shader(GRADE, &format!("{helpers}\n{}", POST_HELPERS)));
    list
}

/// The lights of a moment: the sun, the fill, and the lamp.
///
/// # Invariants
///
/// The sun is a **directional** light and not a point standing far away, because that is what a
/// sun is: it reaches every distance equally, it takes no cluster slot, and its shadow is
/// parallel. A point light far enough away to look parallel is a point light whose own falloff
/// has already made the far end of the valley darker than the near end.
///
/// The fill is a point light with a reach that holds the whole place, so the compiler promotes
/// it to a **global** light: in no cluster list, evaluated by every fragment. That is what
/// keeps the shadow side of a wall readable rather than black, and a fill that was culled per
/// cluster would draw its own terminator as a staircase of grid rectangles.
///
/// The lamp is a point light with a short reach, because it is a lamp: it lights the stones
/// around it and nothing else, and its own falloff is the whole of what it is.
fn lights(tone: &Tone, moment: &str) -> Vec<Value> {
    let mut list = vec![
        directional(
            "sun",
            tone.sun,
            tone.sun_colour,
            tone.sun_intensity,
            // The page is a box around the middle of everything drawn, and everything drawn
            // reaches about seventy units from the origin: the grove and the far terraces. A
            // page narrower than that reads as **unshadowed** outside it rather than as an
            // error, which is a frame whose shadows stop at a circle nobody can explain.
            96.0,
        ),
        light_with_radius(
            "sky",
            tone.fill_position,
            tone.fill,
            tone.fill_intensity * 700.0,
            tone.fill_shadow,
            tone.fill_reach,
        ),
    ];
    let lamp = lamp_of(moment);
    if lamp > 0.01 {
        list.push(light_with_radius(
            "lamp",
            [LAMP[0], LAMP[1] + 0.06, LAMP[2]],
            [1.0, 0.55, 0.22],
            lamp * 26.0,
            9.0,
            15.0,
        ));
    }
    list
}

/// Builds the whole document for one moment.
fn build(moment: &str) -> Value {
    let tone = tone(moment);
    // The module level WGSL every stage of this moment is handed: the air and the sky, the
    // roof's own constants, the noise and the ridge lines, and the per-material arithmetic the
    // lighting body branches into. It is one string because a scene has nowhere else to put
    // one: `WGSL` has no nested functions, so a body cannot carry its own helpers.
    let helpers = format!(
        "{}{SURFACES}",
        tone.wgsl(lamp_of(moment), wetness_of(moment), mist_of(moment))
    );
    let look = eye(moment);

    let mut place = Place::new();
    built_instances(&mut place);
    grove_instances(&mut place);
    let (materials, instances) = place.build();

    Value::structure([
        ("name".to_owned(), Value::String(moment.to_owned())),
        ("time".to_owned(), Value::Float(0.0)),
        (
            "frame".to_owned(),
            Value::structure([
                ("width".to_owned(), Value::Int(1536)),
                ("height".to_owned(), Value::Int(1024)),
            ]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed_on_lens(
                look.at,
                look.look,
                look.focal,
                look.sensor,
                look.roll,
                0.55,
                2800.0,
            )]),
        ),
        ("lights".to_owned(), Value::List(lights(&tone, moment))),
        ("shaders".to_owned(), Value::List(build_shaders(&helpers))),
        ("meshes".to_owned(), Value::List(meshes())),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("develop", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["develop"])),
        ("instances".to_owned(), instances),
    ])
}

/// Builds one moment of the courtyard.
///
/// An unknown moment lists the ones that exist rather than panicking with a backtrace, because
/// the caller is usually a command line.
pub fn courtyard(moment: &str) -> Value {
    if !MOMENTS.contains(&moment) {
        unimplemented!(
            "the courtyard is drawn in {}, and `{moment}` is not one of them",
            MOMENTS.join(", ")
        );
    }
    build(moment)
}
