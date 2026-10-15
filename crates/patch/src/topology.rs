//! Which mesh edges a quad has, which faces meet at each one, and how one face's control net
//! addresses its own four edges.
//!
//! Nothing here knows about continuity. It is the indexing that continuity needs, kept separate so
//! a mesh can be walked without a surface in sight.
//!
//! A quad's corners are in winding order, and the net addresses them as
//!
//! ```text
//!   q3 --- q2        q3 = (0, degree)      q2 = (degree, degree)
//!   |  v=1 |         q0 = (0, 0)           q1 = (degree, 0)
//!   q0 --- q1
//!   u=0    u=degree
//! ```

use std::collections::BTreeMap;

pub type VertexId = u32;
pub type FaceId = u32;

/// A mesh edge as an unordered pair in canonical order, so the two faces sharing it name it the
/// same way without either having to know the other's winding.
pub type EdgeKey = (VertexId, VertexId);

/// Which parameter is fixed along one edge of a quad.
///
/// Naming the edge by the parameter it fixes rather than by a corner keeps the boundary and
/// adjacent-row mappings total: every slot has the same shape, so no case is special.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeSlot {
    MinU,
    MaxU,
    MinV,
    MaxV,
}

impl EdgeSlot {
    pub const ALL: [Self; 4] = [Self::MinU, Self::MaxU, Self::MinV, Self::MaxV];

    /// Whether **`u` varies along** this edge, which makes `du` the edge's tangent.
    ///
    /// `MinU` is the edge at `u = 0` and runs from `corners[0]` to `corners[3]` — that is, along
    /// **`v`**. So the `MinU`/`MaxU` pair are the `v`-direction edges and `MinV`/`MaxV` are the
    /// `u`-direction ones.
    ///
    /// **This is the trap this crate has already fallen into once.** `EdgeSlot` names an edge by the
    /// parameter it *fixes*, which is the opposite of the direction it *runs*, and reading it the
    /// other way round asks each face for a tangent off its perpendicular edge. The result is not a
    /// crash: it is a plausible-looking surface whose tangent field is rotated a quarter turn, and
    /// a seam that compares `S_u` against `S_v` reports a fold that is not there. Measured while
    /// getting this wrong — every interior edge of a 13 by 9 sheet read a split of `8.1e-1`, one
    /// cell away, which is the signature of comparing the wrong two ends.
    pub fn varies_along_u(self) -> bool {
        matches!(self, Self::MinV | Self::MaxV)
    }

    /// Whether this edge **separates its two faces across `u`**, so the faces meet at `s = 1`
    /// against `s = 0` and the shared parameter is `t`.
    ///
    /// The exact complement of [`EdgeSlot::varies_along_u`], and a separate question: the first is
    /// about which derivative the edge carries, the second about which direction the faces are
    /// stacked in. Both are asked by a caller, neither can be inferred from the other without the
    /// winding, and conflating them is what the module documentation above warns about.
    pub fn separates_u(self) -> bool {
        matches!(self, Self::MinU | Self::MaxU)
    }
}

/// The mesh vertices at local positions `0` and `degree` of this face's edge, in that order.
///
/// Position 0 comes first in this pair and the edge's other end last, which is what lets two faces
/// that disagree about winding be reconciled by comparing these.
pub fn edge_endpoints(corners: [VertexId; 4], slot: EdgeSlot) -> [VertexId; 2] {
    match slot {
        EdgeSlot::MinU => [corners[0], corners[3]],
        EdgeSlot::MaxU => [corners[1], corners[2]],
        EdgeSlot::MinV => [corners[0], corners[1]],
        EdgeSlot::MaxV => [corners[3], corners[2]],
    }
}

/// Where local boundary position `t` sits in the face's net, as `(i, j)`.
///
/// `t` runs `0..=degree`, giving `degree + 1` positions. Positions `0` and `degree` are the two
/// mesh vertices at this edge's ends.
pub fn boundary_index(degree: usize, slot: EdgeSlot, t: usize) -> (usize, usize) {
    match slot {
        EdgeSlot::MinU => (0, t),
        EdgeSlot::MaxU => (degree, t),
        EdgeSlot::MinV => (t, 0),
        EdgeSlot::MaxV => (t, degree),
    }
}

/// Where the control point one step in from local boundary position `t` sits.
///
/// This is the row a soft edge's invariant ties: the control points that determine the tangent
/// plane along the boundary.
///
/// **`t` runs `0..=degree` here, and that is the whole reason the rectangle was chosen.** The row
/// has exactly as many entries as the boundary beside it, so position `t` on one face and position
/// `t` on the other name the same place along the edge. A degree-`n` triangle cannot do this: one
/// side of its edges carries `n` interior positions and the other `n - 1`.
///
/// Positions `0` and `degree` are *not* private to this edge. `(1, 0)` on a `MinU` row is also on
/// the face's `MinV` boundary, one step along it from the corner. A continuity invariant must leave
/// those two alone — they belong to the perpendicular edge's boundary — so the private interior of
/// an edge is positions `1..=degree - 1`, and [`PatchMesh::set_control_point`](crate::PatchMesh::set_control_point)
/// is where that becomes a refusal rather than a silent overwrite.
///
/// Requires `degree >= 1`; a degree-0 patch has no interior row and `degree - 1` would wrap.
pub fn interior_index(degree: usize, slot: EdgeSlot, t: usize) -> (usize, usize) {
    debug_assert!(degree >= 1, "a patch has no adjacent row at degree 0");
    match slot {
        EdgeSlot::MinU => (1, t),
        EdgeSlot::MaxU => (degree - 1, t),
        EdgeSlot::MinV => (t, 1),
        EdgeSlot::MaxV => (t, degree - 1),
    }
}

/// The position on `there`'s copy of the same edge that names the same place as `t` on `here`'s.
///
/// Two faces sharing an edge may run along it in the same direction or in opposite ones, and the
/// only reliable way to tell is to compare which mesh vertex each end sits on. Because both rows
/// span `0..=degree`, this is a bijection on every position — there is no position it can send out
/// of range.
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
    /// Faces appear in ascending id, so the first entry is a deterministic master for the edges
    /// where that has to be decided.
    pub edge_faces: BTreeMap<EdgeKey, Vec<(FaceId, EdgeSlot)>>,
}

impl Topology {
    /// Enumerates the edges of every quad.
    pub fn build(quads: &[[VertexId; 4]]) -> Self {
        let mut edge_faces: BTreeMap<EdgeKey, Vec<(FaceId, EdgeSlot)>> = BTreeMap::new();
        for (face_id, corners) in quads.iter().enumerate() {
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

    /// The four edges of one face, with their canonical keys and slots.
    pub fn of_face(corners: [VertexId; 4]) -> [(EdgeKey, EdgeSlot); 4] {
        EdgeSlot::ALL.map(|slot| (canonical(edge_endpoints(corners, slot)), slot))
    }

    /// Edges no second face shares. These cannot be creases between two surfaces, because there
    /// is only one surface there.
    pub fn boundary_edges(&self) -> impl Iterator<Item = (&EdgeKey, &Vec<(FaceId, EdgeSlot)>)> {
        self.edge_faces.iter().filter(|(_, faces)| faces.len() == 1)
    }
}
