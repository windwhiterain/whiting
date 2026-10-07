//! Doo–Sabin subdivision: a triangle mesh becomes an all-quad mesh.
//!
//! Every triangle is replaced by the three quads around its corners — corner, the midpoints of its
//! two incident edges, and its centroid. That is the whole rule, and it needs **no boundary case**:
//! a boundary edge yields one midpoint and one quad, which is the same rule applied to a triangle
//! that simply has no neighbour on the far side. Catmull-Clark would need a separate boundary rule
//! here, and a mesh that came off a mountain silhouette has a boundary.
//!
//! Why quads at all: a rectangle's control net has an adjacent row whose indices line up one to one
//! with the boundary's, which is what a continuity condition needs. A triangle's three adjacent
//! rows have no such correspondence — one side of an edge carries `degree` interior positions and
//! the other `degree - 1` — so the G1 invariant that is exact on a rectangle has nothing to say on
//! a triangle. This module is the price of that correspondence.
//!
//! The property this design leans on is that the subdivision **refines without moving the surface**.
//! The four corners of each quad are coplanar, and the three quads of a triangle cover it exactly,
//! so a model built on the quad mesh with flat patches reproduces the original mesh corner for
//! corner. `examples/subdivision_probe.rs` measures that rather than asserting it.

use std::collections::BTreeMap;

use crate::simplex::Point3;
use crate::topology::{EdgeKey, FaceId, VertexId, canonical};

/// Where a quad-mesh vertex came from.
///
/// Subdivision is a pure insertion: original vertices keep their indices and everything new is
/// appended. Recording the provenance is what lets a later edit address a point in the mesh the
/// caller handed over, rather than only in the mesh we derived from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// An original vertex, at the index it already had.
    Corner(VertexId),
    /// The midpoint of an original edge. Shared by the two quads that lie across it, interior, or
    /// by the one quad that does, on the boundary.
    EdgeMidpoint(EdgeKey),
    /// The centroid of an original face. Shared by that face's three quads.
    FaceCentroid(FaceId),
}

/// An all-quad mesh, and where each of its vertices came from.
#[derive(Clone, Debug, PartialEq)]
pub struct QuadMesh {
    pub positions: Vec<Point3>,
    /// Corner indices in winding order, so a quad is traversed the way its source triangle was.
    pub quads: Vec<[VertexId; 4]>,
    /// Parallel to `positions`.
    pub origin: Vec<Origin>,
}

impl QuadMesh {
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn face_count(&self) -> usize {
        self.quads.len()
    }
}

/// Replaces every triangle by the three quads around its corners.
///
/// New vertices are appended in a fixed order — edge midpoints first, in ascending canonical edge
/// key, then face centroids in face order — so the same input always yields the same output mesh.
pub fn subdivide(positions: &[Point3], triangles: &[[VertexId; 3]]) -> QuadMesh {
    let corner_count = positions.len();
    let mut derived = positions.to_vec();

    // Edge midpoints. One per distinct edge, whether interior or on the boundary: an edge on the
    // boundary is used by one triangle and an edge in the interior by two, but the midpoint is the
    // same point either way and is what makes the two quads across it agree.
    let mut midpoint_of: BTreeMap<EdgeKey, VertexId> = BTreeMap::new();
    for triangle in triangles {
        for ends in [
            [triangle[0], triangle[1]],
            [triangle[1], triangle[2]],
            [triangle[2], triangle[0]],
        ] {
            let key = canonical(ends);
            if midpoint_of.contains_key(&key) {
                continue;
            }
            let [first, second] = ends;
            derived.push((positions[first as usize] + positions[second as usize]) / 2.0);
            midpoint_of.insert(key, derived.len() as VertexId - 1);
        }
    }

    // Face centroids. A centroid is never shared with another face, so it needs no lookup table.
    let centroid_base = derived.len() as VertexId;
    for triangle in triangles {
        let sum = triangle.iter().fold(Point3::ZERO, |accumulated, index| {
            accumulated + positions[*index as usize]
        });
        derived.push(sum / 3.0);
    }

    let quads: Vec<[VertexId; 4]> = triangles
        .iter()
        .enumerate()
        .flat_map(|(face, triangle)| {
            let [alpha, beta, gamma] = *triangle;
            let alpha_beta = midpoint_of[&canonical([alpha, beta])];
            let beta_gamma = midpoint_of[&canonical([beta, gamma])];
            let gamma_alpha = midpoint_of[&canonical([gamma, alpha])];
            let centre = centroid_base + face as VertexId;

            // Each quad walks the source triangle's own winding, so the quads inherit whatever
            // orientation that triangle had and the derived mesh stays consistently wound without
            // a separate normal pass.
            [
                [alpha, alpha_beta, centre, gamma_alpha],
                [beta, beta_gamma, centre, alpha_beta],
                [gamma, gamma_alpha, centre, beta_gamma],
            ]
        })
        .collect();

    let mut origin = vec![Origin::Corner(0); derived.len()];
    for (index, slot) in origin.iter_mut().enumerate().take(corner_count) {
        *slot = Origin::Corner(index as VertexId);
    }
    for (key, index) in &midpoint_of {
        origin[*index as usize] = Origin::EdgeMidpoint(*key);
    }
    for face in 0..triangles.len() {
        origin[(centroid_base as usize) + face] = Origin::FaceCentroid(face as FaceId);
    }

    QuadMesh {
        positions: derived,
        quads,
        origin,
    }
}
