//! One node of a plant: a spine, the law that thickens it, and where it hangs.
//!
//! # The one idea
//!
//! **A node is authored in its parent's attachment frame, not in the world.** [`Attachment`] is three
//! numbers — a station along the parent, a slot around the parent's section, and how far the joint sinks
//! into the parent — and the parent's own frame supplies the rest. So moving the trunk carries the bough,
//! and *"the bough starts at the trunk's skin"* is a consequence of the numbers rather than a constraint
//! someone has to maintain by hand.
//!
//! # What a node is not
//!
//! It is not a mesh. A node's spine and taper are the authored data; the triangles appear only when
//! [`crate::plant::Plant`] is expanded, and every one of them is [`shape::Sweep`]'s. A node also carries no
//! random numbers and no derived positions: its randomness is [`crate::rng::Rng::for_path`]'s, and its
//! placement is [`crate::plant::Plant::placement`]'s, so neither can drift out of step with the tree.

use shape::{Curve, Jet, Scalar, Section};

/// Where a node hangs from its parent: three numbers, and the parent supplies the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attachment {
    /// Where along the parent, as the parent's arc-length fraction.
    pub station: f32,
    /// Where around the parent's section, as an arc-length fraction of the parent's loop.
    pub slot: f32,
    /// How far the node's origin is sunk **into** the parent, along the parent's outward direction.
    pub sink: f32,
}

impl Attachment {
    /// An attachment at a station and a slot, flush with the parent's surface.
    pub fn new(station: f32, slot: f32) -> Self {
        Self {
            station,
            slot,
            sink: 0.0,
        }
    }

    /// The same attachment, sunk `sink` into the parent.
    pub fn sunk(self, sink: f32) -> Self {
        Self { sink, ..self }
    }

    /// The root's attachment: the start of the plant, with nothing to hang from.
    ///
    /// # Invariants
    ///
    /// It is the zero of the type rather than an `Option`, so a node never has to be asked whether it has
    /// an attachment. The root's own placement is the identity, so these three numbers are never read.
    pub fn root() -> Self {
        Self {
            station: 0.0,
            slot: 0.0,
            sink: 0.0,
        }
    }
}

/// What an author has changed about a computed node: the middle tier of the design's three.
///
/// # Invariants
///
/// An offset says *"the law is nearly right here"*, so it is added at every use and never written into the
/// authored data: the taper stays the taper, and the number an author nudged is recoverable. Each field is
/// added to its own computed number — the node's length, its radius at every station, and the two fractions
/// that place it on its parent — so two edits on one node add up and no edit can reinterpret another.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Nudges {
    /// Added to the node's own length, in the plant's units.
    pub length: f32,
    /// Added to the node's taper at every station.
    pub radius: f32,
    /// Added to where the node hangs along its parent, as the parent's arc-length fraction.
    pub station: f32,
    /// Added to where it hangs around the parent's section, as a fraction of the loop.
    pub slot: f32,
}

/// Why a node is a tombstone: a prune records its reason rather than deleting the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prune {
    /// An author deleted it.
    ByAuthor,
    /// A thinning pass removed it to let its neighbours grow.
    Thinned,
    /// It died: a collision, a disease, a broken limb.
    Dead,
}

/// A spine, the law that thickens it, the section it sweeps, and where it hangs.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    spine: Curve,
    taper: Scalar,
    section: Section,
    attachment: Attachment,
    nudges: Nudges,
    shift: [f32; 3],
    pruned: Option<Prune>,
    baked: bool,
    children: Vec<Node>,
}

impl Node {
    /// A node with nothing above it: the plant's root.
    pub fn root(spine: Curve, taper: Scalar) -> Self {
        Self {
            spine,
            taper,
            section: Section::circle(),
            attachment: Attachment::root(),
            nudges: Nudges::default(),
            shift: [0.0; 3],
            pruned: None,
            baked: false,
            children: Vec::new(),
        }
    }

    /// A node that hangs from its parent by `attachment`.
    pub fn attached(spine: Curve, taper: Scalar, attachment: Attachment) -> Self {
        Self {
            spine,
            taper,
            section: Section::circle(),
            attachment,
            nudges: Nudges::default(),
            shift: [0.0; 3],
            pruned: None,
            baked: false,
            children: Vec::new(),
        }
    }

    /// The same node swept with a section other than a circle: a buttressed trunk, a ribbed stalk.
    pub fn with_section(mut self, section: Section) -> Self {
        self.section = section;
        self
    }

    /// The in-place form of [`Node::with_section`], for a node a plant already owns: a species that grows
    /// first and dresses its bole after is the same species as one that authored the section up front.
    pub fn set_section(&mut self, section: Section) {
        self.section = section;
    }

    /// The node's spine, in its parent's attachment frame.
    pub fn spine(&self) -> &Curve {
        &self.spine
    }

    /// The node's spine **as it is drawn**: the authored one, as long as a length nudge says.
    ///
    /// # Invariants
    ///
    /// The whole curve is scaled by one factor, positions and derivatives alike, so every station keeps its
    /// arc-length fraction and a length nudge cannot rotate or bend anything. A node with no nudge returns
    /// its own spine rather than a copy of it, which is the case every node of an untouched plant is in.
    pub fn spine_now(&self) -> Curve {
        if self.nudges.length == 0.0 {
            return self.spine.clone();
        }
        let length = self.spine.length();
        if length <= 0.0 {
            return self.spine.clone();
        }
        let scale = ((length + self.nudges.length) / length).max(1.0e-6);
        let jets: Vec<Jet> = self
            .spine
            .jets()
            .into_iter()
            .map(|jet| {
                Jet::new(
                    [
                        jet.position[0] * scale,
                        jet.position[1] * scale,
                        jet.position[2] * scale,
                    ],
                    [
                        jet.tangent[0] * scale,
                        jet.tangent[1] * scale,
                        jet.tangent[2] * scale,
                    ],
                )
            })
            .collect();
        let mut curve = Curve::new(jets).expect("a scaled valid curve is a valid curve");
        curve.set_initial_normal(self.spine.frame(0.0).normal);
        curve
    }

    /// The law that thickens the node along its own arc length.
    pub fn taper(&self) -> &Scalar {
        &self.taper
    }

    /// The node's taper **as it is drawn**: the authored law with the radius nudge added.
    pub fn taper_now(&self) -> Scalar {
        self.taper.offset(self.nudges.radius)
    }

    /// Adds an author's law to the authored taper: the edit table's `Curve`, which is `Offset` with a law.
    ///
    /// # Invariants
    ///
    /// **It writes the authored law, not the nudge**, so a nudge stays recoverable beside it and a later curve
    /// reads the sum of the curves before it — the same reading [`Node::baked`] gives a spine, one tier down.
    pub(crate) fn add_taper_law(&mut self, law: &Scalar) {
        self.taper = self.taper.plus(law);
    }

    /// The section the node sweeps.
    pub fn section(&self) -> &Section {
        &self.section
    }

    /// Where the node hangs from its parent: the numbers the author wrote.
    pub fn attachment(&self) -> Attachment {
        self.attachment
    }

    /// Where the node hangs **as it is drawn**: the authored attachment with its two nudges added.
    ///
    /// # Invariants
    ///
    /// The sum is not clamped. An edit that would leave `[0, 1]` is refused by
    /// [`crate::ops::apply`] and reported, so a nudge that reached here is in range by construction.
    pub fn effective_attachment(&self) -> Attachment {
        Attachment {
            station: self.attachment.station + self.nudges.station,
            slot: self.attachment.slot + self.nudges.slot,
            sink: self.attachment.sink,
        }
    }

    /// What an author has nudged on this node.
    pub fn nudges(&self) -> Nudges {
        self.nudges
    }

    /// Where the node's origin is moved to, **in the node's own frame**.
    pub fn shift(&self) -> [f32; 3] {
        self.shift
    }

    /// Why this node is a tombstone, or `None` while it is alive.
    pub fn pruned(&self) -> Option<Prune> {
        self.pruned
    }

    /// Whether the node's spine was **authored** rather than derived.
    ///
    /// # Invariants
    ///
    /// A baked spine is the third tier of the design: the author stopped saying *"nearly right"* and said
    /// *"this"*. The marker is what a field reads before it decides to reshape anything — a bake is not field
    /// material — and it is set by [`crate::ops::Change::Spine`] and by nothing else.
    pub fn baked(&self) -> bool {
        self.baked
    }

    pub(crate) fn set_spine(&mut self, spine: Curve) {
        self.spine = spine;
        self.baked = true;
    }

    /// The node's children, in the order they were added.
    pub fn children(&self) -> &[Node] {
        &self.children
    }

    pub(crate) fn nudges_mut(&mut self) -> &mut Nudges {
        &mut self.nudges
    }

    pub(crate) fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    pub(crate) fn shift_mut(&mut self) -> &mut [f32; 3] {
        &mut self.shift
    }

    pub(crate) fn prune(&mut self, reason: Prune) {
        self.pruned = Some(reason);
    }

    /// Adds a child and returns its index, which is its last step in a [`crate::handle::Handle`].
    pub fn push(&mut self, child: Node) -> u32 {
        let index = self.children.len() as u32;
        self.children.push(child);
        index
    }
}
