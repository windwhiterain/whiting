//! The plants an author starts from: one species, as ordinary data.
//!
//! # The one idea
//!
//! **A default is ordinary editable data the moment it exists.** [`bamboo`] is not a generator with a
//! different standing from an author's own tree: it returns a [`Species`] whose every curve, taper, station
//! and density is a number the caller keeps and can change, and nothing downstream can tell which numbers
//! were chosen for it and which were typed.
//!
//! # Why one species
//!
//! P1 needs **one** plant that exercises a station band, a ringed taper, a leaf field and a leaf template at
//! once, and bamboo is that plant: its culm's taper rises at a node and falls between them, its boughs hang at
//! one slot each around the culm, and its leaves are a band on the third generation. A pine and an oak are the
//! same code with different numbers, and they land when a scene wants them — a species nothing renders is a
//! set of numbers nothing checks.
//!
//! # What a species is not
//!
//! Not a random generator: it takes a seed for the *leaf field's* own numbers (its phase and its jitter) and
//! uses it for nothing else. The tree itself is written out, so a change to it is a change to the tree rather
//! than a different draw from the same distribution.

use shape::{Curve, Jet, Scalar};

use crate::leaf::{LeafField, LeafTemplate};
use crate::node::{Attachment, Node};
use crate::plant::Plant;

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

/// A culm with three boughs, two twigs on each, and a leaf band on the twigs.
///
/// # Invariants
///
/// The seed reaches the leaf field and nothing else, so two seeds are the same plant with two leaf layouts.
/// Every number here is a starting point: the field's band is the top of each twig, the taper's rise at
/// `t = 0.32` is a node, and a caller that moves either is editing the species rather than fighting it.
pub fn bamboo(seed: u64) -> Species {
    let mut root = culm();
    for index in 0..3 {
        let station = 0.45 + 0.18 * index as f32;
        let slot = 0.333 * index as f32;
        let mut branch = bough(index as f32, Attachment::new(station, slot).sunk(0.004));
        for twig_index in 0..2 {
            branch.push(twig(
                Attachment::new(
                    0.35 + 0.25 * twig_index as f32,
                    0.3 + 0.2 * twig_index as f32,
                )
                .sunk(0.0015),
            ));
        }
        root.push(branch);
    }
    Species {
        plant: Plant::new(root, seed),
        templates: vec![blade()],
        fields: vec![leaves()],
    }
}

/// A spine through the given `(position, tangent)` pairs.
fn spine(knots: &[([f32; 3], [f32; 3])]) -> Curve {
    Curve::new(
        knots
            .iter()
            .map(|(position, tangent)| Jet::new(*position, *tangent))
            .collect(),
    )
    .expect("the species' knots are distinct")
}

/// The culm: 2.2 metres tall, its taper rising at a node and falling between them.
fn culm() -> Node {
    Node::root(
        spine(&[
            ([0.0, 0.0, 0.0], [0.02, 0.75, 0.0]),
            ([0.02, 0.75, 0.0], [0.05, 0.75, 0.01]),
            ([0.09, 1.5, 0.02], [0.06, 0.7, 0.02]),
            ([0.19, 2.2, 0.05], [0.05, 0.6, 0.02]),
        ]),
        Scalar::new(&[
            (0.0, 0.030),
            (0.28, 0.022),
            (0.32, 0.028),
            (0.62, 0.018),
            (0.66, 0.024),
            (1.0, 0.010),
        ])
        .expect("the culm's taper is a law"),
    )
}

/// A bough: out along `x`, up along `y`, and out again — authored in its parent's attachment frame.
fn bough(lean: f32, attachment: Attachment) -> Node {
    Node::attached(
        spine(&[
            ([0.0, 0.0, 0.0], [0.42, 0.10, 0.0]),
            ([0.30, 0.06, 0.02 * lean], [0.38, 0.30, 0.04]),
            ([0.62, 0.26, 0.06 * lean], [0.28, 0.42, 0.02]),
            ([0.86, 0.52, 0.08 * lean], [0.18, 0.30, 0.0]),
        ]),
        Scalar::new(&[(0.0, 0.014), (0.5, 0.008), (1.0, 0.003)])
            .expect("the bough's taper is a law"),
        attachment,
    )
}

/// A twig: shorter, thinner, and the generation the leaves hang from.
fn twig(attachment: Attachment) -> Node {
    Node::attached(
        spine(&[
            ([0.0, 0.0, 0.0], [0.20, 0.16, 0.02]),
            ([0.16, 0.13, 0.02], [0.10, 0.26, 0.02]),
            ([0.26, 0.38, 0.03], [0.04, 0.20, 0.0]),
        ]),
        Scalar::new(&[(0.0, 0.005), (1.0, 0.0015)]).expect("the twig's taper is a law"),
        attachment,
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
        Scalar::new(&[(0.0, 0.0), (0.45, 0.06), (1.0, 0.0)]).expect("the blade's width is a law"),
        0.06,
        7,
        8,
    )
}

/// The leaves hang from the twigs, which are the third generation: depth 2 below the trunk.
///
/// # Invariants
///
/// **`size` multiplies a template that is 0.31 m long in its own coordinates**, so `0.35` here draws a
/// 11 cm blade and a real bamboo's leaf. The first version of this species asked for `0.05`, which is a
/// 1.5 cm leaf: correct arithmetic on a number chosen as if the template were one unit long, and a plant
/// that reads as a bare broom. The density says four blades per centimetre of twig, which is what makes a
/// crown out of two twigs rather than a row of flecks.
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
