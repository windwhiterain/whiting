//! Walls: the one building element that is not a room.
//!
//! # Why a wall is not a mass
//!
//! [`Mass`](crate::mass::Mass) is this crate's **room** — a shell with an inside, a floor disc and a
//! slab under it. A freestanding wall has no inside: it is a prism, and expressing it as a mass would
//! either leave a cavity a gate could look into or degenerate its floor onto its top (measured once
//! already, on the shed's canopy: eight facets of no area). So a wall is its own kind, and it is
//! deliberately the smallest useful one: **one straight run**, a box with a sloped bottom.
//!
//! # Why a courtyard is several walls and not one
//!
//! A chain of runs would have to be offset into a ribbon — a miter on an open path — and the caps where
//! two runs meet would coincide or interpenetrate inside one solid, which a closed surface cannot do
//! without interior faces. Several wall *solids* may overlap at a corner freely: that is exactly what
//! this crate's **two-level watertightness** rule is for, and it is why a courtyard is four walls (or
//! five, with a gate between two of them) rather than one cleverer element.
//!
//! # The bottom follows the ground, the top does not
//!
//! A run's ends carry the ground's own elevation, so the wall's underside follows the ground it crosses
//! and its top stays level — which is what a built wall does and what a wall on a slope has to do. The
//! bottom is therefore a **quad whose two edges are parallel** (both across the wall's thickness), so it
//! is planar and the solid stays planar everywhere.

use crate::mass::PlanNode;
use crate::math::{Vec2, difference, distance, normalized};

/// The index of a wall in a document's walls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WallId(pub usize);

/// Which surface of a wall a face is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WallSurface {
    /// One of the two long faces. `left` is the walker's left when walking from [`Wall::from`] to
    /// [`Wall::to`] along the wall's own direction, which in this crate's frame is the **outward** side
    /// of a positively wound outline — so a courtyard's walls have their `left` faces looking out.
    Side {
        /// Whether this is the left face.
        left: bool,
    },
    /// One of the two ends.
    End {
        /// Whether this is the `from` end.
        start: bool,
    },
    /// The underside, which follows the ground.
    Bottom,
    /// The top, which is level.
    Top,
}

/// One straight run of wall: a prism from `from` to `to`, standing on the ground they carry.
///
/// # Invariants
///
/// - The **top is an elevation and the ends carry the ground's**, so the wall is as tall as the author
///   says above the ground it crosses rather than above a plane. A run whose top is below the ground at
///   either end is a fault ([`Violation::WallTooLow`]), not a wall of no height.
/// - The **thickness is split evenly** about the line from `from` to `to`, which is the line an author
///   draws on a plan: the wall stands on that line rather than beside it.
/// - A run is a **solid**, so its volume is `length × thickness × height above the ground's own middle`
///   and the probe checks it to the digit. It is one of the few places in this crate where a number can
///   be written down analytically and compared.
///
/// [`Violation::WallTooLow`]: crate::validate::Violation::WallTooLow
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wall {
    /// Where the run starts, and the ground there.
    pub from: PlanNode,
    /// Where it ends, and the ground there.
    pub to: PlanNode,
    /// How thick it is, split evenly about the line an author drew.
    pub thickness: f32,
    /// The elevation of its top, which is level.
    pub top: f32,
}

impl Wall {
    /// Builds a wall run.
    pub fn new(from: PlanNode, to: PlanNode, thickness: f32, top: f32) -> Self {
        Self {
            from,
            to,
            thickness,
            top,
        }
    }

    /// Returns the run's plan length.
    pub fn length(&self) -> f32 {
        distance(self.from.position, self.to.position)
    }

    /// Returns the unit direction the run goes in, or `None` when the two ends coincide.
    pub fn direction(&self) -> Option<Vec2> {
        let span = difference(self.to.position, self.from.position);
        let length = distance(self.from.position, self.to.position);
        if length <= f32::EPSILON {
            return None;
        }
        Some(normalized(span))
    }

    /// Returns the unit normal the thickness is split along: the run's direction turned to the walker's
    /// **left**.
    ///
    /// # Invariants
    ///
    /// It is `up × direction`, the same reading every other wound thing in this crate uses: a positively
    /// wound outline puts its solid on the walker's **right**, so the walker's left is the **outward**
    /// side of a courtyard walked the way the courtyard is wound. Measured once, in the sign: walking
    /// east turns the left normal north, because `z` is south here.
    pub fn normal(&self) -> Option<Vec2> {
        let direction = self.direction()?;
        Some([direction[1], -direction[0]])
    }

    /// Returns the ground elevation at the end the ground is **higher** on.
    pub fn highest_ground(&self) -> f32 {
        self.from.elevation.max(self.to.elevation)
    }

    /// Returns the ground elevation at the end the ground is **lower** on.
    pub fn lowest_ground(&self) -> f32 {
        self.from.elevation.min(self.to.elevation)
    }

    /// Returns how tall the wall stands above the ground's own middle, which is its mean height.
    pub fn mean_height(&self) -> f32 {
        self.top - (self.from.elevation + self.to.elevation) * 0.5
    }

    /// Returns the volume the run encloses, which is exact for a box with a sloped bottom.
    ///
    /// `length × thickness × mean height`, and the probe compares it with the signed volume the
    /// expansion produces: two derivations of one number, one written down and one measured.
    pub fn volume(&self) -> f32 {
        self.length() * self.thickness * self.mean_height()
    }
}
