//! A species as a rule: what grows below a node, and the seed that decides how.
//!
//! # The one idea
//!
//! **A species is a rule and a seed, and a plant is what they make.** The rule says how many children a node
//! makes, where on its surface they hang, and what shape each one takes; the seed decides the rest. A plant's
//! every node is still a handle — [`crate::handle::Handle`] names a path in the tree the rule produced — so the
//! hand edits a grown plant exactly as it edits an authored one, and V5's first stage (*compute the node set*)
//! stops being a promise and becomes [`grow`].
//!
//! # Why the shape is a function and the rest is data
//!
//! Counts, bands, slots and jitter are numbers, and a whorl differs from a spiral in one of them. A branch's
//! **curve** is a small drawing: a rule that could only say angles would force an author to describe a curve
//! they can see. So the shape is a function of the node's own rng and the joint it hangs from — which is also
//! how a taper becomes a fraction of the parent's radius without a second channel.
//!
//! # What the seed draws, in what order
//!
//! For each parent, in this order: **the count** (only when the range can vary, so a fixed count draws
//! nothing), then for each child the station jitter and the slot jitter, then whatever the shape function
//! draws. The jitter draws are unconditional even when the jitter is zero, so editing a jitter moves no other
//! number — the same rule the leaf field follows. Changing this order changes every plant a rule makes, so it
//! is stated here once.

use shape::{Curve, Scalar};

use crate::field::Field;
use crate::handle::Handle;
use crate::node::{Attachment, Node};
use crate::plant::Plant;
use crate::rng::Rng;

/// What a child's spine and taper are, given the node's own rng and the joint it hangs from.
pub type Shape = fn(&mut Rng, &Joint) -> (Curve, Scalar);

/// Where a child hangs from, as much as a shape function needs to know.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    /// The parent's radius at the attachment, so a child's taper can be a fraction of what carries it.
    pub parent_radius: f32,
    /// How deep the child hangs: the trunk's children are depth `1`.
    pub depth: usize,
    /// The child's rank among its siblings, and how many there are.
    pub index: u32,
    pub siblings: u32,
}

/// How a species makes one generation below its parent.
#[derive(Debug, Clone, Copy)]
pub struct Generation {
    /// How many children a node makes, at least and at most: the seed picks between them when they differ.
    pub count: (u32, u32),
    /// The band of stations they hang in, in the order they are made: the first child hangs at `from` and the
    /// last at `to`, and the rest are spread evenly between them. A band of one point puts every child at one
    /// station, which is a whorl.
    pub station: (f32, f32),
    /// Where the first child hangs around the parent, and how far each next one turns.
    pub slot: (f32, f32),
    /// How far the seed may move a child: a fraction of the station band, and a fraction of the slot step.
    pub jitter: (f32, f32),
    /// How many children share one station, `1` for one child to a station.
    ///
    /// A conifer's whorl is the branch-side twin of a leaf field's [`crate::leaf::LeafField::cluster`]:
    /// `count` children gathered at `count / cluster` stations, the children of one station standing
    /// `1 / cluster` of a turn apart and each next station advancing by the slot step. It is the same
    /// word and the same meaning as the leaf's, because it is the same fact — how many siblings share
    /// one place — asked of a branch instead of a leaf.
    pub cluster: u32,
    /// How far the child's origin sinks into the parent, in the plant's units.
    pub sink: f32,
    /// The child's spine and taper, in the child's own frame: its `x` is outward from the parent's surface.
    pub shape: Shape,
}

/// A species: the trunk, one rule per generation below it, and the fields that shape the result.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The trunk's spine and taper.
    pub trunk: fn() -> (Curve, Scalar),
    /// `generations[0]` makes the trunk's children, `generations[1]` theirs, and so on.
    pub generations: Vec<Generation>,
    /// The forces that shape every spine the rule makes, applied in V5's order: after the growth, before the
    /// expansion. A gravity belongs here and a light direction belongs to a scene, which is why they are a list
    /// the caller may replace rather than a constant inside [`grow`].
    pub fields: Vec<Field>,
}

/// Grows a plant from a rule and a seed.
///
/// # Invariants
///
/// **The tree is a function of `(rule, seed)` and nothing else**, node for node and in the same order, so two
/// calls give two identical plants and a handle means the same node in both. A child is appended to its parent
/// in the order it is made, so its index — and therefore its handle — is its rank among its siblings.
pub fn grow(rule: &Rule, seed: u64) -> Plant {
    let (spine, taper) = (rule.trunk)();
    let mut plant = Plant::new(Node::root(spine, taper), seed).with_fields(rule.fields.clone());
    for (depth, generation) in rule.generations.iter().enumerate() {
        let parents: Vec<Handle> = plant
            .walk()
            .filter(|(handle, _)| handle.depth() == depth)
            .map(|(handle, _)| handle)
            .collect();
        for parent in parents {
            let mut rng = plant.rng(&parent);
            let count = if generation.count.0 == generation.count.1 {
                generation.count.0
            } else {
                let drawn = rng.range(generation.count.0 as f32, (generation.count.1 + 1) as f32);
                (drawn as u32).min(generation.count.1)
            };
            // **A cluster of children is one station**, so the band is spread over the stations and not
            // over the children: at a cluster of one the station index is the child index and the
            // within-station turn is nought, which is exactly the spread this rule has always made.
            let cluster = generation.cluster.max(1) as usize;
            let stations = (count as usize).div_ceil(cluster);
            for index in 0..count {
                let tier = index as usize / cluster;
                let within = index as usize % cluster;
                let station = spread(generation.station, tier as u32, stations as u32)
                    + rng.signed()
                        * generation.jitter.0
                        * (generation.station.1 - generation.station.0);
                let slot = generation.slot.0
                    + generation.slot.1 * tier as f32
                    + within as f32 / cluster as f32
                    + rng.signed() * generation.jitter.1 * generation.slot.1;
                let station = station.clamp(0.0, 1.0);
                let parent_radius = plant
                    .get(&parent)
                    .expect("a handle the walk yielded resolves")
                    .taper_now()
                    .at(station);
                let joint = Joint {
                    parent_radius,
                    depth: depth + 1,
                    index,
                    siblings: count,
                };
                let (spine, taper) = (generation.shape)(&mut rng, &joint);
                let attachment = Attachment::new(station, slot).sunk(generation.sink);
                plant
                    .get_mut(&parent)
                    .expect("a handle the walk yielded resolves")
                    .push(Node::attached(spine, taper, attachment));
            }
        }
    }
    plant
}

/// The station of child `index` of `count`, spread evenly over the band.
///
/// # Invariants
///
/// One child hangs at the band's start and, when there are more than one, the last at its end: a band of one
/// point therefore puts every child at that point. The even spread is what makes two bands independent — an
/// author who moves a band's end moves the last child and nobody else's rank.
fn spread(band: (f32, f32), index: u32, count: u32) -> f32 {
    if count <= 1 {
        return band.0;
    }
    band.0 + (band.1 - band.0) * index as f32 / (count - 1) as f32
}

/// Why a graft did not land.
#[derive(Debug, Clone, PartialEq)]
pub enum GraftError {
    /// The handle names no node.
    Orphaned(crate::plant::HandleOrphaned),
    /// The rule has no generation below that node: it is as deep as the species goes.
    NoGeneration {
        /// The depth of the node the graft was aimed at.
        depth: usize,
        /// How deep the rule goes: one generation per level below the trunk.
        generations: usize,
    },
    /// The station or the slot is not a place on the parent.
    OutOfRange {
        /// Where the graft was aimed along the parent.
        station: f32,
        /// Where it was aimed around the parent.
        slot: f32,
    },
}

impl std::fmt::Display for GraftError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraftError::Orphaned(orphaned) => write!(formatter, "{orphaned}"),
            GraftError::NoGeneration { depth, generations } => write!(
                formatter,
                "a node at depth {depth} has no generation below it: the rule has {generations}"
            ),
            GraftError::OutOfRange { station, slot } => write!(
                formatter,
                "station {station} and slot {slot} are not both places on the parent"
            ),
        }
    }
}

/// Adds one child to a node, from the generation the rule puts below it: V5's *click-place a node where you
/// clicked*, with the species still deciding what that node is.
///
/// # Invariants
///
/// **The generation is the node's own depth**, not a parameter: the rule already says what grows below a node at
/// that depth, so there is nothing for a caller to disagree with — a graft is one more of what the rule would
/// have made, at the place the author pointed at. Its shape is the generation's shape function, drawn from **the
/// new node's own stream**, forked by the handle it lands on, so a graft is as deterministic and as local as a
/// grown child and adding one moves no other node's numbers.
pub fn graft(
    plant: &mut Plant,
    rule: &Rule,
    at: &Handle,
    station: f32,
    slot: f32,
) -> Result<Handle, GraftError> {
    let depth = at.depth();
    let generation = *rule
        .generations
        .get(depth)
        .ok_or(GraftError::NoGeneration {
            depth,
            generations: rule.generations.len(),
        })?;
    if !(0.0..=1.0).contains(&station) || !(0.0..=1.0).contains(&slot) {
        return Err(GraftError::OutOfRange { station, slot });
    }
    let parent_radius = plant
        .get(at)
        .map_err(GraftError::Orphaned)?
        .taper_now()
        .at(station);
    let index = plant
        .get(at)
        .map_err(GraftError::Orphaned)?
        .children()
        .len() as u32;
    let handle = at.child(index);
    let mut rng = plant.rng(&handle);
    let joint = Joint {
        parent_radius,
        depth: depth + 1,
        index,
        siblings: index + 1,
    };
    let (spine, taper) = (generation.shape)(&mut rng, &joint);
    let attachment = Attachment::new(station, slot).sunk(generation.sink);
    plant
        .get_mut(at)
        .map_err(GraftError::Orphaned)?
        .push(Node::attached(spine, taper, attachment));
    Ok(handle)
}

/// A spine through the given `(position, tangent)` pairs, in the node's own frame.
///
/// A shape function's drawing tool: it is `shape::Curve` with the species' own refusal message, kept here so
/// every rule builds its curves the same way.
pub fn spine(knots: &[([f32; 3], [f32; 3])]) -> Curve {
    Curve::new(
        knots
            .iter()
            .map(|(position, tangent)| shape::Jet::new(*position, *tangent))
            .collect(),
    )
    .expect("a species' knots are distinct")
}

/// A taper law through the given `(t, radius)` points.
pub fn taper(knots: &[(f32, f32)]) -> Scalar {
    Scalar::new(knots).expect("a species' taper is a law")
}
