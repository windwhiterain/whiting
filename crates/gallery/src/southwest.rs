//! The trees of the Hengduan: one species at a time, each drawn to the bar the proposal sets.
//!
//! # What this module is
//!
//! [`.agents/proposals/southwest-china-trees.md`](../../../.agents/proposals/southwest-china-trees.md)
//! names ten species of the mountains of Southwest China and a bar each must clear; every species here
//! is one arc of that work, and every number in it cites the species' own note in
//! [`.agents/investigate/trees/`](../../../.agents/investigate/trees/), which cites the Flora of China
//! and the photographs in `out/refs/<slug>/`. A species that has not been drawn has not been built, so
//! each one here arrives with its plates, a framing and a palette of its own.
//!
//! # Two plates and two foliages
//!
//! A species arrives as two plates because the bar asks two questions at two distances: the whole tree,
//! for the silhouette, and one bough at arm's length, for the foliage. They cannot share one foliage:
//! the rasteriser samples a pixel at one point, so a blade narrower than a pixel is dropped rather than
//! faded, and the note's 1.1 mm needle is a tenth of a pixel at the distance a whole tree is framed. The
//! whole-tree plate therefore draws [`Foliage::Spray`] — a plume-card whose comb of teeth is one to two
//! narrowest that survives — and the bough plate draws [`Foliage::Needle`], the true one. That is a
//! level of detail, not a fudge: each plate is honest about the blade it draws, and the module says which
//! is which.

use definition::Value;
use vegetation::decorator::{Seat, Template, scatter};
use vegetation::derive::Species;
use vegetation::field::Field;
use vegetation::handle::Handle;
use vegetation::leaf::{LeafField, LeafTemplate};
use vegetation::plant::Placement;
use vegetation::rng::Rng;
use vegetation::species::{Generation, Joint, Rule, grow, spine, taper};

use crate::air::Atmosphere;
use crate::vegetation_mesh::node_ids;
use crate::vegetation_picture::{
    Framing, Ground, Lighting, Palette, Planting, Skin, plant_scene_lit,
};

/// The name the pine's base plate is built and rendered under: the foot of the bole at two metres,
/// which is the frame H4's flare and bark are read in.
pub const YUNNAN_PINE_BASE: &str = "yunnan_pine_base";

/// The name the Yunnan pine's whole-tree plate is built and rendered under.
pub const YUNNAN_PINE: &str = "yunnan_pine";

/// The name the pine's branch plate is built and rendered under.
pub const YUNNAN_PINE_BOUGH: &str = "yunnan_pine_bough";

/// The seed both plates' leaf fields are derived from.
const SEED: u64 = 21;

/// The golden angle as a fraction of a turn, which is the step between two whorls.
const GOLDEN: f32 = 0.381_966_01;

/// The note's needle, half of its 1–1.2 mm across: the blade the bough plate draws.
const NEEDLE_HALF_WIDTH: f32 = 0.00055;

/// Which foliage a pine draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foliage {
    /// The note's own needle, for frames within arm's length of a shoot.
    Needle,
    /// A spray: one blade whose width law is a comb of needle-teeth, standing for a fascicle-plume.
    ///
    /// The whole-tree plate cannot draw two hundred thousand true needles — the scene document pays per
    /// instance, not per triangle — and a true needle at that distance is a tenth of a pixel, which a
    /// point-sampled rasteriser drops whole. A spray is the level of detail a conifer crown actually
    /// has at twenty-five metres: the eye resolves plumes, not needles, and the comb's teeth are one to
    /// two pixels each, which is exactly the grain the reference photographs show at that distance.
    Spray,
    /// The spray at a companion's density: a tree on a site frame's far ridge is the same rule at a
    /// third of the hero's foliage, because a stand reads by its masses and not by its sprigs, and the
    /// scene compile pays for every row a site frame carries.
    Stand,
}

impl Foliage {
    /// The blade this foliage instances.
    fn template(self) -> LeafTemplate {
        match self {
            Foliage::Needle => needle(NEEDLE_HALF_WIDTH),
            Foliage::Spray | Foliage::Stand => spray(),
        }
    }

    /// The fraction of the needle foliage's density this foliage is drawn at: a spray carries a whole
    /// plume's needles in one instance, so it needs a plume's-worth fewer of them.
    fn density_factor(self) -> f32 {
        match self {
            Foliage::Needle => 1.0,
            Foliage::Spray => 0.14,
            Foliage::Stand => 0.03,
        }
    }
}

/// The spray card: a forty-five centimetre blade whose width law is a comb, so its silhouette is a
/// needle-plume's and not a paddle's. The teeth are three centimetres deep on a two-centimetre pitch,
/// which at the whole-tree plate's seven and a half millimetres a pixel is one to two pixels a tooth —
/// the grain `01-habit.jpg` shows at the same distance.
fn spray() -> LeafTemplate {
    let teeth = 34;
    let knots: Vec<(f32, f32)> = (0..=2 * teeth)
        .map(|i| {
            let t = i as f32 / (2 * teeth) as f32;
            // The plume swells to its middle and runs out at both ends, so a spray is a plume and not a
            // rectangle; the comb multiplies that envelope. The sine is clamped at nought because the
            // f32 pi overshoots the true one and its sine at t = 1 is a small NEGATIVE number, whose
            // fractional power is not a number at all.
            let envelope = (std::f32::consts::PI * t).sin().max(0.0).powf(0.7);
            let tooth = if i % 2 == 0 { 0.10 } else { 1.0 };
            (t, 0.028 * envelope * tooth)
        })
        .collect();
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [0.22, 0.01, 0.0]),
            ([0.22, 0.02, 0.005], [0.22, 0.02, 0.005]),
            ([0.45, 0.06, 0.015], [0.20, 0.03, 0.005]),
        ]),
        shape::Scalar::new(&knots).expect("a comb is a law"),
        0.0006,
        24,
        4,
    )
}

/// The pine's rule: a clean bole, whorled boughs that shorten with height, and twigs that carry the
/// fascicles.
///
/// # The numbers and where they come from
///
/// Every figure is [`pinus_yunnanensis.md`](../../../.agents/investigate/trees/pinus_yunnanensis.md)'s
/// modelling table, drawn at a sixteen-metre tree rather than the note's estimated twenty-five to
/// thirty, so that one frame holds the whole crown: the clean bole to 0.44 of the height, four to five
/// whorls of four to five boughs above it, boughs at 55–75° from the leader shortening toward the top,
/// and twigs of ten to twenty centimetres — here thirty, because a twig that carries the note's forty
/// fascicles is the unit the crown reads in.
fn yunnan_pine_rule() -> Rule {
    Rule {
        trunk: bole,
        generations: vec![
            // **The whorls.** Four to five boughs at one station, four to five stations up the bole's
            // upper half: `cluster` is the whorl and the band is the crown's depth. The slot step is the
            // golden angle between whorls and a quarter turn within one, which is what a whorl is.
            Generation {
                count: (24, 28),
                station: (0.44, 0.97),
                slot: (0.0, GOLDEN),
                jitter: (0.05, 0.06),
                sink: 0.03,
                cluster: 5,
                shape: bough,
            },
            twig_generation(0.25),
        ],
        // A pine's bough is a flat plume, not a cylinder of twigs: the light from above turns every
        // twig's tip up, which is what stacks a bough's foliage into the plate the crown is made of.
        fields: vec![Field::light([0.0, 1.0, 0.0], 0.12)],
    }
}

/// The twig generation: the short shoots the fascicles sit on, over a bough's outer three quarters. A
/// bough carries two to three dozen of them because a crown's mass is the number of its twigs.
fn twig_generation(from: f32) -> Generation {
    Generation {
        count: (26, 30),
        station: (from, 1.0),
        slot: (0.0, GOLDEN),
        jitter: (0.12, 0.12),
        sink: 0.008,
        cluster: 1,
        shape: twig,
    }
}

/// The bole: sixteen metres, a flare at the ground and a clean taper to the crown, with the slight lean
/// the photographs show on a tree that stands in the open.
fn bole() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.0, 0.0, 0.0], [0.03, 1.0, 0.0]),
            ([0.06, 5.0, 0.02], [0.05, 1.0, 0.02]),
            ([0.14, 10.5, 0.05], [0.06, 1.0, 0.03]),
            ([0.24, 16.0, 0.08], [0.05, 1.0, 0.02]),
        ]),
        // The flare is two knots at the base and nothing else: a root plate is arc 6's question, and a
        // trunk that meets the ground on a cone is the *floating tree* defect the bar names.
        taper(&[
            (0.0, 0.46),
            (0.015, 0.36),
            (0.08, 0.30),
            (0.44, 0.20),
            (0.72, 0.13),
            (1.0, 0.05),
        ]),
    )
}

/// The bole's section: sixteen sides with eight shallow fissures between them, which is what
/// *"brown-gray, longitudinally fissured, scaly"* is in geometry — vertical ridges, because a fissure
/// runs with the grain and not around it. The taper multiplies it, so the fissures narrow as the bole
/// does.
fn bark() -> shape::Section {
    bark_of(12, 3.0, 0.13)
}

/// A fissured ring of `sides` points with `fissures` ridges around it, each `depth` of the radius deep.
///
/// # Invariants
///
/// **The side count is the surface's scale, and a rose is not.** At the bole's 45 cm radius twelve
/// sides is finer than a pixel at every frame this module draws, so twelve is right and free. A bough
/// at arm's length is 5 cm across, where twelve sides is a visible dodecagon — the *polygonal
/// silhouette* defect drawn in the plate that exists to show bark — so the bough plate asks for
/// twenty-four. `vegetation::expand::wood` samples at the finer of the dial and the authored corners,
/// so the section's own count is what the frame gets.
fn bark_of(sides: usize, fissures: f32, depth: f32) -> shape::Section {
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let ridge = 1.0 - depth * (theta * fissures).sin().powi(2);
            [theta.cos() * ridge, theta.sin() * ridge]
        })
        .collect();
    shape::Section::outline(&points).expect("a fissured ring is a section")
}

/// A twig at arm's length: the same droop as [`twig`] and the note's own shoot diameter instead of a
/// fraction of its parent's.
///
/// # Invariants
///
/// **The whole-tree law is a fraction of the parent and a bough plate's law is the note's number.** On
/// a tree a shoot is 45 % of the radius of the bough it leaves, which at a bough's far end is one
/// centimetre. On the bough plate the bough's own base is the plate's "trunk", so that fraction makes
/// every shoot two centimetres and the wood, not the needles, becomes the subject. The plate authors
/// the note's own 8–11 mm shoot instead, and the crown's law is untouched.
fn twig_at_arm_length(rng: &mut Rng, _joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.46 * (0.85 + 0.30 * rng.unit());
    let departure = 1.05 + 0.30 * rng.signed();
    (
        bend(
            length,
            [
                departure,
                departure + 0.22,
                departure + 0.36,
                departure + 0.46,
            ],
        ),
        taper(&[(0.0, 0.011), (0.35, 0.008), (1.0, 0.0035)]),
    )
}

/// A first-order bough: it leaves the leader at the note's 55–75°, runs out and slightly down, and turns
/// up at the tip, which is the upsweep the mature crown is made of. Its length is a law over the station
/// it hangs at — the crown's own profile — which is what [`Joint::station`] exists for.
fn bough(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    // The top quarter of the crown shortens again: a mature crown is flat-topped, and a bough law that
    // only falls with height draws a spire.
    let top = ((joint.station - 0.78) / 0.22).clamp(0.0, 1.0);
    let length =
        5.6 * (1.0 - 0.35 * joint.station) * (1.0 - 0.9 * top) * (0.75 + 0.50 * rng.unit());
    let departure = 1.45 - 0.55 * joint.station + 0.10 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.16,
                departure + 0.06,
                departure - 0.22,
            ],
        ),
        taper(&[(0.0, radius * 0.34), (0.5, radius * 0.18), (1.0, 0.012)]),
    )
}

/// A twig: the short shoot a fascicle sits on, ten to twenty centimetres in the note and thirty here,
/// leaving the bough at about a right angle and drooping a little under its own needles.
fn twig(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.70 * (0.80 + 0.40 * rng.unit());
    let departure = 1.05 + 0.25 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.18,
                departure + 0.30,
                departure + 0.38,
            ],
        ),
        taper(&[(0.0, radius * 0.45), (1.0, 0.0035)]),
    )
}

/// Four knots along a heading that turns through `angles`, measured from the parent's own tangent: nought
/// is along the parent and a quarter turn is straight out. The tangents are the heading's own direction,
/// so a bough is a chord with three bends in it rather than a straight line with a direction.
fn bend(length: f32, angles: [f32; 4]) -> shape::Curve {
    let step = length / 3.0;
    let at = |angle: f32| [step * angle.sin(), step * angle.cos(), 0.0];
    let tangent = |angle: f32| [length * 0.5 * angle.sin(), length * 0.5 * angle.cos(), 0.0];
    let first = at(angles[0]);
    let second = [
        first[0] + at(angles[1])[0],
        first[1] + at(angles[1])[1],
        0.0,
    ];
    let third = [
        second[0] + at(angles[2])[0],
        second[1] + at(angles[2])[1],
        0.0,
    ];
    spine(&[
        ([0.0, 0.0, 0.0], tangent(angles[0])),
        (first, tangent(angles[1])),
        (second, tangent(angles[2])),
        (third, tangent(angles[3])),
    ])
}

/// A needle blade of `half_width` half-width: twenty-two centimetres of sickle-curved blade, which is the
/// note's 7–30 cm × 1–1.2 mm drawn at the middle of its range and bent the way *"flabellate-triangular,
/// slightly sickle-curved"* reads in a photograph.
fn needle(half_width: f32) -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [0.11, 0.004, 0.0]),
            ([0.11, 0.010, 0.004], [0.11, 0.016, 0.004]),
            ([0.22, 0.034, 0.010], [0.10, 0.022, 0.004]),
        ]),
        taper(&[
            (0.0, half_width * 0.7),
            (0.5, half_width),
            (1.0, half_width * 0.2),
        ]),
        half_width * 0.7,
        5,
        4,
    )
}

/// The cone: the note's 5–11 cm conical-ovoid seed cone on a short peduncle that turns down, because a
/// seat expresses no rotation — the hanging is the template's own shape, drawn once, and every cone the
/// scatter seats then hangs the same way from whatever side of the twig it lands on.
fn cone() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0, 0.0, 0.0], [0.014, 0.0, 0.0]),
            ([0.014, -0.006, 0.0], [0.003, -0.020, 0.0]),
            ([0.016, -0.036, 0.0], [0.0, -0.032, 0.0]),
            ([0.016, -0.068, 0.0], [0.0, -0.020, 0.0]),
        ]),
        shape::Section::circle(),
    );
    sweep.scale = taper(&[
        (0.0, 0.004),
        (0.25, 0.013),
        (0.55, 0.019),
        (0.85, 0.015),
        (1.0, 0.003),
    ]);
    Template {
        sweep,
        stations: 8,
        around: 9,
    }
}

/// The needle fields at `twig_depth`: the leader's candle, a fuzz on the young outer boughs, the sprays
/// over a twig's outer three quarters, and the upright brush at a twig's tip that `04-branch.jpg` shows
/// on every shoot in the crown. Four fields and one template, because a needle is one blade wherever it
/// hangs and only its distribution changes.
///
/// The card foliage is sparser than the needle by [`Foliage::density_factor`], and the reason is not
/// taste but two taxes the rasteriser levies: a blade narrower than a pixel is dropped whole, so the
/// needle's crown is a fifth as dense as its geometry says, and a blade that *is* a pixel wide shades
/// every pixel it covers, so a quarter of a million cards cost a quarter of an hour. The card's density
/// is the count whose total blade area equals the crown's own frontal area — dense enough to be a mass
/// after the sampling tax, sparse enough to render — and the card's width is what pays that tax.
fn needles(twig_depth: usize, foliage: Foliage) -> Vec<LeafField> {
    let factor = foliage.density_factor();
    // A needle foliage carries its needles in fascicles of three, because that is what a pine's sheath
    // holds; a spray is already a plume, and a cluster of plumes is a pom-pom.
    let per_station = match foliage {
        Foliage::Needle => 3,
        Foliage::Spray | Foliage::Stand => 1,
    };
    let mut candle = LeafField::new(0, 0);
    candle.band = (0.86, 1.0);
    candle.density = shape::Scalar::constant(60.0 * factor);
    candle.cluster = per_station;
    candle.fan = shape::Scalar::constant(0.50);
    candle.phyllotaxy = GOLDEN;
    candle.pitch = shape::Scalar::constant(0.35);
    candle.size = shape::Scalar::constant(0.70);
    candle.jitter = 0.10;
    let mut fuzz = LeafField::new(twig_depth - 1, 0);
    fuzz.band = (0.70, 1.0);
    fuzz.density = shape::Scalar::constant(25.0 * factor);
    fuzz.cluster = per_station;
    fuzz.fan = shape::Scalar::constant(0.90);
    fuzz.phyllotaxy = GOLDEN;
    fuzz.pitch = shape::Scalar::constant(0.90);
    fuzz.size = shape::Scalar::constant(0.80);
    fuzz.jitter = 0.15;
    let mut spray = LeafField::new(twig_depth, 0);
    spray.band = (0.25, 1.0);
    spray.density = shape::Scalar::constant(900.0 * factor);
    spray.cluster = per_station;
    spray.fan = shape::Scalar::constant(1.15);
    spray.phyllotaxy = GOLDEN;
    spray.pitch = shape::Scalar::constant(1.00);
    spray.size = shape::Scalar::new(&[(0.0, 0.50), (1.0, 0.75)]).expect("a needle's size is a law");
    spray.jitter = 0.18;
    let mut brush = LeafField::new(twig_depth, 0);
    brush.band = (0.86, 1.0);
    brush.density = shape::Scalar::constant(600.0 * factor);
    brush.cluster = per_station;
    brush.fan = shape::Scalar::constant(0.45);
    brush.phyllotaxy = GOLDEN;
    brush.pitch = shape::Scalar::constant(0.30);
    brush.size = shape::Scalar::constant(0.75);
    brush.jitter = 0.12;
    // The candle is the leader's own brush, and a bough plate has no leader: a field two generations
    // above the twigs exists only where the twigs are two generations below the root.
    if twig_depth >= 2 {
        candle.generation = twig_depth - 2;
        vec![candle, fuzz, spray, brush]
    } else {
        vec![fuzz, spray, brush]
    }
}

/// The pine: its tree, its foliage at the asked scale, its four fields and the cone template its twigs
/// carry.
pub fn yunnan_pine(seed: u64, foliage: Foliage) -> Species {
    let mut species = Species {
        plant: grow(&yunnan_pine_rule(), seed),
        templates: vec![foliage.template()],
        fields: needles(2, foliage),
        decorators: vec![cone()],
    };
    // The bole is dressed after the growth because a section is a node's own and `grow` authors circles:
    // the fissures are the trunk's, not the rule's.
    if let Ok(bole) = species.plant.get_mut(&Handle::root()) {
        bole.set_section(bark());
    }
    species
}

/// The bough plate's species: one bough as the trunk, its twigs as the only generation, and the true
/// needle. A plate at arm's length draws a bough rather than the whole tree, because a camera inside a
/// quarter of a million needles shades every pixel of them and the frame costs minutes for what a bough
/// shows in seconds.
pub fn yunnan_pine_bough(seed: u64) -> Species {
    let rule = Rule {
        trunk: || {
            (
                bend(3.0, [1.15, 1.28, 1.18, 0.86]),
                taper(&[(0.0, 0.055), (0.5, 0.035), (1.0, 0.012)]),
            )
        },
        generations: vec![Generation {
            shape: twig_at_arm_length,
            count: (26, 30),
            station: (0.15, 1.0),
            ..twig_generation(0.15)
        }],
        fields: Vec::new(),
    };
    let mut species = Species {
        plant: grow(&rule, seed),
        templates: vec![needle(NEEDLE_HALF_WIDTH)],
        fields: needles(1, Foliage::Needle),
        decorators: vec![cone()],
    };
    // The bough carries the bole's own section, because a plate at arm's length is the one place the
    // bark's fissures are close enough to read and a smooth rod would be the only smooth surface in a
    // species whose whole wood is fissured.
    if let Ok(bough) = species.plant.get_mut(&Handle::root()) {
        bough.set_section(bark_of(24, 4.0, 0.10));
    }
    species
}

/// The cones a pine's twigs carry at depth `twig_depth`: **old cones persist for years**, so a twig
/// carries nought to two of them and the crown is studded — the note's fourth silhouette fact, and the
/// reason the count is a range the seed decides rather than a cone per twig.
fn pine_cones(species: &Species, twig_depth: usize) -> Vec<Seat> {
    scatter(&species.plant, twig_depth, 0, 0.95, (0, 1))
}

/// The pine's surfaces: a fissured grey-brown bole, grey-green glaucous needles and chestnut cones.
pub fn pine_palette() -> Palette {
    Palette {
        wood: Skin::new([0.14, 0.11, 0.085], 0.95),
        leaves: vec![Skin::new([0.10, 0.17, 0.08], 0.80)],
        seats: vec![Skin::new([0.52, 0.38, 0.24], 0.85)],
    }
}

/// A dry sunny slope at two thousand metres: a deep high sky, a warm key well above the crown, and a
/// cool fill from the valley side.
///
/// # Invariants
///
/// **A light's reach is the distance its irradiance is stated at**, so each reach is the length of the
/// position it belongs to: a reach that is not the light's own distance states an irradiance the subject
/// never sees, and the first pine this scene drew was lit at three times its own number and came out
/// white.
fn pine_lighting() -> Lighting {
    Lighting {
        // A raking key, low and well off the camera axis: a frontal key lights exactly what the lens
        // sees and a crown then has no dark interior to measure, while a low sun from one side is how
        // light falls on a dry slope at nine in the morning anyway.
        key_position: [26.0, 14.0, -10.0],
        key_colour: [1.0, 0.92, 0.78],
        key_irradiance: 3.0,
        key_distance: 31.3,
        // The cutoff must hold the floor's own diagonal, or the light is culled per cluster box and the
        // box boundary draws itself across the ground as a straight edge.
        key_reach: 1200.0,
        // **The page is sized to the tree, not to the floor.** Forty put a 176 m plane over a 16 m
        // pine, and one texel of a 1024 page across that plane is 172 mm — seventeen times the gap
        // between two neighbouring needles, so the shadow test's receiver push swallowed every
        // caster in the crown before the test could see it. Eight is the largest half width that
        // still holds the whole crown with room to spare, and it buys a 34 mm texel; the leaf
        // materials' own bias scale takes it the rest of the way down to a couple of millimetres.
        // The floor beyond eight metres of the tree now falls outside the key's shadow, which is
        // what a page fitted to the subject means.
        key_extent: 8.0,
        fill_position: [-14.0, 8.0, -12.0],
        fill_colour: [0.42, 0.52, 0.72],
        fill_irradiance: 0.70,
        fill_distance: 20.1,
        floor: Skin::new([0.30, 0.26, 0.20], 0.95),
        sky: Atmosphere::new()
            .zenith([0.06, 0.14, 0.38])
            .horizon([0.45, 0.55, 0.66])
            .ground([0.30, 0.28, 0.24])
            .density(0.0006)
            .scale_height(2500.0),
        sky_radiance: 0.55,
        // A clear day at two thousand metres: the air is thin and the far ridge is the only thing it
        // touches, so the coefficient is a fifth of a garden's.
        haze: 0.004,
    }
}

/// The whole-tree frame: taller than wide, standing off at half the crown's height so the bole reads as
/// height and the crown stands against the sky.
const PINE_FRAME: Framing = Framing {
    width: 1400,
    height: 1700,
    eye: [16.0, 7.0, 19.0],
    target: [0.0, 9.0, 0.0],
    field_of_view: 0.70,
    near: 0.2,
    far: 600.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The bough plate's frame: close enough that a needle is two pixels wide, aimed just above the bough
/// so the frame holds the twigs and not the rod the camera is standing on.
const BOUGH_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [2.4, 2.3, 1.4],
    target: [1.4, 1.8, 0.2],
    field_of_view: 0.70,
    near: 0.03,
    far: 20.0,
    floor: 20.0,
    ground: Ground::Plane,
};

/// The bough plate's own light: a key on the lens's side at forty degrees, because the tree plate's
/// raking key stands behind this bough and a needle lit from behind is a hairline silhouette. A plate
/// whose subject is a needle's arrangement needs the needles lit to be seen at all.
fn bough_lighting() -> Lighting {
    Lighting {
        key_position: [3.2, 4.2, 3.0],
        key_colour: [1.0, 0.94, 0.82],
        key_irradiance: 2.8,
        key_distance: 6.1,
        key_reach: 40.0,
        key_extent: 6.0,
        fill_position: [-3.0, 1.5, -2.0],
        fill_colour: [0.45, 0.53, 0.70],
        fill_irradiance: 0.55,
        fill_distance: 4.0,
        floor: Skin::new([0.28, 0.24, 0.17], 0.95),
        sky: Atmosphere::new()
            .zenith([0.10, 0.20, 0.44])
            .horizon([0.50, 0.58, 0.66])
            .ground([0.30, 0.28, 0.24])
            .density(0.0010)
            .scale_height(2500.0),
        sky_radiance: 0.55,
        haze: 0.006,
    }
}

/// The base plate's frame: three and a half metres from the bole at a person's eye height, so the flare
/// and two metres of fissured bark are the subject and the trunk runs out of the top of the frame.
const BASE_FRAME: Framing = Framing {
    width: 1200,
    height: 1400,
    eye: [3.0, 1.3, 3.4],
    target: [0.0, 1.1, 0.0],
    field_of_view: 0.80,
    near: 0.05,
    far: 30.0,
    floor: 10.0,
    ground: Ground::Plane,
};

/// The base plate's own light: a low key on the lens's side, because the tree plate's raking key stands
/// behind the bole from here and a bark plate lit from behind is a silhouette. A tree's crown shadows
/// its own foot under its own light, so a base plate always wants a light of its own.
fn base_lighting() -> Lighting {
    Lighting {
        // Fifteen degrees up and on the lens's side: any higher and the tree's own crown shadows the
        // foot this plate exists to show, which is the trap the base plate of every species must dodge.
        key_position: [7.0, 6.0, 8.0],
        key_colour: [1.0, 0.94, 0.84],
        key_irradiance: 2.4,
        key_distance: 12.2,
        key_reach: 40.0,
        key_extent: 4.0,
        fill_position: [-5.0, 2.0, -4.0],
        fill_colour: [0.45, 0.52, 0.68],
        fill_irradiance: 0.5,
        fill_distance: 8.1,
        floor: Skin::new([0.30, 0.26, 0.20], 0.95),
        sky: Atmosphere::new()
            .zenith([0.10, 0.20, 0.44])
            .horizon([0.50, 0.58, 0.66])
            .ground([0.32, 0.30, 0.26])
            .density(0.0010)
            .scale_height(2500.0),
        sky_radiance: 0.6,
        haze: 0.006,
    }
}

/// Builds the Yunnan pine's whole-tree plate: one tree in the card foliage, its own light and ground.
pub fn yunnan_pine_picture() -> Value {
    let species = yunnan_pine(SEED, Foliage::Spray);
    let planting = Planting::new(species.clone())
        .with_seats(pine_cones(&species, 2))
        .with_palette(pine_palette());
    plant_scene_lit(&[planting], &PINE_FRAME, YUNNAN_PINE, &pine_lighting(), &[])
}

/// Builds the pine's branch plate: one bough in the true needle, at arm's length.
pub fn yunnan_pine_bough_picture() -> Value {
    let species = yunnan_pine_bough(SEED);
    // The bough stands at a crown's height and not on the ground: a branch plate whose bough lies in
    // the dirt reads as a fallen limb.
    let planting = Planting::new(species.clone())
        .at(Placement {
            origin: [0.0, 1.3, 0.0],
            ..Placement::IDENTITY
        })
        .with_seats(pine_cones(&species, 1))
        .with_palette(pine_palette());
    plant_scene_lit(
        &[planting],
        &BOUGH_FRAME,
        YUNNAN_PINE_BOUGH,
        &bough_lighting(),
        &[],
    )
}

/// Builds the pine's base plate: the foot of the bole at two metres, which is the frame H4 reads the
/// flare and the fissured bark in. The foliage is the spray, because the subject is the bark and a
/// quarter of a million true needles around a trunk plate is a quarter of an hour of fill for pixels
/// the frame crops away.
pub fn yunnan_pine_base_picture() -> Value {
    let species = yunnan_pine(SEED, Foliage::Spray);
    let planting = Planting::new(species.clone())
        .with_seats(pine_cones(&species, 2))
        .with_palette(pine_palette());
    plant_scene_lit(
        &[planting],
        &BASE_FRAME,
        YUNNAN_PINE_BASE,
        &base_lighting(),
        &[],
    )
}

/// Resolves a pine frame's `authored_id` into the handle that names the node.
pub fn yunnan_pine_handle_of(id: u32) -> Option<String> {
    node_ids(&yunnan_pine(SEED, Foliage::Needle).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a bough plate's `authored_id` into the handle that names the node: the bough is its own
/// plant, so its walk is its own table and the whole tree's would answer with the wrong node.
pub fn yunnan_pine_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&yunnan_pine_bough(SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}
