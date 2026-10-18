//! A **member**: one straight piece of a frame, from one anchor to another, with a section.
//!
//! # Why this is its own kind of solid
//!
//! A [`Wall`](crate::wall::Wall) and a member are both prisms and they answer different questions. A
//! wall stands on the ground its two ends carry and its top is level, because a wall is masonry: its
//! geometry is the ground's. A member is placed **in the air** — a post whose foot is on a stone and
//! whose head is under a floor, a bracket that spans from a wall to a joist, a purlin along a ridge —
//! so its two ends are *anchors* rather than elevations, and its axis is not necessarily vertical.
//!
//! # Anchors
//!
//! An [`Anchor`] is a point in the building's frame:
//!
//! - [`Anchor::Free`] is the point as authored. It is where a post's foot goes, and it is the only
//!   anchor that can be written without reading another solid.
//! - [`Anchor::OnMember`] is a **fraction of another member's axis**, which is what a frame joint is:
//!   a bracket meets a post at a point *on* it, and the post is what says where. The fraction is
//!   measured from that member's `from` end, so `t = 0` is its foot and `t = 1` its head.
//!
//! **A chain of `OnMember` anchors is resolved before anything is expanded**, so a joint lands at one
//! point rather than two that nearly agree: they are welded by index the way a roof's rows are, and
//! not by a coordinate test. A chain that closes on itself is
//! [`MemberAnchorCycle`](crate::expand::Problem::MemberAnchorCycle), and a member
//! whose anchor names a member the document does not hold is
//! [`MemberAnchorUnresolved`](crate::expand::Problem::MemberAnchorUnresolved) — two different faults
//! because they have two different fixes.
//!
//! # The section, and the two directions that make it
//!
//! A member is a rectangle swept along its axis: [`Member::width`] across [`Member::across`] and
//! [`Member::depth`] across the axis's other perpendicular. `across` is authored as a **plan**
//! direction and projected onto the plane perpendicular to the axis, which is what makes a post's
//! square section square in plan whatever its taper, and a rafter's section square in its own plane.
//! A member whose axis is parallel to `across` has no section to sweep and is
//! [`MemberAcrossDegenerate`](crate::mesh::SolidId).

use crate::math::{Vec2, Vec3, cross3, dot3, length3, normalized3, subtract};

/// Index of a member in the document's own list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemberId(pub usize);

/// Which surface of a member a face is.
///
/// # Invariants
///
/// The four sides are numbered **around the axis**, from the authored [`across`](Member::across)
/// direction: `0` faces `across`, `2` faces away from it, and `1` and `3` are the pair across the
/// axis's other perpendicular — in the order the frame's third vector points, which is right handed
/// with the axis. A square member has four equal sides and the index is still what tells them apart:
/// two sides of one element would be one shading group, and it would round the corner that says the
/// member is square.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemberSurface {
    /// One of the four sides, numbered around the axis from `across`.
    Side {
        /// Which side: `0` `across`, `1` and `3` the other perpendicular's pair, `2` away.
        index: usize,
    },
    /// One of the two ends.
    End {
        /// Whether this is the `from` end.
        start: bool,
    },
}

/// Where one end of a member stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    /// The point as authored, in the building's own frame.
    Free {
        /// Where the end stands.
        at: Vec3,
    },
    /// A fraction of another member's axis, measured from that member's `from` end.
    OnMember {
        /// The member the joint is on.
        member: MemberId,
        /// Where along it, in `[0, 1]`.
        t: f32,
    },
}

/// One straight piece of frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Member {
    /// Where the piece starts.
    pub from: Anchor,
    /// Where it ends.
    pub to: Anchor,
    /// The **plan** direction its width is measured across, projected perpendicular to the axis.
    pub across: Vec2,
    /// How wide it is across [`Member::across`].
    pub width: f32,
    /// How deep it is across the axis's other perpendicular.
    pub depth: f32,
}

impl Member {
    /// Builds a member.
    pub fn new(from: Anchor, to: Anchor, across: Vec2, width: f32, depth: f32) -> Self {
        Self {
            from,
            to,
            across,
            width,
            depth,
        }
    }

    /// Returns the axis's own frame: the unit direction, the unit width direction, and the unit depth
    /// direction, or `None` for an axis with no length or an `across` parallel to it.
    ///
    /// # Invariants
    ///
    /// The three are orthonormal and **right handed** in that order, so `width × depth = axis` and a
    /// face's outward normal is one of `±width, ±depth, ±axis` with no sign guessed at the call site.
    pub fn frame(&self, from: Vec3, to: Vec3) -> Option<(Vec3, Vec3, Vec3)> {
        let axis = subtract(to, from);
        if length3(axis) < 1.0e-6 {
            return None;
        }
        let axis = normalized3(axis);
        let plan = [self.across[0], 0.0, self.across[1]];
        let along = dot3(plan, axis);
        let width = subtract(plan, [axis[0] * along, axis[1] * along, axis[2] * along]);
        if length3(width) < 1.0e-6 {
            return None;
        }
        let width = normalized3(width);
        let depth = normalized3(cross3(axis, width));
        Some((axis, width, depth))
    }

    /// Returns the member's own length, which is what its volume is a section times.
    pub fn length(&self, from: Vec3, to: Vec3) -> f32 {
        length3(subtract(to, from))
    }
}
