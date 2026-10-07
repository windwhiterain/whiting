//! Naming a point on a line, and the form sampling reads a line's nodes in.
//!
//! A [`Station`] is the correspondence's whole vocabulary: a line, and an arc-length
//! parameter along it. `Attach.t` is the same parameter on a ridge, so a junction and a rib
//! are named the same way and neither needs a position of its own. See
//! [SPEC.align/mountain.md](../../../.agents/SPEC.align/mountain.md) for why the
//! correspondence lives here rather than in a triangulation step.

use crate::math::Vec2;
use crate::skeleton::{FootprintNode, RidgeId, RidgeNode, ValleyId, ValleyNode};

/// Which line a station names.
///
/// # Invariants
///
/// Every line is listable, including valleys. A station that could name only a ridge could
/// not describe the region beyond a ridge tip — where the two flanking valleys face each
/// other and nothing else bounds them — and closing that region would need a mechanism of
/// its own. The variants are ordered so that a set of stations has one sorted order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LineRef {
    /// A ridge polyline.
    Ridge(RidgeId),
    /// A valley polyline.
    Valley(ValleyId),
    /// The footprint, whose parameter runs once around the loop.
    Footprint,
}

/// A point named on a line by arc-length parameter.
///
/// # Invariants
///
/// `t` runs from `0.0` at the line's first node to `1.0` at its last, measured in the plan.
/// Its position and elevation are **derived** from the named line and never stored: a
/// station holds no copy, so moving the line moves everything pinned to it. That is the same
/// rule `Attach.t` follows, and it is why there is one junction vertex rather than two that
/// nearly agree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    /// The line the point is on.
    pub line: LineRef,
    /// The arc-length parameter, where `0.0` is the first node and `1.0` the last.
    pub t: f32,
}

impl Station {
    /// Names a point on a line.
    pub fn new(line: LineRef, t: f32) -> Self {
        Self { line, t }
    }

    /// Names a point on a ridge.
    pub fn ridge(ridge: RidgeId, t: f32) -> Self {
        Self::new(LineRef::Ridge(ridge), t)
    }
}

/// Which of a line's two sides something lies on, read along the line's own direction.
///
/// # Invariants
///
/// The side is a fact about the plan and not about the ridge/valley distinction: a line's left is
/// the half-plane the left normal of its direction points into. The tiling check in
/// [`crate::validate`] is per side, because the two valleys facing one ridge are free to subdivide
/// it differently and each still has to cover its own half.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    /// The left half, read along the line's direction.
    Left,
    /// The right half, read along the line's direction.
    Right,
}

/// Returns whether an arc from one parameter to another on a closed line runs forwards.
///
/// # Invariants
///
/// A closed line has **two** arcs between any two of its points, and this picks the shorter one.
/// Picking by index order instead is what makes a rim seam: the loop's parameter runs from zero
/// to one and then back to zero, so the wedge that straddles the seam has a station near one and
/// a station near zero, and the run between them read forwards is the whole rim rather than the
/// edge that closes it. On an open line there is one arc and the direction is the one that goes
/// from `from` to `to`.
pub fn arc_runs_forwards(from: f32, to: f32, closed: bool) -> bool {
    if !closed {
        return to >= from;
    }
    forwards(from, to) <= backwards(from, to)
}

/// Returns the parameter spans an arc covers, split at the seam of a closed line.
///
/// # Invariants
///
/// The spans are inside `[0, 1]` and are what a coverage check unions. An arc that crosses the
/// seam is **two** spans and not one span with its ends the wrong way round, because the thing
/// being computed is a set of parameters and a set that wraps is not an interval.
pub fn arc_spans(from: f32, to: f32, closed: bool) -> Vec<(f32, f32)> {
    if !closed {
        return vec![(from.min(to), from.max(to))];
    }
    if arc_runs_forwards(from, to, true) {
        if to >= from {
            vec![(from, to)]
        } else {
            vec![(from, 1.0), (0.0, to)]
        }
    } else if to <= from {
        vec![(to, from)]
    } else {
        vec![(to, 1.0), (0.0, from)]
    }
}

/// Returns how far forwards round a closed line one parameter is from another.
fn forwards(from: f32, to: f32) -> f32 {
    (to - from).rem_euclid(1.0)
}

/// Returns how far backwards round a closed line one parameter is from another.
fn backwards(from: f32, to: f32) -> f32 {
    (from - to).rem_euclid(1.0)
}

/// One authored node of a line, in the form sampling reads it.
///
/// # Invariants
///
/// Every line's nodes are read through this one shape, so the sampler has one
/// implementation rather than three. The attributes a line does not carry — a valley has no
/// flank width and no crest sharpness — are zero, and a consumer is expected to read them
/// only on the lines that carry them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    /// The node's plan position.
    pub position: Vec2,
    /// The node's elevation.
    pub elevation: f32,
    /// How wide the ridge's flank is here, and zero on a line that is not a ridge.
    pub half_width: f32,
    /// How sharp the crest is here, and zero on a line that is not a ridge.
    pub sharpness: f32,
}

impl From<&RidgeNode> for Node {
    fn from(node: &RidgeNode) -> Self {
        Self {
            position: node.position,
            elevation: node.elevation,
            half_width: node.half_width,
            sharpness: node.sharpness,
        }
    }
}

impl From<&ValleyNode> for Node {
    fn from(node: &ValleyNode) -> Self {
        Self {
            position: node.position,
            elevation: node.elevation,
            half_width: 0.0,
            sharpness: 0.0,
        }
    }
}

impl From<&FootprintNode> for Node {
    fn from(node: &FootprintNode) -> Self {
        Self {
            position: node.position,
            elevation: node.elevation,
            half_width: 0.0,
            sharpness: 0.0,
        }
    }
}
