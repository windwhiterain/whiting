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
//! # A slab says what it is
//!
//! The ground plate a mass stands on, the floor between two storeys and the plate a flat roof is are
//! the same prism and three different things, so the document says which: [`SlabRole`] is authored
//! where the slab is built — [`Slab::plate`], [`Slab::floor`], [`Slab::roof`] — and nothing infers it
//! from where the slab happens to stand. [`validate`](crate::validate) reads the role to decide
//! whether a hosted slab is the mass's plate, which is a question about the building rather than
//! about its elevations.
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

/// What a slab is in the building it stands in.
///
/// # Invariants
///
/// **A role is authored and nothing infers it.** The three are the three plates a building is made
/// of, and each covers a different fraction of its host on purpose: a [`Plate`](Self::Plate) is the
/// floor a mass's wall ring leaves it, a [`Floor`](Self::Floor) is a storey that meets its walls from
/// the inside, and a [`Roof`](Self::Roof) sits on the wall top over the eave. Reading that out of the
/// elevations instead — *a plate is the hosted slab whose top stands at its host's ground line* — asks
/// a reader of a document to know where each slab sits before they can know what it is, and leaves a
/// millimetre of window in which a floor a millimetre above the ground is a plate. Measured on a
/// starter: a floor whose top stood `0.0005` above the ground line was reported as a plate
/// `15.800003` m² short of its floor, by a rule with no tolerance of its own to blame.
///
/// **A role is about the slab and never about its host.** [`Slab::host`] is a seating relation and
/// all three roles carry one; the two fields are independent because a step, a plinth and a canopy
/// are plates with no host at all, and an unhoused slab is left exactly where its author put it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SlabRole {
    /// The ground plate a mass stands on, its top at that mass's ground line.
    Plate,
    /// An intermediate floor between two storeys, inside the walls it meets.
    Floor,
    /// The plate a flat roof is, seated on the wall top over the eave.
    Roof,
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
    /// What this slab is, and the whole of what a reader needs to know about it beyond its two
    /// elevations and its plan.
    ///
    /// # Invariants
    ///
    /// It is **authored where the slab is built** and is set by one of the three constructors, so a
    /// slab cannot exist without saying what it is; it is never a field an author edits afterwards
    /// and nothing derives it from the geometry. [`validate`](crate::validate) reads it to select the
    /// mass's plate, which is the one question the elevations could not answer without a tolerance.
    pub role: SlabRole,
    /// The building this slab moves with, when it is a slab of that building's rather than a step or a
    /// canopy standing on its own.
    ///
    /// # Invariants
    ///
    /// It is what lets seating move the slab with the building it belongs to: a flat roof's elevations
    /// are the wall top's and a first floor's are its storey's, so a seat that moved the mass and not
    /// its slab would leave either behind at the old level. **It is a seating relation and not a
    /// role**: a plate, a floor and a roof all carry one, and a slab with no host does not move — which
    /// is what a canopy hung from a wall head and a step on a path both are.
    pub host: Option<MassId>,
}

impl Slab {
    /// Builds the **ground plate** a mass stands on: the floor its wall ring leaves it, with its top at
    /// that mass's ground line.
    ///
    /// # Invariants
    ///
    /// **It is the only role [`PlateCoverage`](crate::validate::Violation::PlateCoverage) measures**,
    /// because a plate is the one slab whose plan area is a fault when it comes up short: a floor is
    /// inset by the walls it meets and a roof hangs over the eave, so both cover less than the floor
    /// by design and neither has a floor to fall short of.
    ///
    /// A plinth, a step and a canopy are plates too and carry no host; an unhoused plate is left where
    /// its author put it and is never asked to cover a mass. `derive::starter` and the scenes that write
    /// their own footprints are the authoring routes, and both seat the plate under a mass whose walls
    /// stand on the ground line across the annulus outside it.
    pub fn plate(outline: Vec<Vec2>, bottom: f32, top: f32) -> Self {
        Self::new(outline, bottom, top, SlabRole::Plate)
    }

    /// Builds an **intermediate floor**: the storey between two others, seated on the host's inner loop
    /// so it meets its walls rather than running under them.
    ///
    /// # Invariants
    ///
    /// [`derive::floor_slab`](crate::derive::floor_slab) is the authoring route and it refuses an
    /// elevation that is not strictly inside the mass's height: **where a floor is a question of its
    /// own**, and it is answered in one place rather than by a constructor that would have to guess
    /// whether the elevation it was handed is a floor's or a plate's.
    pub fn floor(outline: Vec<Vec2>, bottom: f32, top: f32) -> Self {
        Self::new(outline, bottom, top, SlabRole::Floor)
    }

    /// Builds the plate a **flat roof** is, seated on the wall top over the eave.
    ///
    /// # Invariants
    ///
    /// Its underside is the eave's own elevation, so it sits on the wall top with the same `sink` a
    /// pitched roof gets and its top is one thickness above it. It is hosted, which is what makes
    /// seating move the roof with the building rather than leaving it behind at the old level.
    pub fn roof(outline: Vec<Vec2>, bottom: f32, top: f32) -> Self {
        Self::new(outline, bottom, top, SlabRole::Roof)
    }

    /// Builds a slab of the named role.
    ///
    /// **One derivation for all three**, which is what makes the role a fact about a slab rather than a
    /// default some constructors remembered: every slab in the crate is built through
    /// [`plate`](Self::plate), [`floor`](Self::floor) or [`roof`](Self::roof), and this is the only
    /// place the four fields are set. `O(nodes)` to take the plan loop.
    fn new(outline: Vec<Vec2>, bottom: f32, top: f32, role: SlabRole) -> Self {
        Self {
            outline,
            bottom,
            top,
            role,
            host: None,
        }
    }

    /// Makes this slab the building's own, so seating moves it with the building.
    ///
    /// # Invariants
    ///
    /// **The role is untouched by it.** A plate, a floor and a flat roof all stand at an elevation its
    /// host's seat moves, and a slab with no host stays exactly where its author put it whatever role it
    /// carries — so the two fields are independent and neither one implies the other.
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
