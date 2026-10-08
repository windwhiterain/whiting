//! A massif's facets as an editable patch surface.
//!
//! # Why this crate, and not either of the two it joins
//!
//! `mountain` writes triangle facets and must not know that a freeform surface exists; `patch`
//! patches a mesh and must not know what a massif is. Neither may depend on the other, so the
//! conversion belongs above them — and this is the only crate that sees both. That is the whole
//! argument for its location, and it is a dependency-graph argument rather than an aesthetic one:
//! the alternative is a crate that exists to hold three functions.
//!
//! # What the conversion does, and what it costs
//!
//! Three steps, and the order is forced. The points widen from `f32` to `f64`, the triangles are
//! replaced by Doo–Sabin quads, and the quads become one degree-1 patch each — so the assembled
//! surface **is** the input mesh, corner for corner, with no construction error to absorb later.
//!
//! Two numbers about that are worth carrying, and both are measured by
//! `patchmesh`'s own probe rather than argued:
//!
//! - **The widening is exact.** Every `f32` is representable as an `f64`, so level 0 reproduces the
//!   mesh to `0.000e+00` rather than to some tolerance.
//! - **The precision floor does not move.** The surface is then only ever as accurate as the `f32`
//!   it arrived as: adjacent coordinates on a massif of this scale are about `1.5e-5` apart, and no
//!   `f64` arithmetic downstream changes that. That is eleven orders of magnitude coarser than the
//!   `1e-16` the rest of `patch` measures in, so "reproduces the mesh exactly" and "can be shaped
//!   finely" are two different questions and only the first one is answered exactly.
//!
//! # Why a non-manifold mesh is refused
//!
//! `enforce` makes a shared boundary shared when **exactly two** faces meet on it, and Doo–Sabin
//! gives every triangle on an edge one shared midpoint. So an edge with three triangles on it
//! produces a model whose quads agree about the point and disagree about the edge — and enforcement
//! leaves all three copies alone, because none of the three pairs is the pair it shares. The
//! surface then tears along that edge, silently, with nothing reporting it.
//!
//! A mesh like that is not a case to handle. It is a mesh whose shape the per-edge tag cannot
//! describe, so the honest answer is to name it and refuse. The massif the probe measures has no
//! such edge, which is why this is a guard rather than a repair.

use std::collections::BTreeMap;
use std::fmt;

use mountain::FacetMesh;
use patch::Point3;
use patch::subdivide::subdivide;
use patch::topology::EdgeKey;
use patch::{PatchMesh, VertexId};

/// Builds an editable patch surface over `facet`'s triangles.
///
/// The returned model's level 0 **is** the input mesh: every patch is a degree-1 bilinear net
/// through its own four corners, so evaluating the surface at a face's own corner returns that
/// corner to the last bit. That is why this returns a model rather than an approximation of one.
///
/// # Invariants
///
/// - **Every face is a quad.** A triangle's adjacent rows have no index correspondence with its
///   neighbour's — one side of an edge carries `degree` interior positions and the other
///   `degree - 1` — so the continuity invariant that is exact on a rectangle has nothing to say on
///   a triangle. Doo–Sabin is the price of that correspondence, and it is not optional here.
/// - **The surface is the mesh.** No fitting, no relaxation, no best approximation. If the patches
///   do not reproduce the mesh, that is a fault in this function rather than a tolerance to widen.
///
/// # Errors
///
/// [`FacetError::NonManifold`], which is the only thing about a triangle mesh this cannot turn
/// into a surface. See the module documentation for why it is refused rather than handled.
pub fn from_facet(facet: &FacetMesh) -> Result<PatchMesh, FacetError> {
    let folded = folded_edges(facet);
    if folded != 0 {
        return Err(FacetError::NonManifold { edges: folded });
    }

    let positions: Vec<Point3> = facet
        .points
        .iter()
        .map(|point| Point3::new(point[0] as f64, point[1] as f64, point[2] as f64))
        .collect();
    let triangles: Vec<[VertexId; 3]> = facet.faces.iter().map(|face| face.indices).collect();

    let derived = subdivide(&positions, &triangles);
    Ok(PatchMesh::build(derived.positions, derived.quads))
}

/// How many edges carry three or more triangles.
///
/// Counted over distinct edges rather than over triangles, because a mesh is folded once per
/// offending edge however many triangles are on it, and the answer a caller needs is how many
/// places to look.
fn folded_edges(facet: &FacetMesh) -> usize {
    let mut counts: BTreeMap<EdgeKey, usize> = BTreeMap::new();
    for face in &facet.faces {
        let [alpha, beta, gamma] = face.indices;
        for ends in [[alpha, beta], [beta, gamma], [gamma, alpha]] {
            let key = if ends[0] <= ends[1] {
                (ends[0], ends[1])
            } else {
                (ends[1], ends[0])
            };
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    counts.values().filter(|shared| **shared > 2).count()
}

/// Why a facet mesh cannot become an editable patch surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacetError {
    /// `edges` edges are shared by three or more triangles.
    ///
    /// A model built on such a mesh tears along every one of them, and nothing downstream reports
    /// it: enforcement shares a boundary when two faces meet on it, and with three there is no
    /// pair to share. Named rather than tolerated, because a surface that is quietly wrong is
    /// worse than one that was never built.
    NonManifold {
        /// How many edges carry three or more triangles.
        edges: usize,
    },
}

impl fmt::Display for FacetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NonManifold { edges } => write!(
                formatter,
                "{edges} edges carry three or more triangles, and a per-edge continuity tag \
                 describes a crease rather than a fold"
            ),
        }
    }
}

impl std::error::Error for FacetError {}
