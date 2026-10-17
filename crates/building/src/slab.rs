//! Slabs: a level prism over a plan polygon, which is the shape a flat roof is.
//!
//! # Why this is its own element
//!
//! A flat roof cannot be a [`Roof`](crate::roof::Roof): that model's skin is a strip between a **crest
//! row** and an **eave row**, so zero rise makes the two rows coincide in plan and every facet of the
//! strip has no area — the same failure the square hip already taught this crate. It cannot be a
//! [`Mass`](crate::mass::Mass) either: a mass is a **room**, and a plate with a coincident floor is
//! degenerate (measured on the shed's canopy, eight facets of no area).
//!
//! So the crate's third primitive is the plate: **a prism over a plan polygon**, which is what flat
//! roofs, canopies, plinths, terraces, steps and balcony decks all are.
//!
//! # A slab is level, and that is the domain
//!
//! `bottom` and `top` are **elevations, not per-node values**, so every face of the prism is planar by
//! construction — the two caps are the polygon itself and each side is a rectangle. A *sloped* plate is a
//! roof, and this type refuses to be one rather than growing a second geometry: per-node elevations would
//! make the caps a ruled surface for any real outline, which is the fault the shell's floor already has to
//! avoid.
//!
//! # Winding
//!
//! A positively wound plan loop puts the solid on the walker's **right**, which is why a side face is
//! wound `[bottom_i, top_i, top_i+1, bottom_i+1]` — up, along, down, back — and its right-handed normal
//! points out of the material. The caps take the loop's own winding and an `up` flag decides which way
//! they face, the same treatment the shell's floor and ceiling get.

use crate::mass::MassId;
use crate::math::Vec2;

/// The index of a slab in a document's slabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlabId(pub usize);

/// Which surface of a slab a face is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SlabSurface {
    /// The upper face.
    Top,
    /// The underside.
    Bottom,
    /// One of the edges, numbered by the loop's own start.
    Edge {
        /// The index of the loop's starting node of that edge.
        index: usize,
    },
}

/// A level prism over a plan polygon.
#[derive(Debug, Clone, PartialEq)]
pub struct Slab {
    /// The plan polygon, positively wound and stored open.
    pub outline: Vec<Vec2>,
    /// The elevation of the underside.
    pub bottom: f32,
    /// The elevation of the upper face, which must be above `bottom`.
    pub top: f32,
    /// The building this slab covers, when it is that building's roof.
    ///
    /// # Invariants
    ///
    /// It is what lets seating move the plate with the building it belongs to: a flat roof's elevations are
    /// the wall top's, so a seat that moved the mass and not its slab would leave the roof behind at the
    /// old level. A slab with no host is a plate standing on its own (a canopy, a step) and it does not
    /// move.
    pub host: Option<MassId>,
}

impl Slab {
    /// Builds a slab over a plan polygon.
    pub fn new(outline: Vec<Vec2>, bottom: f32, top: f32) -> Self {
        Self {
            outline,
            bottom,
            top,
            host: None,
        }
    }

    /// Makes this slab the building's own roof plate, so seating moves it with the building.
    pub fn with_host(mut self, host: MassId) -> Self {
        self.host = Some(host);
        self
    }

    /// Returns how thick the prism is, which is negative when the author got the two elevations the wrong
    /// way round — and that is a fault, not a slab of no thickness.
    pub fn thickness(&self) -> f32 {
        self.top - self.bottom
    }

    /// Returns the signed plan area of the polygon.
    ///
    /// # Invariants
    ///
    /// It is signed for the same reason every other plan area in this crate is: a loop wound the wrong way
    /// is a slab inside out, and the number that says so is the same number the volume is built from.
    pub fn plan_area(&self) -> f32 {
        let mut twice = 0.0_f32;
        let count = self.outline.len();
        for index in 0..count {
            let from = self.outline[index];
            let to = self.outline[(index + 1) % count];
            twice += from[0] * to[1] - to[0] * from[1];
        }
        twice * 0.5
    }

    /// Returns the volume the prism encloses: its plan area times its thickness.
    ///
    /// It is the number the probe compares with the mesh's signed volume, so the two derivations of one
    /// quantity — one written down here and one measured by the expansion — have to agree.
    pub fn volume(&self) -> f32 {
        self.plan_area() * self.thickness()
    }
}
