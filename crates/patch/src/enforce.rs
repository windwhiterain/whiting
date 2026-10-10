//! Re-establishing every edge tag in one buffer of control points.
//!
//! This is the only code in the crate that writes a point it did not read, and the order it writes
//! them in is load-bearing rather than incidental:
//!
//! - **hard edges before soft edges**, because the soft invariant *reads* the shared boundary curve.
//!   Deriving a row from a boundary that a later pass copies would leave the row derived from a
//!   curve that has since moved, and the model would not be idempotent under a second pass.
//! - **the shared boundary before the private interior** of one soft edge, for the same reason one
//!   level in: the invariant's boundary term is the shared curve, not the slave's own stale copy.
//!
//! A third ordering rule used to be here and is now gone: **the private interiors were written as
//! each edge was visited**, so where two soft edges on one face claimed the same interior point the
//! last one written won, and the winner was the order the caller had happened to tag the edges in.
//! Measured on a 4 by 4 grid with every interior edge soft, tagging the same 24 edges in reverse
//! moved 9 control points by `6.142e-2` and took the worst fold from `2.325e-1` to `3.330e-1`. Every
//! claim is now collected off one snapshot and the arbitration is keyed on the
//! [`EdgeSlot`], so the surface is a function of the mesh rather than of how a caller walked it. See
//! the `apply_claims` function below for which slot wins and why, and `examples/contested_probe.rs` for the numbers.
//!
//! It is a free function over [`Rules`] and a [`Level`] rather than a method on
//! [`PatchMesh`](crate::patch_mesh::PatchMesh), for two reasons. A hierarchy runs the *same* pass on
//! every level — the base and each displacement above it — and a method taking `&mut self` can only
//! reach one of them at a time. And the total surface, which is a sum of levels nobody stores, is
//! itself a level enforcement can be run on; that is what lets the sum of separately enforced levels
//! be compared against the enforcement of the sum, which `layers_probe` measures rather than
//! assumes.
//!
//! Nothing here reads the mesh's *positions*, only its shape and its tags. That is what lets the one
//! pass enforce absolute levels and displacement levels alike.

use std::collections::{BTreeMap, BTreeSet};

use crate::continuity::{Continuity, SoftEdge, slave_interior};
use crate::level::Level;
use crate::point::Point3;
use crate::topology::{
    EdgeKey, EdgeSlot, FaceId, Topology, VertexId, boundary_index, edge_endpoints, interior_index,
    mirrored_position,
};

/// A mesh edge and the two faces meeting on it, as `(face, which of its edges)`.
type EdgeFace = (FaceId, EdgeSlot);

/// Everything enforcement is allowed to read.
///
/// Borrowed, never owned, and pointed at no control point: enforcement decides *what* each point must
/// become from the mesh's shape and its edge tags alone, so the same [`Rules`] serves every level of
/// a hierarchy.
#[derive(Clone, Copy, Debug)]
pub struct Rules<'a> {
    degree: usize,
    quads: &'a [[VertexId; 4]],
    topology: &'a Topology,
    continuity: &'a BTreeMap<EdgeKey, Continuity>,
    soft: &'a [SoftEdge],
}

impl<'a> Rules<'a> {
    /// Builds the rules from the parts themselves.
    ///
    /// Public because a caller that already holds disjoint borrows cannot go back through
    /// `&PatchMesh`: [`PatchMesh::enforce`](crate::patch_mesh::PatchMesh::enforce) has the base level
    /// and the offsets borrowed mutably at the same moment, which a borrow of the whole mesh would
    /// forbid.
    pub const fn new(
        degree: usize,
        quads: &'a [[VertexId; 4]],
        topology: &'a Topology,
        continuity: &'a BTreeMap<EdgeKey, Continuity>,
        soft: &'a [SoftEdge],
    ) -> Self {
        Self {
            degree,
            quads,
            topology,
            continuity,
            soft,
        }
    }

    /// The degree every patch carries, which is where [`boundary_index`] and
    /// [`interior_index`] read their degree from.
    pub const fn degree(&self) -> usize {
        self.degree
    }

    /// The soft edges, in the order they were made soft.
    pub fn soft_edges(&self) -> &[SoftEdge] {
        self.soft
    }

    /// The tag on `edge`, if the edge exists.
    pub fn continuity(&self, edge: &EdgeKey) -> Option<&Continuity> {
        self.continuity.get(edge)
    }
}

/// Re-establishes every edge tag in `level`: soft edges get their private rows derived, and every
/// shared boundary is made identical on both sides.
///
/// Idempotent, and correct after any edit. Hard edges need only the boundary copy, because a crease
/// is a shared curve with independent interiors — dropping that copy would make it a crack instead.
///
/// `level` must describe the same mesh at the same degree as the rules. Reads outside a face panic
/// rather than yielding zero, because an enforcement pass that silently read a zero would be a bug
/// that looked like a smooth surface.
pub fn enforce(rules: &Rules<'_>, level: &mut Level) {
    debug_assert_eq!(
        level.degrees(),
        (rules.degree, rules.degree),
        "a level is enforced in the degrees the mesh's own patches carry"
    );

    let hard: Vec<(EdgeKey, EdgeFace, EdgeFace)> = rules
        .topology
        .edge_faces
        .iter()
        .filter(|(key, faces)| {
            faces.len() == 2 && !rules.continuity.get(key).is_some_and(Continuity::is_soft)
        })
        .map(|(key, faces)| (*key, faces[0], faces[1]))
        .collect();
    for (key, owner, follower) in hard {
        share_boundary(rules, level, &key, owner, follower);
    }
    // A shared borrow of the edge list and a mutable borrow of the points is what lets this iterate
    // without cloning; the earlier version cloned because it held `&mut self`.
    //
    // Three steps, and the order is load-bearing twice over: every shared boundary before any private
    // interior, because the invariant's boundary term is the shared curve rather than the slave's own
    // stale copy; and every claim collected off one snapshot before any of them is written, so that
    // an edge cannot read a value a previous edge has already replaced.
    for edge in rules.soft {
        share_soft_boundary(rules, level, edge);
    }
    let mut claims: Claims = Claims::new();
    for edge in rules.soft {
        collect_soft(rules, level, edge, &mut claims);
    }
    apply_claims(level, claims);
    reconcile_vertices(rules, level);
}

/// Where each of a face's four corners sits in its own `(i, j)` indexing.
///
/// The winding puts `q0` at `(0, 0)`, `q1` at `(degree, 0)`, `q2` at `(degree, degree)` and `q3` at
/// `(0, degree)`, which is the order `RectNet::from_corners` is given them in.
fn corner_index(degree: usize, corner: usize) -> (usize, usize) {
    match corner {
        0 => (0, 0),
        1 => (degree, 0),
        2 => (degree, degree),
        3 => (0, degree),
        _ => panic!("a quad's corner index is 0 to 3, found {corner}"),
    }
}

/// Writes one value to every control point that sits on the mesh vertex.
///
/// A face's corner is the shared boundary row of *each* of the two edges meeting there, so a vertex
/// is held by several faces and several sharings, and a write that reached only one of them is how a
/// vertex becomes a fan of disagreeing patches. Propagating through all of them at once is what stops
/// a later pass from restoring a stale copy from a face that owns an incident edge.
pub fn write_vertex(rules: &Rules<'_>, level: &mut Level, vertex: VertexId, point: Point3) {
    let degree = rules.degree;
    for (face, corners) in rules.quads.iter().enumerate() {
        for (corner, at) in corners.iter().enumerate() {
            if *at != vertex {
                continue;
            }
            let (i, j) = corner_index(degree, corner);
            level.set_point(face, i, j, point);
        }
    }
}

/// Gives every mesh vertex **one** value across all the faces around it.
///
/// Two edges meeting at a vertex each share their boundary row, and both of those rows end at that
/// same corner, so the corner is written twice per enforcement and the later write wins. When the
/// copies start out equal — which is why this never showed on a base level, where every face's corner
/// is the same mesh vertex to begin with — the ordering does not matter. On a displacement level
/// nothing forces them to start out equal, and the leftover disagreement is a crack at every
/// contested vertex: measured at `3.666e-1` over 19 corners on the sample in `layers_probe`, all 19
/// of them at a row end and none of them anywhere else in the row.
///
/// One value per vertex, read from the lowest incident face, so the choice does not depend on the
/// order the edges happened to be visited in.
fn reconcile_vertices(rules: &Rules<'_>, level: &mut Level) {
    let mut vertices: BTreeSet<VertexId> = BTreeSet::new();
    for corners in rules.quads {
        vertices.extend(corners.iter().copied());
    }
    let degree = rules.degree;
    for vertex in vertices {
        let owner = rules.quads.iter().enumerate().find_map(|(face, corners)| {
            corners
                .iter()
                .position(|corner| *corner == vertex)
                .map(|corner| (face, corner))
        });
        let Some((face, corner)) = owner else {
            continue;
        };
        let (i, j) = corner_index(degree, corner);
        let held = at(level, face, i, j);
        write_vertex(rules, level, vertex, held);
    }
}

fn share_soft_boundary(rules: &Rules<'_>, level: &mut Level, edge: &SoftEdge) {
    let degree = rules.degree;
    if rules
        .continuity
        .get(&edge.edge)
        .and_then(Continuity::mu)
        .is_none()
    {
        return;
    }
    let master = edge.master.0 as usize;
    let slave = edge.slave.0 as usize;
    let master_ends = edge_endpoints(rules.quads[edge.master.0 as usize], edge.master.1);
    let slave_ends = edge_endpoints(rules.quads[edge.slave.0 as usize], edge.slave.1);

    // The boundary is shared and has `degree + 1` positions. Deriving it before any interior row is
    // what makes this correct: the invariant reads the shared curve, not the slave's own stale copy.
    for t in 0..=degree {
        let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
        let (mi, mj) = boundary_index(degree, edge.master.1, t);
        let (si, sj) = boundary_index(degree, edge.slave.1, slave_t);
        let boundary = at(level, master, mi, mj);
        level.set_point(slave, si, sj, boundary);
    }
}

/// What one soft edge wants a control point to be, and which edge asked.
///
/// A map of slot to demand rather than a bare value, because a point can be claimed by more than one
/// edge: `interior_index` puts a face's `MinU` and `MinV` rows on the same point and the two want
/// different values there. Which of them is honoured is the whole of the arbitration.
type Claims = BTreeMap<(usize, usize, usize), BTreeMap<EdgeSlot, Point3>>;

fn collect_soft(rules: &Rules<'_>, level: &Level, edge: &SoftEdge, claims: &mut Claims) {
    let degree = rules.degree;
    let Some(mu) = rules.continuity.get(&edge.edge).and_then(Continuity::mu) else {
        return;
    };
    let master = edge.master.0 as usize;
    let slave = edge.slave.0 as usize;
    let master_ends = edge_endpoints(rules.quads[edge.master.0 as usize], edge.master.1);
    let slave_ends = edge_endpoints(rules.quads[edge.slave.0 as usize], edge.slave.1);

    // The private interior only. See `crate::continuity` for why positions 0 and degree are the
    // perpendicular edge's to decide: forcing them here would demand mu == -1, which is the master
    // row copied rather than reflected.
    //
    // Collected rather than written. Two adjacent soft edges on one face meet at four points of the
    // interior block, and each wants a different value there, so writing in turn makes the winner a
    // function of the order the edges happened to be tagged in.
    for t in 1..degree {
        let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
        let (mi, mj) = boundary_index(degree, edge.master.1, t);
        let (ai, aj) = interior_index(degree, edge.master.1, t);
        let (si, sj) = interior_index(degree, edge.slave.1, slave_t);
        let boundary = at(level, master, mi, mj);
        let master_interior = at(level, master, ai, aj);
        claims
            .entry((slave, si, sj))
            .or_default()
            .insert(edge.slave.1, slave_interior(boundary, master_interior, mu));
    }
}

/// Writes one value to every contested interior point.
///
/// **The winner is the lowest `EdgeSlot`, and that is the whole point.** Letting the last edge in
/// `rules.soft` overwrite instead made the surface a function of the order the edges happened to be
/// tagged in: measured on a 4 by 4 grid with every interior edge soft, tagging the same 24 edges in
/// reverse moved 9 control points by `6.142e-2` and took the worst fold from `2.325e-1` to `3.330e-1`.
/// A rule keyed on the slot is a function of the geometry, so the surface does not depend on how a
/// caller walked the mesh to tag it.
///
/// **Which** slot wins is arbitrary within a face -- the two claimants disagree and only one can
/// hold. It is stated rather than derived. Averaging the two was measured and is worse: it halves the
/// mirror residual but lets the error spread, and the worst fold grows with the grid from `1.58e-1` at
/// 2 by 2 to `2.45e-1` at 4 by 4, where last-writer-wins stays flat at `1.5e-1`.
fn apply_claims(level: &mut Level, claims: Claims) {
    for ((face, i, j), demands) in claims {
        let winner = demands
            .iter()
            .min_by_key(|(slot, _)| **slot)
            .map(|(_, point)| *point)
            .expect("a claim carries at least one demand");
        level.set_point(face, i, j, winner);
    }
}

fn share_boundary(
    rules: &Rules<'_>,
    level: &mut Level,
    key: &EdgeKey,
    owner: EdgeFace,
    follower: EdgeFace,
) {
    let degree = rules.degree;
    let owner_ends = edge_endpoints(rules.quads[owner.0 as usize], owner.1);
    let follower_ends = edge_endpoints(rules.quads[follower.0 as usize], follower.1);

    for t in 0..=degree {
        let follower_t = mirrored_position(degree, owner_ends, follower_ends, t);
        let (oi, oj) = boundary_index(degree, owner.1, t);
        let (fi, fj) = boundary_index(degree, follower.1, follower_t);
        let point = at(level, owner.0 as usize, oi, oj);
        level.set_point(follower.0 as usize, fi, fj, point);
    }
    debug_assert_eq!(rules.topology.edge_faces.get(key).map(Vec::len), Some(2));
}

/// The point at `(i, j)` of `face`, panicking when either is out of range.
///
/// The panic is the point. An enforcement pass that silently read a zero instead would be a bug that
/// looked like a smooth surface.
fn at(level: &Level, face: usize, i: usize, j: usize) -> Point3 {
    level
        .face(face)
        .expect("a face id below the face count always resolves")[(i, j)]
}
