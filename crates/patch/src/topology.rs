//! Which mesh edges a triangle has, which faces meet at each one, and how one face's control net
//! addresses its own three edges.
//!
//! Nothing here knows about continuity. It is the indexing that continuity needs, kept separate so
//! a mesh can be walked without a surface in sight.

use std::collections::BTreeMap;

pub type VertexId = u32;
pub type FaceId = u32;

/// A mesh edge as an unordered pair in canonical order, so the two faces sharing it name it the
/// same way without either having to know the other's winding.
pub type EdgeKey = (VertexId, VertexId);

/// Which barycentric index vanishes along one edge of a triangle.
///
/// A face's corners are `(alpha, beta, gamma)` in that order, so the `OppositeAlpha` edge is the
/// one that does not touch `corners[0]`, and `corners[0]` is the vertex on the far side of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeSlot {
    OppositeAlpha,
    OppositeBeta,
    OppositeGamma,
}

impl EdgeSlot {
    pub const ALL: [Self; 3] = [Self::OppositeAlpha, Self::OppositeBeta, Self::OppositeGamma];
}

/// The mesh vertices at local boundary positions `0` and `degree` of this face's edge, in that
/// order.
///
/// Position 0 comes first in this pair and the edge's other end last, which is what lets two faces
/// that disagree about winding be reconciled by comparing these.
pub fn edge_endpoints(corners: [VertexId; 3], slot: EdgeSlot) -> [VertexId; 2] {
    match slot {
        EdgeSlot::OppositeAlpha => [corners[2], corners[1]],
        EdgeSlot::OppositeBeta => [corners[2], corners[0]],
        EdgeSlot::OppositeGamma => [corners[0], corners[1]],
    }
}

/// Where local boundary position `t` sits in the face's net, as `(i, j)`. The third barycentric
/// index is `degree - i - j` and is zero here, because this is the boundary row.
///
/// `t` runs `0..=degree`, giving `degree + 1` positions.
pub fn boundary_index(degree: usize, slot: EdgeSlot, t: usize) -> (usize, usize) {
    match slot {
        EdgeSlot::OppositeAlpha => (0, t),
        EdgeSlot::OppositeBeta => (t, 0),
        EdgeSlot::OppositeGamma => (degree - t, t),
    }
}

/// Where the control point one step in from local boundary position `t` sits.
///
/// This is the row the soft-edge invariant ties: the only control points that determine the
/// tangent plane at the boundary.
///
/// **`t` runs `0..degree`, not `0..=degree`.** The row beside a boundary has one entry fewer than
/// the boundary, so the last boundary position has no interior point beside it. Passing `t ==
/// degree` returns an index outside the net.
pub fn interior_index(degree: usize, slot: EdgeSlot, t: usize) -> (usize, usize) {
    match slot {
        EdgeSlot::OppositeAlpha => (1, t),
        EdgeSlot::OppositeBeta => (t, 1),
        EdgeSlot::OppositeGamma => (degree - 1 - t, t),
    }
}

/// The position on `there`'s copy of the same edge that names the same place as `t` on `here`'s.
///
/// Two faces sharing an edge may run along it in the same direction or in opposite ones, and the
/// only reliable way to tell is to compare which mesh vertex each end sits on.
pub fn mirrored_position(
    degree: usize,
    here: [VertexId; 2],
    there: [VertexId; 2],
    t: usize,
) -> usize {
    if here == there { t } else { degree - t }
}

/// The canonical key for an edge given its two ends, so both faces name it the same way.
pub fn canonical(ends: [VertexId; 2]) -> EdgeKey {
    if ends[0] <= ends[1] {
        (ends[0], ends[1])
    } else {
        (ends[1], ends[0])
    }
}

/// The mesh's edges and the faces meeting at each one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Topology {
    /// Every edge, and the faces using it. One entry on a boundary edge, two on an interior one,
    /// more where the mesh is not manifold. A non-manifold edge is **recorded, not rejected**:
    /// deciding what a crease means across three faces is a product question, and refusing to build
    /// would hide the mesh instead of surfacing it.
    ///
    /// Faces appear in ascending id, so the first entry is a deterministic owner for the edges
    /// where that has to be decided.
    pub edge_faces: BTreeMap<EdgeKey, Vec<(FaceId, EdgeSlot)>>,
}

impl Topology {
    /// Enumerates the edges of every triangle.
    pub fn build(triangles: &[[VertexId; 3]]) -> Self {
        let mut edge_faces: BTreeMap<EdgeKey, Vec<(FaceId, EdgeSlot)>> = BTreeMap::new();
        for (face_id, corners) in triangles.iter().enumerate() {
            for slot in EdgeSlot::ALL {
                let key = canonical(edge_endpoints(*corners, slot));
                edge_faces
                    .entry(key)
                    .or_default()
                    .push((face_id as FaceId, slot));
            }
        }
        Self { edge_faces }
    }

    /// The three edges of one face, with their canonical keys and slots.
    pub fn of_face(corners: [VertexId; 3]) -> [(EdgeKey, EdgeSlot); 3] {
        let mut result = [(canonical([corners[0], corners[1]]), EdgeSlot::OppositeAlpha); 3];
        for (slot, entry) in EdgeSlot::ALL.into_iter().zip(result.iter_mut()) {
            *entry = (canonical(edge_endpoints(corners, slot)), slot);
        }
        result
    }

    /// Edges no second face shares. These cannot be creases between two surfaces, because there
    /// is only one surface there.
    pub fn boundary_edges(&self) -> impl Iterator<Item = (&EdgeKey, &Vec<(FaceId, EdgeSlot)>)> {
        self.edge_faces.iter().filter(|(_, faces)| faces.len() == 1)
    }
}
