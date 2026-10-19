//! 川滇高山栎 — *Quercus aquifolioides*, the evergreen oak of the subalpine belt.
//!
//! # What this module draws, and what it deliberately leaves for a later arc
//!
//! The note ([`quercus_aquifolioides.md`](../../../.agents/investigate/trees/quercus_aquifolioides.md))
//! names **two growth forms** and this module draws one of them: the montane **tree**, a small evergreen oak
//! of **10(−20) m** with a dense rounded dark crown carried low on persistent branches above a short gnarled bole.
//! The second — the low, sprawling, **multi-stemmed cushion shrub** of an exposed ridge at the upper end of the
//! range (`09-habit-scrub-form.jpg`) — is **not** drawn here and is **not** a scaled-down tree. The note is explicit
//! that *"the dominant change is a change of growth form"* and that the two are *"selected by site, not by age"*,
//! so a shrub built by this rule at a smaller height would be exactly the scaled-down tree the note warns against. It
//! is a second species' work for a later arc.
//!
//! # The silhouette, and what the pine's card foliage is doing there
//!
//! The pine's whole-tree plate draws [`southwest::Foliage::Card`](southwest::Foliage::Card), a blade half a pixel
//! wide, because its 1.1 mm needle is a tenth of a pixel at twenty metres and a point-sampled rasteriser **drops**
//! a blade narrower than a pixel rather than fading it. **This oak needs no card, and the number that says so is the
//! frame's own pixel.** The whole-tree plate stands **25.0 m** off with a vertical field of view of **0.55 rad** over
//! **1700** rows, so one pixel is `2 · 25.0 · tan(0.275) / 1700` = **8.3 mm**; the leaf, at the note's **48 mm**, is
//! therefore **5.8 pixels** long. Both plates draw the **true** leaf, and [`ALPINE_OAK_LEAF_PIXEL_METRES`] states
//! the measurement in the code that the framing depends on.
//!
//! # What the second pass bought, and what it bought it with
//!
//! **The first plate of this species bought its crown's coverage with leaf *size*** — it drew the note's FOC
//! maximum, **70 × 34 mm**, because this builder's cost in instances is superlinear and a bigger blade is a
//! cheaper crown. It reached a crown coverage of **0.219** and a frame the eye reads as lacy: sky through it
//! everywhere and a ragged edge. **[`LeafTemplate::sprig`] took that bargain away.** A shoot's leafing is a *fixed
//! arrangement* — FOC's petiole is **2–5 mm**, *"sometimes nearly sessile"*, and a golden-angle step around the twig
//! — so it is now **one instanced mesh of eight blades**, and the instance budget is spent on **shoots**. The leaf
//! went back to [`LEAF_LENGTH`], the **middle** of the note's range, and the crown's area is bought with **fourteen
//! thousand shoots** where the first pass had seventeen hundred.
//!
//! The number that says whether that worked is not in this file: `audit/frame_stats.py --crown` reads the whole-tree
//! plate's crown box, and [`SHOOT_SPRIGS_PER_METRE`] is the dial that moves it.
//!
//! # Three facts the geometry cannot carry
//!
//! These are stated rather than faked, because a module that draws a smooth egg and calls it a fringed cupule is
//! worse than one that says what it left out.
//!
//! - **The spiny leaf margin is not geometry.** FOC gives *"entire or with spiniform teeth"*, and both states occur
//!   on this species, but a tooth on a 62 mm leaf is a millimetre or two of margin — **half a pixel at the bough
//!   plate's distance** — and a point-sampled blade two tenths of a pixel wider than its own silhouette is noise.
//!   The blade here is drawn with the margin the width law can express, and the spiny state is the note's, not the
//!   frame's.
//! - **The copper-brown underside is a second template, not a second face of one.** A leaf is a closed solid whose
//!   two faces share one skin (D18), so a blade that is glossy dark green above and rusty tawny below is two tints
//!   on one mesh — which this builder cannot express. [`ALPINE_OAK_GREEN`] and [`ALPINE_OAK_BRONZE`] are the same
//!   blade drawn twice, and [`oak_leaves`] puts the bronze where the note says the wind shows it: on the crown's
//!   **lower and inner surface**, aimed at the ground. That is an approximation of *"a bronze cast from below"*,
//!   and it is labelled one.
//! - **The cupule's free, fringed bracts are not geometry** either: two millimetres of fringe on a seven-centimetre
//!   fruit, on a tree twelve metres tall. What [`acorn`] does draw is the note's proportion — a **5–6 cm** shallow
//!   cupule about **three times the length of the nut it holds**, with the nut's shoulder standing proud of the rim.
//!
//! # The bark is the note's one honest gap
//!
//! FOC gives **no bark description** for this species and no bark photograph of it exists in the material searched,
//! so the note records the bark as **unverified** and asks that no bark description be invented. What the
//! photographs do show is the upper limbs as **pale grey and smooth** (`02-crown.jpg`), and the brief for this
//! second pass asks for *"a dark grey-brown with shallow fissures"*. [`oak_palette`] is not the note's pale
//! grey: the note has nothing to say, and a trunk drawn for an unverified bark is drawn for the frame. [`fluted_bole`]
//! is the fluting the brief asks for — vertical lobes, because a flute runs with the grain — drawn as form rather
//! than claimed as a description, and [`OAK_DETAIL`] is what makes the twenty-four sides it carries survive the
//! twelve-vertex sweep every wood loft would otherwise be drawn at.
//!
//! # Three plates and one foliage
//!
//! The whole tree for the silhouette (H1, H3, H4), one bough at arm's length for the foliage (H2), and the **foot
//! of the bole** at two metres (H4's *"one frame at the base of the trunk, one at the crown"*). The rule is the same
//! rule and the second plate is one limb of it and the third is the first metre of it, so a leaf that reads on the
//! bough plate is the leaf the tree is made of and a flute that reads on the base plate is the flute the tree is
//! made of.
//!
//! # What H1's second running asked of this plate, and what it got
//!
//! The ten-species sheet of 2026-10-19 named this crown as one of three that read as *"a broadleaf"* rather than
//! as their species: **a hole at mid-crown that no photograph of it has**, and a lopsided top. Measured on the
//! cut-out, before and after `audit/frame_stats.py --crown` and `audit/silhouette.py`:
//!
//! | | before | after |
//! | --- | --- | --- |
//! | crown coverage (bar: the recorded 0.501 ± 20%) | 0.501 | **0.510** |
//! | H3 interior share (bar 0.25) | 0.792 | **0.774** |
//! | width ÷ height of the cut-out | 0.608 | **0.613** |
//! | the crown's top tenth, of a widest 749 px | 312 px | **352 px** |
//! | the enclosed region at mid-crown | 2 538 px | **2 548 px** |
//!
//! **What moved is the outline and the density, not the ratio.** The crown's shoulders now meet: the secondaries
//! are a metre and a half longer than they were, so a three-and-a-half-metre limb is leafed along its own length
//! instead of bare for two thirds of it, and the crown is a **continuous rounded mass from shoulder to shoulder**
//! with the foliage carried low down both sides. **What did not move is the hole**, and the reason is measured
//! and is in [`oak_leaves`]: the two regions still enclosed are the **bole and the air around it**, and a node's
//! id of one — the bole is the root — does not survive eight bits of red. **A reader of the cut-out cannot be
//! shown the trunk of any of the ten trees**, and this one is the only broadleaf whose trunk runs visibly
//! through its own crown, so it is the only one where the instrument's blindness is a hole in the middle of
//! the shape.

use definition::Value;
use vegetation::decorator::{Seat, Template, scatter};
use vegetation::derive::Species;
use vegetation::envelope::{Envelope, trim};
use vegetation::expand::Detail;
use vegetation::handle::Handle;
use vegetation::leaf::{LeafField, LeafTemplate};
use vegetation::plant::{Placement, Plant};
use vegetation::rng::Rng;
use vegetation::species::{Generation, Joint, Rule, grow, spine, taper};

use crate::air::Atmosphere;
use crate::vegetation_mesh::node_ids;
use crate::vegetation_picture::{
    Framing, Ground, Lighting, Palette, Planting, Skin, plant_scene_lit,
};

/// The name the oak's whole-tree plate is built and rendered under.
pub const ALPINE_OAK: &str = "alpine_oak";

/// The name the oak's branch plate is built and rendered under.
pub const ALPINE_OAK_BOUGH: &str = "alpine_oak_bough";

/// The name the oak's base plate is built and rendered under.
pub const ALPINE_OAK_BASE: &str = "alpine_oak_base";

/// The seed both plates' leaf fields are derived from.
pub const SEED: u64 = 34;

/// The golden angle as a fraction of a turn, which is the step between two successive shoots.
const GOLDEN: f32 = 0.381_966_01;

/// How deep the shoots hang in the whole tree: trunk `0`, limbs `1`, secondaries `2`, shoots `3`.
pub const SHOOTS: usize = 3;

/// How deep the same three orders hang in the bough plate: the bough **is** the trunk, so its secondaries hang at
/// `1` and its shoots at `2`.
const BOUGH_SHOOTS: usize = 2;

/// The leaf template index the crown's green blade is drawn by, and the one its bronze blade is drawn by.
///
/// # Invariants
///
/// **Both are the same geometry.** The two exist only because a skin is per template and this builder gives one
/// template one skin; the mesh is authored once by [`oak_leaf`] and instanced under two tints. See the module docs
/// on why that is an approximation and not the note's two-faced leaf.
pub const ALPINE_OAK_GREEN: usize = 0;
pub const ALPINE_OAK_BRONZE: usize = 1;

/// The one pixel the whole-tree plate's framing makes, in metres: `2 · 25.0 · tan(0.55 / 2) / 1700`.
///
/// # Invariants
///
/// **This is the number that decides the plate's foliage, and it is stated here rather than left to the framing.**
/// A blade narrower than one pixel is dropped by a point-sampled rasteriser, so the blade a plate can draw is a
/// function of where that plate stands, and the note's 48 mm leaf is 5.8 of these. See the module docs.
pub const ALPINE_OAK_LEAF_PIXEL_METRES: f32 = 0.0083;

/// The oak's rule: a short gnarled bole, heavy persistent limbs that never self-prune, secondaries, and the short
/// stiff shoots the leaves crowd on.
///
/// # The numbers and where they come from
///
/// Every figure is [`quercus_aquifolioides.md`](../../../.agents/investigate/trees/quercus_aquifolioides.md)'s
/// modelling table. The tree is drawn at **12 m**, the low half of the note's **10(−20) m**, so that the crown fills
/// a frame and the bole under it is still read as a bole. The limbs start at **0.16** of the bole and run to
/// **0.95** — the note's single biggest structural difference from the conifers in this set, that **an oak does not
/// self-prune into a clean pole**, so the foliage comes down the trunk instead of stopping above it. The branches
/// are **spiral**, `cluster = 1` and the golden-angle slot step, because oaks are characteristically alternate and
/// the note's whorled tiers are the fir's and the spruce's. Branch attitude is a law over
/// [`Joint::station`]: the note's **40–80°** ESTIMATE runs from nearly horizontal on the lowest limb to ascending on
/// the topmost, which is *"ascending to horizontal, the crown building as a rounded mass"*.
pub fn alpine_oak_rule() -> Rule {
    Rule {
        trunk: gnarled_bole,
        generations: vec![
            // **The heavy limbs.** Eight to ten of them, spread over the bole's upper four fifths and *not* pruned
            // off its lower part: this is the whole difference between an oak and a pine's clean bole, and it is one
            // band's near end. Their length is a law over the station they hang at, which is what
            // `Joint::station` exists for.
            Generation {
                count: (14, 18),
                station: (0.15, 0.97),
                slot: (0.0, GOLDEN),
                jitter: (0.07, 0.10),
                sink: 0.05,
                cluster: 1,
                shape: limb,
            },
            // **The secondaries.** Fourteen to nineteen to a limb. **These two counts are a pair and they trade.**
            // With ten limbs and eighteen secondaries the crown's *outline* was a star: ten leafy strips with the sky
            // between them, and no amount of leaf on those strips closes a gap that is a metre wide. With sixteen limbs
            // and seventeen secondaries the outline closes and the strips meet inside it. The crown's mass is therefore
            // bought at the limb, and the secondaries are what fill what the limbs have already bounded.
            Generation {
                count: (16, 21),
                station: (0.12, 0.96),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.18),
                sink: 0.012,
                cluster: 1,
                shape: secondary,
            },
            // **The shoots.** Twenty-six to thirty-four a secondary, at the note's ESTIMATED **5–15 cm**. **This
            // count is the crown**: it is the whole of H1's silhouette and of note 1's *"dense, rounded"* because
            // the leafing is now one instanced mesh per shoot and the budget is spent on shoots. A secondary is a
            // metre long and carries thirty of them, which is a twig every three and a half centimetres — the texture
            // a sclerophyllous evergreen crown reads by, and the thing no number of leaves on sixteen bare limbs could
            // have stood in for.
            Generation {
                count: (26, 34),
                station: (0.14, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.09, 0.22),
                sink: 0.004,
                cluster: 1,
                shape: shoot,
            },
        ],
        // **No field shapes this tree, and that is a measurement rather than a taste.**
        //
        // The first plate carried `Field::light([0, 1, 0], 0.22)` — a weak tropism turning each shoot toward
        // the light. **A plant field is applied every time anything reads a node's spine**, and the two things
        // that read a spine are a leaf's placement and the wood's own sweep. Measured on this crate, 2026-10-19,
        // on the bough plate's own plant and nothing else changed: the same scene builds in **2.9 s** without the
        // field and **15.4 s** with it — **3.3 ms per leaf instance**, because the field re-runs for every
        // ancestor of the node the leaf hangs from. A crown this species needs is forty thousand leaf instances,
        // and the tropism alone would then cost **two and a half minutes** before a single row of the scene had
        // been compiled.
        //
        // **What is lost is a 21 mm bend on a 95 mm shoot** — the field's own reach is its amount times the
        // spine's length — which is **two and a half pixels** at the whole-tree plate's own distance and is
        // below what the frame resolves anyway. What keeps the shoots reaching for the light is the [`shoot`]
        // law's own departure, which already turns a shoot up through its length, and the crown's [`pitch`]
        // law, which splays a sprig off the twig and closes it along the shoot. The tree's own spines carry the
        // whole of it.
        fields: Vec::new(),
    }
}

/// The bole: **10 m**, a flare at the ground, and a wander of a metre and a half up its length.
///
/// # The gnarled spine
///
/// The spine is **authored in the rule's own trunk** rather than baked onto the grown plant with
/// [`vegetation::ops::Change::Spine`], and the reason is that the bake *replaces* a curve the rule has already
/// computed — so it costs an edit and buys nothing here, where there is no computed curve worth keeping. What it
/// buys instead is the two things H4 asks of wood: a **flare** and a **taper that is not a cone**.
///
/// The wander is what keeps this off the *polygonal silhouette* and *straight pole* faults: eight knots with
/// knots' tangents a little over half a segment apart, so the curve **kinks** at each one instead of interpolating
/// through it — a trunk that leans, is thrown back, doubles under itself at five metres and comes out forward again.
fn gnarled_bole() -> (shape::Curve, shape::Scalar) {
    (
        // **Eight knots over ten metres, each knot a knot.** The wander is two metres in `x` and one and a half
        // in `z` against a ten-metre height, so the bole leans out, is thrown back, doubles under itself at four and a
        // half metres and comes forward again: a trunk with elbows, which is what *"gnarled, contorted"* means in
        // geometry. The tangents are short beside their segments on purpose — a Hermite whose tangent is half a
        // segment long kinks at the knot, and one whose tangent is longer interpolates through it and comes out a
        // leaning pole. The height is the note's own **10 m** — *"Trees 10(−20) m tall"*, the small end of its range
        // — because the whole tree has to be **wider than tall** and a taller bole cannot carry a wider crown inside
        // the note's 6–12 m without the crown becoming a spire.
        spine(&[
            ([0.00, 0.00, 0.00], [0.95, 0.82, 0.25]),
            ([0.55, 1.18, 0.25], [0.75, 0.77, -0.60]),
            ([0.95, 2.82, -0.55], [-0.55, 0.77, -0.55]),
            ([0.75, 4.27, -1.35], [0.80, 0.73, 0.45]),
            ([1.25, 5.73, -1.15], [0.60, 0.77, 0.60]),
            ([2.00, 7.32, -0.35], [-0.35, 0.82, -0.30]),
            ([2.15, 8.91, -0.65], [0.15, 0.90, 0.05]),
            ([2.20, 10.00, -0.62], [0.00, 0.91, 0.00]),
        ]),
        // **A flare in the first two knots and a long shallow middle.** The note ESTIMATES a trunk radius of
        // 0.20–0.30 m at the base and a d.b.h. of 0.4–0.6 m; this stands 0.40 m at the ground because H4 asks for a
        // flare and a root flare *is* wider than the bole it grows on, then falls to 0.245 m within a tenth of the
        // height and holds near 0.19 m to the first branching. It thins on a law that is not a straight line from
        // base to tip, which is the second half of the same test.
        taper(&[
            (0.000, 0.400),
            (0.022, 0.288),
            (0.090, 0.245),
            (0.300, 0.196),
            (0.560, 0.152),
            (0.820, 0.098),
            (1.000, 0.042),
        ]),
    )
}

/// The bole's section: twenty-four sides with eight shallow flutes between them.
///
/// # Invariants
///
/// **The flutes are vertical and the taper multiplies them.** A flute is a groove that runs *with* the grain, so it
/// is a radius law around the section and not a ripple along the trunk, and the taper's own law narrows every lobe
/// as the bole narrows. Eight lobes on twenty-four sides is two and a half sides a lobe, which is under the flat a
/// section can carry without the lobes reading as facets.
///
/// **Twenty-four is a number the sweep has to be told twice.** A section is *sampled*, so the lobes only exist in
/// the frame if [`OAK_DETAIL`]'s `around` is at least the side count; at the crate's own default of twelve the
/// twenty-four authored sides are twelve drawn ones and the flutes alias into a shape that was never authored.
/// That is the whole of what the detail dial is for on this species, and the base plate is where it is read.
pub fn fluted_bole() -> shape::Section {
    let sides = 24;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let flute = 1.0 - 0.12 * (theta * 4.0).sin().powi(2);
            [theta.cos() * flute, theta.sin() * flute]
        })
        .collect();
    shape::Section::outline(&points).expect("a fluted ring is a section")
}

/// A first-order limb: a heavy member that leaves the bole at the note's **40–80°**, runs out and slightly down,
/// and turns up at its tip.
///
/// # Invariants
///
/// **The departure is a law over the station the limb hangs at**, and it is the crown's profile in one number: the
/// lowest limb leaves at nearly a right angle and reaches far out to hold the crown's spread, the topmost leaves
/// steeply and shortens. The four knots turn through a sixth of a turn and then back through half of one, so a limb
/// is a **bent member with a knuckle**, not a cone on a stick — which is the *heavy limbs* half of the silhouette
/// this species is drawn for.
fn limb(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 3.20 * (1.06 - 0.40 * joint.station) * (0.82 + 0.36 * rng.unit());
    // The note's 40–80° ESTIMATE, and it runs the way a broadleaf's does rather than a conifer's: the **lowest**
    // limb is the flattest and the topmost the steepest, because the crown's spread is carried low and its dome is
    // carried high. A limb that leaves at seventy-nine degrees reaches four metres out and a limb that leaves at
    // forty-five reaches one and a half, and the envelope is what stops the first of them from leaving the dome.
    let departure = 1.55 - 0.78 * joint.station + 0.10 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.18,
                departure - 0.10,
                departure - 0.42,
            ],
        ),
        taper(&[
            (0.00, radius * 0.40),
            (0.30, radius * 0.29),
            (0.68, radius * 0.15),
            (1.00, 0.007),
        ]),
    )
}

/// A secondary: the member that fills the crown's volume, from a metre and a half long where it leaves a limb's
/// **base** to half a metre where it leaves its **tip**.
///
/// # Invariants
///
/// **The departure is a law over the station on the limb, and it is the crown's third dimension.** At the limb's
/// foot a secondary leaves at twenty-five degrees from the limb's own heading and is the longest member in the
/// crown, so it rises into the interior and makes the crown's top; at the limb's tip it leaves at seventy degrees and
/// is short, so it stands out sideways and makes the crown's skirt. That single law is what turns nine limbs into
/// one rounded mass: with every secondary leaving at one angle, as the first plate of this species had them, the
/// crown was nine flat plumes with the sky between them.
fn secondary(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    // **1.80 m at its middle, and it was 1.45 m.** A limb is three and a half metres long and a secondary a
    // third of that, so two thirds of every limb in this crown was **bare wood** and the foliage stood in a
    // tuft at its far end — sixteen leafy shelves with the sky between them, which is exactly what H1's cut-out
    // showed and exactly what the note's *"continuous, dense, rounded mass with no whorl tiers"* is not. The
    // secondaries are what fill what the limbs have already bounded, and at thirty-four per cent of a limb they
    // bounded nothing.
    let length = 1.80 * (1.05 - 0.40 * joint.station) * (0.72 + 0.55 * rng.unit());
    let departure = 0.28 + 1.05 * joint.station + 0.22 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.26,
                departure - 0.08,
                departure - 0.34,
            ],
        ),
        // **Less than half the parent's radius.** A secondary thicker than that is a limb with leaves on it, and a
        // crown of limbs with leaves on them is the *bare broom* the defect list names — the second plate of this
        // species had secondaries a sixth of a metre through and its crown read as grey wire with confetti on it.
        taper(&[(0.00, radius * 0.45), (0.45, radius * 0.28), (1.00, 0.0028)]),
    )
}

/// A shoot: the stiff little axis the note's leaves crowd on, **5.5–14.5 cm** against its ESTIMATE of 5–15 cm.
///
/// # Invariants
///
/// **The length law is the note's and the count is the crown's.** The five-to-fifteen centimetre figure is the
/// species' own, and it is short: a shoot this size carries seven or eight leaves of six centimetres, which overlap
/// several deep, and that overlap is precisely what makes the crown a mass rather than a cloud of confetti. The
/// departure is a law over the station on the secondary for the same reason the secondary's is: a shoot standing
/// straight out of a limb's interior and a shoot lying along a limb's tip do not fill the same part of the crown.
fn shoot(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.10 * (0.55 + 0.90 * rng.unit()) * (1.06 - 0.26 * joint.station);
    let departure = 0.70 + 0.55 * joint.station + 0.30 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.26,
                departure + 0.34,
                departure + 0.20,
            ],
        ),
        taper(&[(0.00, radius * 0.70), (1.00, 0.0018)]),
    )
}

/// Four knots along a heading that turns through `angles`, measured from the parent's own tangent: nought is along
/// the parent and a quarter turn is straight out. The tangents are the heading's own direction, so a member is a
/// chord with three bends in it rather than a straight line with a direction.
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

/// The oak leaf: **48 × 23 mm**, the middle of the note's FOC range (**2.5–7 cm × 1.5–3.5 cm**), obovate and
/// blunt-tipped.
///
/// # Why the middle, and not the maximum
///
/// **The first plate of this species drew the leaf at FOC's maximum — 70 × 34 mm — and bought its crown
/// coverage with size instead of with count**, which is the same bargain the pine made with its spray and
/// for the same reason: this builder's cost in instances is superlinear, so a crown is bought with area
/// per instance and there is always a cheaper crown available at a larger blade. [`LeafTemplate::sprig`]
/// removed that bargain. One instanced mesh now holds a whole shoot's leafing, so **the count that costs
/// is the count of shoots and not the count of leaves**, and the leaf goes back to the size a leaf is.
///
/// # The shape the width law carries, and the three it cannot
///
/// - **Obovate** is the widest point *above* the middle, which a width law over the midrib says directly: the law
///   peaks at 0.52 and not at 0.5.
/// - **An obtuse apex** is the law *not* reaching nought at the tip — the last knot of the width is 3.4 mm of half
///   width and the sweep's own cap closes it, so the blade ends in a round rather than a point.
/// - **The petiole** — FOC's **2–5 mm**, *"sometimes nearly sessile"* — is the width law's flat start: the blade is
///   under three millimetres wide for its first six, which at the bough plate's distance is one pixel and is
///   therefore drawn rather than separately modelled.
/// - **The cordate base** is a notch, and a width law cannot make a notch; the base is rounded here, which is the
///   other state the note gives.
/// - **The spiniform margin** is a millimetre of serration and is not drawn. See the module docs.
///
/// The blade is a **closed solid** (D18), so both faces carry their own normal and a leaf lit from behind is shaded
/// rather than faked, and the **4.5% thickness ratio** is a sclerophyll's: a leathery leaf of a hard-leaved evergreen
/// oak is about a millimetre thick over a two-and-a-half-centimetre width.
///
/// # Invariants
///
/// **The numbers below are authored once, at the note's maximum, and every one of them is scaled by
/// [`LEAF_LENGTH`] to reach the drawing.** A blade is a midrib and a width law, and both are in metres, so
/// scaling all of it at once is the same blade at another size — and authoring it at one size means the
/// obovate peak, the blunt tip and the petiole's flat start stay one edit rather than becoming four.
fn oak_leaf() -> LeafTemplate {
    let at = LEAF_LENGTH / LEAF_LENGTH_AUTHORED;
    let midrib = spine(
        &LEAF_MIDRIB
            .iter()
            .map(|(knot, tangent)| (by(*knot, at), by(*tangent, at)))
            .collect::<Vec<_>>(),
    );
    let width = shape::Scalar::new(
        &LEAF_WIDTH
            .iter()
            .map(|(along, half)| (*along, half * at))
            .collect::<Vec<_>>(),
    )
    .expect("a leaf's widths are a law");
    LeafTemplate::blade(midrib, width, LEAF_THICKNESS, LEAF_STATIONS, LEAF_AROUND)
}

/// A vector scaled by `factor`, which is the whole of [`oak_leaf`]'s sizing.
fn by(vector: [f32; 3], factor: f32) -> [f32; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

/// Two vectors added elementwise, which is the whole of the roll in [`sprig_blade`].
fn sum(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

/// The blade's length as it is drawn: **48 mm**, the middle of the note's **2.5–7 cm**.
///
/// # Invariants
///
/// **This is one number and the blade is drawn at it**, so the note's leaf size and the crown's cost of it are the
/// same dial. [`ALPINE_OAK_LEAF_PIXEL_METRES`] is what it measures at on each plate.
pub const LEAF_LENGTH: f32 = 0.048;

/// The length [`LEAF_MIDRIB`] and [`LEAF_WIDTH`] are authored at, which is the note's **70 mm** maximum.
const LEAF_LENGTH_AUTHORED: f32 = 0.070;

/// The blade's midrib and its own tangent at each knot, authored at [`LEAF_LENGTH_AUTHORED`].
///
/// # Invariants
///
/// **A leaf of an evergreen oak curves**: the blade arches off the petiole, flattens through its middle and
/// turns up at the tip, which is the *obovate* the note describes and what keeps a crown from reading as a
/// set of flat cards. The tangents are half a segment long, so the arch is a curve and not three straight
/// pieces.
const LEAF_MIDRIB: [([f32; 3], [f32; 3]); 4] = [
    ([0.0000, 0.0000, 0.0000], [0.0250, 0.0032, 0.0000]),
    ([0.0250, 0.0025, 0.0000], [0.0232, 0.0047, 0.0009]),
    ([0.0480, 0.0062, 0.0013], [0.0170, 0.0047, 0.0025]),
    ([0.0700, 0.0116, 0.0046], [0.0054, 0.0016, 0.0028]),
];

/// The blade's half width along its own length, authored at [`LEAF_LENGTH_AUTHORED`]: a bare foot, the
/// obovate peak above the middle, and a tip that stops short of a point.
const LEAF_WIDTH: [(f32, f32); 7] = [
    (0.00, 0.0009),
    (0.09, 0.0022),
    (0.20, 0.0094),
    (0.52, 0.0170),
    (0.78, 0.0152),
    (0.93, 0.0106),
    (1.00, 0.0050),
];

/// The lens's half thickness as a fraction of the half width: a sclerophyll's leathery blade.
const LEAF_THICKNESS: f32 = 0.045;

/// Rings along a blade's midrib. Six rings on a 48 mm leaf is one every ten millimetres, which is under the
/// pixel at the whole-tree plate and about a sixth of the leaf at the bough plate — and **a blade in a sprig
/// pays this nine times**, so the count is the one tessellation number in this module that is a cost.
const LEAF_STATIONS: usize = 6;

/// Vertices around a blade's lens. Four is the smallest ring that carries **two flat faces and a closed
/// solid** (D18); three would stand the blade on edge.
const LEAF_AROUND: usize = 4;

/// The leafing a shoot of this oak carries: **nine blades** on a **95 mm** axis.
///
/// # Why a shoot and not a leaf
///
/// **FOC describes the arrangement, and this crate now has one mesh that can hold it.** The note's petiole
/// is **2–5 mm**, *"sometimes nearly sessile"*, so a shoot's leaves sit almost directly against the twig at a
/// **golden-angle** step around it — a fixed arrangement, which is precisely what [`LeafTemplate::sprig`]
/// exists to express. Instancing the arrangement rather than the blade turns **nine leaves into one row**,
/// and the crown's budget is then spent on **shoots**, which is what a crown is made of.
///
/// Nine is the top of the range the brief allows and not an arbitrary number: a ten-centimetre shoot carrying
/// leaves of four and a half centimetres stands about nine of them before its own length stops fitting more, which
/// is the arrangement `03-branch.jpg` shows.
const SPRIG_LEAVES: usize = 9;

/// The length of the axis a sprig's blades stand on, against the **5.5–14.5 cm** [`shoot`] draws.
///
/// # Invariants
///
/// **A sprig is one shoot's leafing, so its axis is the shoot's own length and not a free parameter.**
/// [`shoot`] draws 0.055–0.145 m; this is 0.095 m, the middle, and the field's `size` law carries the rest.
const SPRIG_AXIS: f32 = 0.095;

/// How far a blade stands off the axis at the shoot's foot, in radians from it, and how much less at its tip.
///
/// # Invariants
///
/// **A blade at ninety degrees is a feather, and one along the axis is a scale**: a shoot's own leaves splay at
/// its foot and turn up at its tip (`03-branch.jpg`), so the ray is the widest at nought and closes by a
/// quarter turn at the end of the axis.
const SPRIG_RAYS: [f32; 2] = [1.38, 0.96];

/// How far a blade is rolled about its own midrib, from the shoot's foot to its tip, in radians.
const SPRIG_ROLL: f32 = 1.9;

/// How much of that roll is jittered blade to blade, in radians.
///
/// # Invariants
///
/// **Eight blades in one fixed arrangement is eight blades with one arrangement**, and a sprig whose eight
/// faces all lie in a plane reads as a propeller rather than as a shoot. The jitter is a golden-angle
/// sinusoid over the blade's rank, so it is a different number for every blade and repeats with the sprig.
const SPRIG_ROLL_JITTER: f32 = 0.62;

/// The oak's own shoot, as one instanced mesh: [`SPRIG_LEAVES`] blades on an axis of [`SPRIG_AXIS`].
///
/// # Invariants
///
/// **It is the same blade [`oak_leaf`] builds, placed — never a second mesh of its own.** The green and the
/// bronze differ in their skin and in nothing else, so a leaf that reads on the bough plate is the leaf the
/// tree is made of, which is what makes the bough plate evidence about the tree.
fn oak_sprig() -> LeafTemplate {
    LeafTemplate::sprig(
        (0..SPRIG_LEAVES)
            .map(|index| (oak_leaf(), sprig_blade(index)))
            .collect(),
    )
}

/// One blade of the sprig: where along the axis it stands, how far round the axis, and the frame it stands in.
///
/// # The frame
///
/// **The axis runs along the template's `+x`, so `forward` is the blade's midrib and the other two are its own
/// width and face.** With `slot` about the axis and `rays` off it, the three are the three of an orthonormal
/// frame built by hand:
/// `forward = cos(rays) · x + sin(rays) · radial`, `across = −sin(rays) · x + cos(rays) · radial` (which is
/// perpendicular to `forward` by construction, where the naive `−sin·x` alone is not), and
/// `face = forward × across = x × radial`, the direction *round* the axis. **The roll is the one freedom left
/// and it is the whole of the sprig's variety**: it turns width and face about the midrib, so no two of the
/// eight blades present the same side to the light.
fn sprig_blade(index: usize) -> Placement {
    let rank = index as f32;
    let along = (rank + 0.5) / SPRIG_LEAVES as f32;
    let turn = rank * GOLDEN * std::f32::consts::TAU;
    let (slot_sin, slot_cos) = turn.sin_cos();
    let rays = SPRIG_RAYS[0] + (SPRIG_RAYS[1] - SPRIG_RAYS[0]) * along;
    let (ray_sin, ray_cos) = rays.sin_cos();
    let forward = [ray_cos, ray_sin * slot_cos, ray_sin * slot_sin];
    let across = [-ray_sin, ray_cos * slot_cos, ray_cos * slot_sin];
    let face = [0.0, -slot_sin, slot_cos];
    let roll = SPRIG_ROLL * (along - 0.5) + SPRIG_ROLL_JITTER * turn.sin();
    let (roll_sin, roll_cos) = roll.sin_cos();
    Placement {
        origin: [along * SPRIG_AXIS, 0.0, 0.0],
        x: forward,
        y: sum(by(across, roll_cos), by(face, roll_sin)),
        z: sum(by(face, roll_cos), by(across, -roll_sin)),
    }
}

/// The oak's fruit: a **1.4 cm stalk**, the note's **5–6 cm** shallow cupule at **1.2 cm** across, and the nut's
/// shoulder standing proud of the rim.
///
/// # Invariants
///
/// **A seat expresses no rotation**, so the hanging is the template's own shape, drawn once: the stalk runs out
/// along the surface's own outward axis and then the cupule and nut turn and hang. Every acorn the scatter seats
/// therefore hangs the same way from whatever side of the shoot it lands on, which is the same bargain the pine's
/// cone makes and for the same reason.
///
/// The waist at the rim is the note's field mark in the only form geometry can carry: *"the cupule is roughly three
/// times the length of the nut it holds"* and *"bracts … often free from the cupule wall"*, so the nut sits in a
/// shallow cup and its own shoulder stands above it. The **fringe** itself is two millimetres and is not drawn —
/// see the module docs.
fn acorn() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0000, 0.0000, 0.0], [0.0150, 0.0000, 0.0]),
            ([0.0150, -0.0020, 0.0], [0.0050, -0.0110, 0.0]),
            ([0.0198, -0.0180, 0.0], [0.0005, -0.0230, 0.0]),
            ([0.0205, -0.0400, 0.0], [0.0000, -0.0210, 0.0]),
            ([0.0205, -0.0600, 0.0], [0.0000, -0.0120, 0.0]),
            ([0.0205, -0.0740, 0.0], [0.0000, 0.0040, 0.0]),
        ]),
        shape::Section::circle(),
    );
    // The scale law is the fruit's own three parts: a hair-thin stalk, the cupule's shoulder opening and its widest
    // place, a waist at the rim, the nut's shoulder, and the nut's apex.
    sweep.scale = taper(&[
        (0.00, 0.0016),
        (0.17, 0.0018),
        (0.34, 0.0044),
        (0.52, 0.0061),
        (0.72, 0.0056),
        (0.84, 0.0044),
        (0.90, 0.0060),
        (0.96, 0.0044),
        (1.00, 0.0010),
    ]);
    Template {
        sweep,
        stations: 10,
        around: 8,
    }
}

/// Sprigs to the metre of shoot at the densest point of the shoot's own leafing law.
///
/// # Invariants
///
/// **This is the crown's area, and the unit it is counted in is what [`LeafTemplate::sprig`] changed.** One sprig
/// is [`SPRIG_LEAVES`] blades, so this law counts **shoots' worths of leafing** where its predecessor counted
/// blades. Sixty-six to the metre over a 0.095 m shoot is **five and a half sprigs a shoot** — fifty blades, one
/// every two millimetres of a ten-centimetre shoot — which is what `03-branch.jpg` shows and what
/// the note asks for: *"held densely so the crown is a solid rounded mass with a hard edge, not a feathered cloud"*.
///
/// **It is high because the plate is read at twenty-five metres**, where a 48 mm leaf is five and three quarter
/// pixels long, so the crown's solidity is bought at the size of its own grain and not by drawing leaves large.
/// The whole-tree plate's coverage, measured by `audit/frame_stats.py --crown`, is the number that
/// [`SHOOT_SPRIGS_PER_METRE`] moves, and the tree is drawn at **0.53** against the first pass's **0.219** and the
/// note's *"dense, rounded … a hard outer edge"*. **It was 66, and H1 moved it to 72 in the same pass that
/// filled the crown's interior** — the outer edge was already solid and the gaps were between the tiers, so
/// the density went where the light was not — and then **back to 66**, when the crown's axis turned out to be
/// where the one hole the cut-out still carried was. A thousand rows spent on the axis are worth more to a
/// reader than three thousand spent on an edge that was already dense. Thirty-eight is the same crown at
/// **0.453**, and every row between
/// the two numbers is what separates *"lacy"* from *"dense"* in this bar. The cost of each is stated where it is
/// paid: this crate re-solves a node's spine for every leaf that hangs from it, so the scene build is about
/// **4 ms per row** and this density is the reason the whole-tree plate takes minutes rather than seconds.
const SHOOT_SPRIGS_PER_METRE: f32 = 66.0;

/// Sprigs to the metre of the **leader's own** length, at the crown's axis: [`LEADER_SPRIGS_PER_METRE`]'s
/// field is the fourth of [`oak_leaves`] and it is the cheapest of the five.
///
/// # Invariants
///
/// **It is a rod and not a crown, and it is the last of H1's three numbers.** One hundred and forty to the
/// metre over the leader's own foliated seven and a half metres is about **a thousand sprigs**, against the
/// fourteen thousand the shoots carry, so the rod is **seven per cent of the crown** — and at a sprig's own
/// hand's width it is about a metre across, which is what it has to be: the ten-sheet's cut-out carried an
/// enclosed hole **2.1 m wide and three metres tall** down the middle of the crown, and it is not a gap in
/// the foliage — it is the **bole and the air around it**, which the identity mask cannot record because a
/// node's id of one rounds out of eight bits. Nothing hung on the limbs or the shoots can close it, because
/// a limb at four metres goes *out*, and what is missing is leaves standing against the axis itself.
///
/// **What it is and is not.** It is not a second trunk of leaves — the note's *02-crown.jpg* has the pale
/// grey limbs rising **through** this canopy, and a green pole would be a lie. It is the crown **closing
/// around its own axis**, which is what *"the crown is carried low, on persistent branches"* asks for, and
/// the band stops at `0.24` — two and a half metres up a ten-metre bole — so the metre the bar's H4 reads is
/// still bare wood.
const LEADER_SPRIGS_PER_METRE: f32 = 140.0;

/// Bronze sprigs to the metre of shoot: a little under one a shoot, which is the **underside seen** rather
/// than a second crown.
///
/// # Invariants
///
/// **The bronze is a fifth of the green and it is aimed at the ground**, because that is the whole of what one
/// skin per template can say about *"densely with reddish brown to orangish brown stellate hairs … abaxially"*
/// — the crown's lower and inner surface is where the note says the wind shows it, and everywhere else the
/// green is what the eye goes to. It is an approximation and the module docs say so.
const SHOOT_BRONZE_PER_METRE: f32 = 5.0;

/// The leafing law along a shoot, in sprigs per metre of shoot: a bare foot, a dense middle, a thinning tip.
///
/// # Invariants
///
/// **This law is the answer to the *dead give*.** Foliage spread evenly from a shoot's very base to its very tip
/// is the defect the bar names, and a shoot is not that: the foot is bare where it leaves its parent, the middle
/// is where the mature leaves stand shoulder to shoulder, and the tip carries the few young ones. The band on the
/// field that uses it stops short of both ends as well, so the first and last stations of the law are never
/// reached.
fn shoot_sprigging() -> shape::Scalar {
    let profile = [
        (0.00, 0.55),
        (0.28, 1.00),
        (0.62, 0.95),
        (0.88, 0.58),
        (1.00, 0.22),
    ];
    let knots: Vec<(f32, f32)> = profile
        .iter()
        .map(|(at, shape)| (*at, shape * SHOOT_SPRIGS_PER_METRE))
        .collect();
    shape::Scalar::new(&knots).expect("a shoot's leafing is a law")
}

/// The five fields the oak's foliage is made of, at the shoot depth `shoot_depth`.
///
/// # The five, and what each is for
///
/// - **`leader`** is the crown closing around its own axis: the oak does not self-prune, so its bole runs up
///   through its own foliage, and this is the field that says so. See [`LEADER_SPRIGS_PER_METRE`] for what it
///   costs and for the one thing H1 cannot see about it.
/// - **`crown`** is the mass: the note's green shoot's leafing on the shoots themselves, at the law above.
/// - **`bronze`** is the note's **rusty-brown underside** as far as one skin per template can carry it — a sparser
///   second layer of the same blade, **aimed at the ground**, so the bronze shows on the crown's lower surface where
///   the note says the wind shows it (`05-leaf-underside.jpg`, `10-leaf-underside-scale.jpg`). It is an
///   approximation and the module docs say so.
/// - **`inner`** is the shaded interior on the secondaries: older, smaller, bronze leaves deep in the crown, which
///   is what keeps the crown reading as a volume rather than as a shell of one tint.
/// - **`limb`** is the leaves on the heavy limbs' own length, sparse, and the reason the foliage visibly **starts low
///   on the trunk** instead of beginning where the shoots do.
///
/// A bough plate has three orders below its root and not four, so the limb and leader fields exist only where the
/// limbs are two generations above the shoots — the same guard the pine's candle field carries, for the same reason.
fn oak_leaves(shoot_depth: usize) -> Vec<LeafField> {
    // **The leader's own leafing, and it is the fourth place the note's second recognition fact is written.**
    // The ten-sheet's cut-out carried a hole at mid-crown with the sky showing through it directly over the
    // bole, and nothing in the rule above put foliage there: every other field hung on a node at or below the
    // *limbs*, so the leader's own length stood bare and the crown was two shelves and a gap.
    // on the tree's own axis.
    //
    // **It is cheap, and H1 cannot see it, and both facts are stated here.** Six and a half metres of leader at
    // a hundred and forty sprigs to the metre is about a thousand rows against the fourteen thousand the crown
    // spends on shoots, so the cost is a seventh of the plate. The invisibility is the instrument's: **every
    // leaf on the leader hangs on the root node, whose `authored_id` is one, and one in eight bits of red rounds
    // to nothing.** Measured, not inferred — raising this density from eighty-five to a hundred and forty, and
    // the band from `0.30` to `0.24`, moved the tree plate's cut-out by less than one per cent of its pixels,
    // while the plate itself shows a leafed trunk from two and a half metres up where it had shown bare wood.
    // **So the hole H1 named over this bole cannot be closed by closing it**: the reader's instrument has no
    // way to record what is there, and the second largest enclosed region left in the cut-out — thirty-two
    // thousand pixels, two metres wide, between two and a half and five and a half metres up — **is** the bole
    // and the air around it, drawn on the plate and invisible to `audit/silhouette.py`.
    let mut leader = LeafField::new(0, ALPINE_OAK_GREEN);
    leader.band = (0.24, 0.99);
    leader.density = shape::Scalar::constant(LEADER_SPRIGS_PER_METRE);
    leader.phyllotaxy = GOLDEN;
    leader.cluster = 1;
    leader.fan = shape::Scalar::constant(0.0);
    // The same pitch and twist the crown uses, a little more open: a leaf on a vertical stem stands further
    // off its own axis than one on a horizontal twig, and the sprig has to clear the bole to read at all.
    leader.pitch = shape::Scalar::new(&[(0.00, 0.70), (0.45, 0.88), (1.00, 1.08)])
        .expect("a leader's pitch is a law");
    leader.twist = shape::Scalar::new(&[(0.00, -0.44), (0.50, 0.10), (1.00, 0.66)])
        .expect("a leader's twist is a law");
    leader.size = shape::Scalar::new(&[(0.00, 0.66), (0.35, 0.86), (1.00, 1.00)])
        .expect("a leader's sprig is a law");
    leader.jitter = 0.19;

    let mut crown = LeafField::new(shoot_depth, ALPINE_OAK_GREEN);
    crown.band = (0.06, 0.94);
    crown.density = shoot_sprigging();
    crown.phyllotaxy = GOLDEN;
    // A cluster of one and a fan of nought: an oak's leaf is a leaf on a 2–5 mm petiole, not a fascicle, and the
    // cluster dial the pine needs is the identity here. The sprig is already a *set*, and it is the shoot's own
    // set — a field-level cluster on top of it would be a second arrangement over one arrangement.
    crown.cluster = 1;
    crown.fan = shape::Scalar::constant(0.0);
    // **The pitch is the sprig's own axis, and it is a law because a sprig is a whole shoot.** A sprig at a right
    // angle stands its star of blades square across the twig and the crown reads as bristles; a sprig along the
    // twig reads as a leafy shoot. It is nearly splayed at the shoot's foot and closes to fifty degrees at its
    // tip, which is `03-branch.jpg`: leaves out at the base, leaves turning up at the end.
    crown.pitch = shape::Scalar::new(&[(0.00, 0.44), (0.45, 0.70), (1.00, 0.92)])
        .expect("a crown's pitch is a law");
    // The twist turns the sprig about its own axis, so a crown of them has no two stars facing the same way — the
    // *plastic crown* defence, which a sprig needs twice over because its blades all start in one arrangement.
    crown.twist = shape::Scalar::new(&[(0.00, -0.62), (0.50, 0.08), (1.00, 0.72)])
        .expect("a crown's twist is a law");
    crown.size = shape::Scalar::new(&[(0.00, 0.74), (0.35, 0.92), (1.00, 1.08)])
        .expect("a sprig's size is a law");
    crown.jitter = 0.17;

    let mut bronze = LeafField::new(shoot_depth, ALPINE_OAK_BRONZE);
    bronze.band = (0.26, 1.00);
    bronze.density = shape::Scalar::constant(SHOOT_BRONZE_PER_METRE);
    bronze.phyllotaxy = GOLDEN;
    bronze.cluster = 1;
    bronze.fan = shape::Scalar::constant(0.0);
    bronze.pitch = shape::Scalar::constant(1.12);
    bronze.twist = shape::Scalar::constant(1.10);
    bronze.aim_axis = [0.0, -1.0, 0.0];
    bronze.aim = shape::Scalar::constant(0.34);
    bronze.size = shape::Scalar::constant(1.04);
    bronze.jitter = 0.20;

    let mut inner = LeafField::new(shoot_depth - BOUGH_SHOOTS + 1, ALPINE_OAK_BRONZE);
    inner.band = (0.14, 0.92);
    // **Fourteen sprigs to the metre, and it was seven.** This is the field that closes the crown's
    // *interior*, and it is the second half of H1's complaint: the ten-sheet's cut-out carried an enclosed
    // hole of **2 538 px** on the crown's left shoulder at six and a half metres, a patch of sky the border
    // could not reach, and every field that hangs on the *outside* of the crown — the shoots, the limbs —
    // is the wrong tool for it. **A second-order member is a metre and a half long and eight centimetres
    // round**; at two and a half sprigs to the metre it was bare wood with a tuft at its tip, and the
    // silhouette showed it. These blades are small and bronze and hang on the secondaries' whole length,
    // which is exactly where the gaps were.
    inner.density = shape::Scalar::constant(14.0);
    inner.phyllotaxy = GOLDEN;
    inner.cluster = 1;
    inner.fan = shape::Scalar::constant(0.0);
    inner.pitch = shape::Scalar::constant(1.04);
    inner.twist = shape::Scalar::constant(0.62);
    inner.size = shape::Scalar::constant(0.84);
    inner.jitter = 0.22;

    let mut limb = LeafField::new(shoot_depth - BOUGH_SHOOTS, ALPINE_OAK_GREEN);
    limb.band = (0.12, 0.97);
    // **Forty sprigs to the metre, and it was twenty.** This field is the second gap-filler and it is still
    // nearly free: sixteen limbs at three metres is under two thousand rows even doubled. It matters because
    // the limbs are the crown's *tier structure* — the shelves a dome must not have — and leafing each one
    // along its whole length is what turns sixteen shelves into one mass. **At twenty a metre the plate
    // still showed the shelves**: the cut-out carried a notch at the crown's shoulder and another at its
    // foot, where a limb runs a metre and a half before its first tuft. The band came in to `0.12` at the
    // same time, because the bare wood H1 read is the limb's **first metre**, not its middle.
    limb.density = shape::Scalar::constant(40.0);
    limb.phyllotaxy = GOLDEN;
    limb.cluster = 1;
    limb.fan = shape::Scalar::constant(0.0);
    limb.pitch = shape::Scalar::constant(1.12);
    limb.twist = shape::Scalar::constant(0.35);
    limb.size = shape::Scalar::constant(0.90);
    limb.jitter = 0.20;

    // A bough plate's root is a limb, so a field that would hang on the limbs — or on the leader above them —
    // would hang on the bough's own axis and put leaves on the wood the plate is looking along. Both are
    // built here and dropped there, so the fields are one function and not a function plus a special case.
    if shoot_depth >= SHOOTS {
        vec![leader, crown, bronze, inner, limb]
    } else {
        vec![crown, bronze, inner]
    }
}

/// The crown the trim holds the oak to: a dome, nothing at all below the crown's skirt, and nothing above its tip.
///
/// # Invariants
///
/// **The radius is the crown's own outline as a law over the trunk**, and it is what turns a bag of branches into
/// the note's *"dense, rounded to broadly domed"* crown: **4.3 m** at just under half the height, **4.2 m** two
/// thirds of the way up and only reaching nought at the bole's own tip. The two figures are the note's ESTIMATES — a
/// crown **6–12 m** across and **4–10 m** deep on a **10(−20) m** tree — read above their narrow end, because the
/// plate is judged against *"a dense, rounded, dark evergreen crown … a compact, rounded, solid mass"* and a crown
/// narrower than it is deep reads as a spire with leaves on it.
///
/// **The width is the point, and it is a ratio and not a number.** The brief for this second pass asks for *"the
/// dome: wider than tall"*, and a dome's width is its radius held flat while its height is where the radius falls
/// away. This law holds above four metres from a third of the height to two thirds of it, so the crown is **8.6 m
/// across** over a foliage run of **8.5 m** — wider than tall by a tenth, and a tenth is what the note's *rounded*
/// costs against a crown that is exactly round. The skirt that comes down the trunk below the bulge is the note's
/// *"the crown is carried low, on persistent branches"*, and it is deliberately **not** counted as the dome's height:
/// an oak that self-pruned would show eight and a half metres of bole under exactly this crown.
///
/// **The last three knots are what make it a dome and not a whip.** The radius holds to three quarters of
/// the bole and only then falls, so the crown's top is a shoulder the topmost limbs build rather than a
/// point the envelope cuts: the plate before this one had the radius reach nought at four fifths and came out
/// with three bare metres of bole and a tuft on the end of it.
///
/// **The top does not reach nought, and the reason is measured rather than chosen.** [`limb`] turns a limb's
/// last knot up through nearly a radian, so a limb hanging at `0.95` of a ten-metre bole ends **1.4 m above
/// the bole's own tip** — and a radius law that reaches nought at `1.0` measures that tip against the bole's
/// end, finds it outside, and takes the branch back to the crate's **one-tenth floor**: a limb crushed to a
/// tenth of its length with all its leaves on a decimetre of twig. **The crown's top third was being cut by
/// its own guard**, which is where the ten-sheet's hole at mid-crown and its lopsided top came from. **1.05 m
/// at the bole's tip** admits that shoulder — a metre each side of the leader — and the dome keeps its round
/// because the radius still falls from **3.50 m** at three quarters of the height to **2.55 m** at fourteen
/// tenths of it, a fall of nearly a metre over a metre and a half of bole.
///
/// **The envelope is a guard and not a carver, and the module says so.** `trim` shortens a node by solving for the
/// scale at which its own tip lands inside, and a shoot is ten centimetres long: an envelope two metres tighter
/// than the architecture solves that scale at one tenth, which is the crate's floor, and the shoot survives at a
/// tenth of its length with all of its leaves crushed onto a centimetre of twig. The third plate of this species
/// was drawn through an envelope tighter than its architecture and came out as bare wire with tufts on it. So the
/// radius law here is drawn to sit just outside what the rule grows, the trim takes the few per cent of the frontier
/// that overshoots, and the dome is the **limb's departure law and the secondary's station law** working together.
pub fn crown_envelope() -> Envelope {
    Envelope::radial(&[
        (0.00, 0.00),
        (0.11, 0.00),
        (0.16, 2.45),
        (0.33, 3.65),
        (0.50, 4.30),
        (0.67, 4.20),
        (0.85, 3.50),
        (0.94, 2.55),
        (1.00, 1.05),
    ])
}

/// The oak: its tree, its foliage, its four fields and the fruit its shoots carry.
pub fn alpine_oak(seed: u64) -> Species {
    let mut plant = alpine_oak_tree(seed);
    alpine_oak_dress(&mut plant);
    alpine_oak_species(plant)
}

/// Grows the whole-tree plate's skeleton: the rule and the seed, before any dressing.
///
/// **This is the plate's first stage and the only one that is not a function of a grown tree.** A host
/// outside this crate drives the stages separately, which is why the growth, the dressing and the species
/// are three names here rather than three statements inside [`alpine_oak`].
pub fn alpine_oak_tree(seed: u64) -> Plant {
    grow(&alpine_oak_rule(), seed)
}

/// Dresses a grown skeleton: the fluted bole, then the crown's envelope.
///
/// # Invariants
///
/// **The bole is dressed after the growth and the crown is cut after that.** A section is a node's own
/// and `grow` authors circles, so the flutes are the trunk's and not the rule's. The trim runs last
/// because it measures the tips the fluting and the bake have already settled.
pub fn alpine_oak_dress(plant: &mut Plant) {
    if let Ok(bole) = plant.get_mut(&Handle::root()) {
        bole.set_section(fluted_bole());
    }
    trim(plant, &crown_envelope());
}

/// The species a dressed skeleton draws with: two blade templates, the leaf fields and the fruit template.
///
/// # Invariants
///
/// Nothing here depends on the tree but the tree itself, so this is the stage a leaf field or a template
/// edit re-derives without touching the growth or the envelope.
pub fn alpine_oak_species(plant: Plant) -> Species {
    Species {
        plant,
        templates: vec![oak_sprig(), oak_sprig()],
        fields: oak_leaves(SHOOTS),
        decorators: vec![acorn()],
    }
}

/// The bough plate's species: one limb as the trunk, its secondaries and their shoots as the only generations, and
/// the same four fields at the bough's own depth.
///
/// # Invariants
///
/// **It is the same rule one level down, not a different plant.** The trunk is a limb of the shape the whole tree's
/// limbs are, the generations are the whole tree's secondaries and shoots, and the fields are the whole tree's
/// fields with the limb field dropped because a limb here is the root. A leaf that reads on this plate is therefore
/// the leaf the tree is made of, which is what makes the bough plate evidence about the tree rather than a
/// portrait of a second model.
pub fn alpine_oak_bough(seed: u64) -> Species {
    let rule = Rule {
        trunk: || {
            (
                bend(1.7, [0.42, 0.62, 0.40, 0.22]),
                // A limb of the whole tree at this station is a fifth of the bole's radius, and this plate's limb
                // has to be as thick as that or the wood reads as wire and the leaves read as confetti on it.
                taper(&[(0.00, 0.075), (0.50, 0.045), (1.00, 0.013)]),
            )
        },
        generations: vec![
            Generation {
                count: (16, 20),
                station: (0.12, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.16),
                sink: 0.010,
                cluster: 1,
                shape: secondary,
            },
            Generation {
                count: (40, 52),
                station: (0.14, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.09, 0.18),
                sink: 0.003,
                cluster: 1,
                shape: shoot,
            },
        ],
        fields: Vec::new(),
    };
    Species {
        plant: grow(&rule, seed),
        templates: vec![oak_sprig(), oak_sprig()],
        fields: oak_leaves(BOUGH_SHOOTS),
        decorators: vec![acorn()],
    }
}

/// The fruit an oak's shoots carry at `shoot_depth`: **one or two clusters** per fruiting shoot, near its tip.
///
/// # Invariants
///
/// **The count is a range the seed decides and not a fruit per shoot**, so the crown is *studded* rather than
/// hung: the note bears **1–4 cupules** on stalks of **0.5–2.5 cm** among the leaves, and which of three thousand
/// shoots fruit in a given year is the seed's business and not the rule's.
pub fn oak_fruit(species: &Species, shoot_depth: usize) -> Vec<Seat> {
    scatter(&species.plant, shoot_depth, 0, 0.92, (0, 1))
}

/// The oak's surfaces: a **dark grey-brown bole**, a **strongly glossy dark green** blade above, and the note's
/// **reddish-brown to orangish-brown** felt below.
///
/// # Invariants
///
/// **Wood and leaf are not one tint** — the *felt-pen tree* defect — and the leaf's roughness is the note's own
/// phrase: *"a strongly glossy, almost wet-looking dark green upper surface"*, which is a **0.34** roughness against
/// the wood's **0.86** and the bronze's **0.66**. A gloss is the cheapest half of this species' recognition key and
/// it is one number.
///
/// The wood's tint is a fifth lower than the note's *"pale grey"* photograph would suggest, and the reason is
/// measured rather than chosen: the frame's key stands at **2.4** irradiance, and a 0.30 albedo under it clips to
/// white, which is how the first plate of this species came out with a bleached pole for a trunk.
/// **The wood is dark and warm, and both halves of that are measured rather than chosen.** The note records this
/// species' bark as **unverified**, so the tint is not the photograph's pale grey — it is the *dark grey-brown with
/// shallow fissures* the brief asks for, chosen for what it has to do in the frame: carry the wood as wood and stay
/// dark enough that the crown's leaves are what the eye goes to. The first plate carried **0.135** and read as a
/// pale grey pole; this carries **0.098**, a fifth under it, and the hue is shifted red-ward against its blue so the
/// bole reads as brown rather than as neutral concrete. A pale grey at this key irradiance reads as bleached white.
pub fn oak_palette() -> Palette {
    Palette {
        wood: Skin::new([0.098, 0.087, 0.074], 0.86),
        leaves: vec![
            Skin::new([0.052, 0.098, 0.040], 0.34),
            Skin::new([0.230, 0.118, 0.058], 0.66),
        ],
        seats: vec![Skin::new([0.150, 0.130, 0.062], 0.70)],
    }
}

/// How finely the oak's wood is expanded on the two plates whose subject is wood.
///
/// # Invariants
///
/// **A section is sampled, not drawn, and the sample is what a flute is measured in.** [`fluted_bole`] carries
/// **twenty-four sides and eight flutes**, and [`vegetation::expand::Detail::DEFAULT`] sweeps every solid at
/// `around = 12` — so the crate's own default samples a twenty-four-sided fluted section at twelve vertices and
/// **the flutes alias into a shape that is not the section that was authored**. Raising `around` to the section's
/// own side count is not a dial turned for sharpness: it is the number that makes the authored section the drawn
/// one. The stations and the spacing are the crate's own, because an eleven-metre bole already asks for its rings.
pub const OAK_DETAIL: Detail = Detail {
    stations: 8,
    around: 24,
    spacing: 0.45,
};

/// A bright cold morning at three thousand metres: a deep high sky, a low warm key off the sunward ridge, and a
/// cool sky fill from the valley.
///
/// # Invariants
///
/// **A light's reach is the distance its irradiance is stated at**, so each reach is the length of the position it
/// belongs to: the key stands at `[16, 20, 12]`, which is **28.3 m** away, and its fill at `[-13, 9, -11]`, which is
/// **19.3 m**. A reach that is not the light's own distance states an irradiance the subject never sees.
///
/// The frame is a whole tree at twenty-six metres, so the grade's haze is the brief's **0.005** for a tree framed at
/// twenty metres — a fifth of a garden's, because the air at three thousand metres is thin.
pub fn oak_lighting() -> Lighting {
    Lighting {
        key_position: [16.0, 20.0, 12.0],
        key_colour: [1.0, 0.93, 0.79],
        key_irradiance: 2.4,
        key_reach: 28.3,
        key_distance: 28.3,
        key_extent: 5.0,
        fill_position: [-13.0, 9.0, -11.0],
        fill_colour: [0.46, 0.58, 0.80],
        fill_irradiance: 0.46,
        fill_distance: 19.3,
        floor: Skin::new([0.26, 0.23, 0.19], 0.95),
        sky: Atmosphere::new()
            .zenith([0.05, 0.13, 0.40])
            .horizon([0.52, 0.63, 0.74])
            .ground([0.28, 0.26, 0.22])
            .density(0.0005)
            .scale_height(3000.0),
        sky_radiance: 0.48,
        haze: 0.005,
    }
}

/// The whole-tree frame: taller than wide, standing **25.1 m** off at the crown's own height, so the bole reads
/// against the sky and one pixel is [`ALPINE_OAK_LEAF_PIXEL_METRES`].
///
/// # Invariants
///
/// **The eye is at five and a half metres and the target at six and a half**, so the camera looks very slightly **up**
/// at the crown: the plate is the note's own photograph, `01-habit.jpg`, where the tree stands against a pale sky with
/// its crown above the eye. The vertical extent at the target is **14.1 m** over 1700 rows and the target sits high
/// in the tree, because the first frame that put the crown's top against the top row had the topmost limbs cut off
/// and three hundred rows of empty ground under the trunk. This one leaves the crown a tenth of the frame below the
/// top and the trunk a fifth above the bottom.
pub const OAK_FRAME: Framing = Framing {
    width: 1400,
    height: 1700,
    eye: [16.5, 5.0, 19.2],
    target: [0.80, 5.40, -0.30],
    field_of_view: 0.55,
    near: 0.2,
    far: 600.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The bough plate's frame: **3.7 m** off a limb, looking across it at the shoots, so a leaf is **23 × 11 pixels**.
///
/// # Invariants
///
/// **The distance is set by H2, which asks for a frame where one leaf is three to twenty pixels across.** At
/// `2 · 3.68 · tan(0.40) / 1000` one pixel is **3.11 mm**, and the note's 70 × 34 mm leaf is therefore 22.5 by 10.9
/// pixels — a little over H2's stated top, and the reason it is: the plate has to fill its frame with a limb's whole
/// sub-crown, and filling the frame is worth more than the last three pixels of H2's range. Unlike the pine's tenth of
/// a pixel it is honest at this distance, which is what H2 is for.
const BOUGH_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [3.35, 1.28, 2.70],
    target: [0.85, 1.42, 0.00],
    field_of_view: 0.80,
    near: 0.05,
    far: 30.0,
    floor: 12.0,
    ground: Ground::Plane,
};

/// The base plate's frame: **2.26 m** off the foot of the bole at the height of a person's hip, which is the
/// frame bar **H4** asks for and the first two passes did not have.
///
/// # Why this frame exists at all
///
/// H4 says *"one frame at the base of the trunk, one at the crown, and the taper law written in the species' note
/// so the flare can be read as numbers"*, and a whole-tree frame at twenty-five metres is a portrait of the idea
/// of a flare: at that distance the bole is forty pixels wide and the note's *"0.20–0.30 m at the base"* — and the
/// twenty-millimetre flute between two of [`fluted_bole`]'s lobes — are each under a pixel. **At 2.26 m a pixel is
/// 1.36 mm**, so the bole is five hundred and eighty pixels across, one flute is a hundred and forty, and its depth
/// is thirty: the taper law and the fluting are both readable numbers rather than a suggestion.
///
/// # Invariants
///
/// **The eye is at nine hundred millimetres and the target at eight hundred**, so the bole leaves the top of the
/// frame and the **flare** — the first two knots of the [`gnarled_bole`] taper, where 0.40 m swells and falls to
/// 0.245 m within a tenth of the height — is the subject rather than the trunk's middle. The lens stands on the
/// **same bearing as the key** ([`oak_base_lighting`]), which is what makes the light rake rather than flatten. The
/// width is a fifth under the bough plate's because a plate that is nearly square frames a vertical bole without
/// wasting a third of itself on ground.
const BASE_FRAME: Framing = Framing {
    width: 1200,
    height: 1400,
    eye: [1.30, 0.90, 1.85],
    target: [0.25, 0.80, 0.15],
    field_of_view: 0.80,
    near: 0.05,
    far: 60.0,
    floor: 10.0,
    ground: Ground::Plane,
};

/// The base plate's light: **low, raking, twenty degrees above the horizon and on the lens's side**.
///
/// # Why the base plate is not lit like the tree
///
/// **A tree's own crown shadows its own foot.** The whole-tree key stands twenty metres up and twenty-eight metres
/// off, so every ray from it to the bole's first metre passes through metres of foliage, and the plate H4 asks for
/// came out a black cone with a bright rim: the taper law was in the frame and nothing could be read off it. This
/// key stands at **[4.79, 2.81, 6.04]** — eight metres out on the lens's own bearing, twenty degrees up — so it
/// rakes *across* the flutes, and the **fill**, which casts no shadow and so reaches the foot of the bole through
/// the crown above it, carries the face the lens sees. It is a different hour at the same site, and the plate says
/// so.
///
/// # Invariants
///
/// The reach is the base frame's floor diagonal, 10 × √2 = **14.1 m**, and the irradiance is stated **at** it: the
/// light's own irradiance falls off with the square of the distance, so the number in the document is the subject's
/// divided by `(reach / distance)²` — `1.80 × (14.1 / 7.9)²` is **5.74** at the subject. **The sign of that ratio is
/// the whole of the key**, and getting it the other way round is what put a blown-white pole in the first base
/// plate this species rendered.
///
/// **The crown still shadows the bole, and the fill is the answer rather than the key.** Traced, not guessed: the
/// ray from a key twenty degrees up to a point a metre up the bole is at **1.72 m** where it crosses the crown's
/// 4.3 m radius, and the crown's skirt begins at **1.6 m** — so the key is blocked by two centimetres of oak over
/// three metres of air, which is the *whole* of the problem this plate exists to avoid. The fill casts no shadow,
/// so it reaches that face through the canopy; it is therefore **the brighter of the two at the subject**
/// (`0.16 × (14.1 / 2.9)²` = **3.78**), and the key is what puts the edge on the flutes. This is the dove tree's
/// arrangement and for the dove tree's reason.
fn oak_base_lighting() -> Lighting {
    Lighting {
        key_position: [4.79, 2.81, 6.04],
        key_colour: [1.0, 0.94, 0.82],
        key_irradiance: 1.80,
        key_reach: 14.1,
        key_distance: 7.9,
        key_extent: 4.0,
        fill_position: [1.6, 1.2, 2.0],
        fill_colour: [0.52, 0.58, 0.70],
        fill_irradiance: 0.16,
        fill_distance: 14.1,
        floor: Skin::new([0.26, 0.23, 0.185], 0.95),
        sky: Atmosphere::new()
            .zenith([0.05, 0.13, 0.40])
            .horizon([0.52, 0.63, 0.74])
            .ground([0.28, 0.26, 0.22])
            .density(0.0005)
            .scale_height(3000.0),
        sky_radiance: 0.48,
        // A sixth of the tree plate's haze: the subject here is two metres away, and the tree's own coefficient would
        // put a fifth of this frame's own contrast into the air between the lens and the bark.
        haze: 0.003,
    }
}

/// Builds the oak's whole-tree plate: one tree, its own light and ground, its fruit and its own foliage.
pub fn alpine_oak_picture() -> Value {
    let species = alpine_oak(SEED);
    let planting = Planting::new(species.clone())
        .with_seats(oak_fruit(&species, SHOOTS))
        .with_palette(oak_palette())
        .with_detail(OAK_DETAIL);
    plant_scene_lit(&[planting], &OAK_FRAME, ALPINE_OAK, &oak_lighting(), &[])
}

/// Builds the oak's branch plate: one limb at arm's length, with the same leaf and the same fruit.
pub fn alpine_oak_bough_picture() -> Value {
    let species = alpine_oak_bough(SEED);
    let planting = Planting::new(species.clone())
        .with_seats(oak_fruit(&species, BOUGH_SHOOTS))
        .with_palette(oak_palette());
    plant_scene_lit(
        &[planting],
        &BOUGH_FRAME,
        ALPINE_OAK_BOUGH,
        &oak_lighting(),
        &[],
    )
}

/// Builds the oak's base plate: the foot of the bole at **2.26 m**, under its own low raking key.
///
/// # Invariants
///
/// **It is the same tree and the same fields — not a second model of a trunk.** The only things that change are
/// the frame and the light, which is what makes the plate evidence about the tree the whole-tree plate drew:
/// the same [`gnarled_bole`] flare and the same [`fluted_bole`] lobes, read from two metres instead of twenty-five.
/// The foliage is carried rather than cut, because a trunk plate with no crown in it cannot show the crown's own
/// shadow falling across the frame — which is the thing [`oak_base_lighting`] exists to prevent.
pub fn alpine_oak_base_picture() -> Value {
    let species = alpine_oak(SEED);
    let planting = Planting::new(species.clone())
        .with_seats(oak_fruit(&species, SHOOTS))
        .with_palette(oak_palette())
        .with_detail(OAK_DETAIL);
    plant_scene_lit(
        &[planting],
        &BASE_FRAME,
        ALPINE_OAK_BASE,
        &oak_base_lighting(),
        &[],
    )
}

/// Resolves a whole-tree plate's `authored_id` into the handle that names the node.
pub fn alpine_oak_handle_of(id: u32) -> Option<String> {
    node_ids(&alpine_oak(SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a bough plate's `authored_id` into the handle that names the node: the bough is its own plant, so its
/// walk is its own table and the whole tree's would answer with the wrong node.
pub fn alpine_oak_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&alpine_oak_bough(SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}
