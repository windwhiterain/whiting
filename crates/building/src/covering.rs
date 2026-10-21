//! **A covering**: the tiles on a roof, as geometry rather than as a texture.
//!
//! # What this is for
//!
//! A Chinese roof is read by its 瓦垄 — the alternating channels of flat tile (板瓦) and the rolls of
//! round tile (筒瓦) that run down the slope — and a roof drawn as one grey plane is a roof with the
//! thing that names it removed. This module puts the rolls on as **solids**: one triangular prism per
//! roll, bent at every purlin the way the skin is, standing on the skin it covers.
//!
//! # Why a roll is its own solid, one per step
//!
//! A solid is one connected closed surface, and **a roll is one solid per 步架**: rolls do not touch
//! each other — they are `period` apart with a channel between them — and a roll that crosses a purlin
//! is a *bend*, which the crate's winding cannot express in one body. A bend inside a solid is a
//! section of it and not a face, so a bent roll's surface has no single consistent corner order to be
//! wound by; measured, a hand-wound bent roll left 24 boundary edges. One prism per step is also what a
//! tiled roof *is*: a course of tiles laid up to the purlin, and the next course above it.
//!
//! # The three numbers, and what each one is measured against
//!
//! - [`Covering::period`] is the **plan** distance between two rolls, measured along the eave. It is a
//!   plan distance and not a slope distance because it is what a tiler lays out on the ground before
//!   the roof exists.
//! - [`Covering::fill`] is how much of that period one roll's base takes, in `(0, 1)`. A fill of one
//!   puts the rolls edge to edge and the covering becomes a slab; a fill of `0.62` leaves the channel
//!   that a 瓦垄 is.
//! - [`Covering::height`] is how far the roll's apex stands above the skin, measured **up**, which is
//!   the direction a tile's own weight holds it in and the one direction a prism can be lifted without
//!   asking the skin's normal.
//!
//! # What is not here
//!
//! The covering has no **lap**: two rolls do not overlap along the slope, because a roll is one solid
//! from the eave to the crest and a lap is what you need when the tiles are cut into courses. A
//! covering of courses (板瓦 laid across the slope) is the `direction` the design record leaves open,
//! and it is not built: what a south-west roof shows is the rolls.

use crate::roof::RoofId;

/// One roll of the covering, named by its roof and its own index on that roof.
///
/// # Invariants
///
/// **The index counts rolls along the roof's own planes, in the order the expansion derives them**, so
/// a roll is named by what it is (the fifth roll on roof 0) rather than by a document-level list it does
/// not appear in. The derivation is deterministic — the same document answers the same indices — which
/// is what makes the name stable enough to be an element's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RollId {
    /// The roof the roll covers.
    pub roof: RoofId,
    /// Which roll, counting along the roof's planes in the order they are expanded.
    pub index: usize,
}

/// Which surface of one roll a face is.
///
/// # Invariants
///
/// **An element is one plane, so a surface that bends is more than one element.** A roll is a chain of prisms, one per
/// step of the roof's profile, and it bends at every purlin: each prism is planar — that is what the guard in the
/// expansion checks — but the chain is not, and naming all of them `Base` or `Slope { side }` puts a bent strip into one
/// planarity group. Measured on a 竹楼's hip: two rolls whose own prisms depart their planes by under a millimetre were
/// reported `FaceNotPlanar` at `1.106e-3`, because the group's fitted plane spans the purlin. The `step` is what makes
/// the name a plane's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RollSurface {
    /// The face the roll stands on, which is on the skin and is the roll's own base.
    Base {
        /// Which step of the roof's profile this segment spans.
        step: usize,
    },
    /// One of the roll's two slopes.
    Slope {
        /// Which side: `0` is the side toward decreasing `across`, `1` the other.
        side: usize,
        /// Which step of the roof's profile this segment spans.
        step: usize,
    },
    /// One of the two ends, which are the only triangles a covering draws.
    End {
        /// Whether this is the eave end.
        start: bool,
    },
}

/// The tiles on a roof, as the rolls that run down its slopes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Covering {
    /// The plan distance between two rolls, measured along the eave, in metres.
    pub period: f32,
    /// How much of the period one roll's base takes, in `(0, 1)`.
    pub fill: f32,
    /// How far the roll's apex stands above the skin, in metres.
    pub height: f32,
}

impl Covering {
    /// Builds a covering.
    pub fn new(period: f32, fill: f32, height: f32) -> Self {
        Self {
            period,
            fill,
            height,
        }
    }

    /// Returns what is wrong with the covering, or `None`.
    ///
    /// # Invariants
    ///
    /// One reading, three callers: [`validate`](crate::validate) reports it, the expansion refuses it,
    /// and the probe asks it. A period that is not a length, a fill outside `(0, 1)` and a height that
    /// is not a rise are three statements about one number each, and each has one fix.
    pub fn fault(&self) -> Option<CoveringFault> {
        if !self.period.is_finite() || self.period <= 0.0 {
            return Some(CoveringFault::PeriodNotALength {
                period: self.period,
            });
        }
        if !self.fill.is_finite() || self.fill <= 0.0 || self.fill >= 1.0 {
            return Some(CoveringFault::FillOutOfRange { fill: self.fill });
        }
        if !self.height.is_finite() || self.height <= 0.0 {
            return Some(CoveringFault::HeightNotARise {
                height: self.height,
            });
        }
        None
    }
}

/// What is wrong with a [`Covering`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoveringFault {
    /// A period that is not a positive length, so no two rolls are a distance apart.
    PeriodNotALength {
        /// The period that was given.
        period: f32,
    },
    /// A fill outside `(0, 1)`: an open interval, because a fill of zero is no roll and a fill of one
    /// is a slab.
    FillOutOfRange {
        /// The fill that was given.
        fill: f32,
    },
    /// A height that is not a positive rise, so the roll is flat and the covering is its own base.
    HeightNotARise {
        /// The height that was given.
        height: f32,
    },
}
