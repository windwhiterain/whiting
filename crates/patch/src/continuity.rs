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
//! decide on their own: `(1, 0)` on a `MinU` row is also a point of the same face's `MinV`
//! boundary, and the boundary is shared geometry.
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
//! ## The ends themselves are not the obstruction, and an earlier claim is withdrawn
//!
//! This module previously argued that enforcing the invariant at `t = 0` and `t = degree` is
//! contradictory — that the two positions are shared geometry, that writing one as
//! `C + mu * (C - A)` while sharing demands it equal `A` forces `mu == -1`, and that `mu == -1` puts
//! the slave's row on the master's side of the seam. **That argument does not survive measurement.**
//!
//! Run the same invariant over `0..=degree`, in [`PatchMesh::enforce`]'s own order, and a lone soft
//! edge comes out exactly G1: **0.000e0 rad** at the interior positions and **2.107e-8 rad** at the
//! ends, the second being the `acos` floor of the probe, with the boundary sharing reclaiming
//! `5.0e-16` of what the soft write moved. The gap above is therefore *not* a limit of the
//! representation. It is the price of this decision, and it is being paid for nothing.
//!
//! What does fail is narrower, and it is countable. A row end is one step in from its own edge,
//! which puts it on the **perpendicular** edge's boundary row, one step along. So when two soft
//! seams meet at a mesh vertex, their two row ends land on the *same place on the same shared edge*:
//! two equations, one value, and the boundary sharing can only give it to one of them. On the probe's
//! sample that is **0** such places for a lone seam, **1** for the minimal two-seam case and **13**
//! on the full sample — and across all three, a seam is exactly continuous **if and only if** no
//! shared place is written by two seams. That equivalence is asserted by the probe, not merely
//! observed.
//!
//! # What is shipped, and what is not
//!
//! Accepted on 2026-10-07, after every route to closing the gap was measured and closed.
//!
//! **Shipped.** A soft edge is **measurably smoother than a crease, never a crack, and
//! tangent-plane continuous along the edge** — the two faces' along-edge derivatives agree to
//! `2.107e-8` rad, which is the `acos` floor of the probe. At degree 3 the tangent-plane gap is
//! `4.771e-2` rad along the interior and `1.307e-1` rad at the two ends, against `1.780e-1` rad for
//! a hard edge.
//!
//! **Not shipped.** A soft edge is **not** continuous across the seam, and the number above is the
//! accepted cost. `quad_continuity_probe` measurement 3 asserts the promise that is actually made and
//! guards the gap against a regression ceiling, so a future change cannot make it worse unnoticed.
//!
//! That gap is **not a limit of the representation.** Measurement 12 runs this same invariant over the
//! whole row and measures exact G1 — `0.000e0` rad — on any seam that no other seam collides with. The
//! cost being paid is the price of leaving the row ends alone where two soft seams meet at a mesh
//! vertex and write the same shared point. Measurements 13, 14 and 15 measured and closed every route
//! found for closing it: deriving one master's corner repairs *fewer* seams, a corner scale is inert
//! across four orders of magnitude, and lazy subdivision cannot remove the collision at all.
//!
//! # What would reopen it
//!
//! Not a formula. Measurements 12 to 15 are kept as **guards**, not as history: each one passes while
//! the route it rules out stays ruled out, and any of them going red means the decision is stale and
//! the question is worth measuring again. The routes they rule out are, in order: the whole-row
//! invariant over a crowded sample, deriving the contested corner, a corner scale, and lazy
//! subdivision.
//!
//! # Elevation is not shape-preserving here, and this is why
//!
//! Degree elevation is exactly shape-preserving on its own — `rect_elevation_probe` measures it at
//! `7.026e-16` — and it is exactly shape-preserving on a mesh of this crate whose edges are all hard.
//! `layers_probe` measurement 5 measures `6.661e-16` there, and `2.176e-15` with a displacement level
//! above it, so a hierarchy does not make it lossy either.
//!
//! With soft edges it is **not**. Elevating a mesh from degree 3 to 4 moves its surface by
//! `8.170e-3`, and by `5.785e-2` when a displacement level sits on top. Both are the same defect as
//! the gap above, one refinement further in: the invariant leaves the two ends of a slave's row
//! un-derived, and elevation's blend pulls those un-derived ends into the *derived* interior
//! positions, so the row that enforcement re-derives sits slightly off where the surface was. The
//! movement is a seam effect, not a representation limit — nothing about degree elevation is
//! approximate here.
//!
//! The practical consequence is that elevation is a **refinement with a price** on a model carrying
//! soft edges, and a caller who needs the surface to hold still across it should refine before
//! tagging the seams soft.

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
