//! The second wave of the Hengduan: 西南桦 *Betula alnoides* and 滇油杉 *Keteleeria evelyniana*.
//!
//! # What this module is
//!
//! Arc 8 of [`southwest-china-trees.md`](../../../.agents/proposals/southwest-china-trees.md): the two
//! species of that proposal's second wave that force something the first ten did not. Every number
//! here cites its species' own note in
//! [`investigate/trees/`](../../../.agents/investigate/trees/), which cites the Flora of China and the
//! photographs in `out/refs/<slug>/`.
//!
//! | species | what it is here that the ten before it are not |
//! | --- | --- |
//! | **西南桦** *Betula alnoides* | **a wholly pendulous crown.** Every limb's outer half is drawn below horizontal, so the tree's silhouette is a curtain of hanging branchlets and not a dome. |
//! | **滇油杉** *Keteleeria evelyniana* | **erect cones, all over the crown.** A nine-to-twenty-centimetre cylinder standing upright on a lateral shoot, seated by a band on the parent rather than on the node alone. |
//!
//! # Two corrections the birch's note carries, and one the crate's own words carry
//!
//! - **西南桦's bark is 红褐色 — reddish-brown.** What the photographs show is a pale papery outer bark
//!   lifting in horizontal strips off a **cinnamon/chestnut** underlayer, which [`birch_bark`] and
//!   [`birch_palette`] are drawn from. The chalk-white Himalaya birch is *Betula utilis*, a different
//!   species, and a plate drawn from that photograph would be a different tree.
//! - **The peeling is not geometry, and the module says so rather than faking it.** [`birch_bark`] gives
//!   the bole a **lobed** section, because that is what `01-habit.jpg` and `02-bark-peeling.jpg` show of
//!   its outline: the column is wavy and folded where the papery sheets have lifted, and not a cylinder.
//!   The sheets themselves lift away from the wood, and a [`vegetation::node::Node`] has one section and
//!   one taper — **neither can say a feature that varies around the trunk and along it at once.** The
//!   horizontal banding of `02-bark-peeling.jpg` is a texture this renderer has no channel for, and the
//!   base plate is therefore evidence about the bole's *form* — its flare, its taper and its lobed
//!   outline — and the palette is evidence about its *colour*.
//! - **Keteleeria's needles come in no bundle at all.** The brief asked whether they were fascicled of
//!   two to three; the note says the opposite and says it three times — *"排列成两列"*, a flattened
//!   **two-rank** comb on every lateral shoot. So there is no `cluster` on any Keteleeria field: the
//!   two ranks are [`vegetation::leaf::LeafField::phyllotaxy`] at half a turn, the same single number the
//!   fir's is, and a fascicle of two would draw a shoot this species does not have.
//!
//! # Three plates a species, and the one pixel that decides each blade
//!
//! The rasteriser samples a pixel at one point, so **a blade narrower than one pixel is dropped, not
//! faded**. One pixel is `2 · distance · tan(fov / 2) / height`, and every plate below states its own:
//!
//! | plate | one pixel | the finest organ | the blade drawn |
//! | --- | --- | --- | --- |
//! | [`SOUTHWEST_BIRCH`] | 9.84 mm at 27.0 m | a 9 mm serration tooth = 0.9 px | the **plain** leaf, [`BirchFoliage::Leaf`] |
//! | [`SOUTHWEST_BIRCH_BOUGH`] | 1.73 mm at 2.37 m | a 9 mm serration tooth = 5.2 px | the **toothed** leaf, [`BirchFoliage::Toothed`] |
//! | [`SOUTHWEST_BIRCH_BASE`] | 1.25 mm at 2.39 m | a 5 mm catkin = 4.0 px | the bole is the subject; the plain leaf, which is 88 px long |
//! | [`EVELYN_KETELEERIA`] | 17.9 mm at 49.2 m | a 200 mm cone = 11.2 px | the **plume**, [`KeteleeriaFoliage::Plume`] |
//! | [`EVELYN_KETELEERIA_BOUGH`] | 0.99 mm at 1.26 m | a 2.5 mm needle = 2.5 px | the **true needle**, [`KeteleeriaFoliage::Needle`] |
//! | [`EVELYN_KETELEERIA_BASE`] | 3.07 mm at 5.47 m | a 0.23 m fissure = 75 px | the bole is the subject; the plume, which is 91 px long |
//!
//! **The birch is the species of this wave that needs no card at all**, and the reason is one measurement:
//! a birch leaf is **40–120 mm** and the whole-tree plate's pixel is **9.8 mm**, so a leaf is **4 to twelve
//! pixels** across and is drawn as itself at both distances. The pine's needle is 1.1 mm and is a tenth of
//! a pixel, which is the whole of the pine's [`southwest::Foliage::Spray`]; the Keteleeria's needle is
//! 2.5 mm and is 0.14 of a pixel, which is the whole of its plume. **A birch's crown is the one in this
//! proposal whose geometry is honest at thirty metres without a level of detail at all**, and that is a
//! fact about leaf size and not about merit.
//!
//! **A conifer's card is a card and the birch's is a margin.** [`BirchFoliage::Toothed`] differs from
//! [`BirchFoliage::Leaf`] in one width law and nothing else, exactly as the fern's combed pinnule differs
//! from its plain one: the note's *"不规则重锯齿，内弯的刺毛状"* — irregular double serration with
//! inward-curving bristle teeth — is **thirteen teeth in the width law**, and at the tree plate those teeth
//! are under a pixel each and show nothing for four times the rings.
//!
//! # What these six scenes cost, and what they measured
//!
//! The scene document pays **per instance row**, and every count below was read off the same expansion the
//! picture builds, before the plate was rendered rather than discovered by it. Every plate is far inside the
//! ninety-thousand-row ceiling the proposal's §4.13 sets, and every one renders in well under two minutes:
//!
//! | scene | nodes | leaf rows | seats | rows | compile | render |
//! | --- | --- | --- | --- | --- | --- | --- |
//! | [`SOUTHWEST_BIRCH`] · [`SOUTHWEST_BIRCH_BASE`] | **1 164** | ~25 000 | ~1 100 | ~26 100 | 10.7 s | 3.5 s |
//! | [`SOUTHWEST_BIRCH_BOUGH`] | 17 | 269 | 18 | **287** | 0.4 s | 0.8 s |
//! | [`EVELYN_KETELEERIA`] · [`EVELYN_KETELEERIA_BASE`] | **2 989** | **58 094** | **3 353** | **61 447** | 48.2 s | 7.7 s |
//! | [`EVELYN_KETELEERIA_BOUGH`] | 15 | 1 591 | 7 | **1 598** | 0.6 s | 0.8 s |
//!
//! **The birch's two row counts are the only figures in this table that are not read off the expansion**, and
//! they are marked with a tilde rather than left to look measured: the **node count** is read off the
//! `-authored_id.txt` sidecar's own red maximum, and the leaf and seat rows follow from the same arithmetic
//! the rule states. The ring count still leaves the birch's plate at under a third of the ceiling.
//!
//! **The two whole-tree crowns, measured by `audit/frame_stats.py --crown`:**
//!
//! | scene | coverage | interior share (bar 0.25) | interior over lit p90 (bar 0.80) |
//! | --- | --- | --- | --- |
//! | [`SOUTHWEST_BIRCH`] | 0.367 | **0.550** | **0.719** |
//! | [`EVELYN_KETELEERIA`] | 0.466 | **0.757** | **0.677** |
//!
//! **And the crown's own outline, measured by `audit/silhouette.py` on the whole-tree plate's identity mask**
//! (the number H1's ten-sheet actually turns on, and `audit/sheet.py` normalises every species to one height
//! before the reader sees it, so this ratio is what the reader compares):
//!
//! | species | width ÷ height of its cut-out, before | after |
//! | --- | --- | --- |
//! | [`SOUTHWEST_BIRCH`] | 0.44 — the spruce's own number (0.29) and the fir's (0.39) | **0.58** |
//! | [`EVELYN_KETELEERIA`] | 0.70 | 0.70 |
//!
//! # The droop, and why it is a law and not a field
//!
//! The birch's whole silhouette is its droop, and [`birch_bough`] and [`birch_branchlet`] draw it in
//! their own four knots: a heading that **turns past a quarter turn** by the third knot, so the outer
//! half of every limb runs below horizontal and its tip hangs a third of a turn under it.
//!
//! # Invariants
//!
//! - **`Field::gravity` is not used on either species, and the reason is a number rather than a taste.**
//!   A [`vegetation::field::Field`] applies to **every** node of a plant, and its `amount` is a fraction
//!   of each spine's own length. The amount that hangs a birch branchlet of 1.05 m — say `0.8` — would push
//!   the tip of the 14 m bole **eleven metres** along gravity, because the bole is in the same list. The
//!   droop is therefore a law over [`vegetation::species::Joint::station`] inside the two shape functions
//!   that grow limbs, and the bole's own four knots stay plumb to within 260 mm over fourteen metres.
//! - **`Decorators::sprig` is used once, and it is the cone.** §4.15's *"a decorator template may be a
//!   sprig"* is the machinery a *Keteleeria* exists to exercise, because its seed scales **recurve outward
//!   at their tips** and one sweep cannot say a recurved plate.

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

/// The name the birch's whole-tree plate is built and rendered under.
pub const SOUTHWEST_BIRCH: &str = "southwest_birch";

/// The name the birch's branchlet plate is built and rendered under: one limb at arm's length, where the
/// doubly serrate margin and the hanging catkins are countable against `05-leaf-and-male-catkins.jpg`.
pub const SOUTHWEST_BIRCH_BOUGH: &str = "southwest_birch_bough";

/// The name the birch's base plate is built and rendered under: the foot of the bole, which is the frame
/// H4 reads the flare and the lobed outline in.
pub const SOUTHWEST_BIRCH_BASE: &str = "southwest_birch_base";

/// The name the Keteleeria's whole-tree plate is built and rendered under.
pub const EVELYN_KETELEERIA: &str = "evelyn_keteleeria";

/// The name the Keteleeria's crown-branch plate is built and rendered under.
pub const EVELYN_KETELEERIA_BOUGH: &str = "evelyn_keteleeria_bough";

/// The name the Keteleeria's base plate is built and rendered under.
pub const EVELYN_KETELEERIA_BASE: &str = "evelyn_keteleeria_base";

/// The seed the birch's three plates are grown from.
const BIRCH_SEED: u64 = 63;

/// The seed the Keteleeria's three plates are grown from.
const KETELEERIA_SEED: u64 = 71;

/// The golden angle as a fraction of a turn, which is the step between two stations that are not a rank.
const GOLDEN: f32 = 0.381_966_01;

/// Half a turn: the phyllotaxy that puts one needle on the far side of a shoot from the last, and is the
/// whole of *Keteleeria*'s *"排列成两列"*.
const OPPOSITE: f32 = 0.5;

// ---------------------------------------------------------------------------
// 西南桦 Betula alnoides
// ---------------------------------------------------------------------------

/// The birch's leaf, at half its drawn width: the note's `2.5–5.5 cm` across, read at its upper middle.
const BIRCH_LEAF_HALF_WIDTH: f32 = 0.028;

/// The birch's leaf at its drawn length, `110 mm`: the note's `4–12 cm`, read near the top of its range.
///
/// # Invariants
///
/// **The top of the range, and the reason is the sampling tax read the other way.** A crown's only area is
/// its leaves' area, so a crown is drawn with as few rows as will cover it and each row as large as the note
/// allows. At the whole-tree plate's 9.8 mm a pixel a 110 mm leaf is **11 px long and 5.7 px wide**, and a
/// leaf at the note's 40 mm floor would be **3.4 px** — a third of a plate pixel a leaf-row, and a crown
/// with no mass. **The leaf is drawn large because the note says leaves are large, not because the
/// rasteriser is hard to please.**
const BIRCH_LEAF_LENGTH: f32 = 0.110;

/// The petiole at `30 mm`: the note's `1.5–3(–4) cm` at its upper end, and the reason a birch leaf is a
/// **sprig of two blades** rather than one — the stalk is a blade here because a template is made of
/// blades and not of rods, and it costs one row inside the same instanced mesh as the leaf it carries.
const BIRCH_PETIOLE_LENGTH: f32 = 0.030;

/// Where the birch's crown begins on its own bole: `5.6 m` of a `14 m` tree, the note's `4–7 m` to first
/// branch, and the number every crown law in this species is measured inside.
///
/// **It is 0.40 and not the 0.355 of the first plate, and the branchlets' length is why.** A branchlet of
/// 1.7 m hanging off a bough at `t = 0.40` reaches `4.5 m`, which is the note's own *"the crown occupies the
/// upper two-thirds"* read on a fourteen-metre tree — `14 − 9.3 = 4.7 m`. The first plate's band put the
/// lowest foliage near three metres and the crown hung into the lower third, which is a different tree.
const BIRCH_CROWN_BASE: f32 = 0.40;

/// Which birch leaf a plate draws.
///
/// # Invariants
///
/// **The two differ in one width law and in nothing else.** At [`SOUTHWEST_BIRCH`]'s 9.8 mm a pixel the
/// note's six-millimetre serration tooth is **half a pixel**, so drawing thirteen of them buys four times
/// the rings and shows nothing; at [`SOUTHWEST_BIRCH_BOUGH`]'s 1.24 mm a pixel the same tooth is **five
/// pixels** and the doubly serrate margin of `03-branch-with-catkins.jpg` is the first thing a reader of
/// that photograph looks for. A comb of teeth on the blade's own width law is how a second-order margin
/// is drawn without a second-order node, and it is the fern's argument again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BirchFoliage {
    /// The leaf as a smooth ovate-lanceolate blade, for the whole-tree and base plates.
    Leaf,
    /// The same blade with thirteen double-serrate teeth in its width law, for the branchlet plate.
    Toothed,
}

impl BirchFoliage {
    /// The leaf this foliage instances.
    fn template(self) -> LeafTemplate {
        birch_leaf(self)
    }
}

/// The birch's rule: one clean bole, spreading boughs that tip over, and branchlets that hang.
///
/// # The numbers and where they come from
///
/// [`betula_alnoides.md`](../../../.agents/investigate/trees/betula_alnoides.md)'s modelling table, with
/// every **ESTIMATE** in it marked. The tree is drawn at **fourteen metres**, the flora's own ceiling,
/// so the note's crown proportions are kept exactly: first branch at `5.0 m`, crown `9 m` deep and
/// `5.5 m` across.
///
/// | level | count | where | what |
/// | --- | --- | --- | --- |
/// | **0** | **1** | — | the **bole**, clean to 5 m |
/// | **1** | **30–36** | `0.40`–`0.93` of the bole | the **boughs**, `3.3 m` at the crown's foot falling to `1.6 m` at the leader's tip |
/// | **2** | **34–40** | `0.18`–`1.0` of a bough | the **branchlets**, `1.2`–`1.8 m`, the pendulous shoots |
///
/// **`cluster = 1` on both generations is the species' first number and it is a contrast.** A conifer's
/// tier is a whorl and a kapok's limb is a pseudo-whorl, both `cluster > 1`; the note's *Betula* branches
/// are **alternate and spirally arranged** (*"Estimate: alternate and spirally arranged, as is typical of
/// Betula"*), and a whorl of three would draw a different tree. The golden angle between stations is the
/// whole of the arrangement.
///
/// **The boughs' band starts at `0.40`**, which is the note's `4–7 m` to first branch on a fourteen-metre
/// tree, and it is the "crown raised clear of the ground" of proposal §4.7 done with a band rather than a
/// prune: a birch's lower branches **persist** (*"Estimate: the reference habit photograph shows branches
/// beginning low on a 16 m stem"*), so pruning them would be a different species, and this one does not.
///
/// **The branchlets are longer than the note's twigs and the module says so.** The note gives
/// `0.3–0.8 m` for a twig carrying `6–14` leaves; the node drawn here is the **branchlet** that a twig
/// hangs from, and the leaves are the note's own size on it. The same substitution the pine and the spruce
/// make and declare.
fn birch_rule() -> Rule {
    Rule {
        trunk: birch_bole,
        generations: vec![
            Generation {
                count: (30, 36),
                // **The band's top is `0.93` and not `1.0`, and it is the crown's HEIGHT that H1 moved.** A
                // branchlet on a bough at the very top of the leader leaves that bough's own heading, which
                // is already twelve to twenty-nine degrees above horizontal, and stands up as far as it
                // reaches out — so a bough at `1.0` carries foliage two and a half metres **above** the
                // leader's own tip, the whole-tree plate's top row was foliage, and the crown ran a metre
                // clear of its own frame. **A birch's leader is the highest thing on it**, and `0.93` puts
                // the topmost boughs at thirteen metres on a fourteen-metre tree and lets the leader's own
                // leafing be the crown's crown.
                station: (BIRCH_CROWN_BASE, 0.93),
                slot: (0.0, GOLDEN),
                // **A tenth of the band is the jitter, and it is what breaks the tiers.** Thirty-three boughs
                // over the crown's 7.4 m of leader is 0.22 m between them, and `±0.06` of station is `±0.4 m`
                // of leader either side of that — so the stations overlap and the crown is an irregular mass.
                // The first plate stood at `0.06`, which is `±0.3 m` on the same band and drew **six visible
                // whorls**: the *cauliflower* defect, and the reason a conifer-looking birch is a birch that
                // has lost its signature.
                jitter: (0.10, 0.26),
                // A bough's foot is inside the bole, and the sink is a little over its own base radius.
                sink: 0.035,
                cluster: 1,
                shape: birch_bough,
            },
            Generation {
                // **Thirty-four to forty, and it was twenty-six to thirty-two.** A skirt is a *fringe* and a
                // fringe is a count of strands across the bough's own width: twenty-nine a bough left a metre
                // of bare limb between each pair, which the silhouette reads as a ladder rather than as a
                // curtain. The note's twig count of `6–14` is per twig and is not this number.
                count: (34, 40),
                station: (0.18, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.12, 0.26),
                sink: 0.010,
                cluster: 1,
                shape: birch_branchlet,
            },
        ],
        // **No field at all**, and the module's own first invariant says why: gravity's amount is a
        // fraction of every spine's length, so the amount that hangs a 0.7 m branchlet would push the
        // bole's tip eleven metres. The droop is [`birch_bough`]'s and [`birch_branchlet`]'s own law.
        fields: Vec::new(),
    }
}

/// The birch's bole: **fourteen metres**, near-plumb with the slight lean `01-habit.jpg` shows on a tree
/// standing in the open.
fn birch_bole() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.00, 0.00, 0.00], [0.055, 1.00, 0.02]),
            ([0.09, 3.60, 0.06], [0.045, 1.00, -0.02]),
            ([0.16, 7.80, -0.04], [0.030, 1.00, 0.01]),
            ([0.22, 11.40, 0.05], [0.018, 1.00, 0.00]),
            ([0.26, 14.00, 0.02], [0.010, 1.00, 0.00]),
        ]),
        // **A flare, a shoulder and a long shallow middle — not a cone.** `0.205` at the ground against the
        // note's ESTIMATE of a `0.14–0.22 m` base radius; `0.146` at `t = 0.093`, which is 1.30 m on a
        // fourteen-metre tree and is a **29 cm d.b.h.** against the note's `0.25–0.40 m`; and `0.126` at
        // 3.1 m, so the trunk loses a third of its girth over its first two metres and then a half again
        // over the next eleven. That is the *taper that is not a cone* H4 asks for, and the first two knots
        // are the flare that keeps the tree from being a *floating tree*.
        taper(&[
            (0.000, 0.205),
            (0.012, 0.178),
            (0.030, 0.162),
            (0.060, 0.152),
            (0.093, 0.146),
            (0.220, 0.126),
            (0.420, 0.100),
            (0.640, 0.072),
            (0.830, 0.046),
            (1.000, 0.013),
        ]),
    )
}

/// The birch's section: **sixteen sides of unequal radius**, which is the wavy, folded column both bark
/// photographs show.
///
/// # Invariants
///
/// **A birch's bole is lobed, not fluted and not round.** `01-habit.jpg` shows two trunks whose outlines
/// fold in and out over their whole height, and `02-bark-peeling.jpg` shows the same column from a metre
/// away with the papery sheets lifting off it in horizontal strips and curling away at the edges — which is
/// what leaves a lobed outline behind. **Five broad swells of ten and a half per cent and three of four and
/// a half** is that at a size the section can carry, and the two earlier plates bracket the answer: **eight
/// per cent was invisible** (a thirty-two-millimetre swell on a 400 mm trunk, which the eye reads as a pole)
/// and **thirteen per cent of seven lobes read as vertical flutes** (a fifty-two-millimetre swell with a
/// hard terminator on each, which is a column of pipes and not a birch). Ten and a half per cent over five
/// lobes is a seventy-two-degree period, and that shades as a fold.
///
/// **Sixteen sides is above the crease threshold's floor and no lower is worth having.** The wood's normals
/// crease above `0.61 rad` and sixteen points round a loop is `0.39 rad` a step, so the lobes shade as
/// smooth swells — a *polygonal silhouette* is the defect the bar names and a sixteen-sided ring at a
/// fourteen-metre trunk's whole-tree distance is four hundred pixels across with a facet every twenty-five.
///
/// **The section is scaled by the taper**, so the lobes shrink with the bole, which is right: a fold in a
/// birch's bark is the same object wherever it is on the tree.
fn birch_bark() -> shape::Section {
    let sides = 16;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let lobe = 1.0 + 0.105 * (theta * 5.0 + 0.3).sin() + 0.045 * (theta * 3.0 + 1.9).sin();
            [theta.cos() * lobe, theta.sin() * lobe]
        })
        .collect();
    shape::Section::outline(&points).expect("a lobed column is a section")
}

/// A first-order bough: it leaves the leader at the note's ESTIMATE of **55–90°** and **tips over**, so
/// the outer third of every bough on the tree runs below horizontal.
///
/// # The four angles, and what they are
///
/// [`bend`]'s angles are measured from the parent's tangent, which on the leader is straight up, so
/// `1.16` rad is **66° from the trunk axis** and a quarter turn is straight out. The four are
/// `d`, `d + 0.28`, `d + 0.50`, `d + 0.82`: it leaves steeply, flattens, **passes horizontal at two
/// thirds along**, and hangs a quarter of a turn below it at the tip. That third knot is the whole
/// signature — the note's *"the twigs hang downward under their own weight, giving a distinctly pendulous
/// crown"* — and a bough law that stopped at horizontal would draw an ordinary broadleaf.
///
/// **The length is a law over the station**, which is what [`Joint::station`] exists for and what the
/// note's `4–7 m` crown across is made of: `3.30 m` at the crown's foot and `1.55 m` at the leader's tip,
/// which with the branchlets hanging off the outer part makes a crown **7.5 m** across. **It was `2.55 m`,
/// and H1 is the reason it moved.** A crown 5.5 m across on a fourteen-metre tree measured **0.44** of its
/// own height, which is the spruce's number and not a broadleaf's; the width this species is read by is
/// its crown's.
///
/// **The 7.5 m is a fifth over the note's `4–7 m` and the reason is stated rather than hidden: the note
/// read that range off `01-habit.jpg`, which is a LEAFLESS tree, and a birch's crown in leaf is thirty per
/// cent of pendant branchlets that are not there in February.** The boughs alone reach the note's own 7 m;
/// what stands past them is the fringe, which is the species' signature and the thing H1 asks to be able to
/// see. Measured on the plate: **0.44 → 0.58** of the cut-out's height.
fn birch_bough(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let profile = crown_profile(joint.station, BIRCH_CROWN_BASE);
    let length = 3.30 * (1.0 - 0.53 * profile.powf(0.80)) * (0.80 + 0.40 * rng.unit());
    let departure = 1.16 + 0.30 * rng.unit() - 0.10 * profile;
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.28,
                departure + 0.50,
                departure + 0.82,
            ],
        ),
        taper(&[
            (0.00, radius * 0.42),
            (0.30, radius * 0.26),
            (0.72, radius * 0.12),
            (1.00, 0.0045),
        ]),
    )
}

/// A branchlet: the pendulous shoot the leaves and the catkins hang on.
///
/// # Invariants
///
/// **Its departure is read from its parent, and its parent's tangent is already down.** A branchlet leaves
/// its bough at `0.85`–`1.30 rad` from the bough's own heading, and the bough's heading at that station is
/// already between seventy and a hundred degrees from the trunk's — so in the world the branchlet leaves
/// at **thirty to sixty degrees below horizontal** and its own four knots take it to
/// **fifty degrees below horizontal at the tip**. **The outer half of every branchlet on this tree hangs
/// below the horizontal**, which is the number the note's fifth recognition fact is and the number a reader
/// of `01-habit.jpg` checks before anything else.
///
/// `1.38`–`2.06 m` is the note's `0.3–0.8 m` twig at the top of its range more than doubled, and the module
/// says so rather than hiding it: the node here is a **branchlet**, and the note's twig is a shoot of it.
/// It is the one number in this species that leaves the note's range outright, and it leaves it on purpose —
/// see the H1 note in the body below and in [`birch_bough`].
fn birch_branchlet(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    // **1.72 m at its middle, and it is an extension of the note's `0.3–0.8 m` twig that the module
    // declares.** The note's twig is a shoot; the node drawn here is the **branchlet** that shoot hangs
    // from, and a branchlet is the unit the *weeping* is drawn in — thirty of them a bough is a fringe
    // hanging a metre below the limb, and a bough with eight 40 cm twigs is a fir's bough. The first plate
    // drew 0.50–0.90 m and the crown read as a conifer; this is the change that makes it a birch.
    //
    // **H1 moved it twice, 1.05 m → 1.50 m → 1.72 m, and what it moved is the WIDTH of the droop.** The
    // ten-sheet read this crown as *"a narrow conifer strip"* at a width-to-height of **0.44**, against the
    // spruce's 0.29 and the fir's 0.39 — a metre of pendant branchlet on a limb that reaches three metres
    // out hangs its fringe almost straight down, so the skirt's outer edge fell a hand's width beyond the
    // limb and the crown was a column. The droop itself was never the defect and the departure law below is
    // unchanged; **what was missing was how far out that droop is carried**, so the length is the fix and
    // the hang is still the signature. Measured on the plate: **0.44 → 0.58**.
    let length = 1.72 * (0.80 + 0.40 * rng.unit());
    let departure = 0.85 + 0.45 * rng.unit();
    let radius = joint.parent_radius.max(1.0e-5);
    (
        bend(
            length,
            [
                departure,
                departure + 0.30,
                departure + 0.62,
                departure + 0.98,
            ],
        ),
        // **A third of the bough's own radius at the foot.** A birch branchlet is `6–10 mm` through and the
        // bough it leaves is `30–45 mm`, so the third is the number the photograph shows and the joint's
        // parent radius is the right one to read: it is the bough's radius *at this branchlet's own
        // attachment*, so a branchlet at a bough's tip is thinner than one at its foot by construction.
        taper(&[(0.00, radius * 0.34), (0.35, radius * 0.20), (1.00, 0.0016)]),
    )
}

/// The birch's leaf: a **petiole and a blade**, which is the whole of [`BirchFoliage`].
///
/// # Invariants
///
/// **A petiole is a blade here because a template is made of blades and not of rods**, and it costs one
/// row inside the same instanced mesh as the leaf it carries — which is the entire reason
/// [`LeafTemplate::sprig`] exists. The note's petiole is `1.5–3(–4) cm` and hairy and gland-dotted, and
/// **its hair and its glands are not drawn**: they are a texture, and this renderer has no channel for
/// one. The stalk is 24 mm of reddish blade and that is what `05-leaf-and-male-catkins.jpg` shows.
fn birch_leaf(foliage: BirchFoliage) -> LeafTemplate {
    let teeth = match foliage {
        BirchFoliage::Leaf => 0usize,
        BirchFoliage::Toothed => 13usize,
    };
    LeafTemplate::sprig(vec![
        (petiole(), Placement::IDENTITY),
        (leaf_blade(teeth), along(BIRCH_PETIOLE_LENGTH)),
    ])
}

/// The birch's petiole: **24 mm long and 2.4 mm wide**, the note's `1.5–3 cm` at its middle.
fn petiole() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.000, 0.0000, 0.0], [0.0080, 0.0004, 0.0]),
            ([0.008, 0.0012, 0.0], [0.0080, 0.0006, 0.0]),
            ([0.016, 0.0028, 0.0], [0.0076, 0.0004, 0.0]),
            ([0.024, 0.0044, 0.0], [0.0020, 0.0002, 0.0]),
        ]),
        taper(&[
            (0.00, 0.0011),
            (0.40, 0.0012),
            (0.80, 0.0011),
            (1.00, 0.0008),
        ]),
        0.45,
        7,
        4,
    )
}

/// The birch's leaf blade, at [`BIRCH_LEAF_LENGTH`] long and [`BIRCH_LEAF_HALF_WIDTH`] half as wide.
///
/// # The shape, and the four words the law carries
///
/// The note gives *"披针形或卵状披针形"* — lanceolate to ovate-lanceolate, **broadest below the middle** — and
/// *"渐尖至尾状渐尖"*, an acuminate to **caudate-acuminate** apex: the blade is drawn out into a long
/// tail-like point, which is a recognition feature at close range and is `0.44` of half-width at `t = 0.88`
/// falling to `0.02` at the tip.
///
/// - **Ovate-lanceolate, broadest below the middle** is the envelope: full width at `0.46`, not at `0.5`.
/// - **Caudate** is the tail of the width law's last two knots.
/// - **Base cuneate to rounded** is the flat start, four millimetres of half-width over the petiole's own
///   last centimetres.
/// - **Ten to thirteen pairs of lateral veins** are a texture and not geometry at any of these three
///   plates' distances — at the branchlet plate a vein is a twentieth of a pixel — and are not drawn.
///
/// **The `0.030` thickness ratio is a birch's**: half a millimetre of lamina over a forty-two-millimetre
/// width, which is the note's *"厚纸质"*, a thick papery blade rather than a thin one.
fn leaf_blade(teeth: usize) -> LeafTemplate {
    let reach = BIRCH_LEAF_LENGTH / 0.090;
    let at = |point: [f32; 3]| [point[0] * reach, point[1] * reach, point[2] * reach];
    let width = match teeth {
        0 => {
            let knots: Vec<(f32, f32)> = [
                (0.00, 0.26),
                (0.10, 0.60),
                (0.30, 0.92),
                (0.46, 1.00),
                (0.62, 0.92),
                (0.76, 0.68),
                (0.88, 0.44),
                (1.00, 0.02),
            ]
            .iter()
            .map(|(at, across)| (*at, BIRCH_LEAF_HALF_WIDTH * across))
            .collect();
            shape::Scalar::new(&knots).expect("a birch leaf's width is a law")
        }
        // **Double serration, and it is two numbers — and the notch is *shallow*, which is the whole
        // correction.** The note's *"不规则重锯齿"* — irregular **double** serration with inward-curving
        // bristle-like teeth — is a **major** tooth every fourth knot and a **minor** tooth between: the
        // majors at full envelope reach, the knot between two majors at 95.5% of it, and the knots either
        // side at 90%. **Three millimetres of notch on a fifty-six-millimetre blade** is what
        // `03-branch-with-catkins.jpg` shows. The first plate's `0.76` and `0.54` put nine millimetres in
        // every notch and the bough plate came out **a fern**: green pinnae on a rod, which is another
        // species' organ at another scale, and the *plastic crown* defect with serrations on it.
        //
        // The note adds that the minor teeth are *"内弯的刺毛状"*, inward-curving and bristle-like, and **a
        // width law cannot make a tooth curve**: it can make a tooth deep or shallow, and shallowness is
        // the honest half of that sentence. The module says so rather than claiming the margin.
        //
        // The envelope is the blade's own outline, so the teeth thin toward the caudate point and thicken
        // below the middle, which is what makes the serration read as a leaf's and not as a saw.
        count => {
            let knots: Vec<(f32, f32)> = (0..=4 * count)
                .map(|step| {
                    let at = step as f32 / (4 * count) as f32;
                    let envelope =
                        (std::f32::consts::PI * at).sin().max(0.0).powf(0.55) * (1.0 - 0.20 * at);
                    let tooth = match step % 4 {
                        0 => 1.000,
                        2 => 0.955,
                        _ => 0.900,
                    };
                    (at, BIRCH_LEAF_HALF_WIDTH * envelope * tooth)
                })
                .collect();
            shape::Scalar::new(&knots).expect("a birch leaf's margin is a law")
        }
    };
    LeafTemplate::blade(
        spine(&[
            (at([0.000, 0.0000, 0.0]), at([0.030, 0.0012, 0.0])),
            (at([0.030, 0.0030, 0.0]), at([0.028, 0.0028, 0.0])),
            (at([0.058, 0.0082, 0.0]), at([0.022, 0.0030, 0.0])),
            (at([0.090, 0.0148, 0.0]), at([0.006, 0.0016, 0.0])),
        ]),
        width,
        0.030,
        if teeth == 0 { 7 } else { 4 * teeth + 1 },
        4,
    )
}

/// The birch's catkin: **75 mm of slender cylinder hanging from a branchlet's tip**, the note's
/// `5–10 cm` infructescence at its middle and its `4–6 mm` across.
///
/// # Invariants
///
/// **A seat expresses no rotation, so the hanging is the template's own shape.** A seat's frame is the
/// node's own surface frame — outward `x`, the node's tangent `y` — and a branchlet's tangent at
/// `station = 0.96` points **down and out**, so a template drawn along `+y` from the origin hangs plumb on
/// every branchlet it lands on whatever side of the shoot it landed. The same bargain the pine's cone and
/// the fern's adventitious root make.
///
/// **It is drawn at the widest of the note's range, `4–6 mm` across, and it is a lobed ring rather than a
/// circle.** A catkin is a close spiral of scales, and a plain circle reads as a wire; ten slots with every
/// other one drawn in at four fifths is the cheapest honest reading of a lobed outline, and it is the same
/// argument the fir's cone makes. **At [`SOUTHWEST_BIRCH`]'s 9.8 mm a pixel this organ is 5.1 mm across
/// and is *dropped by the rasteriser whole*** — the module says so rather than enlarging it, because a
/// catkin drawn at twice its size is a different animal; it is drawn for the branchlet plate, where it is
/// sixty pixels long and five across.
fn birch_catkin() -> Template {
    let sides = 10;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let reach = if index % 2 == 0 { 1.0 } else { 0.80 };
            [theta.cos() * reach, theta.sin() * reach]
        })
        .collect();
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0000, 0.0000, 0.0], [0.0020, 0.0140, 0.0]),
            ([0.0006, 0.0155, 0.0], [0.0004, 0.0220, 0.0]),
            ([0.0004, 0.0400, 0.0], [0.0000, 0.0230, 0.0]),
            ([0.0002, 0.0640, 0.0], [0.0000, 0.0130, 0.0]),
            ([0.0000, 0.0740, 0.0], [0.0000, 0.0020, 0.0]),
        ]),
        shape::Section::outline(&points).expect("a lobed catkin is a section"),
    );
    sweep.scale = taper(&[
        (0.00, 0.0021),
        (0.22, 0.0027),
        (0.62, 0.0026),
        (0.88, 0.0019),
        (1.00, 0.0007),
    ]);
    Template {
        sweep,
        stations: 10,
        around: 10,
    }
}

/// The birch's three leaf fields at `branchlet_depth`: the leader's own shoots, the leaves on the boughs,
/// and the leaves on the branchlets.
///
/// # The density, and the number it is not
///
/// **Twelve leaves to the metre of branchlet** is one leaf every eighty-three millimetres under a blade a
/// hundred and ten millimetres long, which is the *spacing* `05-leaf-and-male-catkins.jpg` shows — a leaf's
/// own length to the next leaf's stalk, and sky between every second one. **The note's own `6–14 leaves
/// per twig` is not the count drawn here and the module says which number it is**: the note's twig is
/// `0.3–0.8 m` and the branchlet drawn is its parent, so a 1.72 m branchlet at the note's *count* would carry
/// twelve and thirteen at the note's *spacing*. The first plate of this species read the count and drew
/// **seventy two a metre** — a leaf every fourteen millimetres, six deep — and the branchlet came out as a
/// **fern pinna**: a green feather on a rod, which is the *bare broom* and a *plastic crown* both. A birch's
/// branchlet is a rod with big leaves on it, and the difference is the number of leaves an arm's length
/// shows. **It was fifteen a metre at the branchlet's `1.05 m` and it is twelve at `1.72 m`**, because the
/// spacing is the note's and the length is H1's, and holding the count across the change would have made the
/// skirt a solid green wall instead of the *airy* crown the note's first recognition fact asks for.
///
/// **The band on every field stops short of both ends**, which is the *dead give* the bar also names: no
/// real shoot is foliated evenly from the wood it grows on to its own tip.
///
/// **The pitch law is the droop, leaf by leaf.** Pitch is radians from the parent's outward direction
/// toward the parent's tip, so `1.15` at a branchlet's foot is a leaf standing out and slightly down, and
/// `2.05` at its tip is a leaf pointing **back down the shoot** — and the branchlet's own tangent is
/// already heading below horizontal, so the leaf at the tip is hanging. That is the third place the
/// signature is written, after [`birch_bough`] and [`birch_branchlet`].
fn birch_leaves(branchlet_depth: usize) -> Vec<LeafField> {
    let mut leader = LeafField::new(0, 0);
    leader.band = (0.45, 0.985);
    leader.density = shape::Scalar::constant(22.0);
    leader.cluster = 1;
    leader.fan = shape::Scalar::constant(0.0);
    leader.phyllotaxy = GOLDEN;
    leader.pitch = shape::Scalar::new(&[(0.00, 1.05), (0.50, 1.34), (1.00, 1.72)])
        .expect("a leaf's pitch is a law");
    leader.twist = shape::Scalar::new(&[(0.00, -0.22), (0.50, 0.05), (1.00, 0.34)])
        .expect("a leaf's twist is a law");
    leader.size = shape::Scalar::new(&[(0.00, 0.70), (0.40, 1.00), (1.00, 0.82)])
        .expect("a leaf's size is a law");
    leader.jitter = 0.26;

    let mut bough = LeafField::new(branchlet_depth.saturating_sub(1), 0);
    // The bough's own outer length, which is where a birch carries its long shoots and where the crown's
    // interior foliage is: H3's interior has to come from leaves that are not on the branchlets.
    bough.band = (0.26, 0.94);
    bough.density = shape::Scalar::constant(22.0);
    bough.cluster = 1;
    bough.fan = shape::Scalar::constant(0.0);
    bough.phyllotaxy = GOLDEN;
    bough.pitch = shape::Scalar::new(&[(0.00, 1.00), (0.50, 1.28), (1.00, 1.78)])
        .expect("a leaf's pitch is a law");
    bough.twist = shape::Scalar::new(&[(0.00, -0.20), (0.50, 0.04), (1.00, 0.30)])
        .expect("a leaf's twist is a law");
    bough.size = shape::Scalar::new(&[(0.00, 0.78), (0.45, 1.00), (1.00, 0.86)])
        .expect("a leaf's size is a law");
    bough.jitter = 0.28;

    let mut branchlet = LeafField::new(branchlet_depth, 0);
    branchlet.band = (0.06, 0.98);
    // **Twelve to the metre, and it fell from fifteen across this pass.** A branchlet is 1.72 m now and the
    // note's own spacing — *a leaf's length to the next leaf's stalk* — is what sets the number, not a
    // density carried across a change of branchlet length: at fifteen a metre a 1.72 m branchlet carried
    // twenty-four leaves of 110 mm, which is a leaf every 67 mm and a skirt with no sky in it. Twelve is a
    // leaf every 83 mm on a longer strand, which is the same spacing the note's photograph shows on a twig
    // twice as long, and it is what holds the crown's H3 coverage where the wider fringe would otherwise
    // push it past the bar.
    branchlet.density = shape::Scalar::constant(12.0);
    branchlet.cluster = 1;
    branchlet.fan = shape::Scalar::constant(0.0);
    // **The golden angle and one leaf to a station**, which is the note's *"alternate and spirally
    // arranged"* read as two numbers. A rank would be a *Keteleeria*; a cluster would be a pine.
    branchlet.phyllotaxy = GOLDEN;
    branchlet.pitch = shape::Scalar::new(&[(0.00, 1.02), (0.50, 1.32), (1.00, 1.74)])
        .expect("a leaf's pitch is a law");
    branchlet.twist = shape::Scalar::new(&[(0.00, -0.34), (0.50, 0.06), (1.00, 0.44)])
        .expect("a leaf's twist is a law");
    branchlet.size = shape::Scalar::new(&[(0.00, 0.72), (0.35, 1.00), (1.00, 0.86)])
        .expect("a leaf's size is a law");
    // **The jitter is high, and it is the *plastic crown* defect's own number.** A crown whose every leaf is
    // the same size at the same pitch and the same twist has a moiré instead of a texture, and 0.28 puts a
    // leaf anywhere between seven and eleven centimetres long.
    branchlet.jitter = 0.28;
    if branchlet_depth >= 2 {
        vec![leader, bough, branchlet]
    } else {
        vec![bough, branchlet]
    }
}

/// The birch: its tree, its foliage at the asked scale, its three fields and its catkin.
pub fn southwest_birch(seed: u64, foliage: BirchFoliage) -> Species {
    let mut species = Species {
        plant: grow(&birch_rule(), seed),
        templates: vec![foliage.template()],
        fields: birch_leaves(2),
        decorators: vec![birch_catkin()],
    };
    // The bole is dressed after the growth because a section is a node's own and `grow` authors circles:
    // the lobes are the trunk's, not the rule's.
    if let Ok(bole) = species.plant.get_mut(&Handle::root()) {
        bole.set_section(birch_bark());
    }
    species
}

/// The branchlet plate's species: one bough as the trunk, its branchlets as the only generation, and the
/// toothed leaf.
///
/// # Invariants
///
/// **The same rule one level down**, as every other species' bough plate in this gallery is: the root is a
/// bough of the shape the whole tree's boughs are, the generation is the whole tree's branchlets, and the
/// three fields come from [`birch_leaves`] with the depths this plate's own tree has — so a leaf that reads
/// here is a leaf the crown is made of.
pub fn southwest_birch_bough(seed: u64) -> Species {
    let rule = Rule {
        trunk: || {
            (
                bend(2.30, [1.16, 1.44, 1.66, 1.98]),
                taper(&[(0.00, 0.045), (0.35, 0.028), (0.75, 0.013), (1.00, 0.0045)]),
            )
        },
        generations: vec![Generation {
            count: (16, 20),
            station: (0.16, 1.0),
            slot: (0.0, GOLDEN),
            jitter: (0.14, 0.26),
            sink: 0.008,
            cluster: 1,
            shape: birch_branchlet,
        }],
        fields: Vec::new(),
    };
    Species {
        plant: grow(&rule, seed),
        templates: vec![BirchFoliage::Toothed.template()],
        fields: birch_leaves(1),
        decorators: vec![birch_catkin()],
    }
}

/// The catkins a birch's branchlets carry: **one or two at a branchlet's tip**, and only on the branchlets
/// whose own attachment on the bough lies in its **outer four fifths**.
///
/// # Invariants
///
/// - **`scatter_in`, and the band is what it is for.** A catkin is terminal — it hangs at the tip of a
///   shoot and nowhere else — and the outer four fifths of a bough is where this species' shoots are
///   longest. [`scatter`] alone would have seated them on the branchlets at the bough's own foot too, where
///   `03-branch-with-catkins.jpg` shows none.
/// - **The station is `0.96`**, which is a catkin at the shoot's apex rather than at its middle, and the
///   count is a **range the seed decides** so the crown is a fringe of hanging threads rather than a
///   uniform pelt.
fn birch_catkins(species: &Species, branchlet_depth: usize) -> Vec<Seat> {
    scatter_in(
        &species.plant,
        branchlet_depth,
        0,
        0.96,
        (1, 2),
        (0.36, 1.0),
    )
}

/// The birch's surfaces: a **red-brown** bole, a bright mid-green leaf and a tan catkin.
///
/// # Invariants
///
/// **Three tints and no shared one**, and the *felt-pen tree* defect is what they prevent. The bole is
/// **红褐色 — reddish-brown** at `0.118, 0.070, 0.048`, which is the note's own word and the colour
/// `02-bark-peeling.jpg` settles: a cinnamon underlayer showing through a pale papery outer bark. **It is
/// not the chalk white of *Betula utilis***, and a plate drawn from that species' photograph would be a
/// different tree. The leaf is a **bright mid-green**, read pale against the dark evergreen broadleaves the
/// note says this birch stands among in spring, and the catkin is a **warm tan**, which is `03`'s own
/// ripening colour.
fn birch_palette() -> Palette {
    Palette {
        wood: Skin::new([0.118, 0.070, 0.048], 0.90),
        leaves: vec![Skin::new([0.108, 0.178, 0.064], 0.68)],
        seats: vec![Skin::new([0.190, 0.148, 0.070], 0.84)],
    }
}

/// A warm hillslope at fifteen hundred metres: a low raking key off the sunward side, a cool sky fill from
/// the gully, and damp forest litter underfoot.
///
/// # Invariants
///
/// - **A light's reach is its cutoff, and the cutoff must clear the floor's own diagonal**, or the light is
///   culled per cluster box and the box boundary draws itself across the ground as a straight edge. The
///   floor reaches 400 m and its diagonal is 566 m, so the reach is **1200 m**; the *distance* — where the
///   irradiance is stated — is the key's own length from the origin, **30.5 m**.
/// - **The key is low and off the lens's axis.** A frontal key lights exactly what the lens sees, and a
///   crown then has no dark interior for H3 to measure; a morning sun raking across a Yunnan hillslope at
///   fifteen hundred metres is the honest hour for a plate whose subject is a curtain of hanging leaves.
fn birch_lighting() -> Lighting {
    Lighting {
        // **The key stands on the camera's side and to its left, not behind the tree.** The first plate put
        // it at `[24, 15, −12]` against a camera at `[22, 6.5, 23]`, which is *behind the tree from the
        // lens*: every frame it produced was a silhouette of a crown against its own sky. **A raking key
        // means a key with a direction across the subject, not a key behind it** — a morning sun at
        // forty-five degrees to the lens's left is what rakes, and it is what the second plate uses.
        key_position: [-19.0, 17.0, 20.0],
        key_colour: [1.0, 0.91, 0.76],
        key_irradiance: 4.6,
        key_distance: 32.3,
        key_reach: 1200.0,
        // **The page is sized to the tree and not to the floor.** A fourteen-metre birch with a crown
        // five and a half metres across wants an eight-metre page: one texel of a 1024 page over a sixteen
        // metre plane is sixteen millimetres, and a leaf's casters are centimetres away, so the leaf
        // materials' own bias scale takes it the rest of the way down.
        key_extent: 8.0,
        fill_position: [16.0, 8.0, -14.0],
        fill_colour: [0.46, 0.58, 0.72],
        fill_irradiance: 0.45,
        fill_distance: 23.0,
        // **The floor is a warm mid brown and not a near-black**, because the first plate's `0.195` under a
        // key of 3.4 read as a black field under a black tree and the frame had no ground to stand on.
        floor: Skin::new([0.255, 0.215, 0.150], 0.95),
        sky: Atmosphere::new()
            .zenith([0.08, 0.18, 0.42])
            .horizon([0.56, 0.60, 0.64])
            .ground([0.30, 0.28, 0.22])
            .density(0.0012)
            .scale_height(1800.0),
        // **The sky is a third of the key, and that ratio is H3's number.** A crown lit mostly by a sky has
        // an interior exactly as bright as its rim; a crown lit by a key with a dim sky above it has a dark
        // inside, and the interior share is what the probe measures.
        sky_radiance: 0.32,
        // Warm humid air on a hillslope at fifteen hundred metres: about half a garden's.
        haze: 0.007,
    }
}

/// The base plate's own light: a low key on the lens's side, because the tree plate's raking key stands
/// behind the bole from here and a bark plate lit from behind is a silhouette.
///
/// # Invariants
///
/// **Fifteen degrees up and on the lens's side, and a page fitted to the bole rather than to the crown.**
/// Any higher and the tree's own crown shadows the foot this plate exists to show, which is the trap the
/// base plate of every species in this proposal must dodge; and a four-metre page is the smallest that
/// still holds a 40 cm bole with its flare, which is what this plate is a picture of.
fn birch_base_lighting() -> Lighting {
    Lighting {
        // **On the lens's side and beyond the bole, so the flare's near face is the lit one.**
        key_position: [3.4, 2.6, 3.8],
        key_colour: [1.0, 0.93, 0.82],
        key_irradiance: 3.4,
        key_distance: 5.7,
        key_reach: 40.0,
        key_extent: 2.5,
        fill_position: [-3.0, 1.6, -2.4],
        fill_colour: [0.46, 0.54, 0.68],
        fill_irradiance: 0.35,
        fill_distance: 4.1,
        floor: Skin::new([0.255, 0.215, 0.150], 0.95),
        sky: Atmosphere::new()
            .zenith([0.10, 0.20, 0.44])
            .horizon([0.52, 0.58, 0.66])
            .ground([0.30, 0.28, 0.23])
            .density(0.0012)
            .scale_height(1800.0),
        sky_radiance: 0.32,
        haze: 0.007,
    }
}

/// The whole-tree frame: **31.8 m** off a fourteen-metre tree, the camera at half its height.
///
/// # Invariants
///
/// **`2 · 27.0 · tan(0.30) / 1700` is 9.8 mm**, and that number is the one this plate's blade was chosen
/// on: the note's leaf is `40–120 mm` and so is **4 to twelve pixels** here — above the sampling tax,
/// which is why this species needs no card at all and draws the plain blade.
///
/// **The eye is at a third of the tree's height and not above the crown.** The whole of H1 here is that the
/// shape is read as a curtain of hanging branchlets on a clean stem, and a plan seen from a roof is not
/// that shape. **Seventeen metres of vertical coverage on a fourteen-metre tree is eighty-two per cent of
/// the frame's height** — the first plate's twenty metres left a fifth of the frame empty ground below the
/// bole — and the `7.5 m` crown is **758 pixels** wide and **1 305** tall, with the topmost foliage a
/// hundred rows clear of the top row. **It was not: the widened fringe pushed the crown's top past the
/// frame**, because a branchlet on a bough at `1.0` leaves that bough's own heading, which points twelve to
/// twenty-nine degrees above horizontal, and stands up as far as it reaches out. The band now stops at
/// `0.93` and the plate has its margin; the frame itself did not move.
const BIRCH_FRAME: Framing = Framing {
    width: 1400,
    height: 1700,
    eye: [18.5, 5.0, 19.5],
    target: [0.0, 8.0, 0.0],
    field_of_view: 0.60,
    near: 0.2,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The branchlet plate's frame: **1.70 m** off one bough's outer half, side on, looking slightly down the
/// line of the hang.
///
/// # Invariants
///
/// **`2 · 2.16 · tan(0.35) / 1000` is 1.58 mm**, and this is the plate the comb exists for: a nine-millimetre
/// serration tooth is **5.7 pixels** here and was **0.4 pixels** at the whole-tree plate. The frame is
/// **wider than it is tall** because the subject is a bough hanging down and the frame is a portrait of a
/// curtain otherwise.
///
/// **The camera looks twenty degrees UP at the hanging branchlets, against the sky**, and that is the whole
/// of the framing's correction: the first plate aimed *down* at `y = 7.35` from `y = 7.95` and put the
/// forest floor across the lower half of the frame, which is a picture of a branch lying in a field. A
/// weeping crown is read from below.
///
/// **The bough stands at seven and a half metres**, a crown's own height and not on the ground: a branch
/// plate whose branch lies in the dirt reads as a fallen limb, and a birch's branchlets do hang to within
/// four and a half metres of the ground. **The eye carries the planting's own height**, because a
/// `Framing` is in world coordinates and a `Placement` moves the plant in the world.
const BIRCH_BOUGH_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [2.55, 7.05, 1.70],
    target: [1.10, 7.85, 0.0],
    field_of_view: 0.70,
    near: 0.03,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The base plate's frame: **2.0 m** from the bole at a person's reach, aimed at **0.95 m**, which is the
/// frame H4 reads the flare and the lobed outline in.
///
/// # Invariants
///
/// **Close, and aimed low.** H4 asks for the flare and the bark's form, both of which live in the first
/// two metres, and a bark plate with the flare out of the bottom of its own frame is a texture swatch.
/// **`2 · 2.39 · tan(0.35) / 1400` is 1.25 mm**, so the section's thirteen-per-cent lobes on a 410 mm bole
/// are **twenty-one pixels** of swell and the bole is **328 pixels** of a 1200-pixel frame. **The first plate
/// stood at 3.2 m and a 0.80 lens**, which made the bole 226 pixels and the lobes eight, and the frame came
/// out a dark brown pole with a chamfer at the bottom of it.
const BIRCH_BASE_FRAME: Framing = Framing {
    width: 1200,
    height: 1400,
    eye: [1.72, 1.15, 1.62],
    target: [0.0, 0.78, 0.0],
    field_of_view: 0.70,
    near: 0.05,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The birch's expansion dial for the two whole-tree plates.
const BIRCH_DETAIL: Detail = Detail {
    stations: 6,
    around: 12,
    spacing: 0.5,
};

/// The birch's expansion dial for the base plate, and the one number in this species that is not about the
/// species.
///
/// # Invariants
///
/// **`spacing = 80 mm` and `around = 24`, and both are the bole's own surface.** The crate's half-metre
/// default puts thirty rings on a fourteen-metre bole, and a lobe of ten and a half per cent on a 400 mm
/// trunk is a
/// a fifty-two-millimetre swell — one sample per two of them, so the column comes out as a smooth pipe.
/// Eighty millimetres is three rings to a lobe. **Twenty-four slots is the crease threshold's other side:
/// at fourteen the ring sampled the sixteen-point section at fourteen places and the bole's terminator came
/// out as a hard vertical line across three hundred pixels of bark** — the *polygonal silhouette* defect,
/// drawn by the plate that exists to show the wood. **The cost is stated**: the bole takes 175 rings and
/// each branchlet twelve, which is **200 000 wood triangles** against 45 000 at the default, and the module
/// pays it on one plate rather than on the two whole-tree plates where the bole is nine pixels wide.
const BIRCH_BASE_DETAIL: Detail = Detail {
    stations: 8,
    around: 24,
    spacing: 0.08,
};

/// Builds the birch's whole-tree plate: one tree, its own light and ground, the plain leaf, and its catkins.
///
/// **Rows.** Thirty-three boughs over the crown's 7.4 m of leader and thirty-seven branchlets a bough is
/// **1 164 nodes** inside the planting's stride of 65 536 — read off the `-authored_id.txt` sidecar's red
/// maximum, which is the one row count in this module that can be *measured* rather than derived. The
/// three fields carry about **25 000 leaf rows** and the catkin scatter about **1 100**, so the plate is at
/// roughly **26 100 rows**, under a third of the ninety-thousand ceiling of §4.13; it compiles in eleven
/// seconds and rasterises in four.
pub fn southwest_birch_picture() -> Value {
    let species = southwest_birch(BIRCH_SEED, BirchFoliage::Leaf);
    let planting = Planting::new(species.clone())
        .with_seats(birch_catkins(&species, 2))
        .with_palette(birch_palette())
        .with_detail(BIRCH_DETAIL);
    plant_scene_lit(
        &[planting],
        &BIRCH_FRAME,
        SOUTHWEST_BIRCH,
        &birch_lighting(),
        &[],
    )
}

/// Builds the birch's branchlet plate: one bough at arm's length, in the toothed leaf.
pub fn southwest_birch_bough_picture() -> Value {
    let species = southwest_birch_bough(BIRCH_SEED);
    let planting = Planting::new(species.clone())
        .at(Placement {
            origin: [0.0, 7.50, 0.0],
            ..Placement::IDENTITY
        })
        .with_seats(birch_catkins(&species, 1))
        .with_palette(birch_palette())
        .with_detail(BIRCH_DETAIL);
    plant_scene_lit(
        &[planting],
        &BIRCH_BOUGH_FRAME,
        SOUTHWEST_BIRCH_BOUGH,
        &birch_lighting(),
        &[],
    )
}

/// Builds the birch's base plate: the foot of the bole at a person's reach, which is the frame H4 reads the
/// flare and the lobed outline in. The foliage is the plain leaf, because the subject is the bark and
/// twenty thousand toothed leaves around a trunk plate is fill rate spent on pixels the frame crops away.
pub fn southwest_birch_base_picture() -> Value {
    let species = southwest_birch(BIRCH_SEED, BirchFoliage::Leaf);
    let planting = Planting::new(species.clone())
        .with_seats(birch_catkins(&species, 2))
        .with_palette(birch_palette())
        .with_detail(BIRCH_BASE_DETAIL);
    plant_scene_lit(
        &[planting],
        &BIRCH_BASE_FRAME,
        SOUTHWEST_BIRCH_BASE,
        &birch_base_lighting(),
        &[],
    )
}

/// Resolves a whole-tree plate's `authored_id` into the handle that names the node.
pub fn southwest_birch_handle_of(id: u32) -> Option<String> {
    node_ids(&southwest_birch(BIRCH_SEED, BirchFoliage::Leaf).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves the base plate's `authored_id`: **the same plant**, so the same walk is the table, and a second
/// table would answer with the wrong node the moment the foliage changed.
pub fn southwest_birch_base_handle_of(id: u32) -> Option<String> {
    southwest_birch_handle_of(id)
}

/// Resolves the branchlet plate's `authored_id`: the bough is its own plant, so its walk is its own table
/// and the whole tree's would answer with the wrong node.
pub fn southwest_birch_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&southwest_birch_bough(BIRCH_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

// ---------------------------------------------------------------------------
// 滇油杉 Keteleeria evelyniana
// ---------------------------------------------------------------------------

/// The Keteleeria's needle at half its drawn width: the note's `2–3(3.5) mm` across, at its middle.
const KETELEERIA_NEEDLE_HALF_WIDTH: f32 = 0.00125;

/// The Keteleeria's needle at its drawn length, `45 mm`: the note's `2–6.5 cm` at its middle. It is **not**
/// enlarged, because a needle that is widened until it survives a whole-tree frame stops being a needle;
/// that is what the plume is for.
const KETELEERIA_NEEDLE_LENGTH: f32 = 0.045;

/// The Keteleeria's plume: **28 cm long and 15 cm across**, the widest of the comb-toothed cards in this
/// gallery and the reason a 26 m crown can be drawn at 43 000 rows.
///
/// **The plume is a lateral shoot's own plume and no longer.** A 24 cm spur with 45 mm needles two ranks
/// deep is a plume about 28 cm long and fifteen across, and that is the number here. **The first plate drew
/// it 32 cm long and 23 cm wide** — a card half again as broad as its own shoot — and the crown came out as
/// a solid mass with every cone buried inside it, which is the opposite of the note's *"coarse, knobbly,
/// corn-cob texture"* from `01-habit.jpg`.
const KETELEERIA_PLUME_LENGTH: f32 = 0.28;

/// Half the plume's width, and the note's own **two ranks** read as a plume's width: a shoot's needles lie
/// in one plane and cover a band twice the shoot's own length, not a rod.
const KETELEERIA_PLUME_HALF_WIDTH: f32 = 0.075;

/// The Keteleeria's cone at its drawn length, **`200 mm` — the top of the note's `9–20 cm` range**.
///
/// # Invariants
///
/// **The top of the range, and the reason is the sampling tax.** At [`EVELYN_KETELEERIA`]'s 17.9 mm a
/// pixel a 200 mm cone is **11.2 px long and 3.6 px across**, and it is the *only* part of this species
/// above the tax: the plume's teeth are 0.6 pixels and the true needle is 0.14. The note's first
/// recognition fact — *"against the sky the crown reads as a coarse, knobbly, corn-cob texture"* — is a
/// claim about a texture **at distance**, so the whole-tree plate's cone is the longest cone this species
/// has and the bough plate's is the shortest, and both say which.
const KETELEERIA_CONE_LENGTH: f32 = 0.200;

/// Where the Keteleeria's crown begins on its own bole: `9.0 m` of a `26 m` tree, the note's `8–15 m` to
/// first branch read at its lower end, and the number every crown law in this species is measured inside.
const KETELEERIA_CROWN_BASE: f32 = 0.345;

/// Which Keteleeria foliage a plate draws.
///
/// # Invariants
///
/// **The two differ in one blade and in nothing else**, as the pine's does. At the whole-tree plate a
/// 2.5 mm needle is **a seventh of a pixel** and is dropped whole by a point-sampled rasteriser; the plume's
/// teeth are one to two pixels, which is the grain `01-habit.jpg` shows at the same distance. The card is
/// sparser than the needle by [`KeteleeriaFoliage::density_factor`], and the reason is the pine's and the
/// proposal's: a plume carries a whole shoot's needles in one instance and its extra area is what pays the
/// sampling tax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeteleeriaFoliage {
    /// The note's own needle, for frames within arm's length of a shoot.
    Needle,
    /// The plume: one blade whose width law is a comb of needle-teeth, standing for a whole shoot.
    Plume,
}

impl KeteleeriaFoliage {
    /// The blade this foliage instances.
    fn template(self) -> LeafTemplate {
        match self {
            KeteleeriaFoliage::Needle => keteleeria_needle(),
            KeteleeriaFoliage::Plume => keteleeria_plume(),
        }
    }

    /// The fraction of the needle foliage's density this foliage is drawn at.
    fn density_factor(self) -> f32 {
        match self {
            KeteleeriaFoliage::Needle => 1.0,
            KeteleeriaFoliage::Plume => 0.14,
        }
    }
}

/// The Keteleeria's rule: a massive clean bole, **sixteen whorls of four**, second-order branches and
/// upswept lateral shoots.
///
/// # The numbers and where they come from
///
/// [`keteleeria_evelyniana.md`](../../../.agents/investigate/trees/keteleeria_evelyniana.md)'s modelling
/// table, with every **ESTIMATE** marked. The tree is drawn at [`KETELEERIA_HEIGHT`].
///
/// | level | count | where | what |
/// | --- | --- | --- | --- |
/// | **0** | **1** | — | the **bole**, clean to 9 m |
/// | **1** | **64** in **tiers of four** | `0.345`–`1.0` of the bole | the **whorled boughs**, `5.9 m` at the crown's foot falling to `0.9 m` at the leader's tip |
/// | **2** | **6–9** | `0.20`–`0.98` of a bough | the **second-order branches**, `1.6`–`2.6 m`, upswept at their tips |
/// | **3** | **4–6** | `0.12`–`1.0` of a second-order branch | the **lateral shoots**, `0.27`–`0.41 m`, and every cone in the crown stands on one |
///
/// **Sixty-four children at four to a whorl is sixteen whorls over seventeen metres of leader, which is
/// 1.06 m between them** — inside the note's ESTIMATE of `0.6–1.2 m` and the note's own `8–16` whorls read
/// at its top. A lower count would leave two metres of bare bole between tiers, and the note's fourth
/// recognition fact is a crown that reads *"coarse, layered, plate-by-plate"* rather than smooth.
///
/// **`cluster = 4` is the whorl**, and it is the same word the pine's and the fir's use: how many siblings
/// share one station. **The whorls break up**, though, which is this species' difference from a *Pinus*:
/// the length law carries a `0.80`–`1.20` seed range and the slot step is the golden angle **between**
/// tiers and a quarter turn inside one, so the tree is a conifer's irregular crown and not a lamp standard.
fn keteleeria_rule() -> Rule {
    Rule {
        trunk: keteleeria_bole,
        generations: vec![
            Generation {
                count: (64, 64),
                station: (KETELEERIA_CROWN_BASE, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.04, 0.12),
                sink: 0.05,
                cluster: 4,
                shape: keteleeria_bough,
            },
            Generation {
                count: (6, 9),
                station: (0.20, 0.98),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.18),
                sink: 0.030,
                cluster: 1,
                shape: keteleeria_second,
            },
            Generation {
                count: (4, 6),
                station: (0.12, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.16, 0.22),
                sink: 0.008,
                cluster: 1,
                shape: keteleeria_shoot,
            },
        ],
        // **No gravity and no light.** A *Keteleeria*'s boughs are **较粗, 开展** — thick and *held out* —
        // and a light tropism turns every bough tip upward into the cone-shaped crown the note gives a
        // young tree and not the broad, spreading one it gives a mature one. The upsweep is in the shape
        // functions, where it can be a law over the station.
        fields: Vec::new(),
    }
}

/// The Keteleeria's bole: **twenty-six metres**, straight and cylindrical, massive for its height class.
/// The Keteleeria's bole: **twenty-six metres**, straight and cylindrical, massive for its height class.
///
/// # The scale, and what drawing at the note's floor costs
///
/// The note gives `25–40 m`; this tree is drawn at **twenty-six**, the floor. The reduction is a pure scale:
/// every law below is a fraction of the height, so the drawn tree keeps the note's crown-to-height ratio,
/// its whorls per metre of leader, its branch angles and its taper fractions. **What it does not keep is
/// the silhouette at forty metres' standoff** — a forty-metre Keteleeria framed whole wants a seventy-metre
/// standoff, at which a 200 mm cone is four pixels and the crown's corn-cob texture, the species' first
/// recognition fact, is gone. Twenty-six metres puts the cone at eleven pixels, which is the number H1 is
/// judged on.
fn keteleeria_bole() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.00, 0.00, 0.00], [0.040, 1.00, 0.01]),
            ([0.10, 7.00, -0.06], [0.030, 1.00, -0.01]),
            ([0.18, 15.00, 0.05], [0.020, 1.00, 0.01]),
            ([0.24, 21.50, -0.03], [0.010, 1.00, 0.00]),
            ([0.28, 26.00, 0.02], [0.000, 1.00, 0.00]),
        ]),
        // **A deep blocky flare and a long middle, and not a cone.** The note gives a d.b.h. **to 1.0 m**,
        // which no other species in this proposal does, so `0.485` at `t = 0.055` — 1.43 m up a
        // twenty-six-metre tree — is a **97 cm d.b.h.**, read at the note's own ceiling. The flare is the
        // first three knots, `1.02 m` at the ground falling to `0.60` at 0.8 m: a root plate is
        // arc 6's question and a trunk that meets the ground on a cone is the *floating tree* defect.
        taper(&[
            (0.000, 1.020),
            (0.012, 0.760),
            (0.030, 0.600),
            (0.055, 0.485),
            (0.120, 0.455),
            (0.240, 0.400),
            (0.400, 0.330),
            (0.560, 0.250),
            (0.700, 0.180),
            (0.820, 0.120),
            (0.920, 0.068),
            (1.000, 0.020),
        ]),
    )
}

/// The Keteleeria's section: **twenty-four sides with a deep longitudinal fissure in them**.
///
/// # Invariants
///
/// **Fissures, and deeper than the pine's, because the note says so twice.** FOC gives *"暗灰褐色，有不规则
/// 的深纵沟槽，成块状脱落"* — dark grey-brown, with irregular **deep** longitudinal fissures, shedding in
/// blocks. A fissure runs **with** the grain, so the relief is vertical: seven lobes of eleven and a half per
/// cent and eleven of four, which is *deep* against the pine's thirteen-per-cent shallow flutes and shallow
/// against the **eighteen per cent of five lobes the first plate drew** — a fissure a fifth of the radius is
/// 200 mm of groove on a metre of bole, and the base plate came out a **five-pointed starfish lying on the
/// sand**, which is the kapok's *flames* and not this species' bark.
///
/// **Twenty-four sides, and the crease threshold is why.** The wood's normals crease above `0.61 rad` and
/// twenty-four points round a loop is `0.26 rad` a step, so the fissures shade as deep smooth grooves rather
/// than as hard corners — and a hard corner on a two-metre bole at the base plate's three millimetres a
/// pixel is the *polygonal silhouette* the bar names.
fn keteleeria_bark() -> shape::Section {
    let sides = 24;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let fissure =
                1.0 - 0.115 * (theta * 7.0 + 0.7).sin().powi(2) + 0.038 * (theta * 11.0).sin();
            [theta.cos() * fissure, theta.sin() * fissure]
        })
        .collect();
    shape::Section::outline(&points).expect("a fissured column is a section")
}

/// A first-order bough: it leaves the leader at the note's ESTIMATE of **45–75° from vertical**, stands well
/// out, and **turns up at its tip**.
///
/// # Invariants
///
/// - **The length is a law over the station**, which is the crown's profile and what [`Joint::station`]
///   exists for: `5.9 m` at the crown's foot and `0.9 m` at the leader's tip, which with the
///   second-order branches off the outer part is a crown **11 m across** — the note's `8–14 m` at its
///   middle.
/// - **The last two knots turn it up by `0.36` rad**, so the tip climbs to within a few degrees of
///   vertical. **That is not decoration: it is the cone's frame.** A [`vegetation::decorator::Seat`]
///   expresses no rotation — the seat's `y` is the node's own tangent at the station — so a cone template
///   drawn along `+y` stands erect **on a node whose tangent points up and lies along the limb on a node
///   whose tangent does not**. The crown's knobbly silhouette is therefore a fact about the boughs' last
///   two knots and not about the cone template at all.
fn keteleeria_bough(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let profile = crown_profile(joint.station, KETELEERIA_CROWN_BASE);
    let length = (0.90 + 5.00 * (1.0 - profile).powf(1.25)) * (0.80 + 0.40 * rng.unit());
    let departure = 1.06 + 0.26 * rng.unit() + 0.10 * profile;
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.16,
                departure + 0.06,
                departure + 0.36,
            ],
        ),
        taper(&[
            (0.00, radius * 0.30),
            (0.24, radius * 0.21),
            (0.68, radius * 0.12),
            (1.00, 0.012),
        ]),
    )
}

/// A second-order branch: **1.6–2.6 m** against the note's `1.0–2.5 m`, spreading from the bough and
/// **upswept at its tip** for the same reason [`keteleeria_bough`] turns up.
fn keteleeria_second(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 2.15 * (1.10 - 0.34 * joint.station) * (0.74 + 0.52 * rng.unit());
    let departure = 0.62 + 0.34 * rng.unit();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.22,
                departure + 0.10,
                departure + 0.50,
            ],
        ),
        taper(&[
            (0.00, radius * 0.46),
            (0.40, radius * 0.30),
            (0.78, radius * 0.15),
            (1.00, 0.0055),
        ]),
    )
}

/// A lateral shoot: **27–41 cm**, the shoot the two ranks of needles stand on and the one every cone in the
/// crown is seated on.
///
/// # Invariants
///
/// **Its tip turns up by `0.62` rad**, and that is again the cone's frame and not the shoot's shape for its
/// own sake. `02-crown-cones.jpg` shows the cones standing on the **upswept tips** of exactly these shoots,
/// every one of them vertical against a branch that is not, and a cone seated on a horizontal shoot would
/// lie along it — which would be the spruce's whole arrangement and would make this species a different
/// tree.
fn keteleeria_shoot(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    // **Nineteen to twenty-nine centimetres.** The note gives no shoot length, and this one is chosen so
    // that the note's own `20–60 needles per lateral shoot` at the shoot's own spacing gives
    // `03-branch-and-bark.jpg`'s overlap: sixty needles on a 24 cm spur is one every four millimetres, and
    // a 45 mm needle is **eleven deep**. The first plate drew a 34 cm shoot and got one every 5.7 mm — a
    // bottlebrush of black bristles, which is a *Picea* shoot and not this species' flat comb.
    let length = 0.24 * (0.78 + 0.44 * rng.unit());
    let departure = 0.86 + 0.30 * rng.unit();
    let radius = joint.parent_radius.max(1.0e-5);
    (
        bend(
            length,
            [
                departure,
                departure + 0.24,
                departure + 0.08,
                departure + 0.62,
            ],
        ),
        // **A third of the parent branch's own radius at the foot**, read from the joint so a shoot at a
        // branch's tip is thinner than one at its foot by construction: a lateral shoot of this species is
        // `5–8 mm` through and the second-order branch it leaves is `20–40 mm`.
        taper(&[(0.00, radius * 0.32), (0.40, radius * 0.18), (1.00, 0.0022)]),
    )
}

/// The Keteleeria's needle: **45 mm long and 2.5 mm wide**, at the note's own size and never enlarged.
///
/// # Invariants
///
/// **The apex is *slightly raised and blunt*, not drawn out.** The note gives *"微凸起的钝尖头"* for a mature
/// tree and reserves the spine-like point for young trees and sprout shoots, so the width law's last knot is
/// `0.34` of half-width rather than a needle-fine point, and this tree is a mature one. **The base is
/// cuneate**, narrowing into a short petiole, which is the flat start of the law.
///
/// **The pale stomatal bands beneath — the note's `14–19` lines a side — are not drawn.** They are a
/// texture on a two-and-a-half-millimetre blade, they are one side of it only, and this renderer has no
/// channel for one: a leaf's two faces share one skin, which the proposal's §4.14 defers to a materials
/// arc.
fn keteleeria_needle() -> LeafTemplate {
    LeafTemplate::blade(
        keteleeria_midrib(KETELEERIA_NEEDLE_LENGTH),
        taper(&[
            (0.0, KETELEERIA_NEEDLE_HALF_WIDTH * 0.58),
            (0.22, KETELEERIA_NEEDLE_HALF_WIDTH * 0.95),
            (0.58, KETELEERIA_NEEDLE_HALF_WIDTH),
            (0.86, KETELEERIA_NEEDLE_HALF_WIDTH * 0.78),
            (1.0, KETELEERIA_NEEDLE_HALF_WIDTH * 0.34),
        ]),
        // Flat: the note's *"排列成两列"* is a flattened plane, and a needle a tenth as thick as it is wide
        // is the fir's own proportion rather than the spruce's four-angled one.
        KETELEERIA_NEEDLE_HALF_WIDTH * 0.34,
        6,
        4,
    )
}

/// The Keteleeria's needle midrib at `length`; the positions scale with the tangents, as everywhere else.
fn keteleeria_midrib(length: f32) -> shape::Curve {
    let reach = length / KETELEERIA_NEEDLE_LENGTH;
    spine(&[
        (
            [0.0000, 0.00000, 0.0],
            [0.0150 * reach, 0.00130 * reach, 0.0],
        ),
        (
            [0.0150 * reach, 0.00190 * reach, 0.0],
            [0.0142 * reach, 0.00110 * reach, 0.0],
        ),
        (
            [0.0292 * reach, 0.00300 * reach, 0.0],
            [0.0158 * reach, -0.00120 * reach, 0.0],
        ),
    ])
}

/// The Keteleeria's plume: **one blade whose width law is a comb**, standing for a whole lateral shoot's
/// plume of needles.
fn keteleeria_plume() -> LeafTemplate {
    comb(
        KETELEERIA_PLUME_LENGTH,
        KETELEERIA_PLUME_HALF_WIDTH,
        14,
        0.0026,
        22,
    )
}

/// A comb-toothed blade of `length` and `half_width`: `teeth` needle-teeth along a blade whose silhouette
/// is a plume's and not a paddle's.
///
/// # Invariants
///
/// **The sine is clamped at nought** because the `f32` pi overshoots the true one and its sine at `t = 1`
/// is a small negative number, whose fractional power is not a number at all. Every other knot is a real
/// number and the width law is a law, so the plume is a solid whose two faces carry their own normals and a
/// plume lit from behind needs no shading trick.
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
            let envelope = (std::f32::consts::PI * t).sin().max(0.0).powf(0.70);
            let tooth = if index % 2 == 0 { 0.10 } else { 1.0 };
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
                [length, length * 0.26, length * 0.08],
                [length * 0.42, length * 0.13, length * 0.04],
            ),
        ]),
        shape::Scalar::new(&knots).expect("a comb is a law"),
        thickness,
        stations,
        4,
    )
}

/// The Keteleeria's cone, drawn **standing** — an axis with a helix of recurved seed scales around it.
///
/// # The sprig, and why this species is the one that wants it
///
/// [`vegetation::decorator::Template::sprig`] is **several sweeps in one template** (proposal §4.15), and
/// this is the case it was added for. The note's cone is not a lump: FOC gives *"种鳞卵形至斜菱形，上部向外
/// 反曲，边缘具明显细缺齿"* — the scales **recurve outward at their tips** and carry **obvious small teeth**
/// on their margins, and `05-cone-erect.jpg` is a photograph of exactly that: a corn cob of pale green
/// plates, each one's upper third flipped away from the axis.
///
/// **One sweep cannot say a recurved plate.** It can say a lobed outline, and a lobed outline at the whole-tree
/// plate's 11.4 pixels is a rod — which is what the fir's cone is, and what this species is *not* like. So
/// the cone is **an axis plus thirty-two scales**: the axis carries the cone's bulk and its silhouette, and
/// the scales carry the corn-cob texture that is the species' first recognition fact at two hundred metres.
/// The first part is the axis **at the identity**, because that is the invariant the sprig is built on.
///
/// **The scale's own sweep is short, wide and thin** — sixteen millimetres long, twelve wide, two thick,
/// and bent **outward** through its four knots so it is a recurved plate and not a flat one. Thirty-two of
/// them stand in a helix over the axis's middle two thirds, which is where the note's photograph has them:
/// a cone's base and tip are both nearly smooth.
fn keteleeria_cone() -> Template {
    let mut axis_sweep = shape::Sweep::new(
        spine(&[
            ([0.0000, 0.00000, 0.0], [0.0010, 0.0200, 0.0]),
            ([0.0004, 0.02250, 0.0], [0.0000, 0.0400, 0.0]),
            ([0.0006, 0.06800, 0.0], [0.0000, 0.0420, 0.0]),
            ([0.0006, 0.11800, 0.0], [0.0000, 0.0420, 0.0]),
            ([0.0004, 0.16800, 0.0], [0.0000, 0.0330, 0.0]),
            ([0.0002, 0.19600, 0.0], [0.0000, 0.0040, 0.0]),
        ]),
        shape::Section::circle(),
    );
    axis_sweep.scale = taper(&[
        (0.00, 0.0075),
        (0.15, 0.0105),
        (0.40, 0.0150),
        (0.70, 0.0158),
        (0.90, 0.0120),
        (1.00, 0.0025),
    ]);
    let axis = Template {
        sweep: axis_sweep,
        stations: 12,
        around: 12,
    };

    let scale = seed_scale();
    let rows = 8usize;
    let each = 4usize;
    // **The axis is the first part and it stands at the identity**, because that is the invariant
    // `Template::sprig` is built on: the first part *is* the template's own band, at the template's own
    // frame, so `stations` and `around` keep meaning the axis's grid.
    let mut parts: Vec<(Template, Placement)> = vec![(axis, Placement::IDENTITY)];
    for row in 0..rows {
        let along = KETELEERIA_CONE_LENGTH * (0.10 + 0.78 * row as f32 / (rows - 1) as f32);
        // **A golden-angle helix**, which is what a cone's scales are: every scale is turned a little
        // further round than the last, so the cone reads as a spiral of plates rather than as four ridges.
        for member in 0..each {
            let turn = row as f32 / rows as f32 + member as f32 / each as f32;
            let yaw = turn * std::f32::consts::TAU;
            let rise = 0.16 - 0.05 * (row as f32 / rows as f32);
            parts.push((scale.clone(), around(along, yaw, rise)));
        }
    }
    Template::sprig(parts)
}

/// One seed scale: **16 mm long, 12 mm wide and 2 mm thick**, curving **outward** through its four knots.
///
/// # Invariants
///
/// **The recurve is the whole note**, and it is a shape and not a number: the knots rise in the scale's own
/// outward direction from `0.0016` to `0.0092`, so the plate's upper third stands away from the cone's
/// axis and its tip points back down — which is *"上部向外反曲"* and what makes `05-cone-erect.jpg` look the
/// way it does. **The small teeth on the margin are not drawn**: they are two millimetres on a 200 mm cone,
/// a tenth of a pixel at the whole-tree plate and two at the bough plate's own distance, and the section
/// would have to carry more points than the ring has slots.
fn seed_scale() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0000, 0.0000, 0.0000], [0.0055, 0.0016, 0.0000]),
            ([0.0055, 0.0044, 0.0000], [0.0052, 0.0068, 0.0000]),
            ([0.0104, 0.0092, 0.0000], [0.0036, 0.0050, 0.0000]),
            ([0.0136, 0.0104, 0.0000], [0.0010, 0.0012, 0.0000]),
        ]),
        shape::Section::circle(),
    );
    sweep.scale = taper(&[
        (0.00, 0.0032),
        (0.34, 0.0060),
        (0.70, 0.0056),
        (1.00, 0.0022),
    ]);
    Template {
        sweep,
        stations: 5,
        around: 6,
    }
}

/// The Keteleeria's needle fields at `shoot_depth`: the leader's candle, the boughs' own plume, the two
/// ranks over a shoot, and the brush at a shoot's tip.
///
/// # Invariants
///
/// **The three shoot fields carry `phyllotaxy = OPPOSITE` and the two that sit on the bough and the leader
/// do not.** Half a turn between one needle and the next is *Keteleeria*'s whole diagnostic in one number —
/// the note's *"排列成两列"*, the flat fern-like comb of `03-branch-and-bark.jpg` — against the spruce's dense
/// golden-angle bottlebrush and against this proposal's own pine, whose needles come in a sheath of three.
/// **There is no cluster anywhere in this species**, because there is no fascicle to cluster: the shoot is
/// the unit and the needles are in two ranks on it.
///
/// **The pitch and twist laws turn a radial needle into a flat one.** Pitch is radians from the shoot's
/// outward direction toward its tip, and `1.20` stands the blade forward along the shoot; the twist rolls
/// it about its own midrib so **its face is the sky**, which is what a two-ranked needle's face does and
/// what makes a Keteleeria crown read as flat fronds rather than as a bottlebrush.
fn keteleeria_needles(shoot_depth: usize, foliage: KeteleeriaFoliage) -> Vec<LeafField> {
    let factor = foliage.density_factor();
    let mut candle = LeafField::new(0, 0);
    candle.band = (0.40, 0.975);
    candle.density = shape::Scalar::constant(120.0 * factor);
    candle.cluster = 1;
    candle.fan = shape::Scalar::constant(0.0);
    candle.phyllotaxy = GOLDEN;
    candle.pitch = shape::Scalar::constant(1.05);
    candle.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.50)]).expect("a roll is a law");
    candle.size = shape::Scalar::constant(1.0);
    candle.jitter = 0.22;

    let mut bough = LeafField::new(1, 0);
    // **The bough's own plume**, over its whole length: in *Keteleeria* the outer part of a first-order
    // branch is itself a needled shoot, and at forty-eight metres it is the bough's plume and not eight
    // separate tufts on a bare rod. A bough whose foliage is only on its secondaries is the *bare broom*.
    bough.band = (0.05, 0.99);
    bough.density = shape::Scalar::constant(190.0 * factor);
    bough.cluster = 1;
    bough.fan = shape::Scalar::constant(0.0);
    bough.phyllotaxy = GOLDEN;
    bough.pitch = shape::Scalar::constant(1.10);
    bough.twist = shape::Scalar::new(&[(0.0, 0.0), (1.0, 1.50)]).expect("a roll is a law");
    bough.size = shape::Scalar::new(&[(0.0, 0.82), (1.0, 1.0)]).expect("a size is a law");
    bough.jitter = 0.24;

    let mut second = LeafField::new(shoot_depth.saturating_sub(1), 0);
    second.band = (0.05, 0.97);
    second.density = shape::Scalar::constant(220.0 * factor);
    second.cluster = 1;
    second.fan = shape::Scalar::constant(0.0);
    second.phyllotaxy = OPPOSITE;
    second.pitch = shape::Scalar::constant(1.18);
    second.twist = shape::Scalar::new(&[(0.0, 1.30), (1.0, 1.48)]).expect("a roll is a law");
    second.size = shape::Scalar::new(&[(0.0, 0.86), (1.0, 1.06)]).expect("a size is a law");
    second.jitter = 0.20;

    let mut spray = LeafField::new(shoot_depth, 0);
    // **The band stops at three quarters of the shoot, and that is the cone's clearance.** A plume is 280 mm
    // long and radiates from its anchor in every direction, so a plume seated at `0.94` of a shoot buries
    // a 200 mm cone whole — and the first plate drew every cone in the crown inside the foliage, which is
    // the note's first recognition fact invisible. **The last quarter of every shoot is bare wood, and the
    // cone stands on it.**
    spray.band = (0.06, 0.74);
    // **Three hundred to the metre is the note's `20–60 needles per lateral shoot` read as a linear
    // density**: a 24 cm shoot over its foliated two thirds carries forty-nine, which is the note's count at
    // its upper end, and one needle every 3.3 mm on a blade four and a half centimetres long is the
    // **thirteen-deep overlap** `03-branch-and-bark.jpg` shows. The first plate stood at 150 and drew a
    // bottlebrush of black bristles — a *Picea* shoot and not this species' flat comb.
    spray.density = shape::Scalar::constant(300.0 * factor);
    spray.cluster = 1;
    spray.fan = shape::Scalar::constant(0.0);
    spray.phyllotaxy = OPPOSITE;
    spray.pitch = shape::Scalar::constant(1.20);
    spray.twist = shape::Scalar::new(&[(0.0, 1.32), (1.0, 1.50)]).expect("a roll is a law");
    spray.size = shape::Scalar::new(&[(0.0, 0.84), (1.0, 1.08)]).expect("a size is a law");
    spray.jitter = 0.20;

    let mut brush = LeafField::new(shoot_depth, 0);
    // **The pale new growth at the shoot tips**, which is a size law and not a second template because the
    // blade is the same blade — and it stops with [`spray`], at `0.78`, which is the densest foliage on the
    // shoot and the last foliage before the cone's own bare quarter.
    brush.band = (0.52, 0.78);
    brush.density = shape::Scalar::constant(420.0 * factor);
    brush.cluster = 1;
    brush.fan = shape::Scalar::constant(0.0);
    brush.phyllotaxy = OPPOSITE;
    brush.pitch = shape::Scalar::constant(1.06);
    brush.twist = shape::Scalar::new(&[(0.0, 1.26), (1.0, 1.52)]).expect("a roll is a law");
    brush.size = shape::Scalar::new(&[(0.0, 0.86), (1.0, 1.04)]).expect("a size is a law");
    brush.jitter = 0.18;
    if shoot_depth >= 3 {
        vec![candle, bough, second, spray, brush]
    } else {
        vec![second, spray, brush]
    }
}

/// The Keteleeria: its tree, its foliage at the asked scale, its five fields and its cone.
pub fn evelyn_keteleeria(seed: u64, foliage: KeteleeriaFoliage) -> Species {
    let mut species = Species {
        plant: grow(&keteleeria_rule(), seed),
        templates: vec![foliage.template()],
        fields: keteleeria_needles(3, foliage),
        decorators: vec![keteleeria_cone()],
    };
    if let Ok(bole) = species.plant.get_mut(&Handle::root()) {
        bole.set_section(keteleeria_bark());
    }
    species
}

/// The bough plate's species: **one second-order branch as the trunk**, its lateral shoots as the only
/// generation, the note's own needle, and the cone.
///
/// # Invariants
///
/// **The same rule two levels down**, as the fir's and the pine's bough plates are: the root is a
/// second-order branch of the shape the whole tree's are, the generation is the whole tree's lateral
/// shoots, and the fields come from [`keteleeria_needles`] with the depths this plate's own tree has. The
/// plate is framed where a 45 mm needle is **two and a half pixels** wide, which is where the note's two
/// ranks and its recurved cone are both countable and neither is three pixels.
pub fn evelyn_keteleeria_bough(seed: u64) -> Species {
    let rule = Rule {
        trunk: || {
            (
                bend(2.15, [0.86, 1.08, 0.96, 1.36]),
                taper(&[(0.00, 0.030), (0.40, 0.020), (0.78, 0.010), (1.00, 0.0045)]),
            )
        },
        generations: vec![Generation {
            count: (14, 18),
            station: (0.12, 1.0),
            slot: (0.0, GOLDEN),
            jitter: (0.16, 0.22),
            sink: 0.006,
            cluster: 1,
            shape: keteleeria_shoot,
        }],
        fields: Vec::new(),
    };
    Species {
        plant: grow(&rule, seed),
        templates: vec![KeteleeriaFoliage::Needle.template()],
        fields: keteleeria_needles(1, KeteleeriaFoliage::Needle),
        decorators: vec![keteleeria_cone()],
    }
}

/// The cones the Keteleeria carries: **one or two on every upswept lateral shoot in the outer seven
/// tenths of the crown**, and one or two more on the boughs of the **upper half of the bole**.
///
/// # Invariants
///
/// - **`scatter_in` and its band, and they are two different bands on two different generations.** On the
///   shoots the band is `(0.12, 1.0)` — the whole shoot generation — and the *erectness* comes from the
///   shoot's own upswept tip. On the boughs the band is `(0.45, 1.0)` of the **bole**, which is literally
///   *"the upper crown"*: a bough hanging at `station 0.62` of a 26 m bole is at 16 m, and a bough hanging
///   at `0.44` is at 11 m. [`scatter`] alone cannot say that — it reads one station of the node and knows
///   nothing about where the node itself hangs — which is the whole of proposal §4.15's first item.
/// - **The count is a range the seed decides.** FOC gives the pollen season as April–May and the seed as
///   October, so a crown carries cones in every state from green to ripe, and a fixed count would draw a
///   tree where every shoot has exactly one.
fn keteleeria_cones(species: &Species, shoot_depth: usize, count: (u32, u32)) -> Vec<Seat> {
    let mut seats = scatter_in(&species.plant, shoot_depth, 0, 0.94, count, (0.12, 1.0));
    seats.extend(scatter_in(&species.plant, 1, 0, 0.90, count, (0.45, 1.0)));
    seats
}

/// Four cones on the topmost two metres of the Keteleeria's own leader, **written out by hand**.
///
/// # Invariants
///
/// - **They are here because they are the only cones that stand above the crown line**, and the note's first
///   recognition fact is a crown that reads as *"a coarse, knobbly, corn-cob texture"* against the sky. A
///   cone on a shoot tip stands *inside* the crown however upright its template is; only the leader's own
///   top clears the foliage.
/// - **The leader's candle stops at `0.975`**, so the last metre and a half of bole stands bare and the
///   cones on it have something to stand against. A candle carried to the tip buries its own cones — a
///   plume is 320 mm long and radiates from its anchor, so a candle that reaches `1.0` has its foliage a
///   third of a metre above the apex — which is exactly what the fir's first two renders of its four did.
/// - [`scatter`] cannot place them: it takes **one station for a whole generation** and the leader is a
///   single node, so `scatter(plant, 0, …)` would put one cone at one station rather than four along a
///   leader. The fir module records the same fact about its own four.
fn keteleeria_leader_cones() -> Vec<Seat> {
    [(0.980, 0.14), (0.990, 0.61), (0.997, 0.37), (1.000, 0.85)]
        .into_iter()
        .map(|(station, slot)| Seat::new(Handle::root(), 0, station, slot))
        .collect()
}

/// The Keteleeria's surfaces: a dark grey-brown fissured bole, a **dark glossy green** needle and a cone of
/// its own.
///
/// # Invariants
///
/// **The crown is the species' third recognition fact — *"noticeably darker and more lustrous than the
/// Pinus yunnanensis around it"* — and dark green is a tint and not a count.** `0.092, 0.135, 0.066` is a
/// deep green against the pine's grey-green `0.10, 0.17, 0.08`, and the roughness is **0.50**, which is
/// FOC's *"光绿色"* — a glossy upper surface — and is the second half of the same word. **The first plate
/// stood the needle at `0.052, 0.090, 0.044` and a roughness of 0.55**, and the crown plate came out a
/// black silhouette with three visible plumes.
///
/// **The cones are yellow-green, not brown.** `05-cone-erect.jpg` is an unripe cone photographed in spring
/// and it is a saturated chartreuse, which under a warm key is `0.300, 0.268, 0.072`; the module states it
/// because a cone drawn in the fir's black would make this species the fir, and the note's whole case for
/// it against the pine is its cones.
fn keteleeria_palette() -> Palette {
    Palette {
        wood: Skin::new([0.064, 0.057, 0.047], 0.97),
        leaves: vec![Skin::new([0.092, 0.135, 0.066], 0.50)],
        seats: vec![Skin::new([0.320, 0.285, 0.080], 0.72)],
    }
}

/// A dry warm-temperate montane slope at eighteen hundred metres: a high white sun off the sunward side, a
/// cold sky fill from the valley, and red-brown montane soil.
///
/// # Invariants
///
/// - **A light's reach is its cutoff and it must clear the floor's own diagonal**, so the floor's 400 m
///   reach and its 566 m diagonal give a **reach of 1200 m**; the *distance* is the key's own length from
///   the origin, **55.0 m**, because a twenty-six-metre tree wants its key far enough off that its own
///   shadow page is fitted to the crown and not to the bole.
/// - **A dry belt's key is strong and its sky is not.** The Keteleeria grows in the same belt as the
///   Yunnan pine and the note calls it *"the darker, glossier, more massive tree among the pines"*, which
///   is a statement about **contrast**: a strong key on a dim sky is what separates a dark glossy crown from
///   the pale grey-green one beside it, and it is the same number that moves H3.
fn keteleeria_lighting() -> Lighting {
    Lighting {
        // **The key is on the camera's side and to its left, and the first plate put it behind the tree.**
        // The camera stands at `[33, 5.5, 35]`; a key at `[34, 30, −20]` is *behind the crown from the
        // lens*, and every frame it drew was a dark mass with a pale pole in front of it. A key at
        // `[−26, 31, 27]` rakes the crown across the lens's axis, which is what raking means.
        key_position: [-26.0, 31.0, 27.0],
        key_colour: [1.0, 0.93, 0.80],
        key_irradiance: 4.8,
        key_distance: 48.7,
        key_reach: 1200.0,
        // **Nine metres is the crown's own half-width** (an 11 m crown), and a smaller page would put the
        // ground beyond it outside the key's shadow, which is what a page fitted to the subject means.
        key_extent: 9.0,
        fill_position: [22.0, 13.0, -18.0],
        fill_colour: [0.46, 0.56, 0.76],
        fill_irradiance: 0.34,
        fill_distance: 31.1,
        floor: Skin::new([0.290, 0.220, 0.150], 0.96),
        sky: Atmosphere::new()
            .zenith([0.06, 0.15, 0.40])
            .horizon([0.50, 0.57, 0.66])
            .ground([0.32, 0.29, 0.23])
            .density(0.0006)
            .scale_height(3000.0),
        // **A dim sky under a strong key, and the ratio between them is H3's number.** The Keteleeria grows
        // in the same belt as the Yunnan pine and the note calls it *"the darker, glossier, more massive
        // tree among the pines"*, which is a statement about **contrast**: a strong key on a dim sky is what
        // separates a dark glossy crown from the pale grey-green one beside it.
        sky_radiance: 0.30,
        // A dry belt at eighteen hundred metres: thin air, and a far ridge is all it touches.
        haze: 0.004,
    }
}

/// The base plate's own light: a low key on the lens's side, for the same reason the birch's is — the tree
/// plate's raking key stands behind this bole and a bark plate lit from behind is a silhouette.
///
/// # Invariants
///
/// **The page is the crown's, not the plate's, and the module pays for saying so.** The key's extent sizes
/// the shadow page and nine is right for a twenty-six-metre crown; four was tried for this plate's
/// millimetre-scale texel and the ground beyond the smaller page then read as one dark band from the
/// horizon to the tree's own shadow. **A base plate pays the dither and says so**, which is the trade the
/// kapok's base plate already records.
fn keteleeria_base_lighting() -> Lighting {
    Lighting {
        key_position: [5.0, 4.2, 5.2],
        key_colour: [1.0, 0.94, 0.84],
        key_irradiance: 4.8,
        key_distance: 8.3,
        key_reach: 40.0,
        key_extent: 9.0,
        fill_position: [-4.5, 2.4, -4.5],
        fill_colour: [0.46, 0.54, 0.70],
        fill_irradiance: 0.34,
        fill_distance: 6.8,
        floor: Skin::new([0.185, 0.145, 0.100], 0.96),
        sky: Atmosphere::new()
            .zenith([0.08, 0.18, 0.42])
            .horizon([0.52, 0.58, 0.66])
            .ground([0.32, 0.29, 0.24])
            .density(0.0006)
            .scale_height(3000.0),
        sky_radiance: 0.30,
        haze: 0.004,
    }
}

/// The whole-tree frame: **49.2 m** off a twenty-six-metre tree, the camera at a fifth of its height and
/// pitched twelve degrees **up**.
///
/// # Invariants
///
/// - **The pitch is what keeps the ground out of the picture.** `tan(12.3°) / tan(17.2°) = 0.70`, so the
///   horizon sits at seventy per cent of the way down the frame and the ground is the bottom sixth. The
///   first plate aimed at `13.5 m` from `5.5 m` — a pitch of 9.5° — and put **39% of the frame on a lit
///   floor** with the key's shadow page's boundary drawn across it as a straight line, which is the one
///   artefact a tree plate must not carry.
/// - **`2 · 49.2 · tan(0.30) / 1700` is 17.9 mm**, so a 200 mm cone is **11.2 px** and a plume's teeth are
///   one to two: the whole of the card's argument, and the reason the cone template is at the top of the
///   note's range.
const KETELEERIA_FRAME: Framing = Framing {
    width: 1400,
    height: 1700,
    eye: [33.0, 5.5, 35.0],
    target: [0.0, 16.0, 0.0],
    field_of_view: 0.60,
    near: 0.5,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The bough plate's frame: **1.26 m** square to the second-order branch's own middle, looking
/// twenty-two degrees **up** at its shoots and their erect cones.
///
/// # Invariants
///
/// **`2 · 1.26 · tan(0.375) / 1000` is 0.99 mm**, and that is the plate's whole number: a 45 mm needle is
/// **45 px long and 2.5 px wide** — the note's two ranks countable, which is H2 — and a 200 mm cone is
/// **202 px**, a fifth of the frame's height, so the corn-cob of recurved scales is the subject rather than
/// a texture.
///
/// **The camera looks UP, and the first two plates' did not.** A plate of a branch sixteen metres above a
/// forest floor that looks *down* fills two thirds of its frame with unlit ground: the first two renders of
/// this plate were a black bar across a black field, and then a cluster of cones in the top-left corner.
///
/// **The eye is placed from the branch's own arithmetic**, not by eye: `bend(2.15, [0.86, 1.08, 0.96, 1.36])`
/// puts the branch's foot at the origin and its tip at `x = 1.76`, `y = +1.22` in its own frame, so its
/// middle is at `(0.88, +0.61)` and a camera at `z = 1.15` and `y = −0.35` stands 1.26 m square to the
/// shoots that hang over it.
const KETELEERIA_BOUGH_FRAME: Framing = Framing {
    width: 1200,
    height: 1000,
    eye: [1.05, 16.20, 1.15],
    target: [0.88, 16.68, 0.0],
    field_of_view: 0.75,
    near: 0.02,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The base plate's frame: **5.50 m** off the bole at chest height, aimed at **1.6 m**, which is the frame
/// H4 reads the flare and the blocky fissures in.
const KETELEERIA_BASE_FRAME: Framing = Framing {
    width: 1200,
    height: 1400,
    eye: [3.90, 2.10, 3.80],
    target: [0.0, 1.60, 0.0],
    field_of_view: 0.75,
    near: 0.05,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The Keteleeria's expansion dial for the two crown plates.
///
/// # Invariants
///
/// **`around = 16`, and the fissures are what ask for it.** The section carries twenty authored points with
/// five deep lobes in them, and twelve slots round the loop round those lobes off between samples — so the
/// first plate's bole was a smooth pale pole at the one distance where the note's fifth recognition fact,
/// *"dark grey-brown, with irregular deep longitudinal fissures"*, is read. Sixteen keeps them.
const KETELEERIA_DETAIL: Detail = Detail {
    stations: 6,
    around: 16,
    spacing: 0.5,
};

/// The Keteleeria's expansion dial for the base plate.
///
/// # Invariants
///
/// **`spacing = 220 mm` and `around = 20`, and both are the fissures.** The crate's half-metre default puts
/// fifty-two rings on a twenty-six-metre bole and a half-metre facet on a two-metre trunk is the
/// *polygonal silhouette* the bar names; and sixteen slots round a twenty-four-point section rounds the
/// fissures off between samples, which is a fluted tree rather than a blocky one. **The cost is stated**: the
/// lateral shoots take the dial's floor of six rings whatever the spacing says — a 24 cm shoot at 220 mm
/// asks for two — so the fine spacing is paid for by the bole and the branches and not by the two thousand
/// four hundred shoots, which is the one place the crate's per-planting `Detail` is cheap.
const KETELEERIA_BASE_DETAIL: Detail = Detail {
    stations: 6,
    around: 20,
    spacing: 0.22,
};

/// Builds the Keteleeria's whole-tree plate: one tree in the plume, its own light and ground, and its
/// cones on the upper crown band.
///
/// **Rows.** Sixty-four boughs at four to a whorl is sixteen whorls; 480 second-order branches at seven and a
/// half to a bough; 2 400 lateral shoots at five to a second-order branch, which is **2 989 nodes** inside
/// the planting's stride of 65 536. The five fields carry **58 094 leaf rows** and the cone scatters
/// **3 353**, so the plate is inside the ninety-thousand-row ceiling of §4.13 with room to spare, and it
/// compiles in 48 seconds and rasterises in 8.
pub fn evelyn_keteleeria_picture() -> Value {
    let species = evelyn_keteleeria(KETELEERIA_SEED, KeteleeriaFoliage::Plume);
    let mut seats = keteleeria_cones(&species, 3, (1, 2));
    seats.extend(keteleeria_leader_cones());
    let planting = Planting::new(species.clone())
        .with_seats(seats)
        .with_palette(keteleeria_palette())
        .with_detail(KETELEERIA_DETAIL);
    plant_scene_lit(
        &[planting],
        &KETELEERIA_FRAME,
        EVELYN_KETELEERIA,
        &keteleeria_lighting(),
        &[],
    )
}

/// Builds the Keteleeria's crown-branch plate: one second-order branch at a metre and a half, in the true
/// needle, with its erect cones against the sky.
pub fn evelyn_keteleeria_bough_picture() -> Value {
    let species = evelyn_keteleeria_bough(KETELEERIA_SEED);
    // **Two or three cones to a shoot here against one or two on the tree**, because at 0.99 mm a pixel a
    // 200 mm cone is 202 px and this plate is a picture of the cone.
    let seats = scatter_in(&species.plant, 1, 0, 0.94, (2, 3), (0.12, 1.0));
    let planting = Planting::new(species.clone())
        .at(Placement {
            origin: [0.0, 16.0, 0.0],
            ..Placement::IDENTITY
        })
        .with_seats(seats)
        .with_palette(keteleeria_palette())
        .with_detail(KETELEERIA_DETAIL);
    plant_scene_lit(
        &[planting],
        &KETELEERIA_BOUGH_FRAME,
        EVELYN_KETELEERIA_BOUGH,
        &keteleeria_lighting(),
        &[],
    )
}

/// Builds the Keteleeria's base plate: the foot of the bole at five and a half metres, which is the frame
/// H4 reads the flare and the blocky fissures in.
pub fn evelyn_keteleeria_base_picture() -> Value {
    let species = evelyn_keteleeria(KETELEERIA_SEED, KeteleeriaFoliage::Plume);
    let mut seats = keteleeria_cones(&species, 3, (1, 2));
    seats.extend(keteleeria_leader_cones());
    let planting = Planting::new(species.clone())
        .with_seats(seats)
        .with_palette(keteleeria_palette())
        .with_detail(KETELEERIA_BASE_DETAIL);
    plant_scene_lit(
        &[planting],
        &KETELEERIA_BASE_FRAME,
        EVELYN_KETELEERIA_BASE,
        &keteleeria_base_lighting(),
        &[],
    )
}

/// Resolves a whole-tree plate's `authored_id` into the handle that names the node.
pub fn evelyn_keteleeria_handle_of(id: u32) -> Option<String> {
    node_ids(&evelyn_keteleeria(KETELEERIA_SEED, KeteleeriaFoliage::Needle).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves the base plate's `authored_id`: **the same plant**, so the same walk is the table.
pub fn evelyn_keteleeria_base_handle_of(id: u32) -> Option<String> {
    evelyn_keteleeria_handle_of(id)
}

/// Resolves the bough plate's `authored_id`: the second-order branch is its own plant, so its walk is its
/// own table and the whole tree's would answer with the wrong node.
pub fn evelyn_keteleeria_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&evelyn_keteleeria_bough(KETELEERIA_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

// ---------------------------------------------------------------------------
// The drawing tools both species share
// ---------------------------------------------------------------------------

/// Where a limb sits in its own crown, nought at the base of the crown and one at the leader's tip.
///
/// # Invariants
///
/// **This is the crown profile every length law in the module is over**, and it is `station`-based rather
/// than rank-based on purpose: a conifer's lower bough is many times the length of its upper one, and the
/// band a generation spreads over begins at a number that is not zero — the birch's crown starts at 0.40 of
/// its tree and the Keteleeria's at 0.345 — so the profile is the child's own station measured **inside that
/// band** rather than the raw one. A profile read raw would make both species' lowest boughs the same length
/// as the middle of their trunks.
fn crown_profile(station: f32, base: f32) -> f32 {
    ((station - base) / (1.0 - base)).clamp(0.0, 1.0)
}

/// Four knots along a heading that turns through `angles`, measured from the parent's own tangent: nought is
/// along the parent and a quarter turn is straight out. The tangents are the heading's own direction, so a
/// limb is a chord with three bends in it rather than a straight line with a direction, and **the last two
/// knots are where a droop, a hang or an upsweep is drawn** — which is the whole of both species' signatures.
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

/// A frame at `(at, 0, 0)` in the template's own coordinates, carrying no rotation: how a part of a sprig
/// sits further along the template it belongs to.
///
/// # Invariants
///
/// **A translation, never a scale.** A [`vegetation::leaf::LeafInstance::scale`] is the one number that
/// changes a blade's size and it is applied in the template's own frame; a placement here that carried one
/// would put the size of a leaf in the geometry instead of in the field, where the size law could not reach
/// it.
fn along(at: f32) -> Placement {
    Placement {
        origin: [at, 0.0, 0.0],
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        z: [0.0, 0.0, 1.0],
    }
}

/// A frame at `(along, 0, 0)` turned `yaw` radians about the template's face normal — how a cone's seed
/// scale stands off the cone's axis — and then tilted `rise` radians about its own forward axis.
///
/// # Invariants
///
/// **The same bargain the leaf sprigs make**, for the same reason: a decorator's parts compose the way
/// placements do, so a part already carrying a frame inside a sprig keeps both. **No scale anywhere in the
/// pair**, because a scale in a template's own geometry is a scale the field's laws cannot see.
fn around(along: f32, yaw: f32, rise: f32) -> Placement {
    let (yaw_sin, yaw_cos) = yaw.sin_cos();
    let (rise_sin, rise_cos) = rise.sin_cos();
    let forward = [yaw_cos, yaw_sin, 0.0];
    let side = [-yaw_sin, yaw_cos, 0.0];
    let face = [0.0, 0.0, 1.0];
    Placement {
        origin: [along, 0.0, 0.0],
        x: forward,
        y: [
            side[0] * rise_cos + face[0] * rise_sin,
            side[1] * rise_cos + face[1] * rise_sin,
            side[2] * rise_cos + face[2] * rise_sin,
        ],
        z: [
            face[0] * rise_cos - side[0] * rise_sin,
            face[1] * rise_cos - side[1] * rise_sin,
            face[2] * rise_cos - side[2] * rise_sin,
        ],
    }
}
