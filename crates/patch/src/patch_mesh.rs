//! A quad mesh whose faces are editable patches, with a hard or soft tag on every shared edge.

use std::collections::BTreeMap;

use crate::continuity::{Continuity, SoftEdge, slave_interior};
use crate::level::{Face, Level};
use crate::point::Point3;
use crate::rect::RectNet;
use crate::topology::{
    EdgeKey, EdgeSlot, FaceId, Topology, VertexId, boundary_index, canonical, edge_endpoints,
    interior_index, mirrored_position,
};

/// What happened to a control point write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    /// Written, and every tag re-established.
    Applied,
    /// `(i, j)` was outside the net.
    OutOfRange,
    /// More than one soft edge owns this point as its own private interior, so one drag cannot
    /// satisfy them all.
    ///
    /// A quad's four adjacent rows meet at `(1, 1)`, `(1, degree - 1)`, `(degree - 1, 1)` and
    /// `(degree - 1, degree - 1)`, so a face with two soft edges has points two of them both
    /// claim. **Every degree from 2 upward has points that belong to exactly one row** — on a
    /// `MinU` row at degree 3 that is `(1, 2)` — which is why this is a refusal and not a
    /// limitation of the representation.
    ///
    /// Refusing here is deliberate. Applying anyway would let the enforcement order silently pick
    /// a winner, so the same drag would give different results on faces whose soft edges were
    /// tagged in a different order. The masters are the thing to move when this happens.
    Overconstrained { by: usize },
}

/// A mesh whose faces carry patches.
///
/// Built from positions and quads; this crate deliberately owns no mesh type of its own, so a caller
/// passes whatever mesh it already has. The intended source is a triangle mesh run through
/// [`subdivide`](crate::subdivide::subdivide), and the two together are what give the representation
/// its continuity invariant.
///
/// **Level 0 reproduces the mesh.** Every patch starts as a degree-1 bilinear net through its own
/// four corners, so the assembled surface *is* the input mesh rather than an approximation of it.
/// There is no construction error to absorb later.
#[derive(Clone, Debug)]
pub struct PatchMesh {
    positions: Vec<Point3>,
    quads: Vec<[VertexId; 4]>,
    topology: Topology,
    /// Every face's control points, in one buffer. A `Vec<RectNet>` would be one allocation per face,
    /// which elevation pays in full and a hierarchy would pay once per level.
    level: Level,
    /// Every edge, so that a query can answer "is this edge soft" without searching.
    continuity: BTreeMap<EdgeKey, Continuity>,
    /// Soft edges only, in the order they were made soft. Enforced in this order.
    soft: Vec<SoftEdge>,
}

/// A mesh edge and the two faces meeting on it, as `(face, which of its edges)`.
type EdgeFace = (FaceId, EdgeSlot);

impl PatchMesh {
    /// Builds a patch per quad at level 0, with every edge hard.
    pub fn build(positions: Vec<Point3>, quads: Vec<[VertexId; 4]>) -> Self {
        let topology = Topology::build(&quads);
        let nets: Vec<RectNet> = quads
            .iter()
            .map(|corners| {
                // Winding order puts q0 at (0,0), q1 at (1,0), q2 at (1,1) and q3 at (0,1).
                let [q0, q1, q2, q3] = corners.map(|index| positions[index as usize]);
                RectNet::from_corners(q0, q1, q3, q2)
            })
            .collect();
        let continuity = topology
            .edge_faces
            .keys()
            .map(|key| (*key, Continuity::Hard))
            .collect();
        Self {
            positions,
            quads,
            topology,
            level: Level::from_nets(&nets),
            continuity,
            soft: Vec::new(),
        }
    }

    pub fn positions(&self) -> &[Point3] {
        &self.positions
    }

    pub fn quads(&self) -> &[[VertexId; 4]] {
        &self.quads
    }

    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Every patch, as it stands after the last edit or enforcement.
    ///
    /// One allocation per face, so it is for model-wide work — measuring, reporting, saving. Code
    /// that wants one patch should ask for it by name: [`PatchMesh::net`] lends and allocates
    /// nothing.
    pub fn nets(&self) -> Vec<RectNet> {
        self.level.to_nets()
    }

    /// One patch, borrowed. The hot path.
    pub fn net(&self, face: FaceId) -> Face<'_> {
        self.level
            .face(face as usize)
            .expect("a face id below the face count always resolves")
    }

    /// The whole level, for bulk work that has to walk every point.
    pub fn level(&self) -> &Level {
        &self.level
    }

    pub fn level_mut(&mut self) -> &mut Level {
        &mut self.level
    }

    /// The degree every patch carries. All of them move together, so the first one answers.
    pub fn degree(&self) -> usize {
        self.level.degrees().0
    }

    pub fn continuity(&self, edge: &EdgeKey) -> Option<&Continuity> {
        self.continuity.get(edge)
    }

    /// The point at `(i, j)` of `face`, indexed the way the topology indexes.
    ///
    /// Panics on an out-of-range face or point, which is what indexing a slice of nets did before the
    /// points moved into one buffer. An enforcement pass that silently read a zero instead would be a
    /// bug that looked like a smooth surface.
    fn at(&self, face: usize, i: usize, j: usize) -> Point3 {
        self.level
            .face(face)
            .expect("a face id below the face count resolves")[(i, j)]
    }

    /// The soft edges, in the order they were made soft.
    pub fn soft_edges(&self) -> &[SoftEdge] {
        &self.soft
    }

    /// Raises every patch one degree without moving the surface.
    ///
    /// Every soft edge's adjacent row is then re-derived, which is why this ends in an enforcement
    /// pass rather than doing the work twice: a slave's row is derived state, not stored intent.
    /// `mu` needs no adjustment, because it is one scalar for the whole edge and the edge did not
    /// gain positions.
    pub fn elevate(&mut self) {
        self.level = self.level.elevated();
        self.enforce();
    }

    /// Makes an edge soft at `mu = 1`, which is C1.
    ///
    /// Returns whether the edge exists and has two faces to join. An edge with one face is on the
    /// boundary of the mesh and has no second side, so there is nothing to make continuous.
    pub fn set_soft(&mut self, edge: EdgeKey) -> bool {
        let Some(faces) = self.topology.edge_faces.get(&edge) else {
            return false;
        };
        let Some(&master) = faces.first() else {
            return false;
        };
        let Some(&slave) = faces.get(1) else {
            return false;
        };
        self.soft.retain(|entry| entry.edge != edge);
        self.soft.push(SoftEdge {
            edge,
            master,
            slave,
        });
        self.continuity.insert(edge, Continuity::Soft(1.0));
        self.enforce();
        true
    }

    /// Makes an edge hard again, leaving the boundary shared and the interiors as they are.
    pub fn set_hard(&mut self, edge: EdgeKey) {
        self.soft.retain(|entry| entry.edge != edge);
        self.continuity.insert(edge, Continuity::Hard);
        self.enforce();
    }

    /// Changes a soft edge's cross-boundary slope and re-establishes it.
    ///
    /// `1` is C1, `0` puts the slave's row on the boundary, and anything else is G1 with a
    /// reparameterization across the seam.
    pub fn set_mu(&mut self, edge: EdgeKey, mu: f64) -> bool {
        if !self.continuity.get(&edge).is_some_and(Continuity::is_soft) {
            return false;
        }
        self.continuity.insert(edge, Continuity::Soft(mu));
        self.enforce();
        true
    }

    /// Edits the control points directly, then re-establishes every continuity tag.
    ///
    /// This bypasses the seam routing in [`PatchMesh::set_control_point`], so a write that lands on
    /// a shared boundary or on a slave's row will be undone by the enforcement that follows. It
    /// exists for bulk work — refining, rebuilding, loading — where the caller knows the whole
    /// model. Single-point editing should go through `set_control_point`.
    ///
    /// The closure gets a [`Level`], not a slice of nets, because that is how the points are stored:
    /// one buffer, one face each. Lend a face with [`Level::face`] rather than materialising the
    /// whole level.
    pub fn edit(&mut self, edit: impl FnOnce(&mut Level)) {
        edit(&mut self.level);
        self.enforce();
    }

    /// Moves one control point and re-establishes every edge tag.
    ///
    /// This is the editing primitive, and it exists because a shared boundary has two faces: a
    /// point written on one of them is a point on the edge, and writing only one side is how a
    /// crease becomes a crack. Two things are routed rather than written where they are:
    ///
    /// - a point on a boundary row is written to **both** faces, so a seam drags as one curve;
    /// - a point in the private interior of a soft edge is the **master's** to decide, so writing a
    ///   slave's row inverts the invariant and moves the master instead of being undone by
    ///   enforcement.
    ///
    /// Returns [`Edit::Overconstrained`] and changes nothing when two or more soft edges own the
    /// same point.
    pub fn set_control_point(&mut self, face: FaceId, i: usize, j: usize, point: Point3) -> Edit {
        let degree = self.degree();
        let corners = self.quads[face as usize];

        // Which soft edges own this point as their slave's row. Counted before anything is
        // written, so a refusal leaves the model untouched.
        let deciding: Vec<(SoftEdge, usize)> = self
            .soft
            .iter()
            .filter_map(|edge| {
                let position = private_interior_position_of(degree, edge.slave.1, i, j)?;
                (edge.slave.0 == face).then(|| (edge.clone(), position))
            })
            .collect();
        if deciding.len() > 1 {
            return Edit::Overconstrained { by: deciding.len() };
        }

        for slot in EdgeSlot::ALL {
            let Some(t) = boundary_position_of(degree, slot, i, j) else {
                continue;
            };
            let ends = edge_endpoints(corners, slot);
            if t == 0 || t == degree {
                // A corner is a mesh vertex and belongs to every face around it, not just to this
                // one and its immediate neighbours. Propagating it through all of them is what
                // stops the next enforcement pass from restoring a stale copy from a face that
                // owns an incident edge.
                self.write_vertex(if t == 0 { ends[0] } else { ends[1] }, point);
                continue;
            }
            let key = canonical(ends);
            let Some(faces) = self.topology.edge_faces.get(&key) else {
                continue;
            };
            for &(other, other_slot) in faces {
                if other == face {
                    continue;
                }
                let other_ends = edge_endpoints(self.quads[other as usize], other_slot);
                let other_t = mirrored_position(degree, ends, other_ends, t);
                let (oi, oj) = boundary_index(degree, other_slot, other_t);
                self.level.set_point(other as usize, oi, oj, point);
            }
        }

        if !self.level.set_point(face as usize, i, j, point) {
            return Edit::OutOfRange;
        }

        // Invert the invariant so that dragging a slave's row moves the master instead. With the
        // master free this way, the drag lands where the user pointed and the row stays consistent.
        for (edge, t) in deciding {
            let Some(mu) = self.continuity.get(&edge.edge).and_then(Continuity::mu) else {
                continue;
            };
            if mu.abs() < f64::EPSILON {
                // The invariant cannot be inverted at mu == 0. The write stands on this face and
                // enforcement puts the master's row back, which is the honest outcome.
                continue;
            }
            let slave_ends = edge_endpoints(corners, edge.slave.1);
            let master_ends = edge_endpoints(self.quads[edge.master.0 as usize], edge.master.1);
            let master_t = mirrored_position(degree, slave_ends, master_ends, t);
            let (bi, bj) = boundary_index(degree, edge.master.1, master_t);
            let boundary = self.at(edge.master.0 as usize, bi, bj);
            // slave = C + mu (C - master)  =>  master = C - (slave - C) / mu
            let master_interior = boundary - (point - boundary) / mu;
            let (mi, mj) = interior_index(degree, edge.master.1, master_t);
            self.level
                .set_point(edge.master.0 as usize, mi, mj, master_interior);
        }

        self.enforce();
        Edit::Applied
    }

    /// The shared boundary curve of an edge, read from the face that owns it.
    pub fn boundary_points(&self, edge: &EdgeKey) -> Option<Vec<Point3>> {
        let faces = self.topology.edge_faces.get(edge)?;
        let (face, slot) = *faces.first()?;
        let degree = self.degree();
        Some(
            (0..=degree)
                .map(|t| {
                    let (i, j) = boundary_index(degree, slot, t);
                    self.at(face as usize, i, j)
                })
                .collect(),
        )
    }

    /// Re-establishes every edge tag: soft edges get their private rows derived, and every shared
    /// boundary is made identical on both sides.
    ///
    /// Idempotent, and correct after any edit. Hard edges need only the boundary copy, because a
    /// crease is a shared curve with independent interiors — dropping that copy would make it a
    /// crack instead.
    pub fn enforce(&mut self) {
        // Hard edges first, soft edges last, for one reason: the soft invariant *reads* the shared
        // boundary curve. Copying a hard edge's boundary after deriving a row from it would leave
        // that row derived from a curve that has since moved, and the model would not be idempotent
        // under a second pass.
        //
        // The other half of the old reason is gone. A soft edge now writes only its private interior,
        // which is never a boundary point, so a soft pass can no longer overwrite a hard edge's copy
        // the way it did on triangles.
        let hard: Vec<(EdgeKey, EdgeFace, EdgeFace)> = self
            .topology
            .edge_faces
            .iter()
            .filter(|(key, faces)| {
                faces.len() == 2 && !self.continuity.get(key).is_some_and(Continuity::is_soft)
            })
            .map(|(key, faces)| (*key, faces[0], faces[1]))
            .collect();
        for (key, owner, follower) in hard {
            self.share_boundary(&key, owner, follower);
        }
        for edge in self.soft.clone() {
            self.enforce_soft(&edge);
        }
    }

    fn enforce_soft(&mut self, edge: &SoftEdge) {
        let degree = self.degree();
        let Some(mu) = self.continuity.get(&edge.edge).and_then(Continuity::mu) else {
            return;
        };
        let master = edge.master.0 as usize;
        let slave = edge.slave.0 as usize;
        let master_ends = edge_endpoints(self.quads[edge.master.0 as usize], edge.master.1);
        let slave_ends = edge_endpoints(self.quads[edge.slave.0 as usize], edge.slave.1);

        // The boundary is shared and has `degree + 1` positions. Deriving it first is what makes
        // this correct: the invariant reads the shared curve, not the slave's own stale copy.
        for t in 0..=degree {
            let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
            let (mi, mj) = boundary_index(degree, edge.master.1, t);
            let (si, sj) = boundary_index(degree, edge.slave.1, slave_t);
            let boundary = self.at(master, mi, mj);
            self.level.set_point(slave, si, sj, boundary);
        }

        // Then the private interior only. See `crate::continuity` for why positions 0 and degree
        // are the perpendicular edge's to decide: forcing them here would demand mu == -1, which is
        // the master row copied rather than reflected.
        for t in 1..degree {
            let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
            let (mi, mj) = boundary_index(degree, edge.master.1, t);
            let (ai, aj) = interior_index(degree, edge.master.1, t);
            let (si, sj) = interior_index(degree, edge.slave.1, slave_t);
            let boundary = self.at(master, mi, mj);
            let master_interior = self.at(master, ai, aj);
            self.level
                .set_point(slave, si, sj, slave_interior(boundary, master_interior, mu));
        }
    }

    fn share_boundary(&mut self, key: &EdgeKey, owner: EdgeFace, follower: EdgeFace) {
        let degree = self.degree();
        let owner_ends = edge_endpoints(self.quads[owner.0 as usize], owner.1);
        let follower_ends = edge_endpoints(self.quads[follower.0 as usize], follower.1);

        for t in 0..=degree {
            let follower_t = mirrored_position(degree, owner_ends, follower_ends, t);
            let (oi, oj) = boundary_index(degree, owner.1, t);
            let (fi, fj) = boundary_index(degree, follower.1, follower_t);
            let point = self.at(owner.0 as usize, oi, oj);
            self.level.set_point(follower.0 as usize, fi, fj, point);
        }
        debug_assert_eq!(self.topology.edge_faces.get(key).map(Vec::len), Some(2));
    }

    /// Writes a mesh vertex position to every boundary control point that sits on it.
    ///
    /// A mesh vertex belongs to every face around it, and on each face it sits at two boundary
    /// positions — one at each end of the two edges meeting there. Writing one face's copy and
    /// leaving the rest is what lets the next enforcement pass restore a stale value from a face
    /// that owns an incident edge, so a corner is propagated through all of them at once.
    fn write_vertex(&mut self, vertex: VertexId, point: Point3) {
        let degree = self.degree();
        let incident: Vec<FaceId> = self
            .quads
            .iter()
            .enumerate()
            .filter(|(_, corners)| corners.contains(&vertex))
            .map(|(face, _)| face as FaceId)
            .collect();
        for face in incident {
            let corners = self.quads[face as usize];
            for slot in EdgeSlot::ALL {
                let ends = edge_endpoints(corners, slot);
                for (t, end) in [(0usize, ends[0]), (degree, ends[1])] {
                    if end != vertex {
                        continue;
                    }
                    let (i, j) = boundary_index(degree, slot, t);
                    self.level.set_point(face as usize, i, j, point);
                }
            }
        }
    }
}

/// The boundary position `t` of this face's edge, if `(i, j)` lies on its boundary row.
fn boundary_position_of(degree: usize, slot: EdgeSlot, i: usize, j: usize) -> Option<usize> {
    (0..=degree).find(|t| boundary_index(degree, slot, *t) == (i, j))
}

/// The position of `(i, j)` in this face's edge row, if it is in that row's **private** interior.
///
/// Private means positions `1..degree`, which excludes the two ends shared with the perpendicular
/// edge's boundary.
fn private_interior_position_of(
    degree: usize,
    slot: EdgeSlot,
    i: usize,
    j: usize,
) -> Option<usize> {
    (1..degree).find(|t| interior_index(degree, slot, *t) == (i, j))
}
