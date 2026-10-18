//! The spire conifers: 丽江云杉 and 苍山冷杉, two species that are the same rule with different numbers.
//!
//! # Why these two are one module
//!
//! [`southwest`](southwest.rs) drew the Yunnan pine and paid for the machinery this module reuses; the
//! proposal's arc 3 asks for **the spire conifers, to prove the pine was numbers and not a special case**.
//! So this module authors one shape vocabulary — a whorled bough, a shoot, a needle blade, a cone —
//! and both species are it with a different count, a different crown profile, a different phyllotaxy
//! and a different cone. The two numbers that make a fir a fir are **a phyllotaxy of one half, which is
//! two ranks instead of a spiral**, and **a cone template that stands up where the spruce's hangs down**;
//! everything else is a dials' turn, and the module says which dial for each fact.
//!
//! # Two plates and two foliages, and why they cannot share one
//!
//! Each species arrives as two plates because the bar asks two questions at two distances: the whole
//! tree, for the silhouette, and one bough at arm's length, for the foliage. They cannot share one
//! foliage, because **the rasteriser samples a pixel at one point and a blade narrower than a pixel is
//! dropped whole, not faded**. A spruce needle is 1.2 mm and a fir needle 2.1 mm across; at the forty
//! metres the whole-tree plate is framed from, one pixel is about twenty-two millimetres. So the
//! whole-tree plate draws [`Foliage::Card`] and the bough plate draws [`Foliage::Needle`], the note's
//! own width. That is a level of detail and not a fudge: each plate is honest about the blade it draws.
//!
//! ## A card stands for a shoot's plume, and not for one needle
//!
//! **The card is widened AND lengthened, and the lengthening is the larger half of it.** The first
//! version of this module widened the card and left its length at the note's own ten millimetres: a
//! blade twenty-two millimetres wide and ten long is one pixel by **half a pixel**, and the frame came
//! out as a crown of bare wood with a haze of specks on it. A card one pixel wide is only honest if it
//! is also *several* pixels long, and what it then stands for is the unit the crown is made of — the
//! plume of a shoot's needles — rather than a single needle. So the spruce's card is 95 mm long and
//! 22 mm wide (a shoot is 160 mm) and the fir's is 85 mm by 15 mm (a shoot is 220 mm), and the density
//! law is cut to keep the crown's total blade area where it was. **The blade is still one pixel wide,
//! which is the number the sampling tax is stated in, and the card is still a needle shape.**
//!
//! # The two silhouettes, and how they are told apart without their foliage
//!
//! A conifer silhouette is one number against another and the bar reads them with the leaves off:
//!
//! | | 丽江云杉 *Picea likiangensis* | 苍山冷杉 *Abies delavayi* |
//! | --- | --- | --- |
//! | crown width ÷ height | **0.20**, the top of the note's 0.15–0.20 | **0.24**, the bottom of the note's 0.24–0.40 |
//! | crown width in metres | **6.8 m** | **6.0 m** |
//! | bough tips | **drooping** — the outer two knots turn below the horizontal | **upswept** — the outer knots turn a quarter turn above it |
//! | cones | **pendant**, purple-red, hung under the foliage | **erect**, blue-black, standing above the crown line |
//! | crown base | broad, and the branches run down near the ground | narrow, and the crown starts at 0.32 of the height |
//!
//! # One place the two notes disagree, and how it is resolved
//!
//! **The spruce's crown-to-height ratio (0.15–0.20) is smaller than the fir's (0.24–0.40), so read as
//! ratios the fir is the broader tree.** The bar asks for the fir to be the narrower spire, and the
//! photographs agree with the bar: `02-crown.jpg` is a stand of narrow tiered spires. So this module
//! honours both notes' **absolute** crown widths at the ends of their bands that make the two
//! distinguishable — the spruce at the top of its 6–10 m band and the fir at the bottom — which puts
//! 6.8 m of crown on a 34 m spruce and 6.0 m on a 25 m fir. The ratio column above is the consequence,
//! not the choice, and it is stated here rather than left to be discovered in a frame.
//!
//! # What is duplicated from [`southwest`](southwest.rs) and why
//!
//! The [`bend`] drawing tool and the [`Foliage`] enum are the pine's, restated. This slice may not edit
//! the pine's module, so the two copies stand until a later slice lifts them into one shared file.

use definition::Value;
use vegetation::decorator::{Seat, Template, scatter};
use vegetation::derive::Species;
use vegetation::field::Field;
use vegetation::handle::Handle;
use vegetation::leaf::{LeafField, LeafTemplate};
use vegetation::rng::Rng;
use vegetation::species::{Generation, Joint, Rule, grow, spine, taper};

use crate::air::Atmosphere;
use crate::vegetation_mesh::node_ids;
use crate::vegetation_picture::{
    Framing, Ground, Lighting, Palette, Planting, Skin, plant_scene_lit,
};

/// The name the Lijiang spruce's whole-tree plate is built and rendered under.
pub const LIJIAN_SPRUCE: &str = "lijian_spruce";

/// The name the Lijiang spruce's branch plate is built and rendered under.
pub const LIJIAN_SPRUCE_BOUGH: &str = "lijian_spruce_bough";

/// The name the Cangshan fir's whole-tree plate is built and rendered under.
pub const CANGSHAN_FIR: &str = "cangshan_fir";

/// The name the Cangshan fir's branch plate is built and rendered under.
pub const CANGSHAN_FIR_BOUGH: &str = "cangshan_fir_bough";

/// The seed both species' leaf fields are derived from.
const SEED: u64 = 34;

/// The golden angle as a fraction of a turn, which is the step between two of a spruce's needles.
const GOLDEN: f32 = 0.381_966_01;

/// The half-turn, which is the fir's phyllotaxy: **two lateral ranks instead of a spiral**.
const OPPOSITE: f32 = 0.5;

/// The spruce needle's own length and half-width: the note's 0.6–1.5 cm × 1–1.5 mm at the middle of
/// both ranges. This is the blade the bough plate draws.
const SPRUCE_NEEDLE_LENGTH: f32 = 0.010;
const SPRUCE_NEEDLE_HALF_WIDTH: f32 = 0.0006;

/// The fir needle's own length and half-width: the note's 1.5–3 cm × 1.7–2.5 mm at the middle of both
/// ranges.
const FIR_NEEDLE_LENGTH: f32 = 0.022;
const FIR_NEEDLE_HALF_WIDTH: f32 = 0.00105;

/// Half of one pixel at the spruce's whole-tree distance, and the card's length there.
///
/// The frame stands 44.7 m off the tree with a 0.80 rad vertical field over 1700 rows, so one pixel is
/// `2 · 44.7 · tan 0.40 / 1700` = **22.2 mm**. The card is **two and a half pixels** wide rather than
/// one, because the width law tapers: a blade one pixel wide at its widest averages about six tenths
/// of a pixel along its length, which is under one, and a blade under one pixel is dropped rather
/// than faded. Its length is 130 mm, 5.9 pixels, and four fifths of a shoot. See the module docs.
const SPRUCE_CARD_HALF_WIDTH: f32 = 0.0280;
const SPRUCE_CARD_LENGTH: f32 = 0.130;

/// Half of two and a half pixels at the fir's whole-tree distance, and the card's length there.
///
/// The frame stands 29.7 m off the tree with a 0.82 rad vertical field over 1700 rows, so one pixel is
/// `2 · 29.7 · tan 0.41 / 1700` = **15.2 mm**, and the card is 50 mm by 110 mm — 3.3 pixels wide and
/// 7.2 long, and half of a shoot.
const FIR_CARD_HALF_WIDTH: f32 = 0.0250;
const FIR_CARD_LENGTH: f32 = 0.110;

/// Which foliage a spire conifer draws.
///
/// # Invariants
///
/// **The two foliages are the two plates and not two moods.** A needle is the needle's own width or it
/// is not a needle; the card exists because a point-sampled pixel cannot keep a blade that is thinner
/// than itself, and it is drawn only by the frame far enough away that it is one pixel wide. See the
/// module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foliage {
    /// The note's own blade, for a frame within arm's length of a shoot.
    Needle,
    /// A blade one pixel wide at the distance the whole-tree plate is framed from.
    Card,
}

impl Foliage {
    /// The blade's half-width for one species: the note's own for the needle, and **two and a half
    /// pixels** at the distance that species' whole-tree plate is framed from for the card.
    ///
    /// The numbers are per species and not per foliage because a pixel is a property of a *frame*, and
    /// the two frames are at different distances: one pixel is 22.2 mm on the spruce's plate and 15.2
    /// mm on the fir's. And **two and a half rather than one** because the width law tapers: a blade
    /// one pixel wide at its widest averages about six tenths of a pixel along its length, which is
    /// under one, and a blade under one pixel is dropped rather than faded. The first four renders of
    /// this module drew one pixel and two and a half and came out as a crown of bare wood with a
    /// measured silhouette coverage of 0.20 in the crown.
    fn half_width(self, needle_half_width: f32, card_half_width: f32) -> f32 {
        match self {
            Foliage::Needle => needle_half_width,
            Foliage::Card => card_half_width,
        }
    }

    /// The blade's own length, for the same reason as [`Foliage::half_width`]: the card is lengthened
    /// as well as widened, or it is a speck.
    fn length(self, needle_length: f32, card_length: f32) -> f32 {
        match self {
            Foliage::Needle => needle_length,
            Foliage::Card => card_length,
        }
    }

    /// The fraction of the needle foliage's density this foliage is drawn at. One for the needle; for
    /// the card the same quarter the pine's card is drawn at, which is the ratio that keeps the two
    /// foliages' total blade area in the same order — the card is both wider and longer, so its
    /// density is cut further than a widening alone would ask, and the crown's mass is judged by eye
    /// against the reference photographs rather than by arithmetic.
    fn density_factor(self) -> f32 {
        match self {
            Foliage::Needle => 1.0,
            Foliage::Card => 0.27,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 丽江云杉 Picea likiangensis
// ---------------------------------------------------------------------------------------------

/// The station on the spruce's leader where its crown begins.
///
/// **The note's clean trunk is 18–30 m on a fifty-metre tree**, and that is the forest-grown habit read
/// from a dense stand. This species is drawn as the open-grown individual of `01-habit.jpg`, whose
/// lower boughs run down toward the ground, which the proposal's own species line states: *"a crown
/// that keeps its lower branches to the ground"*. So the foliage band starts at 0.11 of the height,
/// below the note's 0.4–0.6 and deliberately so.
const SPRUCE_CROWN_BASE: f32 = 0.11;

/// The station on the fir's leader where its crown begins: the note's clean trunk is 8–15 m on a
/// twenty-five-metre tree, an **ESTIMATE** read from `01-habit.jpg` and `02-crown.jpg`, and this is
/// its bottom at 0.32.
const FIR_CROWN_BASE: f32 = 0.32;

/// The spruce's rule: whorled tiers that shorten with height, shoots on the tiers, and a single
/// four-angled needle on every shoot.
///
/// # The numbers and where they come from
///
/// Every figure is [`picea_likiangensis.md`](../../../../.agents/investigate/trees/picea_likiangensis.md)'s
/// modelling table, and the two that are **ESTIMATE** in that table are marked as such here.
///
/// **The tree is drawn at thirty-four metres, not the note's fifty.** FOC's fifty is a maximum for the
/// tallest trees of the belt, and a fifty-metre spire framed whole needs a seventy-metre standoff, at
/// which the crown is a ribbon four hundred pixels wide and every needle is a fifth of a pixel. The
/// reduction is a pure scale: every law here is a fraction of the height, so the drawn tree keeps the
/// note's crown-to-height ratio, its whorl count per metre of leader, its taper fractions and its
/// bough angles, and the slender proportion of 2.5 m d.b.h. on 50 m is kept as 0.85 m on 34 m.
fn lijian_spruce_rule() -> Rule {
    Rule {
        trunk: spruce_bole,
        generations: vec![
            // **The whorls.** Five to eight boughs at one station (the note's ESTIMATE, and six is its
            // middle), spread as `count / cluster` tiers up the crown band: 258 children at six to a
            // whorl is forty-three tiers over 30 m of leader, which is 0.70 m between whorls. That is
            // wider than the note's 0.25–0.50 m, and it is the node budget that widens it:
            // [`PLANTING_STRIDE`](crate::vegetation_picture::PLANTING_STRIDE) is four thousand nodes
            // for a whole planting and the shoot generation below has to fit inside what the whorls
            // leave.
            Generation {
                count: (258, 258),
                station: (SPRUCE_CROWN_BASE, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.05, 0.05),
                sink: 0.04,
                cluster: 6,
                shape: spruce_bough,
            },
            // **The shoots.** Nine to a tier over its outer four fifths: a conifer's crown is the
            // number of its shoots, and a tier carrying four of them would be the bare broom the defect
            // list names. Nine is the number the node budget and the crown's mass between them fix, and
            // it is the second of the three numbers that moved most between this module's iterations.
            Generation {
                count: (10, 10),
                station: (0.20, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.16, 0.14),
                sink: 0.008,
                cluster: 1,
                shape: spruce_shoot,
            },
        ],
        // No light field and no gravity: **a spruce's droop is drawn in its shape functions**, because a
        // field applies to the leader as well and a plumb bole is what the note's `03-bark.jpg` shows.
        fields: Vec::new(),
    }
}

/// The spruce's bole: thirty-four metres, plumb, with the root flare the bar's H4 asks for.
fn spruce_bole() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.00, 0.0, 0.00], [0.02, 1.00, 0.00]),
            ([0.04, 8.5, -0.03], [0.02, 1.00, -0.01]),
            ([0.08, 18.0, 0.02], [0.01, 1.00, 0.01]),
            ([0.12, 27.0, 0.09], [0.01, 1.00, 0.01]),
            ([0.15, 34.0, 0.15], [0.00, 1.00, 0.00]),
        ]),
        // The flare is the first two knots and nothing else: a trunk that meets the ground on a cone is
        // the *floating tree* defect, and a root plate is a different species' question.
        taper(&[
            (0.000, 1.10),
            (0.015, 0.86),
            (0.070, 0.72),
            (0.260, 0.54),
            (0.520, 0.38),
            (0.800, 0.21),
            (1.000, 0.035),
        ]),
    )
}

/// The bole's section: twenty sides of unequal radius, which is what *"dull gray or brown-gray,
/// breaking into thick, irregular plates"* is in geometry. **Plates, not fissures** — the spruce's bark
/// is the opposite of the pine's small scaly plates: thick, and irregular in plan as well as along the
/// trunk, so the outline is lumpy rather than fluted.
fn spruce_bark() -> shape::Section {
    let sides = 20;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let plate = 1.0 + 0.085 * (theta * 3.0).sin() + 0.045 * (theta * 5.0 + 1.3).sin();
            [theta.cos() * plate, theta.sin() * plate]
        })
        .collect();
    shape::Section::outline(&points).expect("a plated ring is a section")
}

/// A first-order bough: it leaves the leader at the note's 70–90°, runs out almost level, and **turns
/// down through its last two knots**, which is the drooping tip `04-branch.jpg` shows on every bough of
/// the crown. Its length is a law over the station it hangs at, which is the crown's own profile, and
/// [`Joint::station`] exists for exactly that law.
fn spruce_bough(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let profile = crown_profile(joint.station, SPRUCE_CROWN_BASE);
    // Three and a third metres at the base of the crown and a quarter of a metre at the leader's tip:
    // 6.8 m of crown is this number doubled, and the crown falls away faster than it narrows, which is
    // what a spire is.
    let length = (0.24 + 3.16 * (1.0 - profile).powf(1.55)) * (0.76 + 0.48 * rng.unit());
    let departure = 1.30 + 0.22 * profile + 0.10 * rng.signed();
    // **The droop is a dials' turn and not a constant**: a lower bough carries far more foliage and
    // hangs far more than a leader-side one, and the difference is the whole of the silhouette.
    let droop = 0.28 + 0.66 * (1.0 - profile) * (0.85 + 0.30 * rng.unit());
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.12,
                departure + 0.06,
                departure + droop,
            ],
        ),
        taper(&[
            (0.00, radius * 0.16),
            (0.18, radius * 0.13),
            (0.62, radius * 0.08),
            (1.00, 0.007),
        ]),
    )
}

/// A shoot: the peg-bearing shoot the needles stand on.
///
/// # Invariants
///
/// **Sixteen centimetres is an extension of the note's ESTIMATE, and it is stated as one.** The note
/// gives 5–12 cm, read from `05-leaf.jpg` and `07-cone-hanging.jpg`, which are close photographs of
/// the *finest* shoot tips; a tier three metres long cannot be clothed by eleven of them. So the node
/// drawn here is the second-order branchlet, which carries the note's own 25–60 needles per shoot and
/// is the unit the crown's mass is counted in — the same substitution the pine makes when it draws a
/// thirty-centimetre twig for a note that says ten to twenty. The needles themselves are the note's own
/// size and are never enlarged.
///
/// It leaves its tier at about a right angle and **hangs below it for the last third of its length**,
/// which is the pendulous spray of `04-branch.jpg`.
fn spruce_shoot(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.16 * (0.80 + 0.40 * rng.unit());
    let departure = 1.12 + 0.28 * rng.signed();
    let droop = 0.52 + 0.45 * rng.unit();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.20,
                departure + 0.34,
                departure + droop,
            ],
        ),
        taper(&[(0.00, radius * 0.46), (0.35, radius * 0.26), (1.00, 0.0018)]),
    )
}

/// The spruce's rule at one bough, for the bough plate: one drooping bough as the trunk, its shoots as
/// the only generation, and the note's own needle. The bough is longer than the plate's frame so the
/// frame shows a run of a dozen shoots the way `04-branch.jpg` does, rather than three.
fn lijian_spruce_bough_rule() -> Rule {
    Rule {
        trunk: || {
            (
                bend(1.95, [1.22, 1.36, 1.44, 1.92]),
                taper(&[(0.0, 0.019), (0.5, 0.012), (1.0, 0.005)]),
            )
        },
        generations: vec![Generation {
            count: (60, 60),
            station: (0.12, 1.0),
            slot: (0.0, GOLDEN),
            jitter: (0.16, 0.12),
            sink: 0.004,
            cluster: 1,
            shape: spruce_shoot,
        }],
        fields: Vec::new(),
    }
}

/// A blade of `length` and `half_width`: the spruce needle drawn at the note's own 0.6–1.5 cm × 1–1.5
/// mm when the foliages are [`Foliage::Needle`], and the plate's card when they are [`Foliage::Card`].
///
/// **Every one of the midrib's six numbers is scaled by `length`, the positions as well as the
/// tangents.** A curve is pinned by its knots and steered only by its tangents, so a card whose
/// tangents were scaled and whose positions were not came out 8 mm long and 22 mm wide — a speck that
/// the frame showed as bare wood. The whole card is the note's blade at a different size, and one
/// number does that.
///
/// It is thick in proportion to its width — four fifths as thick as it is wide — because FOC's section
/// is *"broadly rhombic"*, which is the *Picea* diagnostic against *Abies*' flattened one.
fn spruce_needle(half_width: f32, length: f32) -> LeafTemplate {
    let reach = length / SPRUCE_NEEDLE_LENGTH;
    LeafTemplate::blade(
        spine(&[
            ([0.0000, 0.00000, 0.0], [0.0042 * reach, 0.00070 * reach, 0.0]),
            ([0.0044 * reach, 0.00105 * reach, 0.0], [0.0036 * reach, 0.00080 * reach, 0.0]),
            (
                [0.0081 * reach, 0.00165 * reach, 0.0],
                [0.0022 * reach, 0.00060 * reach, 0.0],
            ),
        ]),
        taper(&[
            (0.0, half_width * 0.75),
            (0.30, half_width),
            (0.72, half_width * 0.70),
            (1.0, half_width * 0.12),
        ]),
        half_width * 0.80,
        5,
        4,
    )
}

/// A cone's section: a ring of **alternating radii**, so the ripe cone's own outline is scalloped.
///
/// # Invariants
///
/// **A circle reads as an egg and this does not.** Both notes name what breaks a ripe cone's outline —
/// the spruce's scales are *"denticulate or undulate"*, the fir's bracts are *"exserted"* with a
/// *"recurved cusp"* — and both of those are one alternation of radius around the ring, which is all a
/// [`shape::Section::outline`] of `2n` points with every other one drawn in is. It is the cheapest
/// honest reading available in the geometry: a bract is a separate solid and this is not, but a cone
/// sampled on a circle has no silhouette to read at all, which is what the first two renders showed.
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

/// The spruce's seed cone, drawn **hanging**.
///
/// # Invariants
///
/// **A seat expresses no rotation**, so the hanging is this template's own shape and not a property of
/// where it was put: the cone's axis runs along the shoot's own tangent, which on the outer shoots of
/// this species points down through the foliage, and every cone the scatter seats then hangs the same
/// way from whatever shoot it lands on. Ten centimetres by two and a half, the middle-upper of the
/// note's 4–12 cm × 1.7–3.5 cm. Eighteen slots and eleven stations, because the note's denticulate
/// scale margins are what give the ripe cone its shaggy outline and they are the section's own star.
fn spruce_cone() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0000, 0.0000, 0.0], [0.0040, 0.0180, 0.0]),
            ([0.0020, 0.0190, 0.0], [0.0005, 0.0330, 0.0]),
            ([0.0024, 0.0530, 0.0], [0.0000, 0.0350, 0.0]),
            ([0.0024, 0.0930, 0.0], [0.0000, 0.0120, 0.0]),
        ]),
        scalloped(18, 0.72),
    );
    sweep.scale = taper(&[
        (0.00, 0.0016),
        (0.18, 0.0030),
        (0.34, 0.0116),
        (0.60, 0.0126),
        (0.84, 0.0070),
        (1.00, 0.0016),
    ]);
    Template {
        sweep,
        stations: 11,
        around: 18,
    }
}

/// The spruce's needle fields at `shoot_depth`: the leader's candle, a spray along the tiers, the
/// sprays over a shoot's outer four fifths, and the denser brush at a shoot's tip. Four fields and one
/// template, because a needle is one blade wherever it stands and only its distribution changes.
///
/// # Invariants
///
/// **Every spruce field carries `cluster = 1` and the golden phyllotaxy.** That is the species'
/// diagnostic against the fir: *"arranged all round the shoot"*, a dense spiral on pegs, and a rank
/// would be the other species. The pitch is steep and the twist is a law along the shoot, so the
/// spiral rolls as it advances and a shoot reads as a bottlebrush rather than as a flat fan.
fn spruce_needles(shoot_depth: usize, foliage: Foliage) -> Vec<LeafField> {
    let factor = foliage.density_factor();
    let mut candle = LeafField::new(0, 0);
    candle.band = (0.88, 1.0);
    candle.density = shape::Scalar::constant(70.0 * factor);
    candle.phyllotaxy = GOLDEN;
    candle.pitch = shape::Scalar::constant(1.05);
    candle.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    candle.size = shape::Scalar::constant(1.0);
    candle.jitter = 0.20;
    // **The bough's own spray.** Ten shoots on a three-metre bough is one every thirty centimetres, and
    // a bough whose foliage is only on those is the *bare broom* the defect list names. So the tier
    // carries a needle field of its own along its whole length at the same linear density as the
    // shoots: in *Picea* the outer part of a first-order branch is itself a needled shoot, and at
    // forty-five metres it is the bough's own spray and not ten separate tufts on a bare rod.
    let mut tier = LeafField::new(1, 0);
    tier.band = (0.06, 0.99);
    tier.density = shape::Scalar::constant(200.0 * factor);
    tier.phyllotaxy = GOLDEN;
    tier.pitch = shape::Scalar::constant(1.12);
    tier.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    tier.size = shape::Scalar::new(&[(0.0, 0.80), (1.0, 1.0)]).expect("a size is a law");
    tier.jitter = 0.24;
    let mut spray = LeafField::new(shoot_depth, 0);
    // **The band stops at the base of the shoot**, which is the *dead give* defect: no real shoot is
    // foliated evenly from the wood it grows on to its own tip.
    spray.band = (0.14, 0.96);
    // **Three hundred and forty a metre is the note's own 25–60 needles over its own 5–12 cm**, read as
    // a linear density rather than a count: one needle every three millimetres, which is what the
    // overlapping spiral of `04-branch.jpg` and `07-cone-hanging.jpg` shows. The first version of this
    // module carried the *count* onto a sixteen-centimetre shoot instead of the density, and the
    // shoots came out as bare bristles with three needles on them.
    spray.density = shape::Scalar::constant(320.0 * factor);
    spray.phyllotaxy = GOLDEN;
    spray.pitch = shape::Scalar::constant(1.15);
    spray.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    spray.size = shape::Scalar::new(&[(0.0, 0.82), (1.0, 1.06)]).expect("a size is a law");
    spray.jitter = 0.26;
    let mut brush = LeafField::new(shoot_depth, 0);
    brush.band = (0.82, 1.0);
    brush.density = shape::Scalar::constant(440.0 * factor);
    brush.phyllotaxy = GOLDEN;
    brush.pitch = shape::Scalar::constant(0.98);
    brush.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    brush.size = shape::Scalar::new(&[(0.0, 0.88), (1.0, 1.04)]).expect("a size is a law");
    brush.jitter = 0.22;
    // A bough plate has no leader, so the candle's generation only exists where the shoots are two
    // generations below the root.
    if shoot_depth >= 2 {
        vec![candle, tier, spray, brush]
    } else {
        vec![tier, spray, brush]
    }
}

/// The spruce: its tree, its foliage at the asked scale, its four fields and the cone template its
/// shoots carry.
pub fn lijian_spruce(seed: u64, foliage: Foliage) -> Species {
    let mut species = Species {
        plant: grow(&lijian_spruce_rule(), seed),
        templates: vec![spruce_needle(
            foliage.half_width(SPRUCE_NEEDLE_HALF_WIDTH, SPRUCE_CARD_HALF_WIDTH),
            foliage.length(SPRUCE_NEEDLE_LENGTH, SPRUCE_CARD_LENGTH),
        )],
        fields: spruce_needles(2, foliage),
        decorators: vec![spruce_cone()],
    };
    // The bole is dressed after the growth because a section is a node's own and `grow` authors circles:
    // the plates are the trunk's, not the rule's.
    if let Ok(bole) = species.plant.get_mut(&Handle::root()) {
        bole.set_section(spruce_bark());
    }
    species
}

/// The spruce's bough plate: one bough as the trunk, its shoots as the only generation, and the note's
/// own needle. A camera inside a hundred thousand needles shades every pixel of them; the frame costs
/// minutes for what one tier shows in seconds.
pub fn lijian_spruce_bough(seed: u64) -> Species {
    Species {
        plant: grow(&lijian_spruce_bough_rule(), seed),
        templates: vec![spruce_needle(
            Foliage::Needle.half_width(SPRUCE_NEEDLE_HALF_WIDTH, SPRUCE_CARD_HALF_WIDTH),
            Foliage::Needle.length(SPRUCE_NEEDLE_LENGTH, SPRUCE_CARD_LENGTH),
        )],
        fields: spruce_needles(1, Foliage::Needle),
        decorators: vec![spruce_cone()],
    }
}

/// The cones a spruce's shoots carry at depth `shoot_depth`: **old cones persist for years** (the
/// note's 4–6 years is an ESTIMATE), so a shoot carries nought to one and the crown is studded rather
/// than stippled — the note's fourth silhouette fact, and the reason the count is a range the seed
/// decides rather than a cone on every shoot.
fn spruce_cones(species: &Species, shoot_depth: usize) -> Vec<Seat> {
    scatter(&species.plant, shoot_depth, 0, 0.94, (0, 2))
}

/// The spruce's surfaces: a dull grey-brown plated bole, a blue-glaucous grey-green needle — the note's
/// *"blue-green to glaucous grey-green, with a distinctly pale, almost silvery cast"* — and the
/// purple-red cones of `07-cone-hanging.jpg`.
///
/// **The wood is darker than the pine's and greyer than its own bark photograph**, because the pine's
/// bole at this size was reading pink under the same key: the *felt-pen tree* defect is two tints too
/// close, and here the wood and the leaf are the pair that must not agree.
fn spruce_palette() -> Palette {
    Palette {
        wood: Skin::new([0.105, 0.098, 0.088], 0.96),
        leaves: vec![Skin::new([0.150, 0.205, 0.168], 0.84)],
        seats: vec![Skin::new([0.250, 0.090, 0.078], 0.86)],
    }
}

/// A subalpine slope at three thousand metres: a deep high sky, a warm key above and behind the
/// crown, and a cold fill off the valley side.
///
/// # Invariants
///
/// **A light's reach is the distance its irradiance is stated at**, so each reach is the length of the
/// position it belongs to: the key at `[26.0, 34.0, 16.0]` is 44.9 m from the origin, and its reach is
/// 44.9 m, so the number the subject is lit at is the number the author wrote.
fn spruce_lighting() -> Lighting {
    Lighting {
        key_position: [26.0, 34.0, 16.0],
        key_colour: [1.0, 0.94, 0.82],
        key_irradiance: 2.4,
        key_reach: 44.9,
        key_extent: 9.0,
        fill_position: [-17.0, 11.0, -15.0],
        fill_colour: [0.40, 0.50, 0.72],
        fill_irradiance: 0.38,
        fill_reach: 25.2,
        floor: Skin::new([0.115, 0.108, 0.082], 0.96),
        sky: Atmosphere::new()
            .zenith([0.05, 0.12, 0.36])
            .horizon([0.42, 0.53, 0.66])
            .ground([0.26, 0.25, 0.21])
            .density(0.0005)
            .scale_height(3000.0),
        sky_radiance: 0.48,
        // A tree framed at forty-five metres is in thinner air than a pine framed at twenty-five, and
        // the air at three thousand metres is thinner than both.
        haze: 0.004,
    }
}

/// The whole-tree frame: a portrait three fifths as wide again as it is tall, standing forty-five
/// metres off so a 1.2 mm needle is one pixel and the spire fills the frame's height without cropping.
const SPRUCE_FRAME: Framing = Framing {
    width: 1000,
    height: 1700,
    eye: [26.0, 6.0, 35.0],
    target: [0.0, 16.0, 0.0],
    field_of_view: 0.80,
    near: 0.5,
    far: 700.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The bough plate's frame: **forty-two centimetres** off a tier on a narrower lens than the tree's,
/// so a spruce needle is four pixels wide and thirty long — the range the bar's H2 asks for — and three
/// to four shoots fill the frame, which is the range `04-branch.jpg` was taken at. The eye sits level
/// with the tier and looks a little up, which puts the dark forest floor low in the frame and the
/// needles against the sky.
const SPRUCE_BOUGH_FRAME: Framing = Framing {
    width: 1200,
    height: 1000,
    eye: [1.17, 0.30, 0.40],
    target: [1.03, 0.35, 0.02],
    field_of_view: 0.75,
    near: 0.02,
    far: 40.0,
    floor: 6.0,
    ground: Ground::Plane,
};

/// Builds the Lijiang spruce's whole-tree plate: one tree in the card foliage, its own light and ground.
pub fn lijian_spruce_picture() -> Value {
    let species = lijian_spruce(SEED, Foliage::Card);
    let planting = Planting::new(species.clone())
        .with_seats(spruce_cones(&species, 2))
        .with_palette(spruce_palette());
    plant_scene_lit(
        &[planting],
        &SPRUCE_FRAME,
        LIJIAN_SPRUCE,
        &spruce_lighting(),
        &[],
    )
}

/// Builds the spruce's branch plate: one tier at arm's length in the note's own needle.
pub fn lijian_spruce_bough_picture() -> Value {
    let species = lijian_spruce_bough(SEED);
    let planting = Planting::new(species.clone())
        .with_seats(spruce_cones(&species, 1))
        .with_palette(spruce_palette());
    plant_scene_lit(
        &[planting],
        &SPRUCE_BOUGH_FRAME,
        LIJIAN_SPRUCE_BOUGH,
        &spruce_lighting(),
        &[],
    )
}

/// Resolves a spruce frame's `authored_id` into the handle that names the node.
pub fn lijian_spruce_handle_of(id: u32) -> Option<String> {
    node_ids(&lijian_spruce(SEED, Foliage::Needle).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a spruce bough plate's `authored_id` into the handle that names the node: the bough is its
/// own plant, so its walk is its own table and the whole tree's would answer with the wrong node.
pub fn lijian_spruce_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&lijian_spruce_bough(SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

// ---------------------------------------------------------------------------------------------
// 苍山冷杉 Abies delavayi
// ---------------------------------------------------------------------------------------------

/// The fir's rule: the same whorled tiers as the spruce's, **shorter and more sharply tiered**, shoots
/// on the tiers whose tips **turn up**, and a flat needle in two ranks on every shoot.
///
/// # The numbers and where they come from
///
/// Every figure is [`abies_delavayi.md`](../../../../.agents/investigate/trees/abies_delavayi.md)'s
/// modelling table, and the two that are **ESTIMATE** in that table are marked as such here. The tree
/// is drawn at the note's **exact** twenty-five metres, because a fir is three quarters the spruce's
/// height and the difference between the two silhouettes is at least part of what tells them apart.
fn cangshan_fir_rule() -> Rule {
    Rule {
        trunk: fir_bole,
        generations: vec![
            // **The tiers.** Six to eight boughs at one station, spaced 0.5–1.0 m up the leader (the
            // note's ESTIMATE), which over the crown's 17 m is twenty-six of them at 0.65 m: the fir's
            // signature is the *gap of bare trunk between the tiers*, and this band and count are
            // where that gap comes from.
            Generation {
                count: (156, 156),
                station: (FIR_CROWN_BASE, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.04, 0.05),
                sink: 0.03,
                cluster: 6,
                shape: fir_tier,
            },
            // **The shoots on a tier**, eleven of them over its outer four fifths.
            Generation {
                count: (11, 11),
                station: (0.18, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.18, 0.12),
                sink: 0.006,
                cluster: 1,
                shape: fir_shoot,
            },
        ],
        // A fir's shoots stand up under their own load and a tier's tips turn to the light, so this is
        // the one field either species carries.
        fields: vec![Field::light([0.0, 1.0, 0.0], 0.12)],
    }
}

/// The fir's bole: twenty-five metres, plumb, its radius the note's exact 1 m d.b.h. halved.
fn fir_bole() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.00, 0.0, 0.00], [0.01, 1.00, 0.00]),
            ([0.03, 6.0, -0.02], [0.01, 1.00, 0.00]),
            ([0.05, 13.0, 0.01], [0.01, 1.00, 0.01]),
            ([0.07, 19.5, 0.05], [0.00, 1.00, 0.00]),
            ([0.08, 25.0, 0.08], [0.00, 1.00, 0.00]),
        ]),
        taper(&[
            (0.000, 0.66),
            (0.015, 0.50),
            (0.060, 0.43),
            (0.220, 0.33),
            (0.500, 0.23),
            (0.780, 0.13),
            (1.000, 0.026),
        ]),
    )
}

/// The bole's section: twenty-four sides with eight shallow longitudinal furrows, which is what
/// *"gray-brown, rough, longitudinally splitting"* is in geometry. **Furrows, not plates**: the fir's
/// bark is ridged along the grain where the spruce's breaks into irregular plates, and the ridges are
/// narrower and more of them.
fn fir_bark() -> shape::Section {
    let sides = 24;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let ridge = 1.0 - 0.075 * (theta * 8.0).sin().powi(2) + 0.022 * (theta * 13.0).sin();
            [theta.cos() * ridge, theta.sin() * ridge]
        })
        .collect();
    shape::Section::outline(&points).expect("a furrowed ring is a section")
}

/// A first-order tier: it leaves the leader at the note's 70–90° as a flat tier, and **turns up through
/// its last two knots** — the opposite of the spruce's droop, and the first of the four rows the two
/// silhouettes are told apart by. Its length is a law over the station it hangs at.
fn fir_tier(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let profile = crown_profile(joint.station, FIR_CROWN_BASE);
    // A narrower crown that falls away faster than it narrows: three metres at the base of the crown
    // for the note's 6 m crown width, and thirty centimetres at the leader's tip.
    let length = (0.28 + 2.70 * (1.0 - profile).powf(1.85)) * (0.78 + 0.44 * rng.unit());
    let departure = 1.36 + 0.16 * profile + 0.09 * rng.signed();
    // **The upsweep**, in the same place the spruce's droop is: the outer two knots turn a quarter turn
    // above the horizontal, so a tier's tips stand in the air rather than hanging in it.
    let sweep = 0.26 + 0.78 * (1.0 - profile) * (0.85 + 0.30 * rng.unit());
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.14,
                departure + 0.10,
                departure + sweep,
            ],
        ),
        taper(&[
            (0.00, radius * 0.19),
            (0.20, radius * 0.14),
            (0.65, radius * 0.08),
            (1.00, 0.006),
        ]),
    )
}

/// A fir shoot: **twenty-two centimetres**, the second-order branchlet that carries the note's own
/// needles, leaving its tier at about a right angle and **turning up through its last third**, so a
/// tier's ends are a comb of upstanding sprays rather than a drooping fringe.
///
/// The length is the same extension of the note's ESTIMATE as the spruce's shoot carries, and for the
/// same reason: the note's 5–12 cm is read from two close photographs of the finest tips.
fn fir_shoot(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.22 * (0.80 + 0.42 * rng.unit());
    let departure = 1.02 + 0.34 * rng.signed();
    let sweep = 0.36 + 0.38 * rng.unit();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.34,
                departure + 0.36,
                departure + sweep,
            ],
        ),
        taper(&[(0.00, radius * 0.46), (0.35, radius * 0.26), (1.00, 0.0018)]),
    )
}

/// The fir's bough plate: a short upright leader as the trunk, and its sprays as the only generation.
/// It is the species' own second photograph — `07-cone-erect-crown.jpg` — in geometry: an upright
/// shoot with lateral sprays and cones standing above them.
fn cangshan_fir_bough_rule() -> Rule {
    Rule {
        trunk: || {
            (
                bend(1.30, [0.14, 0.09, 0.05, 0.02]),
                taper(&[(0.0, 0.018), (0.5, 0.011), (1.0, 0.005)]),
            )
        },
        generations: vec![Generation {
            count: (18, 18),
            station: (0.22, 0.98),
            slot: (0.0, GOLDEN),
            jitter: (0.18, 0.14),
            sink: 0.004,
            cluster: 1,
            // The bough plate's spray is the tier spray with the upsweep doubled, because the node it
            // hangs from is a **vertical** leader here rather than a horizontal tier, and a cone has to
            // stand above the foliage rather than lean out of it.
            shape: fir_bough_spray,
        }],
        fields: Vec::new(),
    }
}

/// The fir's bough plate's spray: the same leaves as a tier's, leaving an upright leader at about the
/// angle a tier leaves the leader at, and turning hard up through its last two knots.
fn fir_bough_spray(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.26 * (0.82 + 0.36 * rng.unit());
    let departure = 1.22 + 0.22 * rng.signed();
    let sweep = 0.92 + 0.46 * rng.unit();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.44,
                departure + 0.34,
                departure + sweep,
            ],
        ),
        taper(&[(0.00, radius * 0.62), (0.30, radius * 0.34), (1.00, 0.0026)]),
    )
}

/// A blade of `length` and `half_width`: the fir needle drawn at the note's own 1.5–3 cm × 1.7–2.5 mm
/// when the foliages are [`Foliage::Needle`], and the plate's card when they are [`Foliage::Card`].
///
/// **Every one of the midrib's six numbers is scaled by `length`, the positions as well as the
/// tangents** — see [`spruce_needle`] for why that is stated rather than assumed.
///
/// It is three tenths as thick as it is wide because FOC's section is *"flattened"* and its margin
/// *"strongly revolute"* — the exact complement of the spruce's *"broadly rhombic"* — and it curves
/// into the note's *"S"* on its way out, and narrows to the emarginate apex.
fn fir_needle(half_width: f32, length: f32) -> LeafTemplate {
    let reach = length / FIR_NEEDLE_LENGTH;
    LeafTemplate::blade(
        spine(&[
            ([0.0000, 0.00000, 0.0], [0.0080 * reach, 0.00190 * reach, 0.0]),
            (
                [0.0110 * reach, 0.00235 * reach, 0.0],
                [0.0076 * reach, 0.00090 * reach, 0.0],
            ),
            (
                [0.0208 * reach, 0.00300 * reach, 0.0],
                [0.0072 * reach, -0.00170 * reach, 0.0],
            ),
        ]),
        taper(&[
            (0.0, half_width * 0.55),
            (0.16, half_width * 0.96),
            (0.52, half_width),
            (0.86, half_width * 0.70),
            (1.0, half_width * 0.28),
        ]),
        half_width * 0.30,
        6,
        4,
    )
}

/// The fir's seed cone, drawn **standing**.
///
/// # Invariants
///
/// **A seat expresses no rotation**, so the standing is this template's own shape and not a property of
/// where it was put: the cone's axis runs along the node's tangent, and the only node this species
/// seats a cone on is one whose tangent at that station points **up** — the tip of an upswept tier, or
/// of a spray on the bough plate that turns to within twenty degrees of vertical. Seven and a half
/// centimetres by three, the middle of the note's **6–11 cm × 3–4 cm**, with ten stations and sixteen
/// slots — **a deep star**, because the note's exserted bracts are what give the ripe cone its
/// bristled outline and are the fir's own fourth diagnostic.
fn fir_cone() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0000, 0.0000, 0.0], [0.0028, 0.0170, 0.0]),
            ([0.0006, 0.0185, 0.0], [0.0002, 0.0270, 0.0]),
            ([0.0007, 0.0490, 0.0], [0.0000, 0.0270, 0.0]),
            ([0.0005, 0.0740, 0.0], [0.0000, 0.0050, 0.0]),
        ]),
        scalloped(16, 0.58),
    );
    sweep.scale = taper(&[
        (0.00, 0.0022),
        (0.18, 0.0056),
        (0.42, 0.0146),
        (0.72, 0.0144),
        (0.88, 0.0094),
        (1.00, 0.0022),
    ]);
    Template {
        sweep,
        stations: 10,
        around: 16,
    }
}

/// The fir's needle fields at `shoot_depth`: the leader's candle, a spray along the tiers, the two
/// ranks over a shoot's outer four fifths, and the pale brush at a shoot's tip.
///
/// # Invariants
///
/// **The shoot's own two fields carry `phyllotaxy = OPPOSITE` and the two that sit on the tier and the
/// leader do not.** Half a turn between two needles is the fir's whole diagnostic in one number: FOC's
/// *"pectinately arranged in 2 lateral sets"*, the flat comb of `04-branch.jpg`, against the spruce's
/// dense golden-angle spiral. The leader and the tier's inner wood stay on the golden angle, because
/// they radiate rather than comb — and that difference is why the crown is not a barcode.
fn fir_needles(shoot_depth: usize, foliage: Foliage) -> Vec<LeafField> {
    let factor = foliage.density_factor();
    let mut candle = LeafField::new(0, 0);
    candle.band = (0.90, 1.0);
    candle.density = shape::Scalar::constant(90.0 * factor);
    candle.phyllotaxy = GOLDEN;
    candle.pitch = shape::Scalar::constant(1.10);
    candle.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    candle.size = shape::Scalar::constant(1.0);
    candle.jitter = 0.22;
    // **The tier's own spray**, at the same linear reading of the note as the shoot's: a fir's tier is
    // clothed along its whole length and a tier with foliage only on its eleven shoots is a bare broom.
    let mut tier = LeafField::new(1, 0);
    tier.band = (0.06, 0.99);
    tier.density = shape::Scalar::constant(180.0 * factor);
    tier.phyllotaxy = GOLDEN;
    tier.pitch = shape::Scalar::constant(1.15);
    tier.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    tier.size = shape::Scalar::new(&[(0.0, 0.80), (1.0, 1.0)]).expect("a size is a law");
    tier.jitter = 0.24;
    let mut spray = LeafField::new(shoot_depth, 0);
    // **The band stops at the base of the shoot**, which is the *dead give* defect.
    spray.band = (0.12, 0.97);
    // The same linear reading of the note's ESTIMATE as the spruce's, on a longer shoot: a fir's
    // needles are twice as long and twice as broad, and a fir shoot is a comb of them rather than a
    // bottlebrush, so the count per shoot comes out a little higher at the same spacing.
    spray.density = shape::Scalar::constant(260.0 * factor);
    // **Two lateral ranks**: half a turn from one needle to the next, so the shoot is a flat comb.
    spray.phyllotaxy = OPPOSITE;
    // Pitch and twist are the two numbers that turn a radial needle into a flat one: the pitch stands
    // the blade forward along the shoot and the twist rolls it so its **face is the sky**, which is
    // what *"strongly revolute"* and *"two white bands abaxially"* look like from below.
    spray.pitch = shape::Scalar::constant(1.20);
    spray.twist = shape::Scalar::new(&[(0.0, 1.32), (1.0, 1.46)]).expect("a roll is a law");
    spray.size = shape::Scalar::new(&[(0.0, 0.84), (1.0, 1.08)]).expect("a size is a law");
    spray.jitter = 0.20;
    // **The fir's own second arrangement, from the same sentence of the note**: *"Leaves spirally
    // arranged, radially spreading ± forward **or** pectinately arranged in 2 lateral sets"*. The comb
    // above is the pectinate half and this is the radial half, at half the density, and without it the
    // two ranks put every needle on one side of a shoot and the spray reads as a fish bone.
    let mut radial = LeafField::new(shoot_depth, 0);
    radial.band = (0.16, 0.94);
    radial.density = shape::Scalar::constant(140.0 * factor);    radial.phyllotaxy = GOLDEN;
    radial.pitch = shape::Scalar::constant(0.78);
    radial.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    radial.size = shape::Scalar::new(&[(0.0, 0.78), (1.0, 0.96)]).expect("a size is a law");
    radial.jitter = 0.28;
    let mut brush = LeafField::new(shoot_depth, 0);
    brush.band = (0.70, 1.0);
    brush.density = shape::Scalar::constant(400.0 * factor);
    brush.phyllotaxy = OPPOSITE;
    brush.pitch = shape::Scalar::constant(1.02);
    brush.twist = shape::Scalar::new(&[(0.0, 1.26), (1.0, 1.52)]).expect("a roll is a law");
    // **The pale new growth at the shoot tips** is the note's *"paler young growth at the shoot tips"*,
    // and it is a size law and not a second template because the blade is the same blade.
    brush.size = shape::Scalar::new(&[(0.0, 0.86), (1.0, 1.04)]).expect("a size is a law");    brush.jitter = 0.18;
    if shoot_depth >= 2 {
        vec![candle, tier, spray, radial, brush]
    } else {
        vec![tier, spray, radial, brush]
    }
}

/// The fir: its tree, its foliage at the asked scale, its five fields and the cone template its tiers
/// carry.
pub fn cangshan_fir(seed: u64, foliage: Foliage) -> Species {
    let mut species = Species {
        plant: grow(&cangshan_fir_rule(), seed),
        templates: vec![fir_needle(
            foliage.half_width(FIR_NEEDLE_HALF_WIDTH, FIR_CARD_HALF_WIDTH),
            foliage.length(FIR_NEEDLE_LENGTH, FIR_CARD_LENGTH),
        )],
        fields: fir_needles(2, foliage),
        decorators: vec![fir_cone()],
    };
    if let Ok(bole) = species.plant.get_mut(&Handle::root()) {
        bole.set_section(fir_bark());
    }
    species
}

/// The fir's bough plate: one upright shoot as the trunk, its sprays as the only generation, and the
/// note's own flat needle.
pub fn cangshan_fir_bough(seed: u64) -> Species {
    Species {
        plant: grow(&cangshan_fir_bough_rule(), seed),
        templates: vec![fir_needle(
            Foliage::Needle.half_width(FIR_NEEDLE_HALF_WIDTH, FIR_CARD_HALF_WIDTH),
            Foliage::Needle.length(FIR_NEEDLE_LENGTH, FIR_CARD_LENGTH),
        )],
        fields: fir_needles(1, Foliage::Needle),
        decorators: vec![fir_cone()],
    }
}

/// The cones a fir's shoots carry at depth `shoot_depth`: one or two to a tier, which is what
/// `07-cone-erect-crown.jpg` shows — conspicuous, but not a cone on every spray, which would be a
/// hundred thousand instances the rasteriser cannot shade.
fn fir_cones(species: &Species, shoot_depth: usize) -> Vec<Seat> {
    scatter(&species.plant, shoot_depth, 0, 0.96, (1, 2))
}

/// The fir's surfaces: a rough grey-brown furrowed bole, a **darker and less glaucous** leaf than the
/// spruce's — the note's *"bright dark green"* against the spruce's *"blue-green to glaucous grey-green"*,
/// which is the other half of the two species' colour difference — and a cone of its own, **black and
/// glaucous**, which is the only tint in this module no other part of either species carries.
fn fir_palette() -> Palette {
    Palette {
        wood: Skin::new([0.098, 0.090, 0.080], 0.97),
        leaves: vec![Skin::new([0.052, 0.100, 0.048], 0.72)],
        // **Low roughness is the bloom**: FOC's *"black at maturity, glaucous"* and the blue-black cast
        // of `06-cone.jpg` are one number — a dark, smooth, cold surface under a warm key.
        seats: vec![Skin::new([0.030, 0.038, 0.070], 0.46)],
    }
}

/// The same subalpine day as the spruce's, an hour colder: the fir is the higher of the two belts and
/// its needles are the darker, so the key is a little cooler and the fill a little weaker.
///
/// # Invariants
///
/// **A light's reach is the distance its irradiance is stated at**, so each reach is the length of the
/// position it belongs to: the key at `[19.0, 27.0, 12.0]` is 34.9 m from the origin, and its reach is
/// 34.9 m.
fn fir_lighting() -> Lighting {
    Lighting {
        key_position: [19.0, 27.0, 12.0],
        key_colour: [0.98, 0.94, 0.86],
        key_irradiance: 2.3,
        key_reach: 34.9,
        key_extent: 8.0,
        fill_position: [-13.0, 9.0, -12.0],
        fill_colour: [0.38, 0.48, 0.72],
        fill_irradiance: 0.34,
        fill_reach: 19.8,
        floor: Skin::new([0.090, 0.105, 0.068], 0.96),
        sky: Atmosphere::new()
            .zenith([0.05, 0.12, 0.36])
            .horizon([0.40, 0.51, 0.65])
            .ground([0.23, 0.24, 0.20])
            .density(0.0005)
            .scale_height(3400.0),
        sky_radiance: 0.46,
        haze: 0.004,
    }
}

/// The whole-tree frame: the same lens and the same standoff fraction as the spruce's, closer because
/// the tree is twenty-five metres rather than thirty-four, and a stop wider because a squat spire wants
/// more sky beside it than a tall one.
const FIR_FRAME: Framing = Framing {
    width: 1100,
    height: 1700,
    eye: [17.0, 4.5, 23.0],
    target: [0.0, 12.5, 0.0],
    field_of_view: 0.82,
    near: 0.3,
    far: 500.0,
    floor: 300.0,
    ground: Ground::Plane,
};

/// The bough plate's frame: the same range on the same lens as the spruce's, **seventy centimetres**
/// off an upright leader and aimed at a spray rather than at the shoot, so two sprays and their cones
/// fill the frame and no cone is close enough to the lens to be a third of it. The camera looks a few
/// degrees up, so the horizon sits low and the sprays and their erect cones are against the sky.
const FIR_BOUGH_FRAME: Framing = Framing {
    width: 1200,
    height: 1000,
    eye: [0.64, 1.02, 0.28],
    target: [0.22, 1.05, 0.02],
    field_of_view: 0.75,
    near: 0.02,
    far: 40.0,
    floor: 6.0,
    ground: Ground::Plane,
};

/// Builds the Cangshan fir's whole-tree plate: one tree in the card foliage, its own light and ground.
pub fn cangshan_fir_picture() -> Value {
    let species = cangshan_fir(SEED, Foliage::Card);
    let planting = Planting::new(species.clone())
        .with_seats(fir_cones(&species, 1))
        .with_palette(fir_palette());
    plant_scene_lit(
        &[planting],
        &FIR_FRAME,
        CANGSHAN_FIR,
        &fir_lighting(),
        &[],
    )
}

/// Builds the fir's branch plate: one upright shoot at arm's length in the note's own flat needle.
pub fn cangshan_fir_bough_picture() -> Value {
    let species = cangshan_fir_bough(SEED);
    let planting = Planting::new(species.clone())
        .with_seats(fir_cones(&species, 1))
        .with_palette(fir_palette());
    plant_scene_lit(
        &[planting],
        &FIR_BOUGH_FRAME,
        CANGSHAN_FIR_BOUGH,
        &fir_lighting(),
        &[],
    )
}

/// Resolves a fir frame's `authored_id` into the handle that names the node.
pub fn cangshan_fir_handle_of(id: u32) -> Option<String> {
    node_ids(&cangshan_fir(SEED, Foliage::Needle).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a fir bough plate's `authored_id` into the handle that names the node: the shoot is its
/// own plant, so its walk is its own table.
pub fn cangshan_fir_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&cangshan_fir_bough(SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

// ---------------------------------------------------------------------------------------------
// The drawing tools both species share
// ---------------------------------------------------------------------------------------------

/// Where a bough sits in its own crown, nought at the base of the crown and one at the leader's tip.
///
/// # Invariants
///
/// **This is the crown profile every law in the module is over**, and it is `station`-based rather
/// than rank-based on purpose: a conifer's lower bough is many times the length of its upper one, and
/// the band a generation spreads over begins at a number that is not zero (the spruce's crown starts at
/// 0.11 of the tree, the fir's at 0.32), so the profile is the child's own station measured **inside
/// that band** rather than the raw one. A profile read raw would make both species' lowest boughs the
/// same length as the middle of the trunk.
fn crown_profile(station: f32, base: f32) -> f32 {
    ((station - base) / (1.0 - base)).clamp(0.0, 1.0)
}

/// Four knots along a heading that turns through `angles`, measured from the parent's own tangent:
/// nought is along the parent and a quarter turn is straight out. The tangents are the heading's own
/// direction, so a bough is a chord with three bends in it rather than a straight line with a
/// direction, and the **last two knots are where a droop or an upsweep is drawn**.
fn bend(length: f32, angles: [f32; 4]) -> shape::Curve {
    let step = length / 3.0;
    let at = |angle: f32| [step * angle.sin(), step * angle.cos(), 0.0];
    let tangent = |angle: f32| [length * 0.5 * angle.sin(), length * 0.5 * angle.cos(), 0.0];
    let first = at(angles[0]);
    let second = [first[0] + at(angles[1])[0], first[1] + at(angles[1])[1], 0.0];
    let third = [second[0] + at(angles[2])[0], second[1] + at(angles[2])[1], 0.0];
    spine(&[
        ([0.0, 0.0, 0.0], tangent(angles[0])),
        (first, tangent(angles[1])),
        (second, tangent(angles[2])),
        (third, tangent(angles[3])),
    ])
}
