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
//! frame's own pixel.** The whole-tree plate stands **22.8 m** off with a vertical field of view of **0.55 rad** over
//! **1700** rows, so one pixel is `2 · 25.1 · tan(0.275) / 1700` = **8.3 mm**; the note's leaf at the large end of its
//! range, **70 × 34 mm**, is therefore **8.4 × 4.1 pixels** — eight times the width the pine needs a card for. Both
//! plates draw the **true** leaf, and [`ALPINE_OAK_LEAF_PIXEL_METRES`] states the measurement in the code that the
//! framing depends on.
//!
//! A card would not have helped anyway, and the reason is arithmetic rather than taste: a crown's covered area is
//! (leaf count × blade area), and widening a blade by three while thinning the count by three leaves that product
//! exactly where it was. The pine's card pays a **sampling** tax; the oak has no such tax to pay, and the crown's
//! mass is bought with **count and size** instead — about thirty thousand leaves of seventeen square centimetres,
//! which covers a fifth of the crown's own box. That is short of opaque, and [`SHOOT_LEAVES_PER_METRE`] says so with
//! the number rather than leaving it to be discovered.
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
//! photographs do show is the upper limbs as **pale grey and smooth** (`02-crown.jpg`). [`ALPINE_OAK_WOOD`] is that
//! pale grey, and [`fluted_bole`] is the fluting the brief asks for — vertical lobes, because a flute runs with the
//! grain — drawn as form rather than claimed as a description.
//!
//! # Two plates and one foliage
//!
//! The whole tree for the silhouette (H1, H3, H4) and one bough at arm's length for the foliage (H2), as
//! [`southwest`](southwest) does it. The rule is the same rule and the second plate is one limb of it, so a leaf
//! that reads on the bough plate is the leaf the tree is made of.

use definition::Value;
use vegetation::decorator::{Seat, Template, scatter};
use vegetation::derive::Species;
use vegetation::envelope::{Envelope, trim};
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

/// The name the oak's whole-tree plate is built and rendered under.
pub const ALPINE_OAK: &str = "alpine_oak";

/// The name the oak's branch plate is built and rendered under.
pub const ALPINE_OAK_BOUGH: &str = "alpine_oak_bough";

/// The seed both plates' leaf fields are derived from.
const SEED: u64 = 34;

/// The golden angle as a fraction of a turn, which is the step between two successive shoots.
const GOLDEN: f32 = 0.381_966_01;

/// How deep the shoots hang in the whole tree: trunk `0`, limbs `1`, secondaries `2`, shoots `3`.
const SHOOTS: usize = 3;

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

/// The one pixel the whole-tree plate's framing makes, in metres: `2 · 25.1 · tan(0.55 / 2) / 1700`.
///
/// # Invariants
///
/// **This is the number that decides the plate's foliage, and it is stated here rather than left to the framing.**
/// A blade narrower than one pixel is dropped by a point-sampled rasteriser, so the blade a plate can draw is a
/// function of where that plate stands, and the note's 70 × 34 mm leaf is 8.4 × 4.1 of these. See the module docs.
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
fn alpine_oak_rule() -> Rule {
    Rule {
        trunk: gnarled_bole,
        generations: vec![
            // **The heavy limbs.** Eight to ten of them, spread over the bole's upper four fifths and *not* pruned
            // off its lower part: this is the whole difference between an oak and a pine's clean bole, and it is one
            // band's near end. Their length is a law over the station they hang at, which is what
            // `Joint::station` exists for.
            Generation {
                count: (13, 17),
                station: (0.20, 0.97),
                slot: (0.0, GOLDEN),
                jitter: (0.07, 0.10),
                sink: 0.05,
                cluster: 1,
                shape: limb,
            },
            // **The secondaries.** Nine to twelve to a limb. **These two counts are a pair and they trade.** With ten
            // limbs and eighteen secondaries the crown's *outline* was a star: ten leafy strips with the sky between
            // them, and no amount of leaf on those strips closes a gap that is a metre wide. With sixteen limbs and
            // eleven secondaries the outline closes and the strips meet inside it. The crown's mass is therefore
            // bought at the limb, and the secondaries are what fill what the limbs have already bounded.
            Generation {
                count: (9, 12),
                station: (0.12, 0.96),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.18),
                sink: 0.012,
                cluster: 1,
                shape: secondary,
            },
            // **The shoots.** Nine to thirteen a secondary, at the note's ESTIMATED **5–15 cm**. The count is small
            // because the *secondaries* are the crown's mass now, and each of these carries fourteen leaves of six
            // centimetres — a shoot with a leaf every seven millimetres, which is what the photographs show.
            Generation {
                count: (9, 13),
                station: (0.14, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.09, 0.22),
                sink: 0.004,
                cluster: 1,
                shape: shoot,
            },
        ],
        // An oak's shoots reach for the light but hold their leaves stiffly out from the twig, so the tropism is
        // weak: enough to turn a shaded shoot's leaves upward and not enough to comb the crown flat.
        fields: vec![Field::light([0.0, 1.0, 0.0], 0.22)],
    }
}

/// The bole: **12 m**, a flare at the ground, and a wander of a metre and a half up its length.
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
        // **Eight knots over twelve metres, each knot a knot.** The wander is two metres in `x` and one and a half
        // in `z` against a twelve-metre height, so the bole leans out, is thrown back, doubles under itself at five
        // metres and comes forward again: a trunk with elbows, which is what *"gnarled, contorted"* means in
        // geometry. The tangents are short beside their segments on purpose — a Hermite whose tangent is half a
        // segment long kinks at the knot, and one whose tangent is longer interpolates through it and comes out a
        // leaning pole.
        spine(&[
            ([0.00, 0.00, 0.00], [0.95, 0.90, 0.25]),
            ([0.55, 1.30, 0.25], [0.75, 0.85, -0.60]),
            ([0.95, 3.20, -0.55], [-0.55, 0.85, -0.55]),
            ([0.75, 5.10, -1.35], [0.80, 0.80, 0.45]),
            ([1.25, 6.90, -1.15], [0.60, 0.85, 0.60]),
            ([2.00, 8.80, -0.35], [-0.35, 0.90, -0.30]),
            ([2.15, 10.70, -0.65], [0.15, 0.99, 0.05]),
            ([2.20, 12.00, -0.62], [0.00, 1.00, 0.00]),
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
fn fluted_bole() -> shape::Section {
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
    let length = 1.45 * (1.05 - 0.40 * joint.station) * (0.72 + 0.55 * rng.unit());
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

/// The oak leaf: **70 × 34 mm**, the note's FOC maximum (**2.5–7 cm × 1.5–3.5 cm**), obovate and blunt-tipped.
///
/// # Why the large end of the note's range
///
/// **A leaf is the crown's only area, the crown is the whole species' silhouette, and this builder's cost in leaves
/// is superlinear.** Full coverage of a seven-metre crown wants some forty thousand leaves; this builder compiles
/// twenty-eight thousand in about a minute and eighty thousand in over ten, so the honest trade at this render budget
/// is *fewer, larger* leaves. At seven centimetres a blade covers seventeen square centimetres; at four and a half,
/// seven. The same twenty-four thousand leaves therefore cover fifty per cent more crown at this size, and the size
/// law below spreads the field down to five centimetres so the crown is not drawn in one leaf size.
///
/// # The shape the width law carries, and the three it cannot
///
/// - **Obovate** is the widest point *above* the middle, which a width law over the midrib says directly: the law
///   peaks at 0.52 and not at 0.5.
/// - **An obtuse apex** is the law *not* reaching nought at the tip — the last knot of the width is 5.0 mm of half
///   width and the sweep's own cap closes it, so the blade ends in a round rather than a point.
/// - **The petiole** — FOC's **2–5 mm**, *"sometimes nearly sessile"* — is the width law's flat start: the blade is
///   under four millimetres wide for its first six, which at the bough plate's distance is two pixels and is
///   therefore drawn rather than separately modelled.
/// - **The cordate base** is a notch, and a width law cannot make a notch; the base is rounded here, which is the
///   other state the note gives.
/// - **The spiniform margin** is a millimetre of serration and is not drawn. See the module docs.
///
/// The blade is a **closed solid** (D18), so both faces carry their own normal and a leaf lit from behind is shaded
/// rather than faked, and the **4.5% thickness ratio** is a sclerophyll's: a leathery leaf of a hard-leaved evergreen
/// oak is about a millimetre and a half thick over a three-and-a-half-centimetre width.
fn oak_leaf() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.0000, 0.0000, 0.0000], [0.0250, 0.0032, 0.0000]),
            ([0.0250, 0.0025, 0.0000], [0.0232, 0.0047, 0.0009]),
            ([0.0480, 0.0062, 0.0013], [0.0170, 0.0047, 0.0025]),
            ([0.0700, 0.0116, 0.0046], [0.0054, 0.0016, 0.0028]),
        ]),
        taper(&[
            (0.00, 0.0009),
            (0.09, 0.0022),
            (0.20, 0.0094),
            (0.52, 0.0170),
            (0.78, 0.0152),
            (0.93, 0.0106),
            (1.00, 0.0050),
        ]),
        0.045,
        7,
        4,
    )
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

/// Leaves per metre of shoot at the densest point of the shoot's own leafing law.
///
/// # Invariants
///
/// **Three hundred is the crown's area, not a texture, and it is the number that costs the build.** This
/// builder's cost in leaf instances is **superlinear** — measured on this species: 17 710 placements compile in about
/// eleven seconds, 23 600 in fifty, 29 800 in a hundred and fifty and 53 620 in over five minutes — so a crown is
/// bought with count and every count is paid for twice. Three hundred to the metre over about seventeen hundred
/// shoots is about thirty thousand leaves, and this plate's crown covers **0.188** of its own box against the Yunnan
/// pine's **0.198**: that ratio, measured by `audit/frame_stats.py`, is the only honest reading of *"dense,
/// rounded"* this builder can afford, and [`SHOOT_LEAVES_PER_METRE`] is the dial that moves it.
///
/// It is high where it is — a leaf every five and a half millimetres of a ten-centimetre shoot, eighteen deep — and
/// it is high **because the note asks for it**: *"held densely so the crown is a solid rounded mass with a hard edge,
/// not a feathered cloud"*.
const SHOOT_LEAVES_PER_METRE: f32 = 300.0;

/// The leafing law along a shoot, in leaves per metre of shoot: a bare foot, a dense middle, a thinning tip.
///
/// # Invariants
///
/// **This law is the answer to the *dead give*.** Foliage spread evenly from a shoot's very base to its very tip is
/// the defect the bar names, and a shoot is not that: the foot is bare where it leaves its parent, the middle is
/// where the mature leaves stand shoulder to shoulder, and the tip carries the few young ones. The band on the
/// field that uses it stops short of both ends as well, so the first and last stations of the law are never
/// reached.
fn shoot_leafing() -> shape::Scalar {
    let profile = [
        (0.00, 0.55),
        (0.28, 1.00),
        (0.62, 0.95),
        (0.88, 0.58),
        (1.00, 0.22),
    ];
    let knots: Vec<(f32, f32)> = profile
        .iter()
        .map(|(at, shape)| (*at, shape * SHOOT_LEAVES_PER_METRE))
        .collect();
    shape::Scalar::new(&knots).expect("a shoot's leafing is a law")
}

/// The four fields the oak's foliage is made of, at the shoot depth `shoot_depth`.
///
/// # The four, and what each is for
///
/// - **`crown`** is the mass: the note's green blade on the shoots themselves, at the leafing law above.
/// - **`bronze`** is the note's **rusty-brown underside** as far as one skin per template can carry it — a sparser
///   second layer of the same blade, **aimed at the ground**, so the bronze shows on the crown's lower surface where
///   the note says the wind shows it (`05-leaf-underside.jpg`, `10-leaf-underside-scale.jpg`). It is an
///   approximation and the module docs say so.
/// - **`inner`** is the shaded interior on the secondaries: older, smaller, bronze leaves deep in the crown, which
///   is what keeps the crown reading as a volume rather than as a shell of one tint.
/// - **`limb`** is the leaves on the heavy limbs' own length, sparse, and the reason the foliage visibly **starts low
///   on the trunk** instead of beginning where the shoots do.
///
/// A bough plate has three orders below its root and not four, so the limb field exists only where the limbs are two
/// generations above the shoots — the same guard the pine's candle field carries, for the same reason.
fn oak_leaves(shoot_depth: usize) -> Vec<LeafField> {
    let mut crown = LeafField::new(shoot_depth, ALPINE_OAK_GREEN);
    crown.band = (0.06, 0.94);
    crown.density = shoot_leafing();
    crown.phyllotaxy = GOLDEN;
    // A cluster of one and a fan of nought: an oak's leaf is a leaf on a 2–5 mm petiole, not a fascicle, and the
    // cluster dial the pine needs is the identity here.
    crown.cluster = 1;
    crown.fan = shape::Scalar::constant(0.0);
    // The pitch and twist are laws and not numbers because *a plastic crown* is the defect the bar names: one pitch,
    // one twist and one size for every leaf gives the crown a moiré instead of a texture. Pitch swings from splayed
    // at the shoot's foot to nearly along it at the tip, and the twist turns a third of a turn over the shoot's
    // length, so a crown of these has no two leaves facing the same way.
    crown.pitch = shape::Scalar::new(&[(0.00, 1.34), (0.45, 1.04), (1.00, 0.68)])
        .expect("a crown's pitch is a law");
    crown.twist = shape::Scalar::new(&[(0.00, -0.55), (0.50, 0.10), (1.00, 0.70)])
        .expect("a crown's twist is a law");
    crown.size = shape::Scalar::new(&[(0.00, 0.70), (0.35, 0.90), (1.00, 1.10)])
        .expect("a leaf's size is a law");
    crown.jitter = 0.17;

    let mut bronze = LeafField::new(shoot_depth, ALPINE_OAK_BRONZE);
    bronze.band = (0.26, 1.00);
    bronze.density = shape::Scalar::constant(4.0);
    bronze.phyllotaxy = GOLDEN;
    bronze.cluster = 1;
    bronze.fan = shape::Scalar::constant(0.0);
    bronze.pitch = shape::Scalar::constant(1.46);
    bronze.twist = shape::Scalar::constant(1.10);
    bronze.aim_axis = [0.0, -1.0, 0.0];
    bronze.aim = shape::Scalar::constant(0.40);
    bronze.size = shape::Scalar::constant(1.06);
    bronze.jitter = 0.20;

    let mut inner = LeafField::new(shoot_depth - BOUGH_SHOOTS + 1, ALPINE_OAK_BRONZE);
    inner.band = (0.14, 0.92);
    // **Forty-five to the metre is a leaf every twenty-two millimetres**, and that is the number that stops the
    // crown reading as wire with confetti on it: a second-order member is a metre long and eight centimetres round,
    // and at fourteen leaves to the metre it was bare wood with a fringe at its tip.
    inner.density = shape::Scalar::constant(8.0);
    inner.phyllotaxy = GOLDEN;
    inner.cluster = 1;
    inner.fan = shape::Scalar::constant(0.0);
    inner.pitch = shape::Scalar::constant(1.22);
    inner.twist = shape::Scalar::constant(0.62);
    inner.size = shape::Scalar::constant(0.88);
    inner.jitter = 0.22;

    let mut limb = LeafField::new(shoot_depth - BOUGH_SHOOTS, ALPINE_OAK_GREEN);
    limb.band = (0.22, 0.95);
    limb.density = shape::Scalar::constant(8.0);
    limb.phyllotaxy = GOLDEN;
    limb.cluster = 1;
    limb.fan = shape::Scalar::constant(0.0);
    limb.pitch = shape::Scalar::constant(1.28);
    limb.twist = shape::Scalar::constant(0.35);
    limb.size = shape::Scalar::constant(0.95);
    limb.jitter = 0.20;

    // A bough plate's root is a limb, so a field that would hang on the limbs would hang on the bough's own axis and
    // put leaves on the wood the plate is looking along. It is built here and dropped there, so the four fields are
    // one function and not a function plus a special case.
    if shoot_depth >= SHOOTS {
        vec![crown, bronze, inner, limb]
    } else {
        vec![crown, bronze, inner]
    }
}

/// The crown the trim holds the oak to: a dome, nothing at all below the crown's skirt, and nothing above its tip.
///
/// # Invariants
///
/// **The radius is the crown's own outline as a law over the trunk**, and it is what turns a bag of branches into
/// the note's *"dense, rounded to broadly domed"* crown: **3.3 m** at just under half the height, **3.0 m** four
/// fifths of the way up and only reaching nought at the bole's own tip. The two figures are the note's ESTIMATES — a
/// crown **6–12 m** across and **4–10 m** deep on a **10(−20) m** tree — read at their narrow end, and the reason is
/// arithmetic the first plate of this species made necessary: this builder compiles placements superlinearly, so a
/// crown is bought with leaves, and a six-and-a-half-metre crown needs half the leaves a thirteen-metre one does for
/// the same solidity.
///
/// **The last two knots are what make it a dome and not a whip.** The radius holds at three metres up to four fifths
/// of the bole and only then falls, so the crown's top is a shoulder the topmost limbs build rather than a point the
/// envelope cuts: the plate before this one had the radius reach nought at four fifths and came out with three bare
/// metres of bole and a tuft on the end of it.
///
/// **The envelope is a guard and not a carver, and the module says so.** `trim` shortens a node by solving for the
/// scale at which its own tip lands inside, and a shoot is ten centimetres long: an envelope two metres tighter
/// than the architecture solves that scale at one tenth, which is the crate's floor, and the shoot survives at a
/// tenth of its length with all of its leaves crushed onto a centimetre of twig. The third plate of this species
/// was drawn through an envelope tighter than its architecture and came out as bare wire with tufts on it. So the
/// radius law here is drawn to sit just outside what the rule grows, the trim takes the few per cent of the frontier
/// that overshoots, and the dome is the **limb's departure law and the secondary's station law** working together.
fn crown_envelope() -> Envelope {
    Envelope::radial(&[
        (0.00, 0.00),
        (0.14, 0.00),
        (0.20, 2.00),
        (0.30, 2.95),
        (0.46, 3.30),
        (0.68, 3.35),
        (0.86, 3.00),
        (0.95, 1.80),
        (1.00, 0.00),
    ])
}

/// The oak: its tree, its foliage, its four fields and the fruit its shoots carry.
pub fn alpine_oak(seed: u64) -> Species {
    let mut plant = grow(&alpine_oak_rule(), seed);
    // **The bole is dressed after the growth and the crown is cut after that.** A section is a node's own and
    // `grow` authors circles, so the flutes are the trunk's and not the rule's. The trim runs last because it
    // measures the tips the fluting and the bake have already settled.
    if let Ok(bole) = plant.get_mut(&Handle::root()) {
        bole.set_section(fluted_bole());
    }
    trim(&mut plant, &crown_envelope());
    Species {
        plant,
        templates: vec![oak_leaf(), oak_leaf()],
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
                count: (26, 36),
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
        templates: vec![oak_leaf(), oak_leaf()],
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
fn oak_fruit(species: &Species, shoot_depth: usize) -> Vec<Seat> {
    scatter(&species.plant, shoot_depth, 0, 0.92, (0, 1))
}

/// The oak's surfaces: the note's **pale grey and smooth** limbs, a **strongly glossy dark green** blade above, and
/// the note's **reddish-brown to orangish-brown** felt below.
///
/// # Invariants
///
/// **Wood and leaf are not one tint** — the *felt-pen tree* defect — and the leaf's roughness is the note's own
/// phrase: *"a strongly glossy, almost wet-looking dark green upper surface"*, which is a **0.34** roughness against
/// the wood's **0.80** and the bronze's **0.66**. A gloss is the cheapest half of this species' recognition key and
/// it is one number.
///
/// The wood's tint is a fifth lower than the note's *"pale grey"* photograph would suggest, and the reason is
/// measured rather than chosen: the frame's key stands at **2.4** irradiance, and a 0.30 albedo under it clips to
/// white, which is how the first plate of this species came out with a bleached pole for a trunk.
pub fn oak_palette() -> Palette {
    Palette {
        // A dark grey-brown, and the reason is stated where it is chosen: the note records this species' bark as
        // **unverified**, so the tint is not the photograph's pale grey but a bark brown chosen for what it has to
        // do in the frame — carry the wood as wood, and stay dark enough that the crown's leaves are what the eye
        // goes to. A pale grey at this key irradiance read as bleached white poles with foliage on them.
        wood: Skin::new([0.135, 0.128, 0.118], 0.82),
        leaves: vec![
            Skin::new([0.052, 0.098, 0.040], 0.34),
            Skin::new([0.230, 0.118, 0.058], 0.66),
        ],
        seats: vec![Skin::new([0.150, 0.130, 0.062], 0.70)],
    }
}

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
fn oak_lighting() -> Lighting {
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
const OAK_FRAME: Framing = Framing {
    width: 1400,
    height: 1700,
    eye: [16.5, 5.6, 19.2],
    target: [0.80, 6.60, -0.30],
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

/// Builds the oak's whole-tree plate: one tree, its own light and ground, its fruit and its own foliage.
pub fn alpine_oak_picture() -> Value {
    let species = alpine_oak(SEED);
    let planting = Planting::new(species.clone())
        .with_seats(oak_fruit(&species, SHOOTS))
        .with_palette(oak_palette());
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
