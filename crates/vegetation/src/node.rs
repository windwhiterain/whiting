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

use shape::{Curve, Scalar, Section};

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

/// A spine, the law that thickens it, the section it sweeps, and where it hangs.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    spine: Curve,
    taper: Scalar,
    section: Section,
    attachment: Attachment,
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
            children: Vec::new(),
        }
    }

    /// The same node swept with a section other than a circle: a buttressed trunk, a ribbed stalk.
    pub fn with_section(mut self, section: Section) -> Self {
        self.section = section;
        self
    }

    /// The node's spine, in its parent's attachment frame.
    pub fn spine(&self) -> &Curve {
        &self.spine
    }

    /// The law that thickens the node along its own arc length.
    pub fn taper(&self) -> &Scalar {
        &self.taper
    }

    /// The section the node sweeps.
    pub fn section(&self) -> &Section {
        &self.section
    }

    /// Where the node hangs from its parent.
    pub fn attachment(&self) -> Attachment {
        self.attachment
    }

    /// The node's children, in the order they were added.
    pub fn children(&self) -> &[Node] {
        &self.children
    }

    /// Adds a child and returns its index, which is its last step in a [`crate::handle::Handle`].
    pub fn push(&mut self, child: Node) -> u32 {
        let index = self.children.len() as u32;
        self.children.push(child);
        index
    }
}
