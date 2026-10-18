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
//! whole-tree plate draws [`Foliage::Plume`] and the bough plate draws [`Foliage::Needle`], the note's
//! own width. That is a level of detail and not a fudge: each plate is honest about the blade it draws.
//!
//! ## A plume stands for a shoot's whole plume of needles, and not for one needle
//!
//! **The plume is one blade whose width law is a comb of needle-teeth**, the pine's spray LOD and for
//! the same reason. The scene document pays per instance, and the first four renders of this module
//! tried to close a thirty-metre crown with the needles alone and came out as a pole with bottlebrushes
//! on it: forty thousand true needles of one and a half pixels, over two hundred thousand of them
//! needed, and the node and instance budgets do not carry two hundred thousand. A plume is what a
//! conifer crown actually resolves to at forty-five metres — the eye reads plumes, not needles — and
//! its comb's teeth are **one and a half to two pixels each**, which is the grain `02-crown.jpg` shows
//! at that distance. So the plume's density is **a ninth** of the needle's, its card is 11.7 × 5.2
//! pixels for the spruce and 19.7 × 7.2 for the fir, and the two plates differ by that and by nothing
//! else: the same rule, the same fields, the same templates, the same cones.
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
//! # What the second art pass changed, and what it is still asking for
//!
//! Four numbers of the platform moved under this module between the two passes, and each one is spent here:
//!
//! | what landed | what this module does with it |
//! | --- | --- |
//! | a plant's surfaces carry a shadow-bias scale of [`vegetation_picture::PLANT_SHADOW_BIAS_SCALE`], so a leaf shadows a leaf | **the crown models itself.** Both whole-tree plates were measured before and after and the number is in the commit message; the interior-over-lit-p90 bar is met by self-shadowing and not by a dark key |
//! | wood draws creased normals at [`vegetation_mesh::WOOD_CREASE`] | **the base plates.** A base plate expands at [`BARK_DETAIL`], whose `around` is the sections' own side count, so the twenty-side plated bole and the twenty-four-side furrowed one are the polygons they were authored as and creases on the corners that turn more than thirty-five degrees |
//! | [`Planting::with_detail`] takes a [`Detail`] | the same dial, for that one reason and no other |
//! | [`LeafTemplate::sprig`] holds several blades in one row | **the bough plates.** [`Foliage::Sprig`] is a shoot's worth of true needles in one card, and it is why the bough plate reads as a bottlebrush rather than as a handful of hairs |
//!
//! **What the crate still cannot say.** Two of them are this module's, not the platform's. First, **a
//! needle is one template and a *plume* is a different template**, so there is no LOD in between: a plate
//! either draws the note's own 1.2 mm blade or a card that stands for twenty of them, and this module
//! chose a sprig of five rather than one. Second, **a seat carries no rotation**, so every cone on a
//! node points along that node's own surface frame and the species' "erect" and "pendant" are drawn in the
//! template's spine rather than in where it is put. A cone could not be seated at an angle to its own
//! shoot, and a fir's cone therefore cannot lean out of the crown line the way a real one does.
//!
//! # What is duplicated from [`southwest`](southwest.rs) and why
//!
//! The [`bend`] drawing tool and the [`Foliage`] enum are the pine's, restated. This slice may not edit
//! the pine's module, so the two copies stand until a later slice lifts them into one shared file.

use definition::Value;
use vegetation::decorator::{Seat, Template, scatter};
use vegetation::derive::Species;
use vegetation::expand::Detail;
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

/// The name the Lijiang spruce's whole-tree plate is built and rendered under.
pub const LIJIAN_SPRUCE: &str = "lijian_spruce";

/// The name the Lijiang spruce's branch plate is built and rendered under.
pub const LIJIAN_SPRUCE_BOUGH: &str = "lijian_spruce_bough";

/// The name the Cangshan fir's whole-tree plate is built and rendered under.
pub const CANGSHAN_FIR: &str = "cangshan_fir";

/// The name the Cangshan fir's branch plate is built and rendered under.
pub const CANGSHAN_FIR_BOUGH: &str = "cangshan_fir_bough";

/// The name the Lijiang spruce's base plate is built and rendered under.
pub const LIJIAN_SPRUCE_BASE: &str = "lijian_spruce_base";

/// The name the Cangshan fir's base plate is built and rendered under.
pub const CANGSHAN_FIR_BASE: &str = "cangshan_fir_base";

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

/// The spruce's plume card: **260 mm long and 116 mm across**, standing for the whole needle-plume of
/// one shoot, with a comb of seven teeth.
///
/// The whole-tree plate stands 44.7 m off the tree with a 0.80 rad vertical field over 1700 rows, so
/// one pixel is `2 · 44.7 · tan 0.40 / 1700` = **22.2 mm**. This card is 11.7 pixels long by 5.2 wide
/// and its comb runs to seven teeth, which is 1.7 pixels a tooth — the grain `02-crown.jpg` shows at
/// that distance, and the reason a needle drawn here instead is a fifth of a pixel and is dropped.
const SPRUCE_PLUME_LENGTH: f32 = 0.260;
const SPRUCE_PLUME_HALF_WIDTH: f32 = 0.058;
const SPRUCE_PLUME_TEETH: usize = 7;

/// The fir's plume card: **300 mm long and 110 mm across**, twelve teeth.
///
/// The fir's plate stands 29.7 m off the tree with a 0.82 rad vertical field over 1700 rows, so one
/// pixel is `2 · 29.7 · tan 0.41 / 1700` = **15.2 mm**. This card is 19.7 pixels long by 7.2 wide and
/// its comb runs to twelve teeth, again 1.6 pixels a tooth. **The fir's card is longer in pixels than
/// the spruce's and the same width in metres**, because the fir's needles are twice as long: the card
/// stands for a plume of *its* needles and not for the same area twice.
const FIR_PLUME_LENGTH: f32 = 0.300;
const FIR_PLUME_HALF_WIDTH: f32 = 0.055;
const FIR_PLUME_TEETH: usize = 12;

/// Which foliage a spire conifer draws.
///
/// # Invariants
///
/// **The foliages are the three plates and not three moods, and a plume is a whole plume's needles in
/// one instance.** A needle is the needle's own width or it is not a needle; the plume exists because
/// the scene document pays per instance and a needle at forty-five metres is a fifth of a pixel, which
/// a point-sampled rasteriser drops whole. This is the pine's [`southwest::Foliage::Spray`](southwest.rs)
/// pattern, and it is the reason the spruce's whole-tree plate can be a crown rather than a pole. The
/// **sprig** is between the two: it is the note's own needle, and *five* of them standing where they
/// grow on one peg, which is [`05-leaf.jpg`](../../../../.agents/investigate/trees/picea_likiangensis.md)'s
/// arrangement — one instance, one mesh, five blades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foliage {
    /// The note's own blade, for a frame within arm's length of a shoot.
    Needle,
    /// **A shoot's worth of the note's own needles in one card**: the bough plate's foliage, and the
    /// reason it reads as a bottlebrush rather than as scattered hairs.
    Sprig,
    /// One blade whose width law is a comb of needle-teeth, standing for a plume.
    Plume,
}

impl Foliage {
    /// The fraction of the needle foliage's density this foliage is drawn at.
    ///
    /// **One for the needle, rather more for the sprig, a ninth for the plume**, and all three draw
    /// **more blades than the needle's own row count and never more rows than it**: a sprig is seven
    /// blades in one row and the field's density is a count of *rows* and not of needles, so the sprig
    /// raises the row count by its own factor and multiplies the needles by seven. **The sprig's factor
    /// is 0.70 and the plume's is 0.09** — and the gap between them is the gap between the note's own
    /// 25–60 needles per five-centimetre twig and the ninth of a plume, and it is why the bough plates
    /// can be a bottlebrush and the whole-tree plates a crown out of the same two numbers. The plume's
    /// ninth is the ratio the first pass measured and not a claim.
    fn density_factor(self) -> f32 {
        match self {
            Foliage::Needle => 1.0,
            Foliage::Sprig => 0.70,
            Foliage::Plume => 0.09,
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
            // middle), spread as `count / cluster` tiers up the crown band: **600 children at six to a
            // whorl is a hundred tiers over thirty metres of leader, which is 0.30 m between whorls and
            // is inside the note's own 0.25–0.50 m.** The first version of this module used forty-three
            // tiers, which put three quarters of a metre of bare leader between two tiers' foliage and
            // left the spire with sky bands cut through it — the note's second silhouette fact is a
            // *column* of foliage, and the node budget is what stopped it, not the count.
            Generation {
                count: (600, 600),
                station: (SPRUCE_CROWN_BASE, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.05, 0.05),
                sink: 0.03,
                cluster: 6,
                shape: spruce_bough,
            },
            // **The shoots.** Five to a tier over its outer four fifths: with a hundred tiers the tier's
            // own spray field clothes the bough along its whole length, and five shoots on it are the
            // second order the crown's depth is made of. Six hundred boughs and three thousand shoots
            // is three thousand six hundred and one nodes, inside the four thousand a planting has.
            Generation {
                count: (5, 5),
                station: (0.14, 1.0),
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

/// The spruce needle, at the note's own 0.6–1.5 cm × 1–1.5 mm, thick in proportion to its width —
/// four fifths as thick as it is wide — because FOC's section is *"broadly rhombic"*, which is the
/// *Picea* diagnostic against *Abies*' flattened one.
fn spruce_needle() -> LeafTemplate {
    LeafTemplate::blade(
        spruce_midrib(SPRUCE_NEEDLE_LENGTH),
        taper(&[
            (0.0, SPRUCE_NEEDLE_HALF_WIDTH * 0.75),
            (0.30, SPRUCE_NEEDLE_HALF_WIDTH),
            (0.72, SPRUCE_NEEDLE_HALF_WIDTH * 0.70),
            (1.0, SPRUCE_NEEDLE_HALF_WIDTH * 0.12),
        ]),
        SPRUCE_NEEDLE_HALF_WIDTH * 0.80,
        5,
        4,
    )
}

/// The spruce's midrib at `length`: the note's stiff, nearly straight, four-angled needle.
///
/// **Every one of the six numbers is scaled by `length`, the positions as well as the tangents.** A
/// curve is pinned by its knots and steered only by its tangents, so a midrib whose tangents were
/// scaled and whose positions were not came out 8 mm long whatever it was asked for.
fn spruce_midrib(length: f32) -> shape::Curve {
    let reach = length / SPRUCE_NEEDLE_LENGTH;
    spine(&[
        (
            [0.0000, 0.00000, 0.0],
            [0.0042 * reach, 0.00070 * reach, 0.0],
        ),
        (
            [0.0044 * reach, 0.00105 * reach, 0.0],
            [0.0036 * reach, 0.00080 * reach, 0.0],
        ),
        (
            [0.0081 * reach, 0.00165 * reach, 0.0],
            [0.0022 * reach, 0.00060 * reach, 0.0],
        ),
    ])
}

/// The spruce's plume: **one blade whose width law is a comb**, standing for a shoot's whole plume of
/// needles. The plume swells to its middle and runs out at both ends, so it is a plume and not a
/// rectangle, and the comb multiplies that envelope — which is what turns a blob into the needle-grain
/// of `02-crown.jpg`.
///
/// The pine's spray is the same construction with a different species' numbers; see
/// [`southwest`](southwest.rs).
fn spruce_plume() -> LeafTemplate {
    comb(
        SPRUCE_PLUME_LENGTH,
        SPRUCE_PLUME_HALF_WIDTH,
        SPRUCE_PLUME_TEETH,
        0.0016,
        16,
    )
}

/// The sprig's own numbers: **seven needles over twenty-four millimetres**, which is the note's own one
/// needle every four millimetres read seven times over, and the pitch they stand at.
///
/// **Seven and not five** is the bough plate's second-pass finding, and it is a density finding rather
/// than a shape one: at five the spray carried thirty-eight needles on its sixteen-centimetre shoot,
/// which is inside the note's own 25–60 per *twig* but on a shoot three times the note's length, and the
/// plate still read as bristles. Seven over two and four centimetres is sixty needles on that shoot,
/// the note's count on a shoot twice the note's length, and the cards overlap by three instead of two.
const SPRIG_NEEDLES: usize = 7;
const SPRIG_SPAN: f32 = 0.024;

/// The spruce's sprig: **five of the note's own needles on one twenty-millimetre run of shoot.**
///
/// # Invariants
///
/// **The card is one instance and it is a sprig rather than a plume, so every needle on it is the
/// note's own 0.6–1.5 cm by 1–1.5 mm.** A plume card would stand for twenty needles and would read as
/// grain, which is what the forty-five-metre plate wants and exactly what a plate at arm's length must
/// not have. [`LeafTemplate::sprig`] is what makes the difference free: five blades, one mesh, one row
/// each — the same five hundred rows a field placing needles one at a time would have cost, and a
/// brush five times as dense.
fn spruce_sprig() -> LeafTemplate {
    sprig(spruce_needle(), SPRIG_NEEDLES, SPRIG_SPAN, 0.82)
}

/// A shoot's worth of needles in one card: `count` blades of `needle`, each at its own station along
/// the card and its own slot around it, every one of them swept `pitch` radians toward the card's tip.
///
/// # Invariants
///
/// - **The stations and the slots are both the golden angle, because the card stands for the shoot's
///   own spiral and not for a rosette.** Five needles at even stations and even slots would be a hand,
///   which is what a fascicle is and not what a *Picea* or an *Abies* shoot is; the two laws are the
///   same number for the same reason the field's `phyllotaxy` is.
/// - **The pitch rises along the card**, because a real spray is combed at its tip and radial at its
///   base, and a card whose five needles all stand at one angle is a starburst whichever way it faces.
/// - **The needle's own face is turned toward the sky**: the blade's `+z` is the projection of the
///   world's up onto the plane across the needle's own axis, so a needle lying nearly along the shoot
///   still shows its flat side upward rather than its edge. That is FOC's *"strongly revolute"* for
///   the fir and the *"pale, almost silvery cast"* on the adaxial face for the spruce.
fn sprig(
    needle: LeafTemplate,
    count: usize,
    span: f32,
    pitch: f32,
) -> LeafTemplate {
    LeafTemplate::sprig(
        (0..count)
            .map(|index| {
                let station = span * (index as f32 + 1.0) / (count + 1) as f32;
                let slot = index as f32 * GOLDEN;
                let angle = pitch * (0.70 + 0.13 * index as f32);
                let azimuth = slot * std::f32::consts::TAU;
                let forward = [
                    angle.cos(),
                    angle.cos() * azimuth.sin(),
                    angle.sin(),
                ];
                (needle.clone(), on_a_peg(forward, station))
            })
            .collect(),
    )
}

/// The frame one needle of a [`sprig`] stands in: its `+x` is `forward`, its origin `station` along the
/// card's own axis, and its `+z` is as close to world up as `forward` allows.
///
/// # Invariants
///
/// **The frame is built rather than placed, because a needle's orientation is three numbers and a
/// [`Placement`] is nine.** The construction is the only one that keeps them consistent: `forward` is a
/// unit vector, `z` is world up with its component along `forward` removed and renormalised, and `y` is
/// their cross product — so the frame is orthonormal to the last bit however far forward leans, and a
/// needle standing square along the shoot (where `forward` *is* up's projection and the remaining
/// component vanishes) still gets a unit `z` rather than a division by zero.
fn on_a_peg(forward: [f32; 3], station: f32) -> Placement {
    let up = [0.0f32, 1.0, 0.0];
    let along = (0..3).map(|axis| forward[axis] * up[axis]).sum::<f32>();
    let raw: [f32; 3] = (0..3)
        .map(|axis| up[axis] - along * forward[axis])
        .collect::<Vec<f32>>()
        .try_into()
        .unwrap_or([0.0, 1.0, 0.0]);
    let length = raw.iter().map(|value| value * value).sum::<f32>().sqrt();
    let z = if length > 1.0e-4 {
        raw.map(|value| value / length)
    } else {
        [0.0, 0.0, 1.0]
    };
    let y = (0..3)
        .map(|axis| {
            z[(axis + 1) % 3] * forward[(axis + 2) % 3] - z[(axis + 2) % 3] * forward[(axis + 1) % 3]
        })
        .collect::<Vec<f32>>()
        .try_into()
        .unwrap_or([0.0, 0.0, 0.0]);
    Placement {
        origin: [station, 0.0, 0.0],
        x: forward,
        y,
        z,
    }
}

/// A comb-toothed blade of `length` and `half_width`: `teeth` needle-teeth along a blade whose
/// silhouette is a plume's and not a paddle's.
///
/// # Invariants
///
/// **The sine is clamped at nought** because the `f32` pi overshoots the true one and its sine at
/// `t = 1` is a small negative number, whose fractional power is not a number at all. Every other
/// knot is a real number and the width law is a law, so the plume is a solid whose two faces carry
/// their own normals and a plume lit from behind needs no shading trick.
fn comb(
    length: f32,
    half_width: f32,
    teeth: usize,
    thickness: f32,
    stations: usize,
) -> LeafTemplate {
    let knots: Vec<(f32, f32)> = (0..=2 * teeth)
        .map(|index| {
            let t = index as f32 / (2 * teeth) as f32;
            let envelope = (std::f32::consts::PI * t).sin().max(0.0).powf(0.7);
            let tooth = if index % 2 == 0 { 0.12 } else { 1.0 };
            (t, half_width * envelope * tooth)
        })
        .collect();
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [length * 0.50, length * 0.05, 0.0]),
            (
                [length * 0.50, length * 0.12, length * 0.04],
                [length * 0.50, length * 0.12, length * 0.04],
            ),
            (
                [length, length * 0.28, length * 0.09],
                [length * 0.42, length * 0.14, length * 0.04],
            ),
        ]),
        shape::Scalar::new(&knots).expect("a comb is a law"),
        thickness,
        stations,
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
/// note's 4–12 cm × 1.7–3.5 cm.
///
/// **The law is ovoid-oblONG and not ovoid**, on the same finding as the fir's: the note says *"ovoid-
/// oblong or ovoid-cylindric"* and the first pass's law rose to a maximum at a third of the length and
/// fell away to a point at both ends, which is an ovoid — a lens. Here the cone holds its two and a
/// half centimetres from 0.42 to 0.66 of its length, rounds off over the last fifth and stands on a
/// short stalk, which is what `09-cone-macro.jpg`'s denticulate margins are seen on. Eighteen slots
/// alternating 1.0 and 0.72, because the scale margins are what give the ripe cone its shaggy outline
/// and they are the section's own star.
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
        (0.10, 0.0026),
        (0.24, 0.0088),
        (0.42, 0.0124),
        (0.66, 0.0126),
        (0.84, 0.0104),
        (0.95, 0.0054),
        (1.00, 0.0016),
    ]);
    Template {
        sweep,
        stations: 14,
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
///
/// **The three outer fields sit one generation further out than the depth that names them.** `tier`
/// is a field on the **parent** of the shoots, which on the whole-tree plate is the leader's bough and
/// on the bough plate is the bough itself — the first version of this module named the tier's
/// generation as the literal `1` in both, so the bough plate put *three* needle fields on its shoots
/// and left its own bough a bare rod across the frame, which is the `bare broom` defect at arm's
/// length. One rule and one number places both.
fn spruce_needles(shoot_depth: usize, foliage: Foliage) -> Vec<LeafField> {
    let factor = foliage.density_factor();
    let mut candle = LeafField::new(shoot_depth.saturating_sub(2), 0);
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
    let mut tier = LeafField::new(shoot_depth - 1, 0);
    // **The bare inner stem, and the plume that crowds at the tip.** The dead-give defect is foliage
    // from 0.05 to 0.99 of every tier, which wraps the leader in an even sleeve and reads as a
    // cylinder; `02-crown.jpg` shows a bough that is wood for its inner third and a plume for the rest,
    // and the density climbs along it because the plume is the whole of the tier's mass. The law is
    // written over the tier's **own** length, so a 0.24 m leader-tip tier and a 3.3 m lower one are
    // both bare for the same third of themselves.
    tier.band = (0.28, 0.99);
    tier.density = rising(190.0, 400.0, 680.0, factor);
    tier.phyllotaxy = GOLDEN;
    tier.pitch = shape::Scalar::constant(1.02);
    tier.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    tier.size = shape::Scalar::new(&[(0.0, 0.80), (1.0, 1.0)]).expect("a size is a law");
    tier.jitter = 0.24;
    let mut spray = LeafField::new(shoot_depth, 0);
    // **The band stops at the base of the shoot**, which is the *dead give* defect: no real shoot is
    // foliated evenly from the wood it grows on to its own tip.
    spray.band = (0.18, 0.97);
    // **Two hundred and fifty a metre is the note's own 25–60 needles over its own 5–12 cm**, read as
    // a linear density rather than a count: one needle every four millimetres, which is what the
    // overlapping spiral of `04-branch.jpg` and `07-cone-hanging.jpg` shows. The first version of this
    // module carried the *count* onto a sixteen-centimetre shoot instead of the density, and the
    // shoots came out as bare bristles with three needles on them.
    spray.density = rising(150.0, 240.0, 330.0, factor);
    spray.phyllotaxy = GOLDEN;
    spray.pitch = shape::Scalar::constant(1.05);
    spray.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    spray.size = shape::Scalar::new(&[(0.0, 0.82), (1.0, 1.06)]).expect("a size is a law");
    spray.jitter = 0.26;
    let mut brush = LeafField::new(shoot_depth, 0);
    brush.band = (0.80, 1.0);
    brush.density = shape::Scalar::constant(340.0 * factor);
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

/// The density law both species' outer fields are written on: **a rise from `low` at the bare end of
/// the node to `high` at its tip, through `middle` at the halfway station**, every number scaled by
/// `factor`.
///
/// # Invariants
///
/// **The rise is the shape of the defect and not a taste.** A field whose density is one constant puts
/// the same number of blades on every centimetre of a tier, and a tier read end to end is then a sleeve
/// rather than a plume; the eye reads the *gradient* long before it reads the count, which is why the
/// first pass failed on this and not on a density that was too low. The knots are at a node's nought,
/// its middle and its tip because [`vegetation::leaf::LeafField`] integrates the law over the band with
/// the midpoint rule, and three knots are the fewest that make a rise rather than a tilt.
fn rising(low: f32, middle: f32, high: f32, factor: f32) -> shape::Scalar {
    shape::Scalar::new(&[
        (0.0, low * factor),
        (0.5, middle * factor),
        (1.0, high * factor),
    ])
    .expect("a rise is a law")
}

/// The spruce: its tree, its foliage at the asked scale, its four fields and the cone template its
/// shoots carry.
pub fn lijian_spruce(seed: u64, foliage: Foliage) -> Species {
    let mut species = Species {
        plant: grow(&lijian_spruce_rule(), seed),
        templates: vec![match foliage {
            Foliage::Needle => spruce_needle(),
            Foliage::Sprig => spruce_sprig(),
            Foliage::Plume => spruce_plume(),
        }],
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

/// The spruce's bough plate: one bough as the trunk, its shoots as the only generation, and the
/// note's own needle **five to a card**. A camera inside a hundred thousand needles shades every pixel
/// of them; the frame costs minutes for what one tier shows in seconds.
pub fn lijian_spruce_bough(seed: u64) -> Species {
    Species {
        plant: grow(&lijian_spruce_bough_rule(), seed),
        templates: vec![spruce_sprig()],
        fields: spruce_needles(1, Foliage::Sprig),
        decorators: vec![spruce_cone()],
    }
}

/// The cones a spruce's shoots carry at depth `shoot_depth`: **old cones persist for years** (the
/// note's 4–6 years is an ESTIMATE), so a shoot carries nought to one and the crown is studded rather
/// than stippled — the note's fourth silhouette fact, and the reason the count is a range the seed
/// decides rather than a cone on every shoot.
fn spruce_cones(species: &Species, shoot_depth: usize, count: (u32, u32)) -> Vec<Seat> {
    scatter(&species.plant, shoot_depth, 0, 0.94, count)
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
        wood: Skin::new([0.090, 0.080, 0.066], 0.96),
        leaves: vec![Skin::new([0.082, 0.142, 0.104], 0.84)],
        // **Purple-red and not salmon.** FOC's *"reddish brown or purple … maturing … purplish brown, or
        // dark purple"*, and `07-cone-hanging.jpg`'s cones are a deep wine rather than a pink; the first
        // pass's `[0.250, 0.090, 0.078]` was two tints too light and read as a row of leaves standing on
        // the shoots. This is the same hue at two thirds the value, which is what the note's *dark
        // purple* is against a blue-green needle.
        seats: vec![Skin::new([0.176, 0.056, 0.062], 0.86)],
    }
}

/// A subalpine slope at three thousand metres: a deep high sky, a warm key **raking in from the
/// camera's right at thirty degrees above the horizon**, and a cold fill off the valley side.
///
/// # Invariants
///
/// - **A light's reach is the distance its irradiance is stated at**, so each reach is the length of
///   the position it belongs to: the key at `[36.13, 22.50, -14.60]` is 45.0 m from the origin, and its
///   reach is 45.0 m, so the number the subject is lit at is the number the author wrote.
/// - **The key is off the camera axis and that is the whole change.** The first pass put it at
///   `[26.0, 34.0, 16.0]` against an eye at `[26.0, 6.0, 35.0]` — twenty-two degrees apart in azimuth
///   and both high, so the key was a frontal light and a spire lit frontally has no lit flank and no
///   dark flank, only a middle. The eye's azimuth is 53.4 degrees and this key's is −22.0, so the light
///   comes in along the camera's own screen-right at thirty degrees of elevation: the right flank of the
///   spire is lit, the left is not, and that asymmetry is what makes the crown a solid rather than a
///   silhouette with a stripe on it. Thirty degrees is also what subalpine light actually does on a
///   north-east slope at the hour these trees are photographed.
fn spruce_lighting() -> Lighting {
    Lighting {
        key_position: [36.13, 22.50, -14.60],
        key_colour: [1.0, 0.94, 0.82],
        key_irradiance: 3.1,
        key_reach: 45.0,
        key_distance: 45.0,
        key_extent: 9.0,
        fill_position: [-17.0, 11.0, -15.0],
        fill_colour: [0.40, 0.50, 0.72],
        fill_irradiance: 0.26,
        fill_distance: 25.2,
        floor: Skin::new([0.115, 0.108, 0.082], 0.96),
        sky: Atmosphere::new()
            .zenith([0.05, 0.12, 0.36])
            .horizon([0.42, 0.53, 0.66])
            .ground([0.26, 0.25, 0.21])
            .density(0.0005)
            .scale_height(3000.0),
        sky_radiance: 0.30,
        // A tree framed at forty-five metres is in thinner air than a pine framed at twenty-five, and
        // the air at three thousand metres is thinner than both.
        haze: 0.004,
    }
}

/// The spruce's **base** plate's light: the same day, the same sky, and **the key brought down to
/// fourteen degrees and in to sixteen metres**.
///
/// # Invariants
///
/// **A crown shadows its own foot, and the only number that decides whether it does is the key's
/// elevation.** A key at thirty degrees reaches the bole at one metre only if the ray clears the lowest
/// bough, and the lowest bough of this species hangs at 3.7 m on a 3.3 m radius: the ray arrives at
/// 0.5 + 3.3 · tan 30° = 2.4 m and clears. A key at sixty degrees arrives at 6.2 m and is stopped by the
/// crown, and the whole plate goes flat. At **fourteen degrees** the same ray arrives at 1.3 m and not
/// only clears but *grazes*, which is the whole point: a light almost level with the ground rakes the
/// twenty-one plates and the eight furrows across the bole and is the only angle at which that relief
/// casts a shadow of its own. The key is also **in to sixteen metres**, because its reach has to end
/// before the crown's own shadow does or the upper half of the tree goes black while the subject is
/// still in daylight.
fn spruce_base_lighting() -> Lighting {
    Lighting {
        key_position: [14.55, 3.87, -5.41],
        key_colour: [1.0, 0.93, 0.79],
        key_irradiance: 2.6,
        key_reach: 16.0,
        key_distance: 16.0,
        key_extent: 2.6,
        fill_position: [-9.31, 5.62, 10.34],
        fill_colour: [0.42, 0.50, 0.68],
        fill_irradiance: 0.62,
        fill_distance: 15.0,
        floor: Skin::new([0.115, 0.108, 0.082], 0.96),
        sky: Atmosphere::new()
            .zenith([0.05, 0.12, 0.36])
            .horizon([0.42, 0.53, 0.66])
            .ground([0.26, 0.25, 0.21])
            .density(0.0005)
            .scale_height(3000.0),
        sky_radiance: 0.30,
        // The ground is a metre away rather than forty, so the air between the lens and the bark is
        // twenty times thinner and the coefficient is a twentieth.
        haze: 0.0002,
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

/// The bough plate's frame: **1.17 metres off a run of shoots**, side-on to the bough and a little
/// above it, so a spruce needle is one and a quarter pixels wide and eleven long — the range the bar's
/// H2 asks for — and **three dozen sprays fill the frame**.
///
/// # Why it moved out and not in
///
/// The first pass stood at **0.54 m**, which is closer, and the plate read thin. The distance was not
/// the whole of it: at 0.54 m a cone of the note's own ten centimetres is a fifth of the frame and six
/// of them are a wall, so the frame was filled with cones and sky and the bottlebrush that the plate
/// exists to show was a scatter of hairs between them. At 1.17 m the cone is a tenth of the frame and
/// the **bough** is the subject: 1.1 m of a 1.95 m tier is in shot, which is thirty-odd of its sixty
/// shoots, and the needle count that makes a spray read — five blades on every peg — comes from
/// [`Foliage::Sprig`] rather than from standing closer and seeing fewer of them larger.
const SPRUCE_BOUGH_FRAME: Framing = Framing {
    width: 1200,
    height: 1000,
    eye: [1.62, 0.30, 1.10],
    target: [1.30, 0.37, 0.06],
    field_of_view: 0.75,
    near: 0.02,
    far: 40.0,
    floor: 6.0,
    ground: Ground::Plane,
};

/// The base plate's frame: **2.9 metres** off the foot of the spruce's bole at a person's eye height on
/// a lens wide enough to hold the flare whole.
///
/// # Why this distance and this lens
///
/// The tree's bole is **2.2 metres across at the ground** — the note's 2.5 m d.b.h. kept in proportion
/// on a thirty-four-metre trunk, with a root flare on top — so a frame narrower than 2.8 m crops the
/// subject and the bar's H4, which is read on the flare and the plating, has nothing to read. At 2.9 m
/// on a 1.10 rad lens the frame is 3.6 m tall and 2.8 m wide, the bole fills four fifths of the width,
/// and the ground, the first boughs and eight metres of bark are all in shot. The eye is at 1.55 m and
/// the target at 1.35 m, so the frame looks very slightly up a bole that is still widening.
const SPRUCE_BASE_FRAME: Framing = Framing {
    width: 1100,
    height: 1400,
    eye: [1.90, 1.55, 2.20],
    target: [0.0, 1.35, 0.0],
    field_of_view: 1.10,
    near: 0.05,
    far: 30.0,
    floor: 10.0,
    ground: Ground::Plane,
};

/// The fir's base plate's frame: **2.7 metres** off a bole half the spruce's, on the same lens and the
/// same eye height, so the two base plates are the same picture of two different feet — and the fir's
/// bole fills **half** the frame's width where the spruce's fills four fifths, which is itself one of
/// the two numbers that tell the species apart.
const FIR_BASE_FRAME: Framing = Framing {
    width: 1100,
    height: 1400,
    eye: [1.75, 1.50, 2.05],
    target: [0.0, 1.25, 0.0],
    field_of_view: 1.10,
    near: 0.05,
    far: 30.0,
    floor: 10.0,
    ground: Ground::Plane,
};

/// The dial a base plate's wood is expanded at: [`Detail::DEFAULT`] with **twenty-four sides**.
///
/// # Invariants
///
/// **The sections are authored at twenty and twenty-four points and this is the number that stops the
/// expansion throwing them away.** [`vegetation::expand::wood`] samples every sweep at
/// [`Detail::around`] regardless of how many points its section carries, so at the default twelve the
/// spruce's twenty-sided plated ring and the fir's twenty-four-sided furrowed one are both resampled
/// to a twelve-gon: the plating and the furrows are averaged away, and the creased normals that
/// [`vegetation_mesh::wood_points_of`] now draws have nothing to crease because a regular twelve-gon
/// turns thirty degrees at every corner and [`vegetation_mesh::WOOD_CREASE`] is thirty-five. At
/// twenty-four the fir's section is the section and the spruce's twenty points are twenty-four, and the
/// lumpiness carries corners past thirty-five degrees, which is where they crease.
///
/// **Twenty-four and not twenty is the fir's own count**, because the fir is the coarser of the two
/// bark textures — eight furrows rather than three plates — and a plate needs fewer sides to read than
/// a furrow does. The stations and the spacing are [`Detail::DEFAULT`]'s and are untouched: the ring
/// count along a spine is not what this plate is about.
const BARK_DETAIL: Detail = Detail {
    stations: 6,
    around: 24,
    spacing: 0.5,
};

/// Builds the Lijiang spruce's whole-tree plate: one tree in the card foliage, its own light and ground.
pub fn lijian_spruce_picture() -> Value {
    let species = lijian_spruce(SEED, Foliage::Plume);
    let planting = Planting::new(species.clone())
        .with_seats(spruce_cones(&species, 2, (0, 2)))
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
        .with_seats(spruce_cones(&species, 1, (0, 1)))
        .with_palette(spruce_palette());
    plant_scene_lit(
        &[planting],
        &SPRUCE_BOUGH_FRAME,
        LIJIAN_SPRUCE_BOUGH,
        &spruce_lighting(),
        &[],
    )
}

/// Builds the spruce's base plate: the foot of its bole at 2.9 metres, which is the frame H4 reads the
/// plating in, under the low raking key [`spruce_base_lighting`] gives.
///
/// The foliage is the plume and the cones are the tree's own, for the pine's reason and this species'
/// harder one: **the spruce's crown starts at 0.11 of thirty-four metres — 3.7 m — and its lowest
/// bough is 3.3 m long**, so unlike the fir's this tree's crown is close enough to its foot to be in
/// the upper half of the frame, and dropping it would be dropping part of the plate. The plume is nine
/// rows to the needle and the frame crops every one of them.
pub fn lijian_spruce_base_picture() -> Value {
    let species = lijian_spruce(SEED, Foliage::Plume);
    let planting = Planting::new(species.clone())
        .with_seats(spruce_cones(&species, 2, (0, 2)))
        .with_palette(spruce_palette())
        .with_detail(BARK_DETAIL);
    plant_scene_lit(
        &[planting],
        &SPRUCE_BASE_FRAME,
        LIJIAN_SPRUCE_BASE,
        &spruce_base_lighting(),
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

/// Resolves a spruce base plate's `authored_id`: the base plate is the **whole tree** at another
/// framing and another dial, not a second plant, so its own table is the whole tree's.
pub fn lijian_spruce_base_handle_of(id: u32) -> Option<String> {
    lijian_spruce_handle_of(id)
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
            // **The tiers.** Six to eight boughs at one station, spread as `count / cluster` up the crown band.
            // **288 children at six to a whorl is forty-eight tiers over seventeen metres of leader,
            // which is 0.36 m between tiers** — inside the note's 0.5–1.0 m **ESTIMATE** and a third
            // again closer than the first version, which put two thirds of a metre of bare bole between
            // two tiers and lost the species' signature. The fir's signature is the *banding*: tiers
            // separated by less than a tier's own foliage depth, so the gaps read as shadow and not as
            // sky.
            Generation {
                count: (288, 288),
                station: (FIR_CROWN_BASE, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.04, 0.05),
                sink: 0.025,
                cluster: 6,
                shape: fir_tier,
            },
            // **The shoots on a tier**, six of them over its outer four fifths; the tier's own spray
            // field clothes the tier along its whole length, so a tier is never a bare rod.
            Generation {
                count: (6, 6),
                station: (0.14, 1.0),
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
///
/// # Invariants
///
/// **The last two knots' angles are *smaller* than the first, and that is the whole of the upsweep.**
/// [`bend`] measures an angle from the parent's tangent, nought along the parent and a quarter turn
/// straight out, so a heading of 1.5 rad is horizontal, **1.5 rad and above turns the tip down** and
/// below turns it up. The first pass wrote `departure + sweep` with `sweep` running to 1.04, which put
/// the outer knots at up to 2.40 rad — forty-three degrees **below** the horizontal — under a comment
/// that said they turned a quarter turn above it. The fir's tiers were therefore drooping by exactly
/// as much as the spruce's, which is the second of the four silhouette rows and which the second-pass
/// silhouettes were asked to keep tellable apart. The numbers here subtract instead, and the same
/// correction is made to [`fir_shoot`] and [`fir_bough_spray`].
fn fir_tier(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let profile = crown_profile(joint.station, FIR_CROWN_BASE);
    // A narrower crown that falls away faster than it narrows: **2.55 metres at the base of the crown for
    // 5.1 m of crown width**, and twenty-eight centimetres at the leader's tip. **Five point one metres
    // is under the note's six-to-ten-metre ESTIMATE**, and it is the one number in this module that
    // goes outside a note: the bar asks for a fir narrower than the spruce, the two notes' ratios say
    // the opposite, and this is the fir's crown at 0.20 of its height beside the spruce's at 0.24, which
    // is what makes the two silhouettes tellable apart with the foliage off.
    let length = (0.26 + 2.29 * (1.0 - profile).powf(1.85)) * (0.78 + 0.44 * rng.unit());
    let departure = 1.46 + 0.14 * profile;
    // **The upsweep, in the same place the spruce's droop is, and subtracted rather than added:** a low
    // tier's last knot is at 1.06 rad — thirty-nine degrees above the horizontal — and a leader-side
    // one's at 1.34, which is seven. A fir tier ends in upstanding tips and a spruce's ends in
    // pendulous ones, and the two numbers are the whole of the difference.
    let sweep = 0.26 + 0.14 * (1.0 - profile);
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.06,
                departure - 0.16,
                departure - sweep,
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
/// needles, leaving its tier a little above the horizontal and **turning up through its last third**, so
/// a tier's ends are a comb of upstanding sprays rather than a drooping fringe.
///
/// **The turn is subtracted, for [`fir_tier`]'s reason and not a second time**: the first pass ended its
/// shoots at 1.38–1.74 rad, which is level to forty-one degrees *below* it, so a fir's tier ended in a
/// fringe of hanging branchlets and a cone seated on one lay along it rather than standing up.
///
/// The length is the same extension of the note's ESTIMATE as the spruce's shoot carries, and for the
/// same reason: the note's 5–12 cm is read from two close photographs of the finest tips.
fn fir_shoot(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.22 * (0.80 + 0.42 * rng.unit());
    let departure = 1.10 + 0.20 * rng.signed();
    let sweep = 0.26 + 0.22 * rng.unit();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure - 0.06,
                departure - 0.16,
                departure - sweep,
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

/// The fir's bough plate's spray: the same leaves as a tier's, leaving an upright leader a little above
/// the horizontal and **turning up through its last two knots**.
///
/// **Subtracted, for [`fir_tier`]'s reason**, and this one had it twice as hard: the first pass ended
/// its sprays at 2.14–2.60 rad — a hundred and twenty-three to a hundred and forty-nine degrees, so
/// every spray on the bough plate curved over and hung **down**, and the cone seated on its tip lay along
/// it and read as a pod on a twig. A fir's cone must stand above the foliage and not lie along a spray
/// that is pointing at the ground.
fn fir_bough_spray(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.26 * (0.82 + 0.36 * rng.unit());
    let departure = 1.34 + 0.18 * rng.signed();
    let sweep = 0.44 + 0.26 * rng.unit();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.06,
                departure - 0.12,
                departure - sweep,
            ],
        ),
        taper(&[(0.00, radius * 0.62), (0.30, radius * 0.34), (1.00, 0.0026)]),
    )
}

/// The fir needle, at the note's own 1.5–3 cm × 1.7–2.5 mm, three tenths as thick as it is wide
/// because FOC's section is *"flattened"* and its margin *"strongly revolute"* — the exact
/// complement of the spruce's *"broadly rhombic"* — curving into the note's *"S"* and narrowing to the
/// emarginate apex.
fn fir_needle() -> LeafTemplate {
    LeafTemplate::blade(
        fir_midrib(FIR_NEEDLE_LENGTH),
        taper(&[
            (0.0, FIR_NEEDLE_HALF_WIDTH * 0.55),
            (0.16, FIR_NEEDLE_HALF_WIDTH * 0.96),
            (0.52, FIR_NEEDLE_HALF_WIDTH),
            (0.86, FIR_NEEDLE_HALF_WIDTH * 0.70),
            (1.0, FIR_NEEDLE_HALF_WIDTH * 0.28),
        ]),
        FIR_NEEDLE_HALF_WIDTH * 0.30,
        6,
        4,
    )
}

/// The fir needle's midrib at `length`; the positions scale with the tangents, as in
/// [`spruce_midrib`].
fn fir_midrib(length: f32) -> shape::Curve {
    let reach = length / FIR_NEEDLE_LENGTH;
    spine(&[
        (
            [0.0000, 0.00000, 0.0],
            [0.0080 * reach, 0.00190 * reach, 0.0],
        ),
        (
            [0.0110 * reach, 0.00235 * reach, 0.0],
            [0.0076 * reach, 0.00090 * reach, 0.0],
        ),
        (
            [0.0208 * reach, 0.00300 * reach, 0.0],
            [0.0072 * reach, -0.00170 * reach, 0.0],
        ),
    ])
}

/// The fir's plume: the same comb as the spruce's, at the fir's own card size and tooth count.
fn fir_plume() -> LeafTemplate {
    comb(
        FIR_PLUME_LENGTH,
        FIR_PLUME_HALF_WIDTH,
        FIR_PLUME_TEETH,
        0.0022,
        20,
    )
}

/// The fir's sprig: the spruce's construction at the fir's own needle, and **standing further forward**,
/// because FOC's two arrangements are *"radially spreading ± forward **or** pectinately arranged in
/// 2 lateral sets"*: a fir shoot is a comb swept along its own axis and a spruce shoot is a bottlebrush
/// on its pegs, and one pitch cannot be both.
fn fir_sprig() -> LeafTemplate {
    sprig(fir_needle(), SPRIG_NEEDLES, SPRIG_SPAN, 1.12)
}

/// The fir's seed cone, drawn **standing**.
///
/// # Invariants
///
/// **A seat expresses no rotation**, so the standing is this template's own shape and not a property of
/// where it was put: the cone's axis runs along the node's tangent, and the only node this species
/// seats a cone on is one whose tangent at that station points **up** — the tip of an upswept tier, or
/// of a spray on the bough plate that turns to within twenty degrees of vertical. **Ten point eight
/// centimetres by four**, the **top** of the note's **6–11 cm × 3–4 cm**, with fourteen stations and
/// sixteen slots.
///
/// **The outline is a cone and not a lens, and the second pass's finding is that this is a shape and
/// not a size.** The first pass's scale law rose from the axis to a maximum at 0.42 of the length and
/// then fell to a point at the end, which is a **biconvex lens** — and at the bough plate's old half
/// metre every cone in the frame read as a leaf standing on a twig. FOC says *cylindric or
/// ovoid-cylindric*, and the law here holds four centimetres across the middle **third** of the cone
/// and rounds off over the last tenth, which is the outline `06-cone.jpg` shows. The sixteen slots
/// alternate 1.0 and 0.66 so the note's *exserted bracts* are a shallow scallop on a cone's own
/// silhouette: at 0.50 they were so deep that the eight lobes merged back into the lens the law had
/// already made.
fn fir_cone() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0000, 0.0000, 0.0], [0.0028, 0.0250, 0.0]),
            ([0.0006, 0.0270, 0.0], [0.0002, 0.0390, 0.0]),
            ([0.0007, 0.0715, 0.0], [0.0000, 0.0390, 0.0]),
            ([0.0005, 0.1080, 0.0], [0.0000, 0.0070, 0.0]),
        ]),
        scalloped(16, 0.66),
    );
    sweep.scale = taper(&[
        (0.00, 0.0026),
        (0.10, 0.0034),
        (0.24, 0.0110),
        (0.46, 0.0186),
        (0.70, 0.0196),
        (0.86, 0.0176),
        (0.96, 0.0104),
        (1.00, 0.0032),
    ]);
    Template {
        sweep,
        stations: 14,
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
///
/// **The tier is a spray and not a sleeve, on the same law as the spruce's** — bare for its inner
/// quarter, crowding toward the tip — which is what turns `02-crown.jpg`'s stacked horizontal plates
/// into plates rather than into a cone of even cloth.
fn fir_needles(shoot_depth: usize, foliage: Foliage) -> Vec<LeafField> {
    let factor = foliage.density_factor();
    let mut candle = LeafField::new(shoot_depth.saturating_sub(2), 0);
    candle.band = (0.80, 0.945);
    candle.density = shape::Scalar::constant(90.0 * factor);
    candle.phyllotaxy = GOLDEN;
    candle.pitch = shape::Scalar::constant(1.10);
    candle.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    candle.size = shape::Scalar::constant(1.0);
    candle.jitter = 0.22;
    // **The tier's own spray**, at the same linear reading of the note as the shoot's: a fir's tier is
    // clothed along its whole length and a tier with foliage only on its eleven shoots is a bare broom.
    let mut tier = LeafField::new(shoot_depth - 1, 0);
    tier.band = (0.24, 0.99);
    tier.density = rising(180.0, 380.0, 640.0, factor);
    tier.phyllotaxy = GOLDEN;
    tier.pitch = shape::Scalar::constant(1.05);
    tier.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    tier.size = shape::Scalar::new(&[(0.0, 0.80), (1.0, 1.0)]).expect("a size is a law");
    tier.jitter = 0.24;
    let mut spray = LeafField::new(shoot_depth, 0);
    // **The band stops at the base of the shoot**, which is the *dead give* defect.
    spray.band = (0.16, 0.97);
    // The same linear reading of the note's ESTIMATE as the spruce's, on a longer shoot: a fir's
    // needles are twice as long and twice as broad, and a fir shoot is a comb of them rather than a
    // bottlebrush, so the count per shoot comes out a little higher at the same spacing.
    spray.density = rising(160.0, 250.0, 340.0, factor);
    // **Two lateral ranks**: half a turn from one needle to the next, so the shoot is a flat comb.
    spray.phyllotaxy = OPPOSITE;
    // Pitch and twist are the two numbers that turn a radial needle into a flat one: the pitch stands
    // the blade forward along the shoot and the twist rolls it so its **face is the sky**, which is
    // what *"strongly revolute"* and *"two white bands abaxially"* look like from below.
    spray.pitch = shape::Scalar::constant(1.12);
    spray.twist = shape::Scalar::new(&[(0.0, 1.32), (1.0, 1.46)]).expect("a roll is a law");
    spray.size = shape::Scalar::new(&[(0.0, 0.84), (1.0, 1.08)]).expect("a size is a law");
    spray.jitter = 0.20;
    // **The fir's own second arrangement, from the same sentence of the note**: *"Leaves spirally
    // arranged, radially spreading ± forward **or** pectinately arranged in 2 lateral sets"*. The comb
    // above is the pectinate half and this is the radial half, at half the density, and without it the
    // two ranks put every needle on one side of a shoot and the spray reads as a fish bone.
    let mut radial = LeafField::new(shoot_depth, 0);
    radial.band = (0.20, 0.96);
    radial.density = rising(80.0, 130.0, 190.0, factor);
    radial.phyllotaxy = GOLDEN;
    radial.pitch = shape::Scalar::constant(0.78);
    radial.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.55)]).expect("a roll is a law");
    radial.size = shape::Scalar::new(&[(0.0, 0.78), (1.0, 0.96)]).expect("a size is a law");
    radial.jitter = 0.28;
    let mut brush = LeafField::new(shoot_depth, 0);
    brush.band = (0.70, 1.0);
    brush.density = shape::Scalar::constant(320.0 * factor);
    brush.phyllotaxy = OPPOSITE;
    brush.pitch = shape::Scalar::constant(1.02);
    brush.twist = shape::Scalar::new(&[(0.0, 1.26), (1.0, 1.52)]).expect("a roll is a law");
    // **The pale new growth at the shoot tips** is the note's *"paler young growth at the shoot tips"*,
    // and it is a size law and not a second template because the blade is the same blade.
    brush.size = shape::Scalar::new(&[(0.0, 0.86), (1.0, 1.04)]).expect("a size is a law");
    brush.jitter = 0.18;
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
        templates: vec![match foliage {
            Foliage::Needle => fir_needle(),
            Foliage::Sprig => fir_sprig(),
            Foliage::Plume => fir_plume(),
        }],
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
        templates: vec![fir_sprig()],
        fields: fir_needles(1, Foliage::Sprig),
        decorators: vec![fir_cone()],
    }
}

/// The cones a fir's nodes carry: `generation` nodes, each seat at `station` along itself, `count` to
/// a node.
///
/// # Invariants
///
/// **The seat is on a node whose tangent at that station points up**, and that node is the tier
/// on the tree and the spray on the bough plate — both of which are drawn with an upsweep in their
/// last two knots. A cone on a drooping node would hang, which is the spruce's whole arrangement and
/// would make the two species the same tree. `station` is therefore **past `0.9` on every call**: the
/// standing is the tangent's, and the tangent only turns up in its last tenth.
fn fir_cones(
    species: &Species,
    generation: usize,
    station: f32,
    count: (u32, u32),
) -> Vec<Seat> {
    scatter(&species.plant, generation, 0, station, count)
}

/// **Four cones on the topmost metre and a half of the fir's own leader, and that is the
/// only cone in either species that is written out by hand.**
///
/// # Invariants
///
/// - **They are here because they are the only cones that stand above the crown line**, which is the
///   fir's second silhouette fact: `07-cone-erect-crown.jpg` shows them *"like dark candlesticks"* above
///   the foliage, and a cone on a tier's tip stands *inside* the crown however upright its template is.
/// - **The leader's candle stops at 0.945 of the height, not at 1.0**, so the last metre and a half
///   of bole stands bare and the cones on it have something to stand against. A candle carried to the
///   tip buries its own cones — a plume is 300 mm long and radiates from its anchor, so a candle that
///   reaches 1.0 has its foliage a third of a metre above the apex — which is what the first two
///   renders of these four did.
/// - **Eight of them and not four**, and that is the second pass's finding: at thirty metres a cone
///   seven pixels tall is read as a *run* of them or as nothing, and four pale marks on the apex of a
///   twenty-five-metre spire are nothing. Eight over the same bare metre and a half is a candlestick
///   row, which is the phrase the note uses.
/// - The leader's own frame has its `y` along the leader's tangent — straight up — so a template drawn
///   along `+y` stands erect here with no rotation of its own. [`scatter`] cannot place them because it
///   takes **one station for a whole generation** and the leader is a single node: `scatter(plant, 0, …)`
///   would put one cone at one station, not eight along a leader.
fn fir_leader_cones() -> Vec<Seat> {
    [
        (0.949, 0.12),
        (0.958, 0.63),
        (0.968, 0.35),
        (0.975, 0.87),
        (0.983, 0.41),
        (0.989, 0.72),
        (0.995, 0.26),
        (1.000, 0.94),
    ]
    .into_iter()
    .map(|(station, slot)| Seat::new(Handle::root(), 0, station, slot))
    .collect()
}

/// The fir's surfaces: a rough grey-brown furrowed bole, a **darker and less glaucous** leaf than the
/// spruce's — the note's *"bright dark green"* against the spruce's *"blue-green to glaucous grey-green"*,
/// which is the other half of the two species' colour difference — and a cone of its own, **glaucous
/// blue-purple and lighter than the leaf**, which is the one tint in this module no other part of
/// either species carries.
///
/// # Invariants
///
/// **The cone is lighter than the foliage and that is the finding, not a departure from the note.**
/// FOC says the ripe cone is *"black at maturity, glaucous"*, and the first pass drew it at
/// `[0.030, 0.038, 0.070]` — which is the black and none of the glaucous, and put a surface at three
/// per cent albedo on a leaf at eight. On the whole-tree plate the two are the same darkness within a
/// factor of two and the cone vanishes. `07-cone-erect-crown.jpg` shows what the note means by
/// *glaucous*: the cones there are **paler than the needles they stand among**, a bloomed blue-grey, and
/// that contrast — not the cone's own size — is what makes a laden fir crown read from a distance. The
/// skin here is three and a half times the leaf's red and four and a half times its blue, and it is
/// still the coldest, darkest-hued surface on the tree.
fn fir_palette() -> Palette {
    Palette {
        wood: Skin::new([0.086, 0.076, 0.062], 0.97),
        leaves: vec![Skin::new([0.042, 0.080, 0.040], 0.72)],
        // **Low roughness is the bloom**: FOC's *"black at maturity, glaucous"* and the blue-black cast
        // of `06-cone.jpg` are one number — a dark, smooth, cold surface under a warm key — and the
        // glaucous bloom is the one that reads at thirty metres, so the albedo rises and the bloom
        // stays.
        seats: vec![Skin::new([0.115, 0.128, 0.180], 0.38)],
    }
}

/// The same subalpine day as the spruce's, an hour colder and **raked from the same side**: the fir is
/// the higher of the two belts and its needles are the darker, so the key is a little cooler and the
/// fill a little weaker.
///
/// # Invariants
///
/// - **A light's reach is the distance its irradiance is stated at**, so each reach is the length of
///   the position it belongs to: the key at `[28.10, 17.50, -11.36]` is 35.0 m from the origin, and its
///   reach is 35.0 m.
/// - **Both plates are raked from the same azimuth and the same elevation**, −22 degrees and thirty, so
///   the two trees are lit by one sun and a reader comparing the plates is comparing the species and not
///   the hour. The fir's eye is at azimuth 53.5 degrees and the spruce's at 53.4, which is why one
///   azimuth serves both.
fn fir_lighting() -> Lighting {
    Lighting {
        key_position: [28.10, 17.50, -11.36],
        key_colour: [0.98, 0.94, 0.86],
        key_irradiance: 3.0,
        key_reach: 35.0,
        key_distance: 35.0,
        key_extent: 8.0,
        fill_position: [-13.0, 9.0, -12.0],
        fill_colour: [0.38, 0.48, 0.72],
        fill_irradiance: 0.24,
        fill_distance: 19.8,
        floor: Skin::new([0.090, 0.105, 0.068], 0.96),
        sky: Atmosphere::new()
            .zenith([0.05, 0.12, 0.36])
            .horizon([0.40, 0.51, 0.65])
            .ground([0.23, 0.24, 0.20])
            .density(0.0005)
            .scale_height(3400.0),
        sky_radiance: 0.30,
        haze: 0.004,
    }
}

/// The fir's **base** plate's light, and the reason the fir's is the easier of the two: **its crown
/// begins at eight metres**, so a key anywhere below about forty degrees reaches the bole's foot
/// without meeting a branch. This one is at fifteen and fifteen metres out, for the same reason the
/// spruce's is at fourteen — a light almost level with the ground is the only angle at which the eight
/// longitudinal furrows of `fir_bark` cast a shadow of their own.
fn fir_base_lighting() -> Lighting {
    Lighting {
        key_position: [13.62, 3.88, -4.96],
        key_colour: [0.98, 0.94, 0.86],
        key_irradiance: 2.5,
        key_reach: 15.0,
        key_distance: 15.0,
        key_extent: 2.4,
        fill_position: [-8.69, 5.24, 9.64],
        fill_colour: [0.40, 0.50, 0.68],
        fill_irradiance: 0.58,
        fill_distance: 14.0,
        floor: Skin::new([0.090, 0.105, 0.068], 0.96),
        sky: Atmosphere::new()
            .zenith([0.05, 0.12, 0.36])
            .horizon([0.40, 0.51, 0.65])
            .ground([0.23, 0.24, 0.20])
            .density(0.0005)
            .scale_height(3400.0),
        sky_radiance: 0.30,
        haze: 0.0002,
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

/// The bough plate's frame: **0.86 metres** off an upright leader and aimed at a spray rather than at
/// the shoot, so two sprays and their erect cones fill the frame and no cone is close enough to the
/// lens to be a third of it.
///
/// It is **pulled back from the first pass's 0.50 m**, for the same reason the spruce's bough plate was:
/// the cone grew to the top of its note's range, and a 10.8 cm cone half a metre from the lens is 280
/// pixels of a thousand-row frame. At 0.86 m it is 125, the spray is the subject again, and a fir needle
/// at its own 2.1 mm is four and a half pixels — inside the bar's H2 window, which is the same window
/// the spruce's plate is in at its own width.
const FIR_BOUGH_FRAME: Framing = Framing {
    width: 1200,
    height: 1000,
    eye: [0.92, 1.06, 0.62],
    target: [0.30, 1.14, 0.02],
    field_of_view: 0.75,
    near: 0.02,
    far: 40.0,
    floor: 6.0,
    ground: Ground::Plane,
};

/// Builds the Cangshan fir's whole-tree plate: one tree in the card foliage, its own light and ground.
///
/// # Invariants
///
/// **Three seat sets, and the two scattered ones are on different nodes.** `07-cone-erect-crown.jpg`
/// puts a cone on the **tip of a spray** and `01-habit.jpg` puts them on the **tip of a tier**, and a
/// tree drawn with only one of the two has its cones in the wrong place by a metre: the tier's tips are
/// what make the crown line, and a cone on a tier tip stands **outside** the plume the tier carries,
/// while a cone on a spray tip stands inside the six sprays around it. So the tree seats nought to two
/// to every shoot and nought to one to every tier, and the eight on the leader are written by hand
/// because [`scatter`] cannot put eight stations on one node.
pub fn cangshan_fir_picture() -> Value {
    let species = cangshan_fir(SEED, Foliage::Plume);
    let mut seats = fir_cones(&species, 2, 0.96, (0, 2));
    seats.extend(fir_cones(&species, 1, 0.97, (0, 1)));
    seats.extend(fir_leader_cones());
    let planting = Planting::new(species.clone())
        .with_seats(seats)
        .with_palette(fir_palette());
    plant_scene_lit(&[planting], &FIR_FRAME, CANGSHAN_FIR, &fir_lighting(), &[])
}

/// Builds the fir's branch plate: one upright shoot at arm's length in the note's own flat needle,
/// five to a card.
pub fn cangshan_fir_bough_picture() -> Value {
    let species = cangshan_fir_bough(SEED);
    let planting = Planting::new(species.clone())
        .with_seats(fir_cones(&species, 1, 0.96, (1, 2)))
        .with_palette(fir_palette());
    plant_scene_lit(
        &[planting],
        &FIR_BOUGH_FRAME,
        CANGSHAN_FIR_BOUGH,
        &fir_lighting(),
        &[],
    )
}

/// Builds the fir's base plate: the foot of its bole at 2.7 metres, which is the frame H4 reads the
/// furrowed bark in, under the low raking key [`fir_base_lighting`] gives.
///
/// The foliage is the plume, for the spruce's reason and the fir's: the lowest tier is at 0.32 of
/// twenty-five metres, which is eight metres up, and every blade of the crown is outside a frame that
/// stops at three and a third metres. The species' whole point on this plate is the bare bole, so the
/// crown is not in it.
pub fn cangshan_fir_base_picture() -> Value {
    let species = cangshan_fir(SEED, Foliage::Plume);
    let mut seats = fir_cones(&species, 2, 0.96, (0, 2));
    seats.extend(fir_cones(&species, 1, 0.97, (0, 1)));
    seats.extend(fir_leader_cones());
    let planting = Planting::new(species.clone())
        .with_seats(seats)
        .with_palette(fir_palette())
        .with_detail(BARK_DETAIL);
    plant_scene_lit(
        &[planting],
        &FIR_BASE_FRAME,
        CANGSHAN_FIR_BASE,
        &fir_base_lighting(),
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

/// Resolves a fir base plate's `authored_id`: the base plate is the **whole tree** at another framing
/// and another dial, not a second plant, so its own table is the whole tree's.
pub fn cangshan_fir_base_handle_of(id: u32) -> Option<String> {
    cangshan_fir_handle_of(id)
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
