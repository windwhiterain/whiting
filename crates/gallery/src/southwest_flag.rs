//! The two flagship broadleaves of arc 5: 珙桐 and 大树杜鹃, each at two distances.
//!
//! # What this module is
//!
//! Arc 5 of [`.agents/proposals/southwest-china-trees.md`](../../../.agents/proposals/southwest-china-trees.md)
//! is *"the flagships, where the leaf stops being green and stops being simple"*, and these two are the species
//! that pay for it: **6 珙桐** is the one whose leaf is not green — two paper-white bracts per inflorescence — and
//! **7 大树杜鹃** is the one whose leaf is not spread along its shoot but crowded into a terminal rosette. Every
//! number below cites that species' own note in
//! [`.agents/investigate/trees/`](../../../.agents/investigate/trees/), which cites the Flora of China and the
//! photographs in `out/refs/<slug>/`.
//!
//! # The two leaves that cannot be one
//!
//! Decision **D5** of the proposal puts a reproductive structure in a decorator *except* 珙桐's bracts, which
//! *"are leaves in every sense that matters here — flat, thin, lit from both sides — and are therefore a second
//! [`LeafTemplate`] with a white tint"*. So each species here carries **two blade templates and one solid**, and a
//! [`Palette`](crate::vegetation_picture::Palette) with one skin per template in template order: a green blade at
//! `leaves[0]` and a white one at `leaves[1]` is the whole of a dove tree's two-colour crown, and the flower head and
//! the truss are decorators with skins of their own.
//!
//! # What is not a decorator: the hang
//!
//! **A seat expresses no rotation at all** (`vegetation::decorator`), so anything that must hang or stand upright is
//! shaped in its own template rather than turned at the seat. A flower head and a truss are both roughly spherical,
//! which is why they need nothing: a sphere has no orientation to express. A bract is not a sphere, and it hangs —
//! so its hang is [`LeafField::aim`] toward world `-y`, the one dial that is stated in world terms, and its **spur** is
//! a node whose own heading turns back down its parent. Both are stated where they are written, because both are
//! choices about where a rotation is allowed to live.
//!
//! # Neither species needs the pine's card LOD
//!
//! [`southwest`](crate::southwest) draws a card foliage at whole-tree distance because a 1.1 mm needle is a tenth of a
//! pixel there and a point-sampled rasteriser **drops** a blade narrower than a pixel rather than fading it. The
//! arithmetic for a broadleaf is the same and lands in the other place: a pixel at the whole-tree frame below is
//! `2 · d · tan(fov / 2) / height`, which at 26 m, 0.66 rad and 1500 rows is **0.0128 m** — a 9.5 cm *Davidia* blade
//! is seven pixels across and a 9 cm *Rhododendron* blade is seven. Both plates of both species therefore draw the
//! **true blade at its own size**, and neither module carries a `Foliage` enum. That is a measurement, not a
//! preference: if a species is added whose blade is under a pixel at its whole-tree frame, it needs the pine's card and
//! this module's claim does not hold for it.
//!
//! # The node budget both species are held to
//!
//! One planting carries at most [`PLANTING_STRIDE`](crate::vegetation_picture::PLANTING_STRIDE) nodes, because that
//! stride is how an `authored_id` names a plant and a node with one number. Both rules below are sized so that their
//! **largest** draw — every count at its top — stays under it, which is why the two innermost twig generations carry
//! fixed counts and only the boughs and their branches draw from a range. Where that budget costs the crown density
//! the note asks for, the species says so in its own verdict rather than quietly exceeding the stride.

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

/// The golden angle as a fraction of a turn, which is the step between two siblings that are not a whorl.
const GOLDEN: f32 = 0.381_966_01;

// ─────────────────────────────────────────────────────────────────── 珙桐 Davidia involucrata

/// The name the dove tree's whole-tree plate is built and rendered under.
pub const DOVE_TREE: &str = "dove_tree";

/// The name the dove tree's bough plate is built and rendered under.
pub const DOVE_TREE_BOUGH: &str = "dove_tree_bough";

/// The seed both of the dove tree's plates are grown from.
const DOVE_SEED: u64 = 41;

/// The dove tree's bole as it is drawn, in metres: **eleven and a half**, inside the note's *"trees to 20 m"* and
/// inside the brief's 10–20 m. The crown above it lifts the top to about fourteen metres, which is a young tree of the
/// note's own height range and a size at which the first branch is still five metres up a bare stem. Every crown
/// number in the rule is a law over [`Generation::station`] on a trunk of this length rather than an absolute length,
/// so the note's *"crown starts at 5–9 m and ends at 20 m"* moves with the constant and not with a hundred numbers.
const DOVE_HEIGHT: f32 = 11.5;

/// The dove tree's rule: a clear bole, a broad crown of ascending first-order boughs, and three twig generations
/// under them.
///
/// # The numbers and where they come from
///
/// From [`davidia_involucrata.md`](../../../.agents/investigate/trees/davidia_involucrata.md)'s modelling table, with
/// every **ESTIMATE** flagged as that table flags it:
///
/// - **8–15 first-order boughs at 40–60° from the trunk axis**, steeply ascending then flattening — the note counts
///   these off `02-crown.jpg`, `03-bark.jpg` and `14-crown-from-below.jpg` and calls the figure an ESTIMATE.
/// - **2–4 second-order branches per first-order, ascending**, also an ESTIMATE from `04-branch.jpg`.
/// - **Clear trunk 5–9 m on a 15–20 m tree** (ESTIMATE), drawn here as [`DOVE_FIRST`] = 0.40, which is 5.4 m of a
///   13.5 m tree and the same fraction.
/// - **The crown is not layered.** The note's own answer to *"is the crown layered?"* is **no**: `02-crown.jpg` and
///   `14-crown-from-below.jpg` show *"one uninterrupted foliage mass with branches at all depths"*. So every
///   generation here has `cluster: 1` — a spiral, never a whorl — which is the same dial the pine sets to 5 and the
///   exact opposite reading of the same tree.
/// - **Twig length 20–40 cm** (ESTIMATE from `13-sapling-shoot-leaf-arrangement.jpg`), drawn at 28 cm for the leafy
///   twig and 50 cm for the shoot that carries it.
fn dove_rule() -> Rule {
    Rule {
        trunk: dove_bole,
        generations: vec![
            // **The first-order boughs.** The note's 8–15, drawn at the top of its own ESTIMATE because a crown is
            // read as a mass and a mass is made of limbs first. The band is spread over the crown's own two thirds of
            // the bole as a spiral, not a whorl: the golden-angle slot step and a quarter turn of jitter are what make
            // a crown that is one shell rather than tiers.
            Generation {
                count: (13, 15),
                station: (DOVE_FIRST, 0.92),
                slot: (0.0, GOLDEN),
                jitter: (0.05, 0.08),
                sink: 0.020,
                cluster: 1,
                shape: dove_bough,
            },
            // **The second-order branches**, sparse and ascending: the note's 2–4 per first-order, and the count is
            // fixed at three because every count below this one is multiplied by it.
            Generation {
                count: (3, 3),
                station: (0.16, 0.92),
                slot: (0.0, GOLDEN),
                jitter: (0.08, 0.10),
                sink: 0.010,
                cluster: 1,
                shape: dove_branch,
            },
            // **The shoots.** A fixed count for the same reason, and the seed still chooses each shoot's own length and
            // departure, so two seeds are two different crowns.
            Generation {
                count: (6, 6),
                station: (0.10, 0.94),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.12),
                sink: 0.005,
                cluster: 1,
                shape: dove_shoot,
            },
            // **The leafy twigs**, the note's 20–40 cm shoot: the generation the green blade field hangs from.
            Generation {
                count: (9, 9),
                station: (0.14, 0.98),
                slot: (0.0, GOLDEN),
                jitter: (0.12, 0.14),
                sink: 0.0025,
                cluster: 1,
                shape: dove_twig,
            },
            // **The flower spurs**: the note's *"flower heads terminal … on short lateral spurs"*, which are **bare**
            // and carry no leaf. They hang at the twig's own tip — `terminal` is the note's word — so a spur and its
            // pair of bracts stand clear of the leaf mass and see the sky, which is the only reason a white bract is
            // white in April. The count is `(0, 1)`, so the seed decides which twigs flower and a spur that carries
            // nothing is a spur — which is what a spur is between flushes.
            Generation {
                count: (0, 1),
                station: (0.78, 1.0),
                slot: (0.0, 1.0),
                jitter: (0.16, 0.10),
                sink: 0.0015,
                cluster: 1,
                shape: dove_spur,
            },
        ],
        // **No field on this rule, and that is a decision.** A tropism is a push on *every* spine in the plant, the
        // bole included, so the pine's `Field::light` at a third of a limb's length would stand the dove tree's
        // thirteen metres up three more. The same effect the pine gets from a field is got here from the leaf field's
        // own [`LeafField::aim`], which is stated in world terms and touches nothing but the blades.
        fields: Vec::new(),
    }
}

/// Where the first branch leaves the dove tree's bole, as a fraction of its height: the note's **ESTIMATE** of a
/// 5–9 m clear trunk on a 15–20 m tree, taken at its lower half.
const DOVE_FIRST: f32 = 0.46;

/// The dove tree's bole: thirteen and a half metres, near-straight and only slightly leaning, which is what
/// `03-bark.jpg` shows — *"several straight, near-vertical grey-brown stems … with no buttressing"*.
///
/// # Invariants
///
/// **The base carries a small swell and no more.** The note's modelling table answers the flare question with a
/// single word — *"Buttress / flare: **none**", `03-bark.jpg`* — while bar **H4** and the *floating tree* defect both
/// ask a trunk to meet the ground with something. The three knots below resolve the conflict the only way that does
/// not lie about either: a 0.34 m radius swollen to 0.30 over the first 0.8 m, which is the basal thickening every
/// trunk has and **not** a root flare. The pine's two-knot flare at the same station is not copied, because a *Davidia*
/// that roots like a pine is a different species drawn to a different note.
fn dove_bole() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.0, 0.0, 0.0], [0.02, 1.0, 0.0]),
            ([0.05, 0.333 * DOVE_HEIGHT, 0.02], [0.06, 1.0, 0.02]),
            ([0.14, 0.667 * DOVE_HEIGHT, 0.05], [0.10, 1.0, 0.03]),
            ([0.27, DOVE_HEIGHT, 0.08], [0.14, 1.0, 0.04]),
        ]),
        // 0.34 m at the base is a 0.68 m DBH, inside the note's *"0.5–1.2 m plausible"* and above its documented floor
        // of 0.50 m for the Dalaoling ancient trees.
        taper(&[
            (0.0, 0.340),
            (0.012, 0.322),
            (0.06, 0.300),
            (0.40, 0.238),
            (0.72, 0.168),
            (1.0, 0.082),
        ]),
    )
}

/// The dove tree's bark section: twenty sides with a shallow vertical ridge.
///
/// The note's whole statement of the bark is *"smooth to faintly striate grey-brown to grey-green"* and
/// *"with ascending bark"*, which is the pattern running **up** the stem: a longitudinal ridge and not a ring of it.
/// The amplitude is a fifth of the pine's fissures because this trunk is three times as thin and the note records no
/// deep fissure anywhere in the fifty-five frames it examined.
fn dove_bark() -> shape::Section {
    let sides = 20;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let ridge = 1.0 - 0.048 * (theta * 6.0).sin().powi(2);
            [theta.cos() * ridge, theta.sin() * ridge]
        })
        .collect();
    shape::Section::outline(&points).expect("a striate ring is a section")
}

/// A first-order bough: it leaves the bole at the note's 40–60° from the axis, runs out and up, flattens, and lifts
/// again at the tip — the note's *"ascending … before flattening"* with *"nothing droops from the woody skeleton"*.
///
/// Its length is a **law over the station it hangs at**: the lowest bough reaches 5.4 m and the highest under a metre,
/// which is the crown profile the note's ESTIMATE of 11–15 m of crown width on a 15–20 m tree asks for.
fn dove_bough(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 6.6 * (1.0 - joint.station).powf(0.55) * (0.80 + 0.40 * rng.unit());
    // The upper boughs stand up more steeply and the lower ones lie further out, so the crown is a dome and not a
    // flat plate; 0.84 rad is 48° from the axis and 0.62 rad is 36°, either side of the note's 40–60°.
    let departure = 0.95 - 0.32 * joint.station + 0.10 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.26,
                departure + 0.16,
                departure - 0.24,
            ],
        ),
        taper(&[(0.0, radius * 0.50), (0.5, radius * 0.24), (1.0, 0.010)]),
    )
}

/// A second-order branch: shorter, ascending, and turning up at its tip like the bough it hangs from. The note calls
/// these *"sparse, 2–4 per first-order, ascending"*.
fn dove_branch(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 2.2 * (0.82 + 0.36 * rng.unit());
    let departure = 0.72 + 0.28 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.22,
                departure + 0.10,
                departure - 0.34,
            ],
        ),
        taper(&[(0.0, radius * 0.48), (0.5, radius * 0.22), (1.0, 0.006)]),
    )
}

/// A shoot: half a metre of the crown's own twig scale, leaving the branch near a right angle and turning up.
fn dove_shoot(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.50 * (0.80 + 0.40 * rng.unit());
    let departure = 0.85 + 0.30 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.30,
                departure + 0.22,
                departure - 0.20,
            ],
        ),
        taper(&[(0.0, radius * 0.60), (0.5, radius * 0.34), (1.0, 0.0045)]),
    )
}

/// A leafy twig: the note's 20–40 cm shoot, drawn at twenty-eight, and the generation the blade field hangs from.
fn dove_twig(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.28 * (0.80 + 0.40 * rng.unit());
    let departure = 0.95 + 0.35 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.26,
                departure + 0.20,
                departure - 0.18,
            ],
        ),
        taper(&[(0.0, radius * 0.62), (0.5, radius * 0.40), (1.0, 0.0028)]),
    )
}

/// A flower spur: nine centimetres of bare twig that turns **back down its own parent**.
///
/// # Invariants
///
/// **The hang is in the heading, because a node's frame carries no "down".** A child is authored in the frame of the
/// surface it hangs from — `x` outward from the parent, `y` along the parent's tip — and world `-y` is not one of those
/// two directions, so a spur that has to point at the ground is drawn pointing at its own parent instead: a heading
/// of 2.5 to 3.0 radians from the parent's tangent is 143° to 172°, which on a horizontal twig *is* downward. This is
/// the branch-side twin of what the pine's hanging cone does in its template, and it is stated because a reader
/// looking for a world-down number here will not find one.
fn dove_spur(_rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(0.20, [2.45, 2.70, 2.88, 3.00]),
        taper(&[(0.0, radius * 0.55), (0.6, radius * 0.34), (1.0, 0.0022)]),
    )
}

/// The green blade: a broadly ovate *Davidia* leaf, 11.5 cm long and 9.5 cm across.
///
/// # The numbers and where they come from
///
/// Flora of China through the note's modelling table: blade **8–15 × 7–12 cm** and *"broadly ovate"*, so the blade
/// here is authored at 11.5 × 9.5 cm, the middle of both ranges. The **width law** is the outline read off
/// `11-leaf-flat-upper.jpg` — a narrow base where the petiole enters the sinus, the widest point just above the middle,
/// and a drawn-out point. Two facts of the outline are *not* in the geometry and are recorded rather than faked:
///
/// - **The cordate sinus.** A sweep widths a blade about its midrib and cannot cut a heart-shaped notch into its base,
///   because the notch is behind the petiole's own attachment. The blade is therefore anchored at the sinus's bottom
///   and widens from there, which is the outline's *silhouette* at every distance a plate is framed from.
/// - **The dentate-serrate margin with acuminate teeth.** The note measures the teeth at 3–5 mm; at 0.0128 m a pixel
///   they are a quarter of a pixel, so no amount of geometry puts them in either plate and a serrated sweep would only
///   be a moiré at the scale it could be seen.
fn dove_leaf() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [0.050, 0.005, 0.002]),
            ([0.045, 0.009, 0.005], [0.048, 0.009, 0.003]),
            ([0.090, 0.018, 0.007], [0.030, 0.005, 0.002]),
        ]),
        // **Seven knots and twelve rings, because the base of the blade is where a sweep shows its sampling.** The
        // first outline rose from a sixteenth of the half-width to two thirds of it inside the first eighth of the
        // length, and at the bough plate's fifty pixels a leaf that is a sharp shoulder at the base is a scallop: the
        // plate read as a row of cockleshells. The knots are now close enough together that a ring lands near each of
        // them, which is the difference between a leaf and its own mesh.
        taper(&[
            (0.00, 0.028),
            (0.06, 0.060),
            (0.16, 0.084),
            (0.32, 0.098),
            (0.48, 0.100),
            (0.68, 0.088),
            (0.86, 0.054),
            (1.00, 0.003),
        ]),
        // The note's *"thin, not succulent"*: 1.8 mm at the blade's widest. The first version used a fortieth, which is
        // 3.3 mm, and the bough plate showed a row of fleshy ribbed pads rather than blades — the keel of a thick lens
        // catches the key light down the middle of every leaf and reads as a cactus rib.
        0.018,
        12,
        4,
    )
}

/// The white bract: the second template, and the reason this species is in the arc.
///
/// # The numbers and where they come from
///
/// The note's table: **bract length to 16 cm, the smaller about half**, **bract width 6–11 cm** (marked ESTIMATE,
/// read off `06-flower-bract-pair.jpg` against the leaves beside it), and the pair **opposite** one another. So the
/// blade is authored at 15 cm by 9 cm — the larger of the pair at the top of the note's length — with the obovate
/// outline `06-flower-bract-pair.jpg` and `09-bracts-two-clear.jpg` show: widest above the middle, rounded at the tip,
/// and recurved along its own length because a bract hangs.
///
/// # What a cluster cannot say
///
/// The note's pair is **asymmetric** — one to 16 cm, one about half — and the dials that place a cluster are a
/// `cluster` count, a `fan` of pitch and one `size` law for the station, so **every member of one cluster is the same
/// size**. The pair is therefore drawn at the larger bract's length, which is the size the crown's silhouette is read
/// at: a second template for the small bract would be a third blade mesh to say a thing no plate can resolve.
fn dove_bract() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [0.050, 0.026, 0.006]),
            ([0.050, 0.026, 0.008], [0.055, 0.012, 0.004]),
            ([0.105, 0.046, 0.008], [0.045, -0.016, 0.0]),
        ]),
        taper(&[
            (0.00, 0.014),
            (0.15, 0.033),
            (0.45, 0.045),
            (0.75, 0.041),
            (0.92, 0.028),
            (1.00, 0.005),
        ]),
        // A bract is thinner than a leaf — it is the note's *"paper-white"* — so 2.7 mm at the widest.
        0.030,
        12,
        4,
    )
}

/// The dove tree's leaf fields at `twig_depth`: the green blade along a leafy twig, and the white bract on a spur.
///
/// # The numbers and where they come from
///
/// - **The green blade**: the note counts *"5–7 leaves per 30 cm of shoot … at clear spiral intervals"* off
///   `13-sapling-shoot-leaf-arrangement.jpg` — an ESTIMATE, and the arithmetic here is **24 leaves per metre** at the
///   twig's base falling to sixteen at its tip, which is seven a twig and **eight per thirty centimetres**: the top of
///   the note's own range, chosen because a crown's density is bounded by the node budget and not by the flora.
///   **The phyllotaxy is a half turn**, because the note's phyllotaxy answer is *spiral/alternate* read off that same
///   photograph and not from flora, and a half turn is the closest single number to an alternate spiral.
/// - **The bracts**: one pair per spur, which is what *"bracts 2 (or 3), opposite"* says, at **no fan** so the pair
///   leaves the same way and the hang is the whole of its attitude. Their aim is world `-y` at full weight, so a
///   bract hangs off whatever side of whatever spur it lands on.
fn dove_fields(twig_depth: usize) -> Vec<LeafField> {
    // **The shoot's own leaves, one generation above the twigs'.** A leaf field is a distribution over a generation and
    // there is nothing in the crate that says a field may only exist on the smallest twig: the note's shoot is a shoot
    // at every scale, and the first crown this rule drew was a set of pompoms — nine leafy twigs on a bare half-metre
    // stick — because the shoot carried nothing but its children. Six leaves a metre fills the gaps between them.
    let mut shoot = LeafField::new(twig_depth - 1, 0);
    shoot.band = (0.10, 0.92);
    shoot.density = shape::Scalar::constant(12.0);
    shoot.phyllotaxy = 0.5;
    shoot.pitch = shape::Scalar::constant(1.10);
    shoot.twist = shape::Scalar::constant(0.15);
    shoot.aim_axis = [0.0, 1.0, 0.0];
    shoot.aim = shape::Scalar::constant(0.25);
    shoot.size = shape::Scalar::constant(0.92);
    shoot.jitter = 0.22;

    let mut leaf = LeafField::new(twig_depth, 0);
    leaf.band = (0.05, 0.99);
    leaf.density =
        shape::Scalar::new(&[(0.0, 36.0), (1.0, 20.0)]).expect("a leaf's density is a law");
    leaf.phyllotaxy = 0.5;
    leaf.cluster = 1;
    leaf.pitch = shape::Scalar::constant(0.95);
    leaf.twist = shape::Scalar::constant(0.15);
    // **A quarter of the way toward the sky**, which is what makes a broad crown read as lit from above: the blades
    // turn up without the rule's own wood moving a millimetre.
    leaf.aim_axis = [0.0, 1.0, 0.0];
    leaf.aim = shape::Scalar::constant(0.25);
    leaf.size = shape::Scalar::constant(1.0);
    leaf.jitter = 0.20;

    let mut bract = LeafField::new(twig_depth + 1, 1);
    // **The density is a number with a floor under it, and the floor is arithmetic.** The integral a field integrates is
    // `length × density × (band width)` rounded to the nearest whole leaf, so a spur of 9.5 cm carrying a density of
    // fourteen over a fifth of its length rounds to **no leaf at all**: the first version of this field drew a crown in
    // April with no bracts in it, which is a flowering dove tree with nothing to flower with. Thirty leaves a metre over
    // the spur's last third is 2.2, which rounds to the **one pair of two** the note's spur carries — and the band is
    // the spur's *lower* third, because the note's second recognition fact is that the bracts *"hang below the foliage
    // line … hanging free of the leaves"*, which is a statement about where on the spur they sit and not only about
    // how many there are.
    bract.band = (0.62, 0.95);
    bract.density = shape::Scalar::constant(30.0);
    bract.phyllotaxy = 0.5;
    // **Two at one station and no fan**: the note's pair is opposite, and `cluster` is the dial that says opposite
    // — two members a half turn apart on the spur's own loop.
    bract.cluster = 2;
    bract.fan = shape::Scalar::constant(0.0);
    bract.pitch = shape::Scalar::constant(0.55);
    bract.twist = shape::Scalar::constant(0.20);
    bract.aim_axis = [0.0, -1.0, 0.0];
    bract.aim = shape::Scalar::constant(1.0);
    bract.size = shape::Scalar::constant(1.0);
    bract.jitter = 0.14;
    vec![shoot, leaf, bract]
}

/// The dove tree's flower heads: one at the tip of every spur that is carrying one.
///
/// The note's inflorescence is *"terminal … ca. 2 cm in diam. on slender peduncles ca. 7 cm, on short lateral spurs"*,
/// so the seat is at station `1.0` of the spur — its tip — and the count is `(1, 1)`: a spur that exists is a spur
/// that is about to flower, and the *absence* of spurs is already expressed by the generation's own `(0, 1)` count.
fn dove_heads(species: &Species, spur_depth: usize) -> Vec<Seat> {
    scatter(&species.plant, spur_depth, 0, 1.0, (1, 1))
}

/// The dove tree's flower head: a two-centimetre ball of purple-brown florets.
///
/// # Invariants
///
/// **A ball has no orientation to express**, which is the whole reason it can be a decorator: a seat carries `x`
/// outward from the spur and no angle, and a roughly spherical solid is the one reproductive structure that needs
/// none. The lobed section gives the note's *"many staminate flowers plus one pistillate"* a lumpy silhouette at the
/// bough plate's 8 pixels without being a second leaf template.
fn dove_head() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0, 0.0, 0.0], [0.008, 0.0, 0.0]),
            ([0.008, 0.0, 0.0], [0.002, 0.0, 0.0]),
        ]),
        shape::Section::circle(),
    );
    sweep.scale = taper(&[(0.0, 0.004), (0.45, 0.009), (1.0, 0.002)]);
    Template {
        sweep,
        stations: 7,
        around: 9,
    }
}

/// The dove tree: its tree, its two blades, its two fields and the head its spurs carry.
pub fn dove_tree(seed: u64) -> Species {
    let mut species = Species {
        plant: grow(&dove_rule(), seed),
        templates: vec![dove_leaf(), dove_bract()],
        fields: dove_fields(4),
        decorators: vec![dove_head()],
    };
    // The bole is dressed after the growth because a section is a node's own and `grow` authors circles: the
    // striations are the trunk's, not the rule's.
    if let Ok(bole) = species.plant.get_mut(&Handle::root()) {
        bole.set_section(dove_bark());
    }
    species
}

/// The dove tree's bough plate: one first-order bough as the trunk, with three twig generations under it and a spur
/// generation above them.
///
/// The depths shift by one, because the bough **is** the tree's depth-0 node: its branches are at depth 1 where the
/// tree's are at depth 2. One rule and one depth argument therefore draws both plates, which is the pine's own
/// arrangement and the reason the two plates cannot drift apart in any number but their framing.
pub fn dove_tree_bough(seed: u64) -> Species {
    Species {
        plant: grow(&dove_bough_rule(), seed),
        templates: vec![dove_leaf(), dove_bract()],
        fields: dove_fields(3),
        decorators: vec![dove_head()],
    }
}

/// The bough plate's rule: a first-order bough is the trunk, and every generation below it is the tree's own shifted
/// up one depth. The generations are the same numbers as [`dove_rule`]'s — that is the point of keeping both plates
/// on one rule — and only the trunk and the three sinks differ, because a bough's own radius carries them.
fn dove_bough_rule() -> Rule {
    Rule {
        trunk: || {
            let radius = 0.075;
            (
                bend(3.4, [0.92, 1.16, 1.08, 0.86]),
                taper(&[(0.0, radius), (0.5, radius * 0.48), (1.0, 0.010)]),
            )
        },
        generations: vec![
            Generation {
                count: (3, 4),
                station: (0.16, 0.92),
                slot: (0.0, GOLDEN),
                jitter: (0.08, 0.10),
                sink: 0.008,
                cluster: 1,
                shape: dove_branch,
            },
            Generation {
                count: (5, 5),
                station: (0.10, 0.94),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.12),
                sink: 0.004,
                cluster: 1,
                shape: dove_shoot,
            },
            Generation {
                count: (8, 8),
                station: (0.14, 0.98),
                slot: (0.0, GOLDEN),
                jitter: (0.12, 0.14),
                sink: 0.002,
                cluster: 1,
                shape: dove_twig,
            },
            Generation {
                count: (0, 1),
                station: (0.78, 1.0),
                slot: (0.0, 1.0),
                jitter: (0.16, 0.10),
                sink: 0.001,
                cluster: 1,
                shape: dove_spur,
            },
        ],
        fields: Vec::new(),
    }
}

/// The dove tree's surfaces: a striate grey-brown bole, a bright green leaf, a paper-white bract and a dark head.
///
/// # Invariants
///
/// **The leaf is bright green and the bract is white, and they are two skins rather than two tints of one**: the note's
/// *"adaxially bright green"* against a bract the note's own source calls white *"sometimes pale yellow"*. The head is
/// the note's *"small dark purplish-brown globular head"*, which is the dove's body and the only dark thing in the
/// inflorescence.
fn dove_palette() -> Palette {
    Palette {
        wood: Skin::new([0.145, 0.115, 0.092], 0.94),
        leaves: vec![
            Skin::new([0.125, 0.235, 0.088], 0.60),
            Skin::new([0.97, 0.96, 0.92], 0.88),
        ],
        seats: vec![Skin::new([0.22, 0.10, 0.13], 0.80)],
    }
}

/// A wet April morning at two thousand metres on the Shennongjia flank.
///
/// # Invariants
///
/// **A light's reach is the distance its irradiance is stated at**, so each reach is the length of the position it
/// belongs to: the key stands 38.3 m from the origin and reaches 38.3 m, and the fill 25.5 m from it reaches 25.5 m.
/// The air is the note's own belt — 1100–2600 m in montane mixed forest — so it is thicker than the pine's two
/// thousand metres of dry slope: a density of 0.0010 over a 1900 m scale height, and a post haze of 0.006 for a tree
/// framed at twenty-six metres.
fn dove_lighting() -> Lighting {
    Lighting {
        key_position: [22.0, 27.0, 14.0],
        key_colour: [1.0, 0.96, 0.88],
        key_irradiance: 3.0,
        key_reach: 37.3,
        key_extent: 6.0,
        fill_position: [-17.0, 9.0, -15.0],
        fill_colour: [0.44, 0.54, 0.74],
        fill_irradiance: 0.30,
        fill_reach: 24.2,
        floor: Skin::new([0.20, 0.18, 0.14], 0.95),
        sky: Atmosphere::new()
            .zenith([0.10, 0.22, 0.46])
            .horizon([0.55, 0.62, 0.68])
            .ground([0.26, 0.25, 0.21])
            .density(0.0010)
            .scale_height(1900.0),
        // **A third of a stop under the fill, and that is H3's dial.** The first frame measured a crown p90/p10 of 1.7,
        // which is a bag of leaves lit from one side: with a sky radiance this high the interior of the crown is filled
        // by the sky and nothing in the canopy is ever darker than its neighbour. Dropping the ambient is what turns a
        // lit outside and a dark inside into a volume, and it is the same lever at every plate distance.
        sky_radiance: 0.38,
        haze: 0.006,
    }
}

/// The dove tree's whole-tree frame: twenty-six metres out, level with the crown's foot, so the crown stands against
/// the sky and the five metres of clear bole read as height.
const DOVE_FRAME: Framing = Framing {
    width: 1400,
    height: 1500,
    eye: [15.0, 6.9, 18.0],
    target: [0.0, 8.6, 0.0],
    field_of_view: 0.58,
    near: 0.2,
    far: 600.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The dove tree's bough frame: **from below**, two and a half metres off a three-metre bough, so the bough crosses the
/// frame against the sky and a leaf is fifty pixels across.
///
/// # Why the eye is under the bough
///
/// The bough plate is the plate bar **H2** is judged on — *"at a range where one leaf is 3–20 pixels across"*, here
/// fifty, so the judgement is comfortably inside its own range — and a bough shot from above puts the ground behind
/// it, where the leaf's pale underside and the bract's white cannot be told from the floor. Shot from below, both are
/// against the sky, which is the same choice `04-branch.jpg` makes.
const DOVE_BOUGH_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [1.35, 0.30, 2.70],
    target: [1.55, 1.15, 0.05],
    field_of_view: 0.75,
    near: 0.05,
    far: 40.0,
    floor: 12.0,
    ground: Ground::Plane,
};

/// Builds the dove tree's whole-tree plate: one tree in flower, its own light and ground.
pub fn dove_tree_picture() -> Value {
    let species = dove_tree(DOVE_SEED);
    let planting = Planting::new(species.clone())
        .with_seats(dove_heads(&species, 5))
        .with_palette(dove_palette());
    plant_scene_lit(&[planting], &DOVE_FRAME, DOVE_TREE, &dove_lighting(), &[])
}

/// Builds the dove tree's bough plate: one first-order bough at arm's length, in flower.
pub fn dove_tree_bough_picture() -> Value {
    let species = dove_tree_bough(DOVE_SEED);
    let planting = Planting::new(species.clone())
        .with_seats(dove_heads(&species, 4))
        .with_palette(dove_palette());
    plant_scene_lit(
        &[planting],
        &DOVE_BOUGH_FRAME,
        DOVE_TREE_BOUGH,
        &dove_lighting(),
        &[],
    )
}

/// Resolves a dove tree frame's `authored_id` into the handle that names the node.
pub fn dove_tree_handle_of(id: u32) -> Option<String> {
    node_ids(&dove_tree(DOVE_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a dove tree bough frame's `authored_id` into the handle that names the node: the bough is its own plant,
/// so its walk is its own table and the whole tree's would answer with the wrong node.
pub fn dove_tree_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&dove_tree_bough(DOVE_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Four knots along a heading that turns through `angles`, measured from the parent's own tangent: nought is along the
/// parent and a quarter turn is straight out. The tangents are the heading's own direction, so a limb is a chord with
/// three bends in it rather than a straight line with a direction.
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

// ───────────────────────────────── 大树杜鹃 Rhododendron protistum var. giganteum

/// The name the giant rhododendron's whole-tree plate is built and rendered under.
pub const GIANT_RHODODENDRON: &str = "giant_rhododendron";

/// The name the giant rhododendron's bough plate is built and rendered under.
pub const GIANT_RHODODENDRON_BOUGH: &str = "giant_rhododendron_bough";

/// The seed both of the giant rhododendron's plates are grown from.
const RHODO_SEED: u64 = 53;

/// The giant rhododendron's height as it is drawn, in metres: **twelve**.
///
/// # The conflict this number settles
///
/// [`rhododendron_protistum.md`](../../../.agents/investigate/trees/rhododendron_protistum.md) opens its Habit section
/// with a boxed FLAGGED DISCREPANCY: Flora of China calls the species *"trees, 5–10 m tall"*, the Chinese field record
/// for var. *giganteum* says **"树高可达30米"**, and Forrest's 1918 type was 25 m at 87 cm DBH. The note's own
/// recommendation is *"treat FOC's 5–10 m as the species' **typical** stature and use 10–30 m for var. giganteum"* —
/// so **twelve metres** is drawn: the low end of the note's recommended range for this variety, and near enough to
/// FOC's typical stature that a reader who has seen a cultivated plant is not lost. Every other number in this rule is
/// a fraction of the height rather than an absolute length, so a reader who wants the 25 m tree changes one constant.
const RHODO_HEIGHT: f32 = 12.0;

/// The giant rhododendron's rule: a stout clear bole, a few thick candelabra limbs, and rosette-bearing shoots on
/// sinuous branches.
///
/// # The numbers and where they come from
///
/// From [`rhododendron_protistum.md`](../../../.agents/investigate/trees/rhododendron_protistum.md), ESTIMATE-marked
/// where the note marks it:
///
/// - **4–8 first-order limbs at 30–60° from the trunk axis, then arching to horizontal and down** (ESTIMATE counted
///   off `01-habit-whole-tree-slope.jpg` and `03-trunk-bark-CONGENER-delavayi.jpg`). `cluster: 1`: a candelabra
///   *forks*, it does not whorl, and the pine's whorled crown is the shape this species is chosen to be unlike.
/// - **Clear trunk 0.35–0.55 × height** (ESTIMATE), drawn at 0.42 = 5.0 m.
/// - **A branch is sinuous and often contorted**, and *"unlike Davidia, the limbs here **droop**"* — so the shapes
///   turn through a wider angle than the dove tree's and [`Field::gravity`] is on the rule.
/// - **Rosette spacing 20–40 cm along a branch** (ESTIMATE from `06-flower-truss-masses.jpg`), which is what the
///   shoot generation's band and count say.
fn rhodo_rule() -> Rule {
    Rule {
        trunk: rhodo_bole,
        generations: vec![
            // **The candelabra limbs**, four to eight of them, spiralled up the bole's upper half.
            Generation {
                count: (5, 7),
                station: (0.42, 0.92),
                slot: (0.0, GOLDEN),
                jitter: (0.06, 0.10),
                sink: 0.040,
                cluster: 1,
                shape: rhodo_limb,
            },
            // **The branches** off each limb, sinuous and few.
            Generation {
                count: (3, 4),
                station: (0.18, 0.94),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.14),
                sink: 0.020,
                cluster: 1,
                shape: rhodo_branch,
            },
            // **The rosette shoots**: fifteen to thirty centimetres, crowded along a branch at the note's 20–40 cm
            // spacing, each carrying exactly one rosette at its own tip and nothing else along it.
            Generation {
                count: (16, 20),
                station: (0.12, 0.96),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.12),
                sink: 0.006,
                cluster: 1,
                shape: rhodo_shoot,
            },
        ],
        // **The limbs droop**, and the note is explicit that they do. A tropism is a push whose size is a fraction of
        // the spine's own length, so one weight droops a 3 m limb by thirty centimetres and a 24 cm shoot by two, and
        // leans the twelve-metre bole by six degrees — which is a thing a tree of this size does anyway. The weight is
        // a tenth and not the fifth a shrub would take because a trunk this long is the one spine in the plant a field
        // must not fold.
        fields: vec![Field::gravity(0.10)],
    }
}

/// The giant rhododendron's bole: twelve metres, stout, straight, and barely tapered — the note's *"straight and stout"*
/// and *"a long clean bole"* from `03-trunk-bark-CONGENER-delavayi.jpg`.
///
/// # What is not modelled
///
/// **The bark exfoliates *"into thin and small irregular flakes"*, and the flakes are a skin, not geometry.** The
/// section below is a twenty-four-sided ring with a low-amplitude irregular radius, which is what a flaking bole reads
/// as at four metres; at the distance the flakes themselves would be one pixel across, a modelled flake is a moiré and
/// not a bark. This is the same answer `southwest.rs` gives the pine's fissures, where the form *is* legible, and it
/// is recorded here rather than left for a reader to find a smooth cylinder and wonder.
fn rhodo_bole() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.0, 0.0, 0.0], [0.05, 1.0, 0.01]),
            ([0.08, 0.317 * RHODO_HEIGHT, 0.04], [0.14, 1.0, 0.05]),
            ([0.26, 0.633 * RHODO_HEIGHT, 0.09], [0.24, 1.0, 0.08]),
            ([0.60, RHODO_HEIGHT, 0.15], [0.36, 1.0, 0.12]),
        ]),
        // 0.50 m at the base is the note's *"0.44–0.50 m typical"* for a tree of Forrest's 0.87 m DBH, swollen to
        // 0.62 over the first half metre because the note also records the largest individual as buttressed at its
        // 3.30 m basal diameter — at this scale, a stem swell and nothing more.
        taper(&[
            (0.0, 0.620),
            (0.012, 0.560),
            (0.05, 0.510),
            (0.42, 0.360),
            (0.72, 0.240),
            (1.0, 0.130),
        ]),
    )
}

/// The giant rhododendron's bole section: twenty-four sides, each a little out of round.
fn rhodo_bark() -> shape::Section {
    let sides = 24;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            // Two incommensurate wobbles stand in for flakes: irregular at every scale without a random number that
            // two runs could disagree about. The amplitude is a twentieth, which at twelve metres is under a pixel of
            // silhouette and is there for the bole's own shading, not for its outline.
            let scale = 1.0 + 0.045 * (theta * 3.0).sin() + 0.028 * (theta * 7.0 + 1.1).cos();
            [theta.cos() * scale, theta.sin() * scale]
        })
        .collect();
    shape::Section::outline(&points).expect("an irregular ring is a section")
}

/// A first-order limb: it leaves the bole at the note's 30–60°, runs out, and arches through horizontal into a droop.
fn rhodo_limb(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    // **A candelabra's limbs are not a pine's boughs.** The law shrinks with the station but only to a quarter, so the
    // upper limbs are two thirds the length of the lower ones: a square-root law put a spire on this tree in the first
    // plate, and the note's silhouette is *"a candelabra in form, not a dome"* with limbs *"forking upward and
    // outward"* over a crown as wide as it is deep.
    let length = 3.7 * (1.0 - 0.45 * joint.station) * (0.85 + 0.30 * rng.unit());
    let departure = 0.92 + 0.24 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.36,
                departure + 0.56,
                departure + 0.80,
            ],
        ),
        taper(&[(0.0, radius * 0.52), (0.5, radius * 0.26), (1.0, 0.014)]),
    )
}

/// A branch of the crown: shorter, sinuous — the note's *"the branches themselves sinuous and often contorted"*.
fn rhodo_branch(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 1.9 * (0.80 + 0.40 * rng.unit());
    let departure = 1.20 + 0.32 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.34,
                departure - 0.18,
                departure + 0.30,
            ],
        ),
        taper(&[(0.0, radius * 0.50), (0.5, radius * 0.24), (1.0, 0.009)]),
    )
}

/// A rosette shoot: the note's 15–30 cm, drawn at 24, hanging out of its branch under gravity and carrying nothing at
/// all but the rosette at its own tip.
fn rhodo_shoot(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.24 * (0.80 + 0.40 * rng.unit());
    let departure = 1.18 + 0.30 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.24,
                departure + 0.30,
                departure + 0.38,
            ],
        ),
        taper(&[(0.0, radius * 0.65), (0.5, radius * 0.36), (1.0, 0.0045)]),
    )
}

/// The rosette blade: 34 cm long and 9 cm across, the note's **20–45 × 5–20 cm** for var. *giganteum* at its middle.
///
/// # The numbers and where they come from
///
/// The outline is `10-leaf-with-hand-scale.jpg` and `09-rosette-top-view-count-leaves.jpg`: **oblanceolate**, widest
/// above the middle, tapering to a **cuneate** base and ending in an **obtuse, mucronate** apex — which is why the width
/// law does *not* come to a point at `1.0` the way the dove tree's does. The midrib arches and turns down at its tip,
/// because the note's fourth recognition fact is *"the leaves hang"* and `04-branch-rosette-with-truss.jpg` shows a blade
/// that rises out of the rosette and falls away again.
fn rhodo_leaf() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [0.100, 0.018, 0.004]),
            ([0.110, 0.032, 0.010], [0.110, 0.028, 0.006]),
            ([0.230, 0.078, 0.016], [0.090, 0.012, 0.002]),
            ([0.320, 0.104, 0.018], [0.020, -0.012, 0.0]),
        ]),
        taper(&[
            (0.00, 0.010),
            (0.10, 0.034),
            (0.35, 0.058),
            (0.58, 0.065),
            (0.80, 0.052),
            (0.94, 0.032),
            (1.00, 0.022),
        ]),
        // The note's *"thickly leathery"*: 4.4 mm at the blade's widest, which is what a sclerophyll is.
        0.034,
        9,
        6,
    )
}

/// The flower truss: an upright cluster of corollas, 12–18 cm across.
///
/// # The numbers and where they come from
///
/// The note's truss is **20–30-flowered on a rachis of ca. 6 cm**, and the diagnostic plan — 8 calyx lobes, 8 corolla
/// lobes, 16 stamens — is why *"the truss reads as a densely many-petalled, almost spherical mass"* in
/// `06-flower-truss-masses.jpg`. A sweep cannot draw twenty corollas, so the template is what the mass **is**: a short
/// rachis swept by a twelve-lobed section, swollen to **17 cm** across, which is the note's own reading of the truss as
/// a ball — and the note's own size for it, *"one round flower ball of about the same width as the cluster"* beside a
/// 35–40 cm rosette. The first version drew it at 12 cm and the plate showed a pink berry beside a leaf. Its lobing is
/// not decoration: it is the only place the corolla count is expressible without a second leaf kind, and at the bough
/// plate's distance it is the difference between a sphere and a flower.
fn rhodo_truss() -> Template {
    let sides = 30;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            // **A shallow lobe on a thirty-point ring, not a deep one on a fourteen-point ring.** A section is sampled
            // along its own outline, so the ring's corners are the outline's corners however many points the sampling
            // asks for: the first truss was a fourteen-gon and the plate showed a pink rock. Thirty points with a
            // twentieth of a ripple reads as a round ball with the corolla texture surviving as a faint scallop.
            let lobe = 1.0 + 0.05 * (theta * 9.0).sin();
            [theta.cos() * lobe, theta.sin() * lobe]
        })
        .collect();
    let section = shape::Section::outline(&points).expect("a corolla ring is a section");
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.0, 0.0, 0.0], [0.045, 0.030, 0.0]),
            ([0.045, 0.030, 0.0], [0.050, 0.010, 0.0]),
            ([0.095, 0.040, 0.0], [0.010, -0.020, 0.0]),
        ]),
        section,
    );
    sweep.scale = taper(&[(0.0, 0.016), (0.30, 0.082), (0.70, 0.074), (1.0, 0.014)]);
    Template {
        sweep,
        stations: 16,
        around: 30,
    }
}

/// The giant rhododendron's one leaf field: the terminal rosette on the shoot, and nothing anywhere else.
///
/// # The numbers and where they come from
///
/// This field **is** the species, and every dial is the note's:
///
/// - **`band: (0.80, 1.0)`** — the leaves are *"crowded in a TERMINAL ROSETTE … not spread along it"*, which is the
///   note's own contrast with *Davidia*, so the band is the shoot's last fifth and the count is what is in it.
/// - **`cluster: 7` and `phyllotaxy: 1/7`** — *"leaves per rosette 6–9 (ESTIMATE, counted)"* from
///   `09-rosette-top-view-count-leaves.jpg`, which shows seven in one rosette and nine in the other. The members of a
///   cluster stand `1 / cluster` of a turn apart, so seven leaves at a seventh of a turn each is a rosette rather than
///   a fan. **The count the field actually produces is the note's range without being told to be**: the density law is
///   150 per metre over the shoot's last fifth and the shoot's own length law runs 0.19–0.29 m, so the integral lands
///   between six and nine and the rosette is seven leaves or eight or nine, drawn by the seed.
/// - **`fan: 1.15` and `pitch: 0.80`** — *"angle of a leaf from the shoot axis 40–70° (ESTIMATE)"*, opened into the
///   star the note's photographs show: the fan spreads the seven members by ±0.58 rad, so the rosette runs from a leaf
///   laid almost flat out to one standing nearly along the shoot.
fn rhodo_fields(shoot_depth: usize) -> Vec<LeafField> {
    let mut rosette = LeafField::new(shoot_depth, 0);
    rosette.band = (0.80, 1.0);
    rosette.density = shape::Scalar::constant(150.0);
    rosette.phyllotaxy = 1.0 / 7.0;
    rosette.cluster = 7;
    rosette.fan = shape::Scalar::constant(1.15);
    rosette.pitch = shape::Scalar::constant(0.80);
    rosette.twist = shape::Scalar::constant(0.25);
    rosette.size = shape::Scalar::constant(1.0);
    rosette.jitter = 0.22;
    vec![rosette]
}

/// The trusses a giant rhododendron's shoots carry: **one at the tip of every rosette-bearing shoot**, which is where
/// the note puts it — *"the inflorescence is terminal on the shoot and its pedicels radiate out of, and just above,
/// the whorl of leaves, so the flower ball sits inside the star of the rosette"*.
///
/// The count is `(1, 1)` because the note's fact 3 is *"nearly every cluster carries one round flower ball"* and this
/// species flowers only once it is over twenty years old; a count of `(0, 1)` would draw a flowering tree that is half
/// juvenile, which the note names as a wrong model outright.
fn rhodo_trusses(species: &Species, shoot_depth: usize) -> Vec<Seat> {
    scatter(&species.plant, shoot_depth, 0, 1.0, (1, 1))
}

/// The giant rhododendron: its tree, its rosette blade, its one field and the truss its shoots carry.
pub fn giant_rhododendron(seed: u64) -> Species {
    let mut species = Species {
        plant: grow(&rhodo_rule(), seed),
        templates: vec![rhodo_leaf()],
        fields: rhodo_fields(3),
        decorators: vec![rhodo_truss()],
    };
    if let Ok(bole) = species.plant.get_mut(&Handle::root()) {
        bole.set_section(rhodo_bark());
    }
    species
}

/// The giant rhododendron's bough plate: one branch as the trunk, its shoots as the only generation, and the same
/// rosette field one depth up.
pub fn giant_rhododendron_bough(seed: u64) -> Species {
    Species {
        plant: grow(&rhodo_bough_rule(), seed),
        templates: vec![rhodo_leaf()],
        fields: rhodo_fields(1),
        decorators: vec![rhodo_truss()],
    }
}

/// The bough plate's rule: a crown branch is the trunk and the rosette shoots are its only generation, so a rosette is
/// at depth 1 where the whole tree's is at depth 3.
fn rhodo_bough_rule() -> Rule {
    Rule {
        trunk: || {
            let radius = 0.055;
            (
                bend(2.2, [1.02, 1.36, 0.84, 1.32]),
                taper(&[(0.0, radius), (0.5, radius * 0.50), (1.0, 0.009)]),
            )
        },
        generations: vec![Generation {
            count: (14, 18),
            station: (0.12, 0.96),
            slot: (0.0, GOLDEN),
            jitter: (0.10, 0.12),
            sink: 0.005,
            cluster: 1,
            shape: rhodo_shoot,
        }],
        fields: vec![Field::gravity(0.10)],
    }
}

/// The giant rhododendron's surfaces: a grey flaking bole, a dark glossy leathery leaf, and a rose truss.
///
/// # Invariants
///
/// **Three tints and no shared one**, which is the *felt-pen tree* defect's own reading: the note gives the leaf
/// (*"glabrous … bright to dark green, glossy"*, roughness low), the bole (*"grey to grey-brown"*, roughness high) and
/// the truss (*"creamy-white tinged rose"* in Flora of China, a saturated pink in every photograph of var. *giganteum*,
/// so the photographs win the tint and the flora wins the roughness) three different numbers.
fn rhodo_palette() -> Palette {
    Palette {
        wood: Skin::new([0.070, 0.064, 0.058], 0.95),
        leaves: vec![Skin::new([0.115, 0.185, 0.095], 0.42)],
        seats: vec![Skin::new([0.62, 0.30, 0.40], 0.78)],
    }
}

/// A misty mid-morning at two thousand six hundred metres on the Gaoligongshan flank.
///
/// # Invariants
///
/// **A light's reach is the distance its irradiance is stated at**: the key's position is 37.3 m long and reaches 37.3 m,
/// the fill's 25.5 m and reaches 25.5 m. The belt is the note's Chinese field record — **2100–3200 m in evergreen
/// broad-leaved forest** — so the sky is a pale high overcast rather than the pine's deep blue, and the air is the
/// thickest of the three trees here: a density of 0.0016 over a 1400 m scale height, which is what puts the far slope
/// of `01-habit-whole-tree-slope.jpg` behind the crown in the air rather than in the picture.
fn rhodo_lighting() -> Lighting {
    Lighting {
        key_position: [21.0, 25.0, 13.0],
        key_colour: [1.0, 0.97, 0.93],
        key_irradiance: 2.6,
        key_reach: 35.4,
        key_extent: 7.0,
        fill_position: [-16.0, 10.0, -15.0],
        fill_colour: [0.48, 0.55, 0.70],
        fill_irradiance: 0.28,
        fill_reach: 24.0,
        floor: Skin::new([0.135, 0.150, 0.115], 0.95),
        sky: Atmosphere::new()
            .zenith([0.24, 0.36, 0.54])
            .horizon([0.70, 0.74, 0.75])
            .ground([0.22, 0.22, 0.20])
            .density(0.0016)
            .scale_height(1400.0),
        // **H3's dial, at the rhododendron's own dial value.** This crown is a scatter of leaf stars with sky between
        // them, so its ten-percent pixel is the sky *through* the crown and its ninety is a lit blade: a plate lit only
        // from the key reads as a flat cut-out, and the interior of every rosette has to be dark for the star to be a
        // star.
        sky_radiance: 0.42,
        haze: 0.006,
    }
}

/// The giant rhododendron's whole-tree frame: twenty-two metres out and level with the crown's foot, so a twelve-metre
/// candelabra stands whole in a frame taller than it is wide — the crown is 0.5–0.8 of the height across and only
/// 0.3–0.5 of it deep, so this tree is taller than it is wide and the pine's frame would crop nothing and centre
/// wrongly.
const RHODO_FRAME: Framing = Framing {
    width: 1300,
    height: 1600,
    eye: [13.2, 5.4, 16.0],
    target: [0.0, 6.8, 0.0],
    field_of_view: 0.60,
    near: 0.2,
    far: 600.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The giant rhododendron's bough frame: **from below and to the side** of a two-metre crown branch, so three or four
/// rosettes cross the frame against the sky and a leaf is a hundred and fifty pixels long.
///
/// # Why from below
///
/// The same reason as the dove tree's, and for the same reason: a rosette seen from above against a forest floor is a
/// green star on brown, and the two things that make this species a rhododendron — a ball of corollas in the middle of
/// each star and a fringe of drooping blades below it — are both read against the sky.
const RHODO_BOUGH_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [1.05, 1.02, 2.30],
    target: [1.15, 0.70, 0.05],
    field_of_view: 0.75,
    near: 0.05,
    far: 30.0,
    floor: 12.0,
    ground: Ground::Plane,
};

/// Builds the giant rhododendron's whole-tree plate: one tree in flower, its own light and ground.
pub fn giant_rhododendron_picture() -> Value {
    let species = giant_rhododendron(RHODO_SEED);
    let planting = Planting::new(species.clone())
        .with_seats(rhodo_trusses(&species, 3))
        .with_palette(rhodo_palette());
    plant_scene_lit(
        &[planting],
        &RHODO_FRAME,
        GIANT_RHODODENDRON,
        &rhodo_lighting(),
        &[],
    )
}

/// Builds the giant rhododendron's bough plate: one crown branch at arm's length, with its rosettes and trusses.
pub fn giant_rhododendron_bough_picture() -> Value {
    let species = giant_rhododendron_bough(RHODO_SEED);
    let planting = Planting::new(species.clone())
        .with_seats(rhodo_trusses(&species, 1))
        .with_palette(rhodo_palette());
    plant_scene_lit(
        &[planting],
        &RHODO_BOUGH_FRAME,
        GIANT_RHODODENDRON_BOUGH,
        &rhodo_lighting(),
        &[],
    )
}

/// Resolves a giant rhododendron frame's `authored_id` into the handle that names the node.
pub fn giant_rhododendron_handle_of(id: u32) -> Option<String> {
    node_ids(&giant_rhododendron(RHODO_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves a giant rhododendron bough frame's `authored_id` into the handle that names the node: the branch is its own
/// plant, so its walk is its own table.
pub fn giant_rhododendron_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&giant_rhododendron_bough(RHODO_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}
