//! How the two faces meeting at an edge join.
//!
//! Both kinds of join share one thing and differ in one. **Shared**: the boundary itself is a
//! single curve, so both faces interpolate the same control points — that is what keeps a crease
//! from being a crack. **Different**: a soft edge also ties the control points immediately inside
//! the boundary, which is what makes the tangent plane continuous across it.
//!
//! A hard edge is therefore *not* "no relation between the two sides". It is a shared curve with
//! independent interiors.

use crate::simplex::Point3;
use crate::topology::{EdgeKey, EdgeSlot, FaceId};

/// How an edge joins the faces on it.
#[derive(Clone, Debug, PartialEq)]
pub enum Continuity {
    /// A crease. The boundary row is shared; the rows either side of it are independent.
    Hard,
    /// Tangent-plane continuous.
    ///
    /// `mu` carries one scalar per boundary control point, indexed by the **master** face's local
    /// position along the edge. It is the cross-boundary slope the slave takes relative to the
    /// master:
    ///
    /// - `1` — the cross-boundary derivatives match, so the join is C1.
    /// - anything else — the derivatives are parallel but not equal, so the join is geometrically
    ///   (G1) continuous with a reparameterization across the seam.
    /// - `0` — the slave's interior row sits on the boundary, so the tangent plane is defined by
    ///   the master side alone.
    ///
    /// The values are free. The crease curve and both interior rows stay shapeable; a soft edge
    /// gives up only the two components by which the two sides could otherwise tilt the
    /// cross-boundary direction independently.
    Soft(Vec<f64>),
}

impl Continuity {
    /// A soft edge with every position at the same slope.
    ///
    /// `1` is C1, and is the right default because it is the join a surface looks smooth at; the
    /// per-position values exist for the cases where the seam should flatten out.
    pub fn soft_uniform(degree: usize, mu: f64) -> Self {
        Self::Soft(vec![mu; degree + 1])
    }

    pub fn is_soft(&self) -> bool {
        matches!(self, Self::Soft(_))
    }

    pub fn mu(&self) -> Option<&[f64]> {
        match self {
            Self::Soft(mu) => Some(mu),
            Self::Hard => None,
        }
    }
}

/// The point the slave's interior row must hold at one boundary position.
///
/// `boundary` is the shared curve point, `master_interior` the master face's row one step in from
/// it, and `mu` the continuity parameter there.
///
/// The master and slave interiors sit on opposite sides of the edge, so the slave's offset from the
/// boundary is the master's offset reversed and scaled. `mu == 1` is the C1 case, where the two
/// sides are mirror images about the boundary.
pub fn slave_interior(boundary: Point3, master_interior: Point3, mu: f64) -> Point3 {
    boundary + (boundary - master_interior) * mu
}

/// A soft edge, and which side of it decides.
#[derive(Clone, Debug, PartialEq)]
pub struct SoftEdge {
    /// The mesh edge this is, in canonical order.
    pub edge: EdgeKey,
    /// The face whose interior row is authoritative.
    pub master: (FaceId, EdgeSlot),
    /// The face that follows it.
    pub slave: (FaceId, EdgeSlot),
    /// One scalar per boundary position, in the master's local order.
    pub mu: Vec<f64>,
}
