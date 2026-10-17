//! The openings a wall is cut by: S2's second half.
//!
//! The grid says where a facade's bays are; an opening says which of them is **cut through**. The
//! wall is not composed around the hole from pieces the author places — `gallery::built`'s
//! `WallGap` is that hand-made version, where a gap is two finished runs and the author owns the
//! jambs — and the expander cuts the wall it would have emitted anyway.
//!
//! # The one idea, here
//!
//! **An opening is authored relative to its bay and is measured against its wall.** It names a bay
//! by slot (so a re-split carries it, D10) and gives four insets in metres from that bay's own edges;
//! what it may not do is reach the wall face's boundary, because a hole that touches an edge is a
//! different shape — the wall stops being a rectangle with holes in it and becomes a set of pieces
//! joined at the author's own corners — and this phase cuts rectangles. The refusal is named
//! (`Violation::OpeningOutsideWall`) rather than half-built.
//!
//! A **door** is not a special case and needs no second mechanism: the wall face spans from the
//! slab's underside up to the wall top, so a hole whose sill sits at ground level has a positive
//! margin below it — the slab's own thickness — and is an interior hole like any other.

use std::collections::BTreeMap;

use crate::grid::{Bay, FaceRef, GRID_TOLERANCE};

/// Which surface of the wall's thickness inside an opening a face is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RevealSide {
    /// The face at the opening's lower arc-length end.
    Left,
    /// The face at its upper arc-length end.
    Right,
    /// The face across its head.
    Head,
    /// The face across its sill.
    Sill,
}

impl RevealSide {
    /// Returns every side, in a fixed order.
    pub const ALL: [RevealSide; 4] = [
        RevealSide::Left,
        RevealSide::Right,
        RevealSide::Head,
        RevealSide::Sill,
    ];
}

/// How far an opening is inset from its bay's own four edges, in metres.
///
/// # Invariants
///
/// All four are distances **inward** and none may be negative: an opening that reached past its bay
/// is not a different opening, it is a fault ([`Violation::OpeningExceedsBay`]), and the arithmetic
/// is arranged so that it cannot be read as a smaller one.
///
/// [`Violation::OpeningExceedsBay`]: crate::validate::Violation::OpeningExceedsBay
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Inset {
    /// From the bay's start along the edge.
    pub from: f32,
    /// From the bay's end along the edge.
    pub to: f32,
    /// Up from the bay's sill.
    pub sill: f32,
    /// Down from the bay's head.
    pub head: f32,
}

impl Inset {
    /// Builds an inset.
    pub fn new(from: f32, to: f32, sill: f32, head: f32) -> Self {
        Self {
            from,
            to,
            sill,
            head,
        }
    }

    /// Returns whether every one of the four is a distance inward.
    pub fn is_inward(&self) -> bool {
        self.from >= 0.0 && self.to >= 0.0 && self.sill >= 0.0 && self.head >= 0.0
    }
}

/// The rectangle an opening cuts, in its wall face's own coordinates: arc length along the edge, and
/// elevation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extents {
    /// Where it starts along the edge.
    pub from: f32,
    /// Where it ends along the edge.
    pub to: f32,
    /// The elevation of its sill.
    pub low: f32,
    /// The elevation of its head.
    pub high: f32,
}

impl Extents {
    /// Returns how wide the hole is.
    pub fn width(&self) -> f32 {
        self.to - self.from
    }

    /// Returns how tall the hole is.
    pub fn height(&self) -> f32 {
        self.high - self.low
    }

    /// Returns the area of wall the hole removes.
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// Returns whether two holes share any of their area.
    ///
    /// # Invariants
    ///
    /// Touching is not sharing: two openings that meet exactly along an edge remove no wall twice and
    /// are legal, which is why the comparison is strict on both axes. This is `O(1)` and is what the
    /// overlap check is built from.
    pub fn overlaps(&self, other: &Extents) -> bool {
        self.from < other.to
            && other.from < self.to
            && self.low < other.high
            && other.low < self.high
    }
}

/// A hole cut through one wall of one bay.
///
/// # Invariants
///
/// - It names its bay by **slot** — face, row, column — and never by coordinates, so a re-split of
///   the face carries it (D10) and a slot that no longer exists is reported rather than silently
///   landing somewhere else.
/// - It is always **through** the wall in this phase: a niche or a recess is a different shape (one
///   that has a back), and it arrives with the parts vocabulary rather than as a flag here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opening {
    /// The wall it is cut into.
    pub face: FaceRef,
    /// Which storey of that wall, counting from the face's base.
    pub row: usize,
    /// Which bay across, counting from the edge's start.
    pub column: usize,
    /// How far it is inset from the bay's four edges.
    pub inset: Inset,
}

impl Opening {
    /// Builds an opening in a bay.
    pub fn new(face: FaceRef, row: usize, column: usize, inset: Inset) -> Self {
        Self {
            face,
            row,
            column,
            inset,
        }
    }

    /// Returns the rectangle this opening cuts inside a bay, or `None` when it has no area there.
    ///
    /// # Invariants
    ///
    /// The answer is **the bay's own extents moved inward by the insets**, so an opening is a
    /// statement about its bay and never about the wall: an opening that exceeds its bay comes out
    /// with a negative width or height, which is refused here — the same refusal as a rhythm that
    /// cannot span its face, and for the same reason. `O(1)`.
    pub fn extents(&self, bay: Bay) -> Option<Extents> {
        if !self.inset.is_inward() {
            return None;
        }
        let extents = Extents {
            from: bay.from + self.inset.from,
            to: bay.to - self.inset.to,
            low: bay.low + self.inset.sill,
            high: bay.high - self.inset.head,
        };
        if extents.width() <= GRID_TOLERANCE || extents.height() <= GRID_TOLERANCE {
            return None;
        }
        Some(extents)
    }
}

/// The openings of a building, in the order they were authored.
///
/// # Invariants
///
/// A list and not a map, because the **order is the author's** and two openings in one wall are one
/// edit apart: a map keyed by face would lose which of two holes in the same wall came first, and the
/// mesh's provenance names an opening by its index in this list. Every walk that needs a fixed order
/// — the expander's decomposition — sorts what it reads rather than depending on this one, so the
/// mesh does not move when an opening is added elsewhere in the document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Openings {
    list: Vec<Opening>,
}

impl Openings {
    /// Builds an empty set of openings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an opening and returns the index it was given.
    pub fn push(&mut self, opening: Opening) -> usize {
        self.list.push(opening);
        self.list.len() - 1
    }

    /// Removes one opening by index, returning it.
    pub fn remove(&mut self, index: usize) -> Option<Opening> {
        (index < self.list.len()).then(|| self.list.remove(index))
    }

    /// Returns one opening by index.
    pub fn get(&self, index: usize) -> Option<&Opening> {
        self.list.get(index)
    }

    /// Returns the index and the opening of every hole in one wall, in the order they were authored.
    pub fn of_face(&self, face: FaceRef) -> Vec<(usize, &Opening)> {
        self.list
            .iter()
            .enumerate()
            .filter(|(_, opening)| opening.face == face)
            .collect()
    }

    /// Returns the index of every hole in one wall, grouped by the edge they are in.
    ///
    /// # Invariants
    ///
    /// This is the lookup the expander walks, and it is `O(openings)` once rather than a scan per
    /// edge: an expansion that asked "which openings are in this edge" per edge would put the whole
    /// document into the cost of every wall. The group order is the key order, so two runs of an
    /// unchanged document build the same mesh.
    pub fn by_face(&self) -> BTreeMap<FaceRef, Vec<usize>> {
        let mut grouped: BTreeMap<FaceRef, Vec<usize>> = BTreeMap::new();
        for (index, opening) in self.list.iter().enumerate() {
            grouped.entry(opening.face).or_default().push(index);
        }
        grouped
    }

    /// Returns how many openings the document holds.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Returns whether the document holds no openings.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Returns every opening, in the order they were authored.
    pub fn iter(&self) -> impl Iterator<Item = &Opening> {
        self.list.iter()
    }
}
