//! How the two faces meeting at an edge join.
//!
//! Both kinds of join share one thing and differ in one. **Shared**: the boundary itself is a
//! single curve, so both faces interpolate the same control points — that is what keeps a crease
//! from being a crack. **Different**: a soft edge also ties the control points immediately inside
//! the boundary, which is what makes the tangent plane continuous across it.
//!
//! A hard edge is therefore *not* "no relation between the two sides". It is a shared curve with
//! independent interiors.
//!
//! # The soft-edge invariant
//!
//! Write `C` for the shared boundary, `A` for the master face's adjacent row and `B` for the
//! slave's, all indexed by position along the edge. The cross-boundary derivative at the boundary is
//!
//! ```text
//! D(v) = degree * sum over j of B(degree-1, j, v) * (A[j] - C[j])
//! ```
//!
//! so both sides' derivatives are degree-`degree - 1` Bézier curves, and their control points are
//! just the rows' offsets from the boundary. G1 asks for the two to be **parallel at every `v`** —
//! the same cross-boundary direction, possibly at different magnitudes.
//!
//! Placing
//!
//! ```text
//! B[j] = C[j] + mu * (C[j] - A[j])
//! ```
//!
//! makes the slave's offset the master's reversed and scaled by `mu`, so its whole derivative
//! curve is `-mu` times the master's. Parallel everywhere, which is G1.
//!
//! **Why `mu` is one scalar for the whole edge and not one per position.** Writing `B[j] = C[j] +
//! mu[j] * (C[j] - A[j])` looks like the same construction with more freedom, and it is not. The
//! slave's derivative curve then has control points `mu[j] * (C[j] - A[j])`, which is a *different*
//! curve from `-mu` times the master's unless every `mu[j]` is equal. Allowing `k(v)` to vary
//! would need `k(v) * D_master(v)` to land back in the degree-`degree - 1` Bézier space, which it
//! generically does not — the product is degree `2(degree-1)`. So `k` is constant in general, one
//! free scalar per edge, and a per-position slope was never available to give up.
//!
//! - `1` — the cross-boundary derivatives match, so the join is C1.
//! - anything else — the derivatives are parallel but not equal, so the join is geometrically (G1)
//!   continuous with a reparameterization across the seam.
//! - `0` — the slave's row sits on the boundary, so the tangent plane is defined by the master side
//!   alone.
//!
//! # Why the invariant stops one short of each end
//!
//! [`interior_index`](crate::topology::interior_index) hands out positions `0..=degree`, but the
//! invariant is applied only at `1..=degree - 1`. The two end positions are not this edge's to
//! decide: `(1, 0)` on a `MinU` row is also a point of the same face's `MinV` boundary, and the
//! boundary is shared geometry.
//!
//! Those two demands are not merely in tension, they are contradictory. Shared, `(1, 0)` on the
//! slave equals `(1, 0)` on the master. Written by the invariant at `t = 0`, it would instead equal
//! `C[0] + mu * (C[0] - A[0])`, and equating the two forces `mu = -1` — the case where the slave's
//! row is the master's row copied rather than reflected, which is the wrong side of the seam. So
//! the ends are left to the boundary, and the private interior is what the invariant owns.
//!
//! What that costs is **more than the ends of the seam**. The cross-boundary derivative sums over
//! the *whole* adjacent row, and the Bernstein weights at positions `0` and `degree` are positive
//! at every interior parameter — so the two offsets the invariant leaves free enter the derivative
//! along the entire edge, not only at its ends. Measured at degree 3: a tangent-plane gap of
//! **4.8e-2 rad** at the interior positions and **1.3e-1 rad** at the ends, against **1.8e-1 rad**
//! for a hard edge. A soft edge is measurably smoother than a crease and is not continuous.
//!
//! Raising the degree does not fix this. The end weights shrink but never reach zero.
//!
//! # What would fix it
//!
//! The **corner stencil**. At a mesh vertex the corner-adjacent control points satisfy
//! `q_i = -mu_i * q_(i-1)` around the cycle of incident faces, where `q` is measured from the vertex
//! itself, so closing the cycle requires
//!
//! ```text
//! (-1)^valence * product of the mu's around the vertex == 1
//! ```
//!
//! A Doo–Sabin face centroid has valence 3, so every-`mu == 1` asks for `(-1)^3 == 1` and fails. The
//! only consistent solution is `q == 0` — every corner-adjacent control point collapsed onto the
//! vertex, which is smooth and is a pinch. That is a real constraint on what `mu` a user may choose,
//! not something a formula absorbs.
//!
//! Until the stencil exists, this module deliberately leaves the row ends to boundary sharing
//! rather than applying the invariant over them. A crack is worse than a crease, so the failure mode
//! here is a bounded smoothness defect, measured by `examples/quad_continuity_probe.rs` measurement 3.

use crate::point::Point3;
use crate::topology::{EdgeKey, EdgeSlot, FaceId};

/// How an edge joins the faces on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Continuity {
    /// A crease. The boundary row is shared; the rows either side of it are independent.
    Hard,
    /// Tangent-plane continuous, with `mu` the cross-boundary slope the slave takes relative to
    /// the master. See the module documentation for what the values mean and why there is only one
    /// of them per edge.
    Soft(f64),
}

impl Continuity {
    pub fn is_soft(&self) -> bool {
        matches!(self, Self::Soft(_))
    }

    pub fn mu(&self) -> Option<f64> {
        match self {
            Self::Soft(mu) => Some(*mu),
            Self::Hard => None,
        }
    }
}

/// The point the slave's adjacent row must hold at one position.
///
/// `boundary` is the shared curve point, `master_interior` the master face's row one step in from
/// it, and `mu` the continuity parameter of the edge.
///
/// The two rows sit on opposite sides of the edge, so the slave's offset from the boundary is the
/// master's offset reversed and scaled. `mu == 1` is the C1 case, where the two sides are mirror
/// images about the boundary.
pub fn slave_interior(boundary: Point3, master_interior: Point3, mu: f64) -> Point3 {
    boundary + (boundary - master_interior) * mu
}

/// A soft edge, and which side of it decides.
#[derive(Clone, Debug, PartialEq)]
pub struct SoftEdge {
    /// The mesh edge this is, in canonical order.
    pub edge: EdgeKey,
    /// The face whose adjacent row is authoritative.
    pub master: (FaceId, EdgeSlot),
    /// The face that follows it.
    pub slave: (FaceId, EdgeSlot),
}
