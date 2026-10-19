//! The last two of the ten: 华山松 and 筇竹, a pine the Yunnan pine is not and a bamboo that reaches
//! the bar on the machinery the pine already paid for.
//!
//! # What this module is
//!
//! [`.agents/proposals/southwest-china-trees.md`](../../../.agents/proposals/southwest-china-trees.md)
//! §5 puts these two in arc 3 and arc 6, and the proposal's own **control** is the second of them:
//! *"筇竹 … must reach the bar with the fewest new capabilities"*. So this module adds **no crate
//! change at all** — every number below is spelled with machinery the pine's module
//! ([`southwest`](southwest.rs)) and the spire module ([`southwest_spire`](southwest_spire.rs)) already
//! use: [`Generation::cluster`] for a whorl and for a clump, [`Joint::station`] for a crown profile and
//! for "fruit on the upper crown only", [`LeafField::cluster`] for a fascicle of five and for a bamboo's
//! two to four leaves at a tip, a [`LeafField`]'s own laws for everything else, `scatter_in` for the
//! cones, and a taper law for the swollen node.
//!
//! # The two separations that decide H1 for these species
//!
//! | | 云南松 [`southwest`](southwest.rs) | 华山松 *this module* |
//! | --- | --- | --- |
//! | crown envelope | broad and **flattened** at the top, 0.59 of the height in width | **cylindric-pyramidal into maturity** — FOC's own words — 0.34 of the height |
//! | needles a fascicle | **three**, 7–30 cm, flattened and slightly sickle | **five**, 8–15 cm, **triangular in section** and stiff |
//! | a bough's tip | turns **up**, into the upsweep a broad crown is made of | **droops**, under the weight of the foliage — the note's *"the outer portions drooping"* |
//! | cone | 5–11 cm, on a short stalk | **10–20 cm**, on the note's **2–3 cm peduncle**, pendant, and *persist two seasons* |
//!
//! The first row is the silhouette and the second is the foliage, and the two are read at different
//! distances by different plates, which is why both rows exist rather than one.
//!
//! # Two plates and two foliages, and why they cannot share one
//!
//! The rasteriser samples a pixel at one point, so a blade narrower than a pixel is **dropped, not
//! faded**. The note's 1–1.5 mm needle is **0.09 of a pixel** at the thirty-seven metres the whole-tree
//! plate is framed from (one pixel is `2 · 37 · tan 0.40 / 1700` = **18.4 mm**), and at the 0.72 m the
//! bough plate stands at it is **2.3 pixels** — which is the range the bar's H2 asks for. So:
//!
//! | plate | distance | one pixel | foliage | why |
//! | --- | --- | --- | --- | --- |
//! | `armand_pine` | 37.0 m | 18.4 mm | [`Foliage::Spray`] | a true needle is 0.09 of a pixel and the rasteriser drops it whole; the eye resolves plumes at this distance |
//! | `armand_pine_bough` | 0.72 m | 0.53 mm | [`Foliage::Needle`], `cluster: 5` | the note's own 1.2 mm needle is 2.3 pixels, and five of them at one station is the fascicle `05-leaf.jpg` photographs |
//! | `armand_pine_base` | 2.6 m | 5.4 mm | [`Foliage::Spray`] | the subject is the bark; a quarter of a million true needles around a trunk plate is fill for pixels the frame crops away |
//!
//! # What the bamboo's plate says about its five photographs
//!
//! The note holds **five** photographs and no sixth: PPBC serves this species four filler frames (they
//! appear under 木棉 and 西南桦 too), GBIF holds no image of the taxon at all, and the substitute species
//! the brief allowed share no swollen node. **What is missing and would have helped most is a whole
//! clump in its site** — the one frame that settles the culms-per-clump count, which is an **ESTIMATE**
//! in the note and is the number this module's clump rests on. The five that exist are enough for the
//! node, which is the species' name: `01-nodes-dark-culms.jpg` and `02-nodes-green-culms.jpg` are the
//! clasped discs at their clearest, `03-culm-sheath-bristles.jpg` a young culm still in its sheath,
//! `04-foliage-mass.jpg` the mass of a stand, and `05-leaf.jpg` the blade.
//!
//! # What is duplicated from [`southwest`](southwest.rs) and why
//!
//! The [`bend`] drawing tool, the [`Foliage`] enum's shape and the three-plate pattern are the pine's,
//! restated. This slice may not edit another module's file, so the copies stand until a later slice
//! lifts them into one shared file — the same debt [`southwest_spire`](southwest_spire.rs) records.

use definition::Value;
use vegetation::decorator::{Seat, Template, scatter_in};
use vegetation::derive::Species;
use vegetation::expand::Detail;
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

// ---------------------------------------------------------------------------------------------
// 华山松 Pinus armandii
// ---------------------------------------------------------------------------------------------

/// The name the Armand pine's whole-tree plate is built and rendered under.
pub const ARMAND_PINE: &str = "armand_pine";

/// The name the Armand pine's branch plate is built and rendered under.
pub const ARMAND_PINE_BOUGH: &str = "armand_pine_bough";

/// The name the Armand pine's base plate is built and rendered under.
pub const ARMAND_PINE_BASE: &str = "armand_pine_base";

/// The seed all three plates' leaf fields are derived from.
const ARMAND_SEED: u64 = 23;

/// The golden angle as a fraction of a turn, which is the step between two whorls.
const GOLDEN: f32 = 0.381_966_01;

/// The note's needle: **8–15 cm × 1–1.5 mm**, at the middle of both ranges. Half of the 1.2 mm across
/// is the blade's half-width, and 11.5 cm is the length — three times shorter than the Yunnan pine's
/// and twenty per cent wider, which is the foliage separation the two species are judged on.
const NEEDLE_LENGTH: f32 = 0.115;
const NEEDLE_HALF_WIDTH: f32 = 0.0006;

/// **The needle is triangular in section and that is in the geometry, not in the prose.** FOC says
/// *"triangular"*, the note calls it *"angular and prickle-like"*, and `05-leaf.jpg` shows needles
/// that catch the light along a hard edge rather than along a flat face. So the blade's section is a
/// four-point kite — one face carried twice, so the keel stays a hard corner — rather than the flat
/// lens [`southwest`](southwest.rs)'s yunnanensis needle uses. The section is in the blade's own
/// units of the width law, so the kite's `y` half-extent of 0.5 is a **0.6 mm thickness on a 1.2 mm
/// needle**: thick enough that a needle has a shaded face and a lit one at arm's length.
fn needle_section() -> shape::Section {
    shape::Section::outline(&[[1.0, 0.0], [-0.30, 0.50], [-1.0, 0.0], [-0.30, -0.50]])
        .expect("a triangular section with one face doubled is a section")
}

/// The spray card: **260 mm long and 76 mm across**, a comb of eight teeth.
///
/// The whole-tree plate stands 37.0 m off a 28 m tree with a 0.80 rad vertical field over 1700 rows,
/// so one pixel is **18.4 mm**. This card is 14.1 pixels long by 4.1 wide and its comb runs to eight
/// teeth, which is **1.8 pixels a tooth** — the grain `01-habit.jpg` shows at that distance, and the
/// reason a true needle drawn here instead is 0.09 of a pixel and is dropped whole.
const SPRAY_LENGTH: f32 = 0.260;
const SPRAY_HALF_WIDTH: f32 = 0.038;
const SPRAY_TEETH: usize = 8;

/// Which foliage an Armand pine draws.
///
/// # Invariants
///
/// **Three levels and one rule.** [`Foliage::Needle`] is the note's own blade and the only foliage
/// that carries [`LeafField::cluster`] at the fascicle's own five — a card that *is* a fascicle would
/// make five cards a fascicle, so the cluster belongs to the blade that counts needles. The two card
/// levels are the pine's [`southwest::Foliage`](southwest.rs) pattern with this species' own widths and
/// their own density factors, measured rather than inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foliage {
    /// The note's own needle, for frames within arm's length of a shoot.
    Needle,
    /// One blade whose width law is a comb of needle-teeth, standing for a fascicle-plume.
    Spray,
    /// The spray at a companion's density: the same rule at a third of the foliage, because a stand
    /// reads by its masses and not by its sprigs and the scene compile pays for every row a site frame
    /// carries.
    Stand,
}

impl Foliage {
    /// The blade this foliage instances.
    fn template(self) -> LeafTemplate {
        match self {
            Foliage::Needle => needle(),
            Foliage::Spray | Foliage::Stand => spray(),
        }
    }

    /// The fraction of the needle foliage's density this foliage is drawn at.
    ///
    /// **The spray's 0.12 is measured, not claimed.** The needle fields at `1.0` are 189 000 rows —
    /// the note's own 15–30 fascicles of five on a 10–18 cm twig — and the whole-tree plate cannot
    /// carry them: 189 000 rows is over the 90 000 a scene is budgeted at, and §4.13 measures 51 s of
    /// scene compile for 87 000. At `0.12` the plate draws 22 700 cards of 14 by 4 pixels each, which
    /// is about 1 270 000 pixels of card against a crown of some 213 000: six times the cover, which
    /// is what a crown with a dark interior wants and what a crown drawn at `1.0` cannot be, because
    /// a 1.2 mm needle is 0.09 of a pixel at this distance and is dropped whole.
    ///
    /// `Stand` is the same card at the third, for a companion on a far ridge.
    fn density_factor(self) -> f32 {
        match self {
            Foliage::Needle => 1.0,
            Foliage::Spray => 0.12,
            Foliage::Stand => 0.04,
        }
    }
}

/// The spray card: a plume of a shoot, its width law a comb of needle-teeth on the plume's own
/// envelope, so its silhouette is a brush and not a paddle.
fn spray() -> LeafTemplate {
    let teeth = SPRAY_TEETH;
    let knots: Vec<(f32, f32)> = (0..=2 * teeth)
        .map(|index| {
            let along = index as f32 / (2 * teeth) as f32;
            // The plume swells to its middle and runs out at both ends, so a spray is a plume and not
            // a rectangle. The sine is clamped at nought because the `f32` pi overshoots the true one
            // and its sine at t = 1 is a small NEGATIVE number, whose fractional power is not a number.
            let envelope = (std::f32::consts::PI * along).sin().max(0.0).powf(0.65);
            let tooth = if index % 2 == 0 { 0.12 } else { 1.0 };
            (along, SPRAY_HALF_WIDTH * envelope * tooth)
        })
        .collect();
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [0.11, 0.006, 0.0]),
            ([0.115, 0.018, 0.004], [0.10, 0.020, 0.004]),
            ([SPRAY_LENGTH * 0.85, 0.062, 0.014], [0.075, 0.030, 0.006]),
        ]),
        shape::Scalar::new(&knots).expect("a comb is a law"),
        0.0005,
        22,
        4,
    )
}

/// The true needle: eleven and a half centimetres of stiff blade, straight where the Yunnan pine's is
/// sickle-curved, because *"angular and prickle-like"* and *"slightly sickle-curved"* are the two
/// species' own words and a fascicle of five straight bristles and a fascicle of three curved ones do
/// not read the same way at arm's length.
fn needle() -> LeafTemplate {
    LeafTemplate::of(
        spine(&[
            ([0.0, 0.0, 0.0], [0.052, 0.002, 0.0]),
            ([0.052, 0.006, 0.001], [0.048, 0.006, 0.0]),
            ([NEEDLE_LENGTH, 0.012, 0.003], [0.030, 0.004, 0.0]),
        ]),
        needle_section(),
        taper(&[
            (0.0, NEEDLE_HALF_WIDTH * 0.75),
            (0.35, NEEDLE_HALF_WIDTH),
            (0.80, NEEDLE_HALF_WIDTH * 0.62),
            (1.0, NEEDLE_HALF_WIDTH * 0.10),
        ]),
        5,
        4,
    )
}

/// The station on the leader where the crown begins.
///
/// **The note's 12–20 m of clean bole on a thirty-five-metre tree is 0.34 to 0.57**, and 0.46 is its
/// middle. The species' third silhouette fact — *"a clean, tall, branchless bole, crown starting well
/// above half the tree's height"* — is the note reading it from `01-habit.jpg`, and both photographs of
/// a whole tree (`01`, `02`, `09`) show a bole with nothing on it over most of its length.
const CROWN_BASE: f32 = 0.46;

/// The station on the leader where the crown stops: **0.985**, so the leader's last four tenths of a
/// metre is the candle's own brush and not a tier's.
const CROWN_TOP: f32 = 0.985;

/// Boughs to a whorl: the note's **ESTIMATE** of 3–5, at four.
const WHORL: u32 = 4;

/// Whorls up the crown: **thirty-four**, so the boughs sit `14.8 / 33 = 0.45 m` apart, inside the
/// note's own **ESTIMATE** of 0.30–0.60 m between whorls, and 136 first-order branches in all.
const WHORLS: u32 = 34;

/// The Armand pine's rule: a clean bole, whorled boughs whose length is a cylindric-pyramidal law over
/// the station they hang at, and twigs that carry the fascicles.
///
/// # The numbers and where they come from
///
/// Every figure is [`pinus_armandii.md`](../../../../.agents/investigate/trees/pinus_armandii.md)'s
/// modelling table. **The tree is drawn at twenty-eight metres, not the note's thirty-five**, and the
/// reduction is the spire module's own precedent: a thirty-five-metre tree framed whole needs a
/// forty-six-metre standoff, at which one pixel is 22.9 mm and the card's teeth are one pixel. Every
/// law here is a fraction of the height, so the drawn tree keeps the note's crown-to-height ratio, its
/// whorl spacing in metres of leader, its bough angles and its taper fractions; and the note's
/// absolute crown width of 8–14 m is honoured as the middle of that band at this height, 9.6 m.
fn armand_rule() -> Rule {
    Rule {
        trunk: bole,
        generations: vec![
            // **The whorls.** Four boughs at one station — `cluster` is the whorl — and thirty-four
            // stations up the crown's band, the slot step being the golden angle between whorls and a
            // quarter turn within one, which is what a whorl is.
            Generation {
                count: (WHORL * WHORLS, WHORL * WHORLS),
                station: (CROWN_BASE, CROWN_TOP),
                slot: (0.0, GOLDEN),
                jitter: (0.05, 0.06),
                sink: 0.04,
                cluster: WHORL,
                shape: bough,
            },
            Generation {
                // **Twenty to twenty-four twigs on a five-metre bough is one every twenty-two
                // centimetres**, and the first pass's twelve to fifteen — one every forty — is what
                // left the bough plate with five tufts in a frame and the whole tree's crown with
                // visible wood between its sprays. A shoot is short; a bough carries one every two or
                // three of its own length, not one every two.
                count: (20, 24),
                station: (0.20, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.14, 0.12),
                sink: 0.010,
                cluster: 1,
                shape: twig,
            },
        ],
        // **No field.** The pine's `Field::light` and the oak's tropism buy a foliage mass at seconds
        // per thousand leaves, and this species gets its mass from its counts instead: the droop of a
        // bough's tip is in the shape function, where it can be written for the tip only and not for
        // the leader as well.
        fields: Vec::new(),
    }
}

/// The bole: twenty-eight metres, plumb to within a hand's breadth, with the modest flare the note
/// reads off `03-bark.jpg` — *"the bole widens modestly into the first whorl, with a slight flare at
/// the ground and no strong buttressing"*.
fn bole() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.000, 0.0, 0.000], [0.012, 1.00, 0.000]),
            ([0.045, 7.0, 0.035], [0.020, 1.00, 0.010]),
            ([0.110, 14.0, -0.025], [0.018, 1.00, 0.010]),
            ([0.165, 21.0, 0.050], [0.010, 1.00, 0.000]),
            ([0.195, 28.0, 0.095], [0.000, 1.00, 0.000]),
        ]),
        // **The flare is the first three knots and nothing else**, and the d.b.h. knot is stated: FOC's
        // *"trunk to 1 m d.b.h."* on thirty-five metres is 0.80 m on twenty-eight, so 0.40 of radius at
        // 1.3 m up. A trunk that meets the ground on a cone is the *floating tree* defect, and a root
        // plate is a different species' question.
        taper(&[
            (0.000, 0.505),
            (0.012, 0.440),
            (0.046, 0.400),
            (0.200, 0.320),
            (0.460, 0.230),
            (0.750, 0.120),
            (1.000, 0.026),
        ]),
    )
}

/// The bole's section: **twenty-four sides of which eight are the fissures**, which is *"gray, fissured
/// into square plates or shed"* in geometry — a plate is a flat face between two sharp shoulders, and
/// the shoulders are the fissures.
///
/// # Invariants
///
/// **Twenty-four points and eight plates, and the base plate expands at [`BARK_DETAIL`] whose `around`
/// is twenty-four.** [`vegetation::expand::wood`] samples every sweep at [`Detail::around`] whatever
/// number of points its section carries, so at the default twelve a plated section is resampled to a
/// twelve-gon and the plates are averaged away; at twenty-four this section is the section, and each
/// fissure's two shoulders turn through more than [`vegetation_mesh::WOOD_CREASE`] — which is why the
/// creased wood normals have anything to crease.
fn bark() -> shape::Section {
    let sides = 24;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            // **Every third point is a fissure, and it is twenty per cent deep.** The first version put
            // the notch on a single point and it did not survive: [`vegetation::expand::wood`] resamples
            // the section at [`Detail::around`] by **arc length**, and a fifteen-degree notch is
            // narrower than the perimeter the resample has to hit, so the twenty-four samples landed
            // either side of it and the base plate came out a smooth grey column with soft vertical
            // banding and no bark at all. A fissure a whole point wider is what the resample can land
            // in. **Twenty per cent is the second pass's depth**: at fourteen the base plate showed a
            // trunk with thin dark lines drawn on it rather than plates, and a crack that reads has to
            // be a crack you could lose a thumbnail in.
            //
            // **The three-lobe term and the per-plate depth are the third pass's, and they are what
            // stop the bark being eight identical stripes.** FOC says *"fissured into square plates or
            // shed"*, and shed plates are not a regular fluting: the three-lobe term varies each
            // plate's width by six per cent and each crack's own depth by four, so no two of the eight
            // plates is the width of its neighbour and the second render's eight identical vertical
            // bands — which read as a fluted column and not as bark — do not come back.
            let plate = 1.0 + 0.060 * (theta * 3.0).sin();
            let crack = 1.0 - 0.20 - 0.04 * (theta * 8.0 + 1.1).sin();
            let radius = if index % 3 == 2 { crack } else { 1.0 };
            [theta.cos() * radius * plate, theta.sin() * radius * plate]
        })
        .collect();
    shape::Section::outline(&points).expect("a plated ring is a section")
}

/// The first-order bough: it leaves the leader at the note's **50–80°**, runs out nearly level,
/// **turns in and out twice** — the note's *"markedly sinuous/contorted"* limbs, which is the fourth
/// of its five silhouette facts — and **droops at its outer end** under the weight of the foliage it
/// carries. The Yunnan pine's tip turns *up* instead, and that one number is most of the foliage
/// separation between the two plates.
///
/// # The crown's own law
///
/// **Cylindric-pyramidal, which is FOC's own wording and the cleanest separation from the Yunnan
/// pine's flat-topped crown.** So the reach is held nearly full through the crown's middle and falls
/// to nothing over its top sixth: a cone over the whole crown would be a spruce, and a fall through
/// the whole crown would be the pine. The exponent on the cap is 1.1 and not 1.0 because the top of
/// this species' crown *rounds off* in old open-grown trees, which is the note's own caveat read off
/// `09-habit-second.jpg`.
fn bough(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let across = ((joint.station - CROWN_BASE) / (CROWN_TOP - CROWN_BASE)).clamp(0.0, 1.0);
    let reach = if across < 0.62 {
        // The crown's base: the lowest whorls are short, so the crown has a rounded shoulder rather
        // than a disc, and then it *holds* its width.
        0.58 + 0.42 * (across / 0.62)
    } else {
        // The conical cap, rounded at its own apex.
        (1.0 - (across - 0.62) / 0.38).clamp(0.0, 1.0).powf(1.1)
    };
    // **4.6 m is the note's crown half-width at this height** — 9.2 m of crown on 28 m is 0.32, the
    // middle of the note's own 0.23–0.40 band — and the length's random spread is one tenth either way
    // rather than the first pass's one fifth, because a bough law whose longest bough is a fifth longer
    // than its mean makes the crown's *measured* width a fifth wider than its law says, and this
    // species' crown width is the number H1 separates it by.
    //
    // The floor of 0.22 on `reach` stops the topmost tier from collapsing to a point and leaving the
    // apex to the candle alone.
    let length = 4.6 * reach.max(0.22) * (0.82 + 0.28 * rng.unit());
    // 50° to 80° from the leader, opening a little with height as the note's photographs show.
    let departure = 1.16 - 0.16 * joint.station + 0.09 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure - 0.44,
                departure + 0.26,
                departure + 0.52,
            ],
        ),
        taper(&[(0.0, radius * 0.30), (0.45, radius * 0.16), (1.0, 0.010)]),
    )
}

/// A twig: the note's **10–18 cm** shoot, taken at its top end because a twig is the unit a fascicle
/// stands on and a short one carries too few to read. It leaves the bough at about a right angle and
/// **turns up**, because the whole of this species' foliage is held in upward-facing brushes.
fn twig(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.18 * (0.82 + 0.36 * rng.unit());
    let departure = 0.95 + 0.22 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.16,
                departure + 0.10,
                departure - 0.18,
            ],
        ),
        taper(&[(0.0, radius * 0.40), (1.0, 0.0030)]),
    )
}

/// The bough plate's species: **one bough as the trunk**, its twigs as the only generation, and the
/// note's own needle in fascicles of five.
///
/// A camera inside a quarter of a hundred thousand needles shades every pixel of them, so the plate
/// grows a species whose trunk *is* a bough — the proposal's D9 — and its ids are the bough's own walk.
///
/// **The bough is 1.1 m and nearly horizontal**, which is the plate's third design and the one that
/// works: a 3.2 m bough rising at thirty-five degrees puts its own two-inch base where the lens is and
/// puts the horizon through the middle of the frame, and the first two renders were a grey cylinder
/// across the middle of the picture with a few hairs above it. One metre of bough at seventy degrees
/// from vertical runs out sideways, its radius at the framed point is four centimetres, and the
/// camera can sit below it and have sky behind the whole subject.
pub fn armand_pine_bough(seed: u64) -> Species {
    let rule = Rule {
        trunk: || {
            (
                bend(1.1, [1.34, 1.14, 1.52, 1.66]),
                taper(&[(0.0, 0.030), (0.5, 0.017), (1.0, 0.005)]),
            )
        },
        generations: vec![Generation {
            // **One shoot every four centimetres**, so the frame holds a dozen of them and the bough is
            // a bottlebrush and not a bare rod with five pom-poms on it — which is what
            // `04-branch.jpg` shows and what the first two renders of this plate did not.
            count: (24, 28),
            station: (0.10, 1.0),
            slot: (0.0, GOLDEN),
            jitter: (0.14, 0.12),
            sink: 0.005,
            cluster: 1,
            shape: twig,
        }],
        fields: Vec::new(),
    };
    let mut species = Species {
        plant: grow(&rule, seed),
        templates: vec![needle()],
        fields: armand_needles(1, Foliage::Needle),
        decorators: vec![cone()],
    };
    // **The bough carries the bole's own plating**, for the reason the base plate exists: at 0.63 m a
    // four-centimetre branch with a smooth round section is a length of pipe, and a first-order bough
    // of a pine is plated grey bark all the way to its twigs. The bough plate is cheap — twenty-eight
    // nodes — so it is expanded at the bark dial and the plates are the section.
    if let Ok(bough) = species.plant.get_mut(&Handle::root()) {
        bough.set_section(bark());
    }
    species
}

/// Four knots along a heading that turns through `angles`, measured from the parent's own tangent:
/// nought is along the parent and a quarter turn is straight out. The tangents are the heading's own
/// direction, so a bough is a chord with three bends in it rather than a straight line with a
/// direction.
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

/// The cone: **10–20 cm** on the note's **2–3 cm peduncle**, hanging, because a seat expresses no
/// rotation — the hanging is the template's own shape, drawn once, and every cone the scatter seats
/// then hangs the same way from whatever side of the twig it lands on.
///
/// This is **twice the length of the Yunnan pine's cone** (`07-cone-pendant-pair.jpg` shows two of
/// them hanging clear of the foliage, forearm-length and heavy) and it is the species' fourth
/// silhouette fact: at two hundred metres they read as distinct pale blobs under the crown, because
/// FOC's *"seed matures in the second year"* means a mature tree carries them **two seasons**.
fn cone() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0, 0.0, 0.0], [0.012, 0.004, 0.0]),
            ([0.012, 0.004, 0.0], [0.002, -0.024, 0.0]),
            ([0.013, -0.026, 0.0], [0.0, -0.075, 0.0]),
            ([0.013, -0.105, 0.0], [0.0, -0.070, 0.0]),
            ([0.013, -0.176, 0.0], [0.0, -0.034, 0.0]),
        ]),
        // **Twenty shallow scallops and not the spruce's deep ones.** FOC's armandii apophyses are
        // *"rhombic or triangular, not ridged"*, so the scales are big flat plates with a step at
        // each one's edge and no star between them — `07-cone-pendant-pair.jpg` shows exactly that: a
        // pineapple of overlapping shields.
        scalloped(20, 0.86),
    );
    sweep.scale = taper(&[
        (0.0, 0.005),
        (0.10, 0.011),
        (0.34, 0.027),
        (0.62, 0.033),
        (0.86, 0.026),
        (1.0, 0.004),
    ]);
    Template {
        sweep,
        stations: 14,
        around: 20,
    }
}

/// A section of `sides` points whose radius alternates between one and `waist`, which is a cone's
/// scales read as a star and nothing else is.
fn scalloped(sides: u32, waist: f32) -> shape::Section {
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let radius = if index % 2 == 0 { 1.0 } else { waist };
            [theta.cos() * radius, theta.sin() * radius]
        })
        .collect();
    shape::Section::outline(&points).expect("a scalloped ring is a section")
}

/// The needle fields at `twig_depth`: the leader's candle, the bough's own spray over its outer two
/// fifths, the fascicles over a twig's length, and the upright brush at a twig's tip that
/// `04-branch.jpg` shows on every shoot in the crown. Four fields and one template, because a needle
/// is one blade wherever it hangs and only its distribution changes.
///
/// # Invariants
///
/// **`cluster` is five and only on the needle foliage.** A pine's sheath holds five needles and that
/// is the species' defining number — the note's `05-leaf.jpg` is a photograph of five of them on a
/// white background. A card is *already* a plume, so a cluster of five on a card is twenty-five
/// needles a row and the card's area would be paid for twice; the field's cluster is therefore the
/// fascicle where the blade is a needle, and one card where the blade is a plume.
fn armand_needles(twig_depth: usize, foliage: Foliage) -> Vec<LeafField> {
    let factor = foliage.density_factor();
    let per_station = match foliage {
        Foliage::Needle => 5,
        Foliage::Spray | Foliage::Stand => 1,
    };
    let mut candle = LeafField::new(0, 0);
    candle.band = (0.90, 1.0);
    candle.density = shape::Scalar::constant(180.0 * factor);
    candle.cluster = per_station;
    candle.fan = shape::Scalar::constant(0.85);
    candle.phyllotaxy = GOLDEN;
    candle.pitch = shape::Scalar::constant(0.80);
    candle.size = shape::Scalar::constant(0.88);
    candle.jitter = 0.14;
    let mut limb = LeafField::new(twig_depth - 1, 0);
    // **The bough's own spray**, for the spruce's reason stated once more: the outer part of a
    // first-order branch of this species is itself a needled shoot, and at thirty-seven metres it is
    // the bough's own brush and not thirteen separate tufts on a bare rod.
    limb.band = (0.58, 0.99);
    limb.density = rising(90.0, 160.0, 260.0, factor);
    limb.cluster = per_station;
    limb.fan = shape::Scalar::constant(1.05);
    limb.phyllotaxy = GOLDEN;
    limb.pitch = shape::Scalar::constant(1.00);
    limb.size = shape::Scalar::new(&[(0.0, 0.78), (1.0, 1.0)]).expect("a size is a law");
    limb.jitter = 0.20;
    let mut spray = LeafField::new(twig_depth, 0);
    // **The band stops short of the wood on both ends**, because foliage from 0.05 to 0.99 of every
    // twig is the *dead give* the defect list names, and because a shoot's own base carries fascicles
    // and not the whole of one.
    spray.band = (0.10, 0.97);
    // **Two hundred and sixty to six hundred and twenty a metre** is the note's own 15–30 fascicles
    // over its 10–18 cm twig, read as a linear density: five needles at a station every eight
    // millimetres, which is the dense overlapping brush `04-branch.jpg` shows. The first pass of this
    // module carried a fifth of it — thirty needles on a twig where the note counts a hundred and
    // fifty — and the bough plate came out as a hedgehog of hairs rather than a bottlebrush.
    spray.density = rising(300.0, 520.0, 700.0, factor);
    spray.cluster = per_station;
    // **The widest fan of any field in this module**, because a fascicle of this species *radiates*:
    // `05-leaf.jpg` is a spray of needles spreading through most of a half-circle from one sheath.
    spray.fan = shape::Scalar::constant(1.30);
    spray.phyllotaxy = GOLDEN;
    spray.pitch = shape::Scalar::constant(1.08);
    spray.size = shape::Scalar::new(&[(0.0, 0.86), (1.0, 1.06)]).expect("a size is a law");
    spray.jitter = 0.22;
    let mut brush = LeafField::new(twig_depth, 0);
    brush.band = (0.78, 1.0);
    brush.density = shape::Scalar::constant(900.0 * factor);
    brush.cluster = per_station;
    brush.fan = shape::Scalar::constant(0.70);
    brush.phyllotaxy = GOLDEN;
    brush.pitch = shape::Scalar::constant(0.72);
    brush.size = shape::Scalar::constant(0.94);
    brush.jitter = 0.16;
    // The candle is the leader's own brush, and a bough plate has no leader: a field two generations
    // above the twigs exists only where the twigs are two generations below the root.
    if twig_depth >= 2 {
        candle.generation = twig_depth - 2;
        vec![candle, limb, spray, brush]
    } else {
        vec![limb, spray, brush]
    }
}

/// The density law the outer fields are written on: a rise from `low` at the bare end of the node to
/// `high` at its tip, through `middle` at its halfway station, every number scaled by `factor`.
///
/// The rise is the shape and not a taste: a field whose density is one constant puts the same number
/// of fascicles on every centimetre of a twig, and a twig read end to end is then a sleeve rather than
/// a brush. The eye reads the gradient long before it reads the count.
fn rising(low: f32, middle: f32, high: f32, factor: f32) -> shape::Scalar {
    shape::Scalar::new(&[
        (0.0, low * factor),
        (0.5, middle * factor),
        (1.0, high * factor),
    ])
    .expect("a rise is a law")
}

/// The Armand pine: its tree, its foliage at the asked scale, its four fields and the cone template
/// its twigs carry.
pub fn armand_pine(seed: u64, foliage: Foliage) -> Species {
    let mut species = Species {
        plant: grow(&armand_rule(), seed),
        templates: vec![foliage.template()],
        fields: armand_needles(2, foliage),
        decorators: vec![cone()],
    };
    // The bole is dressed after the growth because a section is a node's own and `grow` authors circles:
    // the plates are the trunk's, not the rule's.
    if let Ok(bole) = species.plant.get_mut(&Handle::root()) {
        bole.set_section(bark());
    }
    species
}

/// The cones a pine's twigs carry: **on the upper crown only**, and one to three of them.
///
/// # Invariants
///
/// **The band is on the boughs' own attachment, which is where the note puts them.** FOC and
/// `01-habit.jpg` both show the cones in the top third of the crown, hanging under the foliage, and
/// `scatter` alone would seed the whole crown evenly — the §4.15 want, fixed. The band `(0.62, 1.0)`
/// reads the *twig's* station on its parent, which is the bough, and the bough's own station on the
/// bole: so `(0.62, 1.0)` is *"on the boughs of the top third of the leader"*, and it is the number
/// that keeps the crown's lower two thirds clear of blobs.
fn armand_cones(species: &Species, twig_depth: usize) -> Vec<Seat> {
    scatter_in(&species.plant, twig_depth, 0, 0.94, (1, 3), (0.62, 1.0))
}

/// The Armand pine's surfaces: **grey** plated bark (FOC's own word, and the Yunnan pine's is
/// reddish-brown), **dark green glaucous** needles that *"age to yellow-green"*, and the cones' own
/// yellow-brown.
///
/// # Invariants
///
/// **The wood is greyer and lighter than the Yunnan pine's `[0.14, 0.11, 0.085]`, and the leaves are
/// darker and bluer than its `[0.10, 0.17, 0.08]`.** The *felt-pen tree* defect is wood and leaf in one
/// tint, and this species' two plates are read **beside the pine's** — so the two species cannot be
/// told apart by their colour if the two palettes are the same two colours.
pub fn armand_palette() -> Palette {
    Palette {
        wood: Skin::new([0.168, 0.156, 0.142], 0.95),
        leaves: vec![Skin::new([0.086, 0.142, 0.114], 0.82)],
        // **A dark chestnut and not the first pass's straw.** FOC's cones *"maturing yellow or
        // brown-yellow"*, and `07-cone-pendant-pair.jpg`'s green cones are a deep bottle green; the
        // `[0.430, 0.352, 0.180]` of the first render came out a bright khaki that read as a row of
        // leaves of another species hanging in the crown — the *felt-pen tree* defect reached through
        // a cone.
        seats: vec![Skin::new([0.268, 0.212, 0.122], 0.84)],
    }
}

/// A montane slope at two thousand metres: a deep high sky, a **raking key off the camera's axis**,
/// and a cool fill from the valley side.
///
/// # Invariants
///
/// - **A light's reach is the distance its irradiance is stated at**, and the reach is also past the
///   floor's own diagonal, or the light is culled per cluster box and the box boundary draws itself
///   across the ground as a straight edge.
/// - **The key is off the camera axis by seventy-four degrees of azimuth.** The camera's azimuth is
///   forty and the key's is a hundred and fourteen, so the light comes in across the lens's screen
///   right rather than from behind it: a frontal key lights exactly what the lens sees and a crown
///   then has no dark interior to measure.
/// - **Sixty-two and a half metres, and the distance is the point.** Thirty-three degrees of elevation
///   is a fixed number and a twenty-eight-metre tree is not: at forty-five metres out the sun sits at
///   29.3 m, which is *above* this tree's apex, and at thirty metres it sits at 19.5 m — below the
///   middle of the crown, so the key arrived from underneath the foliage and the whole upper half of
///   the tree went dark. Sixty-two and a half puts it at 34 m.
/// - **The page is sized to the tree.** Twelve put a 24 m plane over a 28 m pine, so one texel of a
///   1024 page is 23 mm — twenty-three times the gap between two neighbouring needles — and the leaf
///   materials' own bias scale takes it the rest of the way to about a millimetre.
fn armand_lighting() -> Lighting {
    Lighting {
        key_position: [48.0, 34.0, -21.0],
        key_colour: [1.0, 0.93, 0.80],
        // **Six and a half, and not three and a bit.** The first pass wrote the spruce's fraction of
        // a clear day's key and the frame came back with a crown median luminance of 0.20: the tree's
        // shape was readable and its foliage was not. This species' needles are a fifth as wide as
        // the spruce's and so catch less of it, so it wants more key and not less.
        key_irradiance: 7.2,
        key_distance: 62.5,
        key_reach: 1200.0,
        key_extent: 12.0,
        fill_position: [-26.0, 12.0, -18.0],
        fill_colour: [0.42, 0.50, 0.72],
        fill_irradiance: 0.26,
        fill_distance: 33.7,
        floor: Skin::new([0.115, 0.108, 0.082], 0.96),
        sky: Atmosphere::new()
            .zenith([0.05, 0.12, 0.36])
            .horizon([0.42, 0.53, 0.66])
            .ground([0.26, 0.25, 0.21])
            .density(0.0005)
            .scale_height(3000.0),
        sky_radiance: 0.44,
        haze: 0.004,
    }
}

/// The bough plate's own light: the same day, **the key on the lens's side and its page a metre**.
///
/// # Invariants
///
/// **The key was behind the bough in the first two passes and that is why both were grey.** The eye
/// is at `[0.62, 1.28, 0.60]` and the key was at `[7.6, 3.1, -2.6]` — on the far side of the subject
/// — so every needle was lit from behind and the plate was a silhouette of its own bough. A bough
/// plate is the one frame in this module where the key has to be on the **lens's** side, the same rule
/// the base plate obeys and for the same reason: a two-degree-of-freedom subject at arm's length has
/// no back.
///
/// **The page is 1.2 m across**, which is a 2.4 mm texel — a fifth of the gap between two neighbouring
/// needles at this distance — and the leaf materials' own bias scale takes it to a tenth of a
/// millimetre, so a needle can shadow the needle beside it and H3's interior reading means something.
fn armand_bough_lighting() -> Lighting {
    Lighting {
        key_position: [2.6, 2.35, 3.1],
        key_colour: [1.0, 0.94, 0.82],
        key_irradiance: 4.2,
        key_distance: 4.7,
        key_reach: 40.0,
        key_extent: 1.2,
        fill_position: [-2.0, 1.1, -1.6],
        fill_colour: [0.42, 0.50, 0.70],
        fill_irradiance: 0.42,
        fill_distance: 2.9,
        floor: Skin::new([0.115, 0.108, 0.082], 0.96),
        sky: Atmosphere::new()
            .zenith([0.09, 0.19, 0.44])
            .horizon([0.50, 0.60, 0.70])
            .ground([0.27, 0.26, 0.21])
            .density(0.0006)
            .scale_height(3000.0),
        sky_radiance: 0.60,
        // The subject is a metre from the lens rather than thirty-seven, so the air between them is
        // thirty-seven times thinner and the coefficient is a fiftieth.
        haze: 0.0002,
    }
}

/// The base plate's own light: **a low key on the lens's side at fifteen degrees**, for the pine's
/// reason and this species' harder one — a 28 m pine whose crown starts at 12.9 m is *further* from
/// its own foot than the Yunnan pine's, but a key at thirty-three degrees still has to clear the
/// lowest bough, and this one has no root plate to be lit by anything but its own side.
fn armand_base_lighting() -> Lighting {
    Lighting {
        key_position: [8.0, 2.2, 7.0],
        key_colour: [1.0, 0.94, 0.84],
        key_irradiance: 3.0,
        key_distance: 11.2,
        key_reach: 40.0,
        key_extent: 3.0,
        fill_position: [-5.0, 2.0, -4.0],
        fill_colour: [0.45, 0.52, 0.68],
        fill_irradiance: 0.34,
        fill_distance: 7.0,
        floor: Skin::new([0.115, 0.108, 0.082], 0.96),
        sky: Atmosphere::new()
            .zenith([0.08, 0.17, 0.42])
            .horizon([0.48, 0.57, 0.67])
            .ground([0.27, 0.26, 0.22])
            .density(0.0008)
            .scale_height(3000.0),
        sky_radiance: 0.42,
        haze: 0.0006,
    }
}

/// The whole-tree frame: a portrait three fifths as wide again as it is tall, standing **37.0 m** off
/// so a 28 m tree fills the frame's height without cropping and one pixel is 18.4 mm.
const ARMAND_FRAME: Framing = Framing {
    width: 1000,
    height: 1700,
    eye: [24.0, 9.0, 28.6],
    target: [0.0, 13.0, 0.0],
    field_of_view: 0.80,
    near: 0.5,
    far: 700.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The bough plate's frame: **0.72 m off a run of shoots**, side-on to the bough and a little above
/// it, so a needle is **2.3 pixels** wide and 190 long, which is the range the bar's H2 asks for, and
/// about four fascicles fill the width.
///
/// # Why this distance and these two numbers
///
/// The first frame stood at 1.6 m, which is the spruce's own distance, and the needle came out at
/// 0.7 of a pixel — dropped whole by the sampler for most of its length, which is exactly the tax the
/// proposal's §4.10 names. Half the distance is the whole of the difference between a bough plate that
/// shows a bottlebrush and one that shows a few hairs.
///
/// **The eye is 1.30 m and the target 1.42 m, so the camera looks up and the horizon sits below the
/// subject.** The bough plate's plant is planted at a crown's height — 1.30 m — because a branch plate
/// whose bough lies in the dirt reads as a fallen limb, and it runs out from there; an eye above the
/// bough's own height puts the horizon through the middle of the frame and half the picture is ground,
/// which is what the first two renders were. The target is a point 0.55 m along the bough's own spine,
/// which is `(0.55, 1.42, 0)`, and the eye is 0.63 m off it.
const ARMAND_BOUGH_FRAME: Framing = Framing {
    width: 1200,
    height: 1000,
    eye: [0.62, 1.28, 0.60],
    target: [0.55, 1.42, 0.0],
    field_of_view: 0.70,
    near: 0.02,
    far: 40.0,
    floor: 8.0,
    ground: Ground::Plane,
};

/// The base plate's frame: **3.4 m** off the foot of the bole at a person's eye height, on a lens
/// wide enough to hold a one-metre trunk whole and to show the flare standing against the ground.
///
/// The bole is **1.01 m across at the ground** (0.505 of radius, the note's 1 m d.b.h. reduced with
/// the tree), so the camera's distance is measured to the axis and the frame has to be measured to
/// the nearest skin, 2.9 m away: `tan 0.425 / 2.9 = 0.155` is under a fifth of the half-width, so the
/// trunk fills about a fifth of the frame's width. The first frame stood at 2.6 m on a 0.95 rad lens
/// and filled two fifths of it, which is a column and not a foot.
const ARMAND_BASE_FRAME: Framing = Framing {
    width: 1100,
    height: 1400,
    eye: [2.20, 1.25, 2.55],
    target: [0.0, 1.55, 0.0],
    field_of_view: 0.85,
    near: 0.05,
    far: 40.0,
    floor: 12.0,
    ground: Ground::Plane,
};

/// The dial a base plate's wood is expanded at: [`Detail::DEFAULT`] with **twenty-four sides**.
///
/// The invariant is the spire module's, and the reason is this module's section: [`vegetation::expand`]
/// samples every sweep at [`Detail::around`] whatever its section carries, and at the default twelve
/// a twenty-four-point plated ring is resampled to a twelve-gon whose eight fissures are averaged into
/// six shallow scallops. At twenty-four this section is the section.
const BARK_DETAIL: Detail = Detail {
    stations: 6,
    around: 24,
    spacing: 0.5,
};

/// Builds the Armand pine's whole-tree plate: one tree in the card foliage, its own light and ground.
pub fn armand_pine_picture() -> Value {
    let species = armand_pine(ARMAND_SEED, Foliage::Spray);
    let planting = Planting::new(species.clone())
        .with_seats(armand_cones(&species, 2))
        .with_palette(armand_palette());
    plant_scene_lit(
        &[planting],
        &ARMAND_FRAME,
        ARMAND_PINE,
        &armand_lighting(),
        &[],
    )
}

/// Builds the pine's branch plate: one bough in the true needle, at three quarters of a metre.
pub fn armand_pine_bough_picture() -> Value {
    let species = armand_pine_bough(ARMAND_SEED);
    // The bough stands at a crown's height and not on the ground: a branch plate whose bough lies in
    // the dirt reads as a fallen limb.
    let planting = Planting::new(species.clone())
        .at(Placement {
            origin: [0.0, 1.30, 0.0],
            ..Placement::IDENTITY
        })
        .with_seats(armand_cones(&species, 1))
        .with_palette(armand_palette());
    plant_scene_lit(
        &[planting],
        &ARMAND_BOUGH_FRAME,
        ARMAND_PINE_BOUGH,
        &armand_bough_lighting(),
        &[],
    )
}

/// Builds the pine's base plate: the foot of the bole at 2.6 m, which is the frame H4 reads the flare
/// and the plated bark in. The foliage is the spray, for the pine's reason.
pub fn armand_pine_base_picture() -> Value {
    let species = armand_pine(ARMAND_SEED, Foliage::Spray);
    let planting = Planting::new(species.clone())
        .with_seats(armand_cones(&species, 2))
        .with_palette(armand_palette())
        .with_detail(BARK_DETAIL);
    plant_scene_lit(
        &[planting],
        &ARMAND_BASE_FRAME,
        ARMAND_PINE_BASE,
        &armand_base_lighting(),
        &[],
    )
}

/// Resolves an Armand pine frame's `authored_id` into the handle that names the node.
pub fn armand_pine_handle_of(id: u32) -> Option<String> {
    node_ids(&armand_pine(ARMAND_SEED, Foliage::Needle).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a bough plate's `authored_id`: the bough is its own plant, so its walk is its own table
/// and the whole tree's would answer with the wrong node.
pub fn armand_pine_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&armand_pine_bough(ARMAND_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a base plate's `authored_id`: the base plate is the **whole tree** at another framing and
/// another dial, not a second plant, so its own table is the whole tree's.
pub fn armand_pine_base_handle_of(id: u32) -> Option<String> {
    armand_pine_handle_of(id)
}

// ---------------------------------------------------------------------------------------------
// 筇竹 Qiongzhuea tumidinoda
// ---------------------------------------------------------------------------------------------

/// The name the Qiong bamboo's clump plate is built and rendered under.
pub const QIONG_BAMBOO: &str = "qiong_bamboo";

/// The name the culm plate is built and rendered under: one culm at one metre, the nodes the subject.
pub const QIONG_BAMBOO_CULM: &str = "qiong_bamboo_culm";

/// The name the base plate is built and rendered under: the clump's foot and its rhizome crown.
pub const QIONG_BAMBOO_BASE: &str = "qiong_bamboo_base";

/// The seed the clump's leaf field is derived from.
const QIONG_SEED: u64 = 58;

/// The note's **ESTIMATE** of 8–20 culms to a clump, at twelve, and the whole of a clump's own
/// identity: FRPS gives the *stand* at 12–26 culms per square metre and says nothing per clump, so the
/// number is the note's estimate and the frame says so.
const CULMS: u32 = 12;

/// How many internodes a culm holds: the note's 15–25 cm on a 3.3 m culm is seventeen.
const INTERNODES: usize = 17;

/// The culm's radius law: the note's 1–3 cm diameter, at 3.3 cm low down and 1.8 cm at the top.
const RADIUS_LOW: f32 = 0.0165;
const RADIUS_HIGH: f32 = 0.0090;

/// **The node's half-height, as a fraction of the culm's own length: 0.005.** On a 3.31 m culm that is
/// **16.5 mm a side, 33 mm of collar**, which is 0.157 of the note's 21 cm internode — the ratio
/// `01-nodes-dark-culms.jpg` and `02-nodes-green-culms.jpg` both show, where the collar stands about
/// a sixth of the distance to the next one and the culm's diameter grows to 1.55 of its own.
///
/// # Why this is the species
///
/// **The name *tumidinoda* is this number.** FRPS: *"the node ring is extremely raised, forming a
/// prominent rounded ridge shaped like **two discs clasped one above the other**, with a shallow
/// ring-shaped seam-like groove between them"*. The gallery's own `derive::culm` already rises at a
/// node and falls between them — two knots — and this law is the same fact at seven, which is the
/// difference between a bamboo and this bamboo.
const NODE_HALF: f32 = 0.005;

/// The internode lengths, in metres, on a culm of scale one: the note's lowest few at 10–15 cm and
/// the rest holding 15–25 cm. They sum to 3.309 m, and their cumulative fractions are the fractions
/// every other law in this section is written at — which is what makes a branch land **on** a node.
fn internode_lengths() -> Vec<f32> {
    (0..INTERNODES)
        .map(|index| {
            if index < 3 {
                0.115 + 0.008 * index as f32
            } else {
                0.21
            }
        })
        .collect()
}

/// The fraction of the culm's own length at which each node stands: seventeen of them, the first at
/// one internode and the last at the tip.
fn node_fractions() -> Vec<f32> {
    let lengths = internode_lengths();
    let total: f32 = lengths.iter().sum();
    let mut running = 0.0;
    (1..=INTERNODES)
        .map(|index| {
            running += lengths[index - 1];
            running / total
        })
        .collect()
}

/// The internode radius at a fraction of the culm: a slow fall from a thick, nearly solid base to a
/// thinner tip, which is FRPS's *"the lowest internodes are almost solid and hollow only gradually
/// upward"*.
fn internode_radius(fraction: f32) -> f32 {
    RADIUS_LOW * 1.18 + (RADIUS_HIGH - RADIUS_LOW * 1.18) * fraction.powf(0.85)
}

/// The culm's taper law: **a sawtooth**, seven knots to a node.
///
/// # Invariants
///
/// **The knots are the seven of a clasped disc, in this order and no other.** Rising from the internode
/// to the *lower* disc's crest, dipping to the seam groove, rising again to the *upper* disc — which is
/// the larger of the two — and then undercutting back below the internode's own radius, which is the
/// sharp step under the upper disc that both photographs show and that no smooth law can draw. The
/// undercut below the internode radius is the whole of the "clasped" reading: a collar that only ever
/// rises is a bead, and a bead is every bamboo; two discs with a waist between them and a step under
/// them is a name.
///
/// **A node's knots never touch its neighbour's.** The internode is 0.06 of the culm and a node is
/// 0.010 of it, so the seven knots fit inside their own sixth with room to spare — and a law that
/// overran its node would put a node's undercut on its neighbour's crest.
fn culm_taper(scale: f32) -> shape::Scalar {
    let mut knots: Vec<(f32, f32)> = vec![(0.0, internode_radius(0.0) * scale)];
    // **The topmost node is not given a collar.** A culm ends in a thin whip — the note's photographs
    // show every one of them running out over its last two internodes — and a collar at t = 1 would
    // put its undercut on the tip itself, so the sixteenth node carries the last bead and the law
    // ends on a plain radius.
    let mut reached = 0.0;
    for fraction in node_fractions() {
        if fraction >= 0.999 {
            break;
        }
        let base = internode_radius(fraction) * scale;
        for (offset, radius) in [
            (-1.00, 0.99),
            (-0.58, 1.28),
            (-0.26, 1.43),
            (0.04, 1.16),
            (0.30, 1.58),
            (0.64, 1.42),
            (1.00, 0.90),
        ] {
            let station = (fraction + offset * NODE_HALF).clamp(0.0, 1.0);
            // **A knot that cannot advance is dropped rather than written twice**: the first node's
            // lower shoulder and the culm's own first knot both land at nought, and
            // [`shape::Scalar::new`] refuses two knots at one parameter.
            if station > reached {
                knots.push((station, base * radius));
                reached = station;
            }
        }
    }
    knots.push((1.0, RADIUS_HIGH * scale * 0.80));
    shape::Scalar::new(&knots).expect("a sawtooth of seventeen nodes is a law")
}

/// The culm's own spine: near-vertical, leaning by a few degrees with age, on a length of
/// `3.309 m × scale`.
///
/// # Invariants
///
/// **The spine is authored at four knots and the arc length is not the sum of its rises**, so a node's
/// fraction and the branch that hangs there differ by well under a per cent — which at a 33 mm collar
/// is under a millimetre, and is stated rather than assumed away.
fn culm_spine(scale: f32, lean: f32) -> shape::Curve {
    let height = 3.309 * scale;
    spine(&[
        ([0.0, 0.0, 0.0], [0.004 * lean, 0.330 * scale, 0.003 * lean]),
        (
            [0.012 * lean, height * 0.26, 0.009 * lean],
            [0.006 * lean, 0.331 * scale, 0.004],
        ),
        (
            [0.024 * lean, height * 0.55, 0.004 * lean],
            [0.004 * lean, 0.328 * scale, -0.005],
        ),
        (
            [0.030 * lean, height * 0.82, 0.026 * lean],
            [0.002, 0.330 * scale, 0.008],
        ),
        (
            [0.032 * lean, height, 0.052 * lean],
            [0.0, 0.325 * scale, 0.006],
        ),
    ])
}

/// The culm's section: **twelve points, one flat face and a round back**.
///
/// # Invariants
///
/// **One flat face and not two.** FRPS says the internodes *"carry one extremely narrow groove, and
/// the groove is on the same side of every internode"*, and separately that *"internodes at branching
/// nodes flatten on the side the branches emerge from"*. The second half of that — a flat face that
/// **alternates** with the branch positions along the culm's length — is not expressible: a section is
/// a node's own and a culm is one node, which is §4.14's *won't fix* read from the other side. The
/// first half is the persistent face, and it is the half that reads at a metre, because a culm's whole
/// silhouette is a straight line and its only relief is this.
fn culm_section() -> shape::Section {
    let sides = 12;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            // A face on the `+x` side, rounding away over sixty degrees either side of it.
            let flat = 1.0 - 0.30 * theta.cos().max(0.0).powf(4.0);
            [theta.cos() * flat, theta.sin() * flat]
        })
        .collect();
    shape::Section::outline(&points).expect("a flat-faced ring is a section")
}

/// The clump's rule: a rhizome crown at the root, twelve culms standing on it at one station, three
/// branches at every node above the bare lower zone, and twigs that carry the leaf clusters.
fn qiong_rule() -> Rule {
    Rule {
        trunk: rhizome,
        generations: vec![
            // **The clump.** Twelve culms at station 0.86 of a short vertical rhizome crown, and
            // `cluster` is the culm count — which is what the note's *"8–20 culms per clump"* is: one
            // station on the parent's skin and twelve children around it, so the clump's width is the
            // crown's own circumference, 0.22 m across, which is the note's 0.20–0.30 m culm spacing
            // read as a distance and not as a density.
            Generation {
                count: (CULMS, CULMS),
                station: (0.86, 0.86),
                slot: (0.0, 0.0),
                jitter: (0.0, 0.0),
                sink: 0.070,
                cluster: CULMS,
                shape: culm_shape,
            },
            // **Three branches at every node of the branched part.** The band is the nodes four to
            // fifteen and the count is 36 over twelve stations of three, so the stations land on
            // **exactly** those nodes: twelve stations spread evenly over eleven internode lengths is
            // one internode apart, because every internode past the third is the same length.
            Generation {
                count: (36, 36),
                station: branch_band(),
                slot: (0.0, GOLDEN),
                jitter: (0.0, 0.0),
                sink: 0.004,
                cluster: 3,
                shape: qiong_branch,
            },
            Generation {
                count: (2, 2),
                station: (0.42, 0.98),
                slot: (0.0, GOLDEN),
                jitter: (0.06, 0.10),
                sink: 0.002,
                cluster: 1,
                shape: qiong_twig,
            },
        ],
        // **No field.** A bamboo's culms are plumb and its branches arch under their own leaves, and
        // both are written in the shape functions where they can be written for the branches alone.
        fields: Vec::new(),
    }
}

/// The rhizome crown: the low swollen mound the culms stand out of, more than half of it below the
/// ground line.
///
/// FRPS describes the underground part as *"5 internodes below the soil surface, each bearing a ring
/// of 12 adventitious roots"*, and a clump's foot in a photograph is therefore not a stem end but a
/// **mound**. Its width is the number that fixes the clump: twelve culms stand on its rim, and the
/// note's **0.20–0.30 m between culms** is a statement about that rim's circumference and not about a
/// scatter. The first version made the crown 0.11 of radius, which puts twelve culms 0.06 m apart and
/// the whole clump inside a third of a metre — the clump plate came out as **one thick stem**, which
/// is the *lollipop* defect in a bamboo's own dress. At 0.38 of radius the rim is 2.4 m round and the
/// culms stand 0.19 m apart, the note's own figure.
fn rhizome() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.000, -0.30, 0.000], [0.008, 0.140, 0.006]),
            ([0.014, -0.14, 0.012], [0.014, 0.150, 0.010]),
            ([0.024, 0.02, 0.020], [0.010, 0.110, 0.006]),
            ([0.030, 0.12, 0.024], [0.004, 0.040, 0.0]),
        ]),
        // **A dome and not a saucer, and not a hat either.** Two earlier versions: the first closed
        // from its widest ring to a narrow top in the last fifth of its length and the clump plate came
        // out with a pale disc under the culms — the *floating tree* defect in a bamboo's dress; the
        // second held the width to 0.62 and then dropped it to a tenth, which is a flat-topped drum with
        // a cone on it and reads as a collar. This one **rises from a tenth of a metre at the buried
        // end to thirty-five centimetres at the rim and rounds off in the last tenth of its length**,
        // which is a mound, and the culms stand on its widest ring.
        taper(&[
            (0.00, 0.100),
            (0.30, 0.195),
            (0.60, 0.300),
            (0.80, 0.352),
            (0.93, 0.318),
            (1.00, 0.090),
        ]),
    )
}

/// Each culm is its own height and its own lean, drawn from its own stream.
///
/// A clump of twelve identical culms is a fence; the note's *"near-vertical, ≤10° from vertical with
/// age"* is a range, and the seed decides which of the twelve is the one that leans and which is the
/// tall one. The length law scales with the height, so a culm's internodes stay inside the note's
/// 15–25 cm at every one of the three scales.
fn culm_shape(rng: &mut Rng, _joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let scale = 0.92 + 0.34 * rng.unit();
    let lean = 0.6 + 0.8 * rng.unit();
    let girth = 0.88 + 0.20 * rng.unit();
    (culm_spine(scale, lean), culm_taper(scale * girth))
}

/// The band the branches hang over: the nodes **four to fifteen** of seventeen.
///
/// # Invariants
///
/// **Written from [`node_fractions`] and not typed, so the band and the sawtooth cannot drift apart.**
/// The lowest four internodes are the note's unbranched lower zone — the photographs show a bare
/// lower culm on every shoot — and the top two are the tip's own thin whip. Twelve stations spread
/// evenly over eleven internode lengths is one internode apart, which is the only reason a branch
/// lands *on* a node rather than a centimetre beside it: the internodes past the third are all the
/// same length, so equal steps in arc length are equal steps in node index.
fn branch_band() -> (f32, f32) {
    let nodes = node_fractions();
    (nodes[3], nodes[14])
}

/// A branch: it leaves the culm at about a right angle, runs out and **droops**, because the note's
/// fifth recognition fact is a *"soft, drooping, grey-green leaf mass … arching outward from the
/// culms"*.
fn qiong_branch(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.42 * (0.80 + 0.40 * rng.unit());
    // The higher on the culm a branch hangs, the shorter it is, which is the one law a bamboo's crown
    // has and the note does not state: `04-foliage-mass.jpg` is thickest in its middle.
    let rise = 0.95 + 0.45 * joint.station + 0.10 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(length, [rise, rise - 0.28, rise + 0.34, rise + 0.62]),
        taper(&[(0.0, radius * 0.46), (1.0, 0.0022)]),
    )
}

/// A twig: half the branch, drooping harder, and the thing the leaf clusters stand on.
fn qiong_twig(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.28 * (0.80 + 0.40 * rng.unit());
    let droop = 1.15 + 0.20 * joint.station + 0.12 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-5);
    (
        bend(length, [droop, droop - 0.22, droop + 0.30, droop + 0.52]),
        taper(&[(0.0, radius * 0.62), (1.0, 0.0011)]),
    )
}

/// The single culm's own rule, for the culm plate: the culm **is** the trunk, so its nodes are the
/// taper law's and there is no clump under it. Its branches hang over the same twelve nodes and its
/// leaves therefore sit one generation shallower than the clump's — which is the whole of what
/// `branch_depth` is, and the reason it is a parameter and not a constant.
fn culm_rule() -> Rule {
    Rule {
        trunk: || (culm_spine(1.06, 0.9), culm_taper(1.02)),
        generations: vec![
            Generation {
                count: (18, 18),
                station: (0.34, 0.90),
                slot: (0.0, GOLDEN),
                jitter: (0.04, 0.05),
                sink: 0.003,
                cluster: 3,
                shape: qiong_branch,
            },
            Generation {
                count: (2, 2),
                station: (0.42, 0.98),
                slot: (0.0, GOLDEN),
                jitter: (0.06, 0.10),
                sink: 0.002,
                cluster: 1,
                shape: qiong_twig,
            },
        ],
        fields: Vec::new(),
    }
}

/// The leaf: **narrowly lanceolate, 5–14 cm × 6–12 mm**, at the middle of both ranges, and **arching
/// down** — the note's blade as `05-leaf.jpg` photographs it, a long narrow strap that hangs off its
/// twig rather than standing out from it.
fn leaf() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [0.046, -0.002, 0.0]),
            ([0.044, -0.006, 0.001], [0.040, -0.014, 0.001]),
            ([0.080, -0.026, 0.002], [0.030, -0.020, 0.0]),
        ]),
        // Lanceolate: widest at a fifth of the length, drawn out to a point, which is the note's
        // *"narrowly lanceolate"* and not the lens a generic leaf gets.
        taper(&[
            (0.0, 0.0018),
            (0.18, 0.0046),
            (0.52, 0.0040),
            (0.82, 0.0024),
            (1.0, 0.0003),
        ]),
        0.00028,
        6,
        4,
    )
}

/// The leaf fields at `branch_depth`: **two to four leaves at a twig's tip** and a thinner scatter
/// along the branch itself.
///
/// # Invariants
///
/// **`cluster` is three and it is the note's own number.** FRPS: *"小枝具2-4叶"* — two to four leaves
/// to a twig — and three is the middle. The fan is wide (1.35 rad) because a bamboo's leaf cluster is
/// a **fan**: the leaves leave the tip of one twig within a narrow cone and spread through it, which
/// is what `05-leaf.jpg` shows and what a [`LeafField::cluster`] of three expresses as one station.
///
/// **The band starts at 0.30 of the twig and not at nought**, because a bamboo's twig is wood for its
/// inner two thirds and leaves for the rest — the *dead give* defect, foliage evenly from the base to
/// the tip of every shoot, is the one thing this species most obviously does not do.
///
/// **`branch_depth` is the depth the branches hang at**, which is 2 on the clump (rhizome, culm,
/// branch, twig) and 1 on the culm plate (culm, branch, twig). One number places both, and the same
/// mistake the spire module records — a field named on the literal generation of one plate and left
/// there for the other — is what it prevents.
fn qiong_leaves(branch_depth: usize) -> Vec<LeafField> {
    let mut branch_leaves = LeafField::new(branch_depth - 1, 0);
    branch_leaves.band = (0.55, 0.99);
    branch_leaves.density = rising(64.0, 114.0, 170.0, 1.0);
    branch_leaves.cluster = 2;
    branch_leaves.fan = shape::Scalar::constant(1.05);
    branch_leaves.phyllotaxy = GOLDEN;
    branch_leaves.pitch = shape::Scalar::constant(1.14);
    branch_leaves.size = shape::Scalar::constant(0.92);
    branch_leaves.jitter = 0.18;
    let mut tips = LeafField::new(branch_depth, 0);
    tips.band = (0.30, 1.0);
    // **Two and a half times the first pass's count**, and the first pass was the finding: at 24–56 a
    // metre the clump plate rendered as a bundle of stems with a haze of blades between them, where
    // `04-foliage-mass.jpg` is a mass that hides its own culms. The plate's own pixel count says the
    // same thing — a 9 mm leaf is 1.5 pixels at nine metres and a third of them face away from the
    // lens, so the count has to be roughly tripled to draw the leaf area the eye expects.
    tips.density = rising(88.0, 152.0, 214.0, 1.0);
    tips.cluster = 3;
    tips.fan = shape::Scalar::constant(1.35);
    tips.phyllotaxy = GOLDEN;
    // **A pitch of 1.22 rad is seventy degrees off the twig's own outward direction**, which is how a
    // bamboo's leaf stands: out and up, with the blade's own arch taking the tip down again.
    tips.pitch = shape::Scalar::constant(1.22);
    tips.size = shape::Scalar::new(&[(0.0, 0.78), (1.0, 1.04)]).expect("a size is a law");
    tips.jitter = 0.20;
    vec![branch_leaves, tips]
}

/// The clump: twelve culms from one rhizome crown, the whole rule grown at one seed.
pub fn qiong_bamboo(seed: u64) -> Species {
    let mut species = Species {
        plant: grow(&qiong_rule(), seed),
        templates: vec![leaf()],
        fields: qiong_leaves(2),
        decorators: Vec::new(),
    };
    dress_culms(&mut species.plant);
    species
}

/// The culm plate's species: **one culm as the trunk**, so that the plate is a culm at one metre and
/// the collar's thirty-three millimetres are forty pixels of it.
pub fn qiong_culm(seed: u64) -> Species {
    let mut species = Species {
        plant: grow(&culm_rule(), seed),
        templates: vec![leaf()],
        fields: qiong_leaves(1),
        decorators: Vec::new(),
    };
    dress_culms(&mut species.plant);
    species
}

/// Gives every culm in a plant its flat face: the root's own children are the clump's culms and the
/// root itself is the rhizome crown, which keeps its own round section.
///
/// A section is a node's own, so this is a walk over the culms after `grow` and not a field of the
/// rule — the same ordering the pine's bark takes.
fn dress_culms(plant: &mut vegetation::plant::Plant) {
    for index in 0..CULMS {
        if let Ok(culm) = plant.get_mut(&Handle::root().child(index)) {
            culm.set_section(culm_section());
        }
    }
}

/// The bamboo's surfaces: a **smooth waxy green culm** (FRPS: *"绿色，光滑无毛，无蜡粉"* — green,
/// smooth, glabrous, **without any wax bloom**) and a **grey-green leaf** (the note's *"green above,
/// grey-green beneath"*).
///
/// # Invariants
///
/// **The two greens are far apart on purpose and the culm is the lighter of them.** A culm that is as
/// dark as its own leaves is the *felt-pen tree* defect, and this species' plate is the one where it
/// would be least noticed and most wrong: the culms are thin, near-vertical and mostly bare, so the
/// only colour in the lower two thirds of the frame is the culm's.
pub fn qiong_palette() -> Palette {
    Palette {
        wood: Skin::new([0.232, 0.372, 0.148], 0.60),
        leaves: vec![Skin::new([0.176, 0.318, 0.150], 0.74)],
        seats: Vec::new(),
    }
}

/// A moist north-facing gully at two thousand metres: a **cool sky-dominant light** under a canopy of
/// broadleaved trees, which is where FRPS places this species — *"大面积集中成片生长于山区上部到山脊
/// 的常绿阔叶林"* — and it is why the key is weak and the sky is not.
fn qiong_lighting() -> Lighting {
    Lighting {
        key_position: [9.0, 11.0, -5.0],
        key_colour: [0.98, 0.98, 0.90],
        key_irradiance: 2.2,
        key_distance: 15.0,
        key_reach: 1200.0,
        // **Three, and not six.** The page is the key's own extent doubled across a 1024 map, so six
        // is a twelve-metre plane and one texel is 11.7 mm — a 30 mm culm is two and a half texels and
        // the clump's own shadow came out a dithered black stipple laid over two metres of ground. At
        // three a texel is 5.9 mm and a culm is five of them.
        key_extent: 3.0,
        fill_position: [-7.0, 4.0, 6.0],
        fill_colour: [0.46, 0.56, 0.74],
        fill_irradiance: 0.30,
        fill_distance: 10.0,
        floor: Skin::new([0.180, 0.190, 0.110], 0.96),
        sky: Atmosphere::new()
            .zenith([0.12, 0.22, 0.44])
            .horizon([0.56, 0.62, 0.62])
            .ground([0.24, 0.25, 0.18])
            .density(0.0010)
            .scale_height(1200.0),
        sky_radiance: 0.55,
        haze: 0.004,
    }
}

/// The culm plate's own light: **brought in to three metres and down to twenty-two degrees**, so a
/// thirty-three-millimetre collar at one metre casts a shadow of its own — the whole point of the
/// plate, since the collar's reading is the step under the upper disc and a step needs a light nearly
/// level with it.
fn culm_lighting() -> Lighting {
    Lighting {
        key_position: [2.55, 0.98, -1.05],
        key_colour: [1.0, 0.95, 0.84],
        key_irradiance: 2.6,
        key_distance: 2.90,
        key_reach: 40.0,
        key_extent: 0.55,
        fill_position: [-1.4, 0.7, 1.5],
        fill_colour: [0.46, 0.56, 0.74],
        fill_irradiance: 0.34,
        fill_distance: 2.2,
        floor: Skin::new([0.180, 0.190, 0.110], 0.96),
        sky: Atmosphere::new()
            .zenith([0.14, 0.24, 0.46])
            .horizon([0.58, 0.64, 0.63])
            .ground([0.24, 0.25, 0.18])
            .density(0.0012)
            .scale_height(1200.0),
        sky_radiance: 0.62,
        haze: 0.0002,
    }
}

/// The base plate's own light: the culm light turned onto its side and lifted, because the foot of a
/// clump is lit by the light that reaches the forest floor and not by the light that reaches a collar.
///
/// **The page is eight metres across and the key is at thirty-four degrees**, and both are the second
/// pass's answers to a striped floor: at 1.8 of extent a vertical culm is four shadow texels wide and
/// its own shadow came out as a fan of hard alternating bands laid across the whole ground. A wider
/// page and a higher key put one culm at eleven texels and turn the same shadow into a soft band.
fn base_lighting() -> Lighting {
    Lighting {
        key_position: [5.2, 3.6, -3.4],
        key_colour: [1.0, 0.96, 0.86],
        key_irradiance: 3.0,
        key_distance: 7.2,
        key_reach: 60.0,
        key_extent: 4.0,
        fill_position: [-3.0, 1.4, 3.4],
        fill_colour: [0.46, 0.56, 0.74],
        fill_irradiance: 0.40,
        fill_distance: 4.9,
        floor: Skin::new([0.180, 0.190, 0.110], 0.96),
        sky: Atmosphere::new()
            .zenith([0.13, 0.23, 0.45])
            .horizon([0.57, 0.63, 0.62])
            .ground([0.24, 0.25, 0.18])
            .density(0.0011)
            .scale_height(1200.0),
        sky_radiance: 0.62,
        haze: 0.0004,
    }
}

/// The clump's frame: **nine metres** off a three-metre clump, so a 9 mm leaf is 1.5 pixels and the
/// crown reads as the mass `04-foliage-mass.jpg` shows rather than as individual blades.
///
/// The frame is taller than it is wide because a bamboo's habit is its own — a vertical thicket two
/// and a half times as tall as it is wide — and a landscape frame either crops the culms or stands so
/// far back that a 3 cm culm is a line. **The eye is 1.55 m**, which is the gully floor's own height
/// and not half the culm's, so the clump is seen from a person standing in front of it.
const QIONG_FRAME: Framing = Framing {
    width: 1100,
    height: 1500,
    eye: [5.2, 1.55, 7.0],
    target: [0.0, 1.90, 0.0],
    field_of_view: 0.72,
    near: 0.10,
    far: 600.0,
    floor: 300.0,
    ground: Ground::Plane,
};

/// The culm plate's frame: **1.0 m off one culm at the height of its fourth node**, on a lens wide
/// enough to hold a metre of culm whole — the frame the note's `01-nodes-dark-culms.jpg` is
/// photographed at, and the one in which the clasped discs are the subject.
///
/// One pixel here is `2 · 1.0 · tan 0.45 / 1400` = **0.69 mm**, so the 33 mm collar is 48 pixels, the
/// 21 cm internode is 300 of them, and a 9 mm leaf is thirteen.
const CULM_FRAME: Framing = Framing {
    width: 1200,
    height: 1400,
    eye: [0.86, 1.18, 0.52],
    target: [0.0, 1.10, 0.0],
    field_of_view: 0.90,
    near: 0.02,
    far: 30.0,
    floor: 10.0,
    ground: Ground::Plane,
};

/// The base plate's frame: **4.6 m** off the clump's foot and a metre above it, so the mound the
/// culms come out of is the subject and not a disc at the bottom of the frame.
///
/// The first frame stood at 2.6 m on a 0.80 rad lens and the crown's own lowest boughs were in the
/// top third with the foot in the bottom sixth, which is a portrait of a tree and not of a foot.
const QIONG_BASE_FRAME: Framing = Framing {
    width: 1200,
    height: 1000,
    eye: [2.90, 1.05, 3.40],
    target: [0.0, 0.42, 0.0],
    field_of_view: 0.80,
    near: 0.05,
    far: 40.0,
    floor: 12.0,
    ground: Ground::Plane,
};

/// The dial a culm plate is expanded at: **spacing 0.008**, so a node's collar is resolved by four
/// rings and the sawtooth is a step rather than a hint.
///
/// # Invariants
///
/// **The default half-metre spacing would give a 3.3 m culm seven rings** — two rings to a node, which
/// is a smooth sausage and not a bead — so this is one of the two plates in this repository whose
/// subject is a feature *smaller than the default ring spacing*. [`Detail`] is per planting and not per
/// node (§4.14's *won't fix*), which is why the culm plate grows its own single-culm species rather
/// than framing the clump's twelve: twelve culms at eight-millimetre spacing is 165 rings each and
/// eight hundred twigs at the same spacing is three hundred and fifty thousand corners.
const CULM_DETAIL: Detail = Detail {
    stations: 6,
    around: 12,
    spacing: 0.008,
};

/// The dial the clump plate's wood is expanded at: **spacing 0.030**, which gives a culm 110 rings
/// (2.6 to a collar, so the node reads as a bead at nine metres) and a 22 cm twig eleven.
const CLUMP_DETAIL: Detail = Detail {
    stations: 5,
    around: 12,
    spacing: 0.030,
};

/// Builds the bamboo's clump plate: twelve culms from one crown, at nine metres.
pub fn qiong_bamboo_picture() -> Value {
    let species = qiong_bamboo(QIONG_SEED);
    let planting = Planting::new(species)
        .with_palette(qiong_palette())
        .with_detail(CLUMP_DETAIL);
    plant_scene_lit(
        &[planting],
        &QIONG_FRAME,
        QIONG_BAMBOO,
        &qiong_lighting(),
        &[],
    )
}

/// Builds the culm plate: **one culm at one metre**, where the swollen clasped nodes are the subject.
pub fn qiong_bamboo_culm_picture() -> Value {
    let species = qiong_culm(QIONG_SEED);
    let planting = Planting::new(species)
        .with_palette(qiong_palette())
        .with_detail(CULM_DETAIL);
    plant_scene_lit(
        &[planting],
        &CULM_FRAME,
        QIONG_BAMBOO_CULM,
        &culm_lighting(),
        &[],
    )
}

/// Builds the base plate: the clump's foot and its rhizome crown at 2.6 m.
pub fn qiong_bamboo_base_picture() -> Value {
    let species = qiong_bamboo(QIONG_SEED);
    let planting = Planting::new(species)
        .with_palette(qiong_palette())
        .with_detail(CLUMP_DETAIL);
    plant_scene_lit(
        &[planting],
        &QIONG_BASE_FRAME,
        QIONG_BAMBOO_BASE,
        &base_lighting(),
        &[],
    )
}

/// Resolves a clump frame's `authored_id` into the handle that names the node.
pub fn qiong_bamboo_handle_of(id: u32) -> Option<String> {
    node_ids(&qiong_bamboo(QIONG_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a culm plate's `authored_id`: the culm is its own plant, so its walk is its own table and
/// the clump's would answer with the wrong node.
pub fn qiong_bamboo_culm_handle_of(id: u32) -> Option<String> {
    node_ids(&qiong_culm(QIONG_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a base plate's `authored_id`: the base plate is the **clump** at another framing.
pub fn qiong_bamboo_base_handle_of(id: u32) -> Option<String> {
    qiong_bamboo_handle_of(id)
}
