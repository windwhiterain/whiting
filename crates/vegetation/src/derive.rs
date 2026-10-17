//! The plants an author starts from: a rule, a seed, and the numbers they landed on.
//!
//! # The one idea
//!
//! **A default is ordinary editable data the moment it exists.** [`bamboo`] and [`shrub`] are not generators
//! with a different standing from an author's own tree: each returns a [`Species`] whose every curve, taper,
//! station and density is a number the caller keeps and can change, and nothing downstream can tell which
//! numbers were chosen for it and which were typed.
//!
//! # A species is a rule, and the seed decides what the rule left open
//!
//! [`crate::species::Rule`] says how many children a node makes, where they hang and what shape they take; the
//! seed fills in what the rule left to chance. **Bamboo's rule leaves nothing to chance** — its counts, bands,
//! slots and jitters are fixed and its shapes draw no numbers — so its seed reaches the leaf field and nothing
//! else, which is why two bamboo seeds are the same plant with two leaf layouts. A shrub's rule leaves the
//! counts, the stations and each limb's length to the seed, so two shrub seeds are two shrubs.
//!
//! # Why these two
//!
//! Bamboo is the plant P1 renders: a culm with a station band, a ringed taper, a leaf field and a leaf template
//! at once. The shrub is the rule's control: it uses the same machinery with a *computed* shape — limbs built
//! from a length, a departure angle and a droop, and tapers that are fractions of the parent's radius — so the
//! two together show that a species is numbers and a drawing, not a special case per plant.

use shape::{Curve, Scalar};

use crate::leaf::{LeafField, LeafTemplate};
use crate::plant::Plant;
use crate::rng::Rng;
use crate::species::{Generation, Joint, Rule, grow, spine, taper};

/// A plant and what hangs on it: its tree, the templates it instances, and the fields that place them.
#[derive(Debug, Clone, PartialEq)]
pub struct Species {
    /// The spine tree.
    pub plant: Plant,
    /// The blades a field instances, in the order a field's `template` index names them.
    pub templates: Vec<LeafTemplate>,
    /// Where each field puts its leaves.
    pub fields: Vec<LeafField>,
}

/// A culm with three boughs, two branches on each, and a leaf band on the branches.
pub fn bamboo(seed: u64) -> Species {
    Species {
        plant: grow(&bamboo_rule(), seed),
        templates: vec![blade()],
        fields: vec![leaves()],
    }
}

/// A shrub: three generations of self-similar limbs, each a fraction of its parent's radius.
pub fn shrub(seed: u64) -> Species {
    Species {
        plant: grow(&shrub_rule(), seed),
        templates: vec![blade()],
        fields: vec![shrub_leaves()],
    }
}

/// The bamboo's rule: fixed counts and bands, shapes that draw nothing, so its seed cannot change the tree.
fn bamboo_rule() -> Rule {
    Rule {
        trunk: culm,
        generations: vec![
            Generation {
                count: (3, 3),
                station: (0.45, 0.81),
                slot: (0.0, 0.333),
                jitter: (0.0, 0.0),
                sink: 0.004,
                shape: bough_shape,
            },
            Generation {
                count: (2, 2),
                station: (0.35, 0.60),
                slot: (0.3, 0.2),
                jitter: (0.0, 0.0),
                sink: 0.0015,
                shape: branch_shape,
            },
        ],
    }
}

/// The shrub's rule: three generations, each smaller and thinner than the one that carries it.
fn shrub_rule() -> Rule {
    Rule {
        trunk: stem,
        generations: vec![
            Generation {
                count: (3, 4),
                station: (0.35, 0.95),
                slot: (0.0, 0.618),
                jitter: (0.2, 0.1),
                sink: 0.004,
                shape: limb_one,
            },
            Generation {
                count: (3, 3),
                station: (0.30, 0.90),
                slot: (0.0, 0.618),
                jitter: (0.2, 0.1),
                sink: 0.003,
                shape: limb_two,
            },
            Generation {
                count: (2, 3),
                station: (0.30, 0.90),
                slot: (0.1, 0.5),
                jitter: (0.2, 0.1),
                sink: 0.002,
                shape: limb_three,
            },
        ],
    }
}

/// The culm's spine and taper: 2.2 metres tall, its taper rising at a node and falling between them.
fn culm() -> (Curve, Scalar) {
    (
        spine(&[
            ([0.0, 0.0, 0.0], [0.02, 0.75, 0.0]),
            ([0.02, 0.75, 0.0], [0.05, 0.75, 0.01]),
            ([0.09, 1.5, 0.02], [0.06, 0.7, 0.02]),
            ([0.19, 2.2, 0.05], [0.05, 0.6, 0.02]),
        ]),
        taper(&[
            (0.0, 0.030),
            (0.28, 0.022),
            (0.32, 0.028),
            (0.62, 0.018),
            (0.66, 0.024),
            (1.0, 0.010),
        ]),
    )
}

/// A bough: out along `x`, up along `y`, and out again, leaning further the higher its rank.
fn bough_shape(_rng: &mut Rng, joint: &Joint) -> (Curve, Scalar) {
    let lean = joint.index as f32;
    (
        spine(&[
            ([0.0, 0.0, 0.0], [0.42, 0.10, 0.0]),
            ([0.30, 0.06, 0.02 * lean], [0.38, 0.30, 0.04]),
            ([0.62, 0.26, 0.06 * lean], [0.28, 0.42, 0.02]),
            ([0.86, 0.52, 0.08 * lean], [0.18, 0.30, 0.0]),
        ]),
        taper(&[(0.0, 0.014), (0.5, 0.008), (1.0, 0.003)]),
    )
}

/// A branch: shorter and thinner than the bough it hangs from — depth 2, which the handle's own vocabulary
/// calls a *branch*.
fn branch_shape(_rng: &mut Rng, _joint: &Joint) -> (Curve, Scalar) {
    (
        spine(&[
            ([0.0, 0.0, 0.0], [0.20, 0.16, 0.02]),
            ([0.16, 0.13, 0.02], [0.10, 0.26, 0.02]),
            ([0.26, 0.38, 0.03], [0.04, 0.20, 0.0]),
        ]),
        taper(&[(0.0, 0.005), (1.0, 0.0015)]),
    )
}

/// The shrub's stem: half a metre, thin, leaning slightly.
fn stem() -> (Curve, Scalar) {
    (
        spine(&[
            ([0.0, 0.0, 0.0], [0.02, 0.32, 0.0]),
            ([0.05, 0.26, 0.01], [0.06, 0.30, 0.02]),
            ([0.13, 0.5, 0.03], [0.03, 0.20, 0.0]),
        ]),
        taper(&[(0.0, 0.012), (0.5, 0.009), (1.0, 0.005)]),
    )
}

/// The shrub's first generation: the longest limbs, leaving the stem at about a radian.
fn limb_one(rng: &mut Rng, joint: &Joint) -> (Curve, Scalar) {
    limb(rng, joint, 0.42, 1.05, 0.45, 0.55, 0.18)
}

/// The shrub's second generation: shorter, and turned further from the limb they hang from.
fn limb_two(rng: &mut Rng, joint: &Joint) -> (Curve, Scalar) {
    limb(rng, joint, 0.26, 1.15, 0.35, 0.55, 0.20)
}

/// The shrub's third generation: the twigs the leaves hang from.
fn limb_three(rng: &mut Rng, joint: &Joint) -> (Curve, Scalar) {
    limb(rng, joint, 0.16, 1.25, 0.30, 0.60, 0.22)
}

/// A computed limb: it leaves the parent at `departure` radians from the parent's tangent, turns a further
/// `droop` along its length, and its taper is a fraction of the radius that carries it.
///
/// # Invariants
///
/// The three knots are laid out along a heading that turns from `departure` to `departure + droop`, and the
/// tangents are that heading's own direction — so a limb is a chord with a bend in it rather than a straight
/// line with a direction, which is what makes a shrub read as a shrub. The seed draws the length (a factor
/// between 0.82 and 1.18) and the departure (a quarter radian either way), in that order, and then nothing.
fn limb(
    rng: &mut Rng,
    joint: &Joint,
    length: f32,
    departure: f32,
    droop: f32,
    base: f32,
    tip: f32,
) -> (Curve, Scalar) {
    let length = length * (0.82 + 0.36 * rng.unit());
    let departure = departure + 0.25 * rng.signed();
    let middle = departure + droop * 0.35;
    let end = departure + droop;
    let half = length * 0.5;
    let first = [half * departure.sin(), half * departure.cos(), 0.0];
    let second = [
        first[0] + half * middle.sin(),
        first[1] + half * middle.cos(),
        0.0,
    ];
    let tangent = |angle: f32| [angle.sin() * length * 0.6, angle.cos() * length * 0.6, 0.0];
    let radius = joint.parent_radius.max(1.0e-4);
    (
        spine(&[
            ([0.0, 0.0, 0.0], tangent(departure)),
            (first, tangent(middle)),
            (second, tangent(end)),
        ]),
        taper(&[
            (0.0, radius * base),
            (0.5, radius * (base + tip) * 0.5),
            (1.0, radius * tip),
        ]),
    )
}

/// The blade: a midrib that arcs up from the origin, widest at 45% of its own length.
fn blade() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.0, 0.0, 0.0], [0.20, 0.05, 0.0]),
            ([0.17, 0.06, 0.01], [0.16, 0.11, 0.02]),
            ([0.31, 0.17, 0.03], [0.07, 0.10, 0.01]),
        ]),
        taper(&[(0.0, 0.0), (0.45, 0.06), (1.0, 0.0)]),
        0.06,
        7,
        8,
    )
}

/// The bamboo's leaves: a band on the branches, four blades per centimetre of branch.
///
/// # Invariants
///
/// **`size` multiplies a template that is 0.31 m long in its own coordinates**, so `0.35` draws an 11 cm blade
/// and a real bamboo's leaf. The first version of this species asked for `0.05`, which is a 1.5 cm leaf:
/// correct arithmetic on a number chosen as if the template were one unit long, and a plant that reads as a
/// bare broom.
fn leaves() -> LeafField {
    let mut field = LeafField::new(2, 0);
    field.band = (0.25, 0.9);
    field.density = Scalar::constant(30.0);
    field.phyllotaxy = crate::leaf::GOLDEN_ANGLE;
    field.pitch = Scalar::constant(std::f32::consts::FRAC_PI_4);
    field.size = Scalar::new(&[(0.0, 0.35), (1.0, 0.50)]).expect("the leaf's size is a law");
    field.jitter = 0.15;
    field
}

/// The shrub's leaves: the same blade, smaller and denser, on the third generation.
fn shrub_leaves() -> LeafField {
    let mut field = LeafField::new(2, 0);
    field.band = (0.2, 0.95);
    field.density = Scalar::constant(45.0);
    field.phyllotaxy = crate::leaf::GOLDEN_ANGLE;
    field.pitch = Scalar::constant(std::f32::consts::FRAC_PI_3);
    field.size = Scalar::new(&[(0.0, 0.22), (1.0, 0.30)]).expect("the leaf's size is a law");
    field.jitter = 0.2;
    field
}
