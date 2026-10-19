//! The brush: which nodes a change covers, as a predicate rather than a list.
//!
//! # The one idea
//!
//! **A selector names nodes; it does not edit them.** This crate's edits are keyed by a
//! [`Handle`], so a predicate's only job is to say which handles a brush covers: an agent prints
//! [`Selector::handles`] before it applies anything, and the same list can be filtered, logged or
//! shown in a report. That is also why `handles` is walk-ordered and duplicate-free rather than a
//! set — it is a list a reader can read, and the walk already says what order a plant is in.
//!
//! # Two routes to one answer
//!
//! [`Selector::matches`] is the predicate, and [`Selector::handles`] is that predicate evaluated
//! over [`Plant::walk`]. A selector that answers the two differently is two tools with one name.
//!
//! # What a band reads, and what a box reads
//!
//! [`Selector::StationBand`] and [`Selector::SlotBand`] read the node's **authored** attachment
//! ([`Node::attachment`]), never a derived position: a brush over "the lower third of every parent"
//! has to mean the number the author wrote, and has to keep meaning it after the parent's curve is
//! edited. [`Selector::Inside`] is the other kind, and it reads the node's **world origin** through
//! [`Plant::placement`], because a box in a frame is a brush a picture can draw.
//!
//! Design D10 of
//! [`.agents/proposals/agentic-vegetation-generation.md`](../../../.agents/proposals/agentic-vegetation-generation.md):
//! the brush is a predicate, and a `Selector` holding a closure over the mesh — which cannot be
//! written into a document and cannot be printed for a report — is the rejected alternative.

use shape::math::Vec3;

use crate::handle::Handle;
use crate::node::Node;
use crate::plant::{Derived, Plant};

/// A set of nodes, as a predicate rather than a list: the brush.
///
/// # Invariants
///
/// Every variant is a total function of `(plant, handle, node)` — none of them panics, none of them
/// edits, and none of them looks for a nearest neighbour. Composition is the whole language:
/// [`Selector::Any`] and [`Selector::Not`] build every set a caller needs from these leaves, and a
/// predicate that cannot be written as one of the leaves is a predicate this crate cannot print.
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// Every node a plant has.
    Every,
    /// The nodes `depth` steps below the root: the trunk is `0`.
    ///
    /// # Invariants
    ///
    /// A depth no node has names nothing, so `Generation(9)` is an empty brush rather than an error.
    Generation(usize),
    /// A node and everything below it.
    ///
    /// # Invariants
    ///
    /// The node itself is inside its own brush, and **a handle the plant cannot resolve names
    /// nothing**: a path that does not resolve cannot be a prefix of one that does, so a subtree
    /// pruned out from under a stale handle takes the whole brush with it rather than re-binding it
    /// to whatever took the place.
    Descendants(Handle),
    /// The nodes whose attachment station lies in `[from, to]` on their parent.
    ///
    /// # Invariants
    ///
    /// The test is against the number the author wrote ([`Node::attachment`]), not against a
    /// position the plant computed, so the band survives an edit to the parent's spine. Both ends
    /// are inclusive, and the root carries [`crate::node::Attachment::root`]'s station of `0`, so a
    /// band containing zero includes the trunk — a caller who wants "every parent" excludes it with
    /// `Not(Generation(0))` rather than with a special case here.
    StationBand {
        /// The lowest station in the band, inclusive.
        from: f32,
        /// The highest station in the band, inclusive.
        to: f32,
    },
    /// The nodes whose attachment slot lies in `[from, to]` around their parent.
    ///
    /// # Invariants
    ///
    /// As [`Selector::StationBand`], and the slot is the **authored** arc-length fraction in
    /// `[0, 1)` of the parent's loop. A band is a plain interval and **does not wrap at the seam**:
    /// `[0.9, 1.0]` and `[0.0, 0.1]` are two different brushes, because a caller that wants the
    /// nodes on both sides of the seam says `Any` of the two.
    SlotBand {
        /// The lowest slot in the band, inclusive.
        from: f32,
        /// The highest slot in the band, inclusive.
        to: f32,
    },
    /// The nodes whose world origin lies inside the box two corners span.
    ///
    /// # Invariants
    ///
    /// The origin is [`Plant::placement`]'s, so a box is a world-space test, and **a node whose
    /// placement cannot be resolved is not inside** — it is not anywhere. The two corners are
    /// normalised per axis, so a caller may give them in either order, and both faces are inclusive.
    Inside {
        /// One corner of the box; paired with `opposite` axis by axis.
        corner: Vec3,
        /// The corner diagonally across from `corner`; paired with it axis by axis.
        opposite: Vec3,
    },
    /// Exactly these handles.
    ///
    /// # Invariants
    ///
    /// **Membership is the test, and the plant is not consulted**: `matches` answers `true` for any
    /// handle in the list, including a stale one, because membership is what the list says. The
    /// plant's opinion arrives through [`Selector::handles`], which walks the plant and therefore
    /// cannot return a handle the plant no longer has. So a stale handle is *named* by
    /// `matches` and *absent* from `handles` — and, since [`Selector::Not`] is over the plant's
    /// nodes, `Not(Ids(stale))` subtracts nothing and names every node the plant has.
    Ids(Vec<Handle>),
    /// Any of these.
    ///
    /// # Invariants
    ///
    /// An empty list names nothing, which is what makes `Any` a safe join: a caller assembling a
    /// brush from optional parts does not have to know whether any part was present.
    Any(Vec<Selector>),
    /// Everything these do not.
    ///
    /// # Invariants
    ///
    /// The complement is over **the plant's nodes**, not over the whole address space: `Not(Every)`
    /// names nothing, and `Not(Generation(2))` names the four nodes above the twigs. A selector with
    /// no names of its own therefore loses to any node it does not name, rather than naming every
    /// handle that has ever existed.
    Not(Box<Selector>),
}

impl Selector {
    /// Whether the selector names this node.
    ///
    /// `node` is the node `handle` names in `plant`, which is what a caller walking the plant
    /// already has; passing a node that is not the one `handle` names would make the answer
    /// meaningless, so the two travel together rather than one being looked up again.
    pub fn matches(&self, plant: &Plant, handle: &Handle, node: &Node) -> bool {
        self.matches_in(plant, handle, node, &mut Derived::default())
    }

    /// [`Selector::matches`] with the walk's derived table, so a selector that reads a node's placement
    /// composes each ancestor chain once over a whole walk.
    pub(crate) fn matches_in(
        &self,
        plant: &Plant,
        handle: &Handle,
        node: &Node,
        derived: &mut Derived,
    ) -> bool {
        match self {
            Selector::Every => true,
            Selector::Generation(depth) => handle.depth() == *depth,
            // A handle the plant cannot resolve is not a prefix of a handle it can, so this one
            // test is also the whole stale-handle rule.
            Selector::Descendants(base) => handle.path().starts_with(base.path()),
            Selector::StationBand { from, to } => {
                let station = node.attachment().station;
                station >= *from && station <= *to
            }
            Selector::SlotBand { from, to } => {
                let slot = node.attachment().slot;
                slot >= *from && slot <= *to
            }
            Selector::Inside { corner, opposite } => match plant.placement_in(handle, derived) {
                Ok(placement) => spans(corner, opposite, placement.origin),
                Err(_) => false,
            },
            Selector::Ids(ids) => ids.contains(handle),
            Selector::Any(selectors) => selectors
                .iter()
                .any(|selector| selector.matches_in(plant, handle, node, derived)),
            Selector::Not(selector) => !selector.matches_in(plant, handle, node, derived),
        }
    }

    /// The handles the selector names, in `plant.walk()` order, without duplicates.
    ///
    /// # Invariants
    ///
    /// The walk is the only source, so a handle appears exactly once whatever the selector says —
    /// a repeated [`Selector::Ids`] entry does not repeat an answer — and the order is the plant's
    /// own, which is the order a report and an edit both read. Because it walks the plant, the list
    /// is also `walk().filter(matches)` by construction; the two routes are one route.
    ///
    /// **One derived table for the walk**, dropped when it returns: a selector that reads a node's
    /// placement over a whole plant otherwise re-composes every ancestor chain once per node.
    pub fn handles(&self, plant: &Plant) -> Vec<Handle> {
        let mut derived = Derived::default();
        plant
            .walk()
            .filter(|(handle, node)| self.matches_in(plant, handle, node, &mut derived))
            .map(|(handle, _)| handle)
            .collect()
    }
}

/// Whether `point` is in the box `corner` and `opposite` span, the two given in either order.
fn spans(corner: &Vec3, opposite: &Vec3, point: Vec3) -> bool {
    (0..3).all(|axis| {
        let low = corner[axis].min(opposite[axis]);
        let high = corner[axis].max(opposite[axis]);
        (low..=high).contains(&point[axis])
    })
}
