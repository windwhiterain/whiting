//! The knobs the expansion reads, and the names an override is recorded under.
//!
//! Nothing here is a property of the mountain. It is all "how finely, and which way the
//! facets break", which is why it is one small value rather than fields on the lines.

use std::collections::BTreeSet;

use crate::line::Side;
use crate::skeleton::ValleyId;

/// How the mesh is sampled out of the skeleton, and how its strips are stepped.
#[derive(Debug, Clone, PartialEq)]
pub struct Tessellation {
    /// The plan spacing a line's edges are subdivided at, before its own `step_scale`.
    pub step: f32,
    /// Which way a belt's strip steps when it has a choice.
    pub diagonal: DiagonalRule,
    /// Cells whose diagonal the author has overridden.
    pub overrides: BTreeSet<DiagonalOverride>,
}

impl Default for Tessellation {
    /// Returns the sampling a massif is built with when the author says nothing.
    ///
    /// A step of two units across a massif measured in hundreds is around a hundred samples
    /// along a flank: fine enough that a facet slope reads as the slope it is, coarse enough
    /// that the mesh is a few thousand triangles rather than a few hundred thousand.
    fn default() -> Self {
        Self {
            step: 2.0,
            diagonal: DiagonalRule::Progress,
            overrides: BTreeSet::new(),
        }
    }
}

impl Tessellation {
    /// Sets the plan spacing lines are subdivided at.
    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Sets which way a belt's strip steps.
    pub fn with_diagonal(mut self, diagonal: DiagonalRule) -> Self {
        self.diagonal = diagonal;
        self
    }
}

/// Which way a belt's triangle strip breaks each of its cells.
///
/// # Invariants
///
/// A belt is a strip between two polylines, and every cell in it has two diagonals: the strip
/// reaches a vertex of the top boundary first or a vertex of the bottom first. Which one it
/// picks is what decides how the facet pair tilts, which is what the eye reads as the block
/// structure of the rock. It is therefore an authoring decision and not an implementation
/// detail, which is why it is named here and can be overridden per cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagonalRule {
    /// Step whichever boundary is behind in arc length. The straightest strip, and the
    /// default: on a flank that is a ruled surface between crest and valley this produces
    /// the diagonal that follows the fall line.
    Progress,
    /// On a near-tie, step the boundary that was not stepped last. This staggers the strip,
    /// which breaks up the regular rhythm a long flank otherwise has — the same reason a
    /// mason does not lay every course the same way.
    Alternate,
}

/// One belt: the surface between one valley segment and the line it faces.
///
/// # Invariants
///
/// The three parts are the valley it belongs to, which of the valley's two sides it is on,
/// and which of the valley's segments it spans. That is enough to name it again after an
/// edit that did not change the valley's own node list, which is what makes an override
/// survive a rebuild rather than being dropped at the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BeltRef {
    /// The valley the belt hangs off.
    pub valley: ValleyId,
    /// Which side of that valley.
    pub side: Side,
    /// Which segment of the valley, by its first node's index.
    pub segment: usize,
}

/// One cell of a belt whose diagonal the author has chosen against the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiagonalOverride {
    /// The belt the cell is in.
    pub belt: BeltRef,
    /// The cell, counted from the belt's first node.
    pub cell: usize,
}
