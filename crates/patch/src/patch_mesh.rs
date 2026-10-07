//! A mesh whose faces are editable patches, with a hard or soft tag on every shared edge.

use std::collections::BTreeMap;

use crate::continuity::{Continuity, SoftEdge, slave_interior};
use crate::simplex::{Point3, SimplexNet};
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
    /// More than one soft edge decides this point, so one drag cannot satisfy them all.
    ///
    /// A triangle's three adjacent rows cross, and a control point `(i, j, k)` belongs to the `i`
    /// row exactly when `i == 1`, and to no other. So a point with all three indices at least `2`
    /// is unconstrained, a point with exactly one index equal to `1` is constrained by one soft
    /// edge, and a point with two or three equal to `1` is constrained by two or three. **The
    /// first degree with a point constrained by exactly one edge is 5**: `1 + 2 + 2`.
    ///
    /// Refusing here is deliberate. Applying anyway would let the enforcement order silently pick
    /// a winner, so the same drag would give different results on faces whose soft edges were
    /// tagged in a different order. The masters are the thing to move when this happens.
    Overconstrained { by: usize },
}

/// A mesh whose faces carry patches.
///
/// Built from positions and triangles; this crate deliberately owns no mesh type of its own, so a
/// caller passes whatever mesh it already has. `whiting-mountain`'s `FacetMesh` is the first
/// intended source, and it is `f32` — the conversion into [`Point3`] is exact, and the conversion
/// back is one rounding at render time.
///
/// **Level 0 reproduces the mesh.** Every patch starts as a degree-1 net through its own three
/// corners, so the assembled surface *is* the input mesh rather than an approximation of it. There
/// is no construction error to absorb later.
#[derive(Clone, Debug)]
pub struct PatchMesh {
    positions: Vec<Point3>,
    triangles: Vec<[VertexId; 3]>,
    topology: Topology,
    nets: Vec<SimplexNet>,
    /// Every edge, so that a query can answer "is this edge soft" without searching.
    continuity: BTreeMap<EdgeKey, Continuity>,
    /// Soft edges only, in the order they were made soft. Enforced in this order.
    soft: Vec<SoftEdge>,
}

/// A mesh edge and the two faces meeting on it, as `(face, which of its edges)`.
type EdgeFace = (FaceId, EdgeSlot);

impl PatchMesh {
    /// Builds a patch per triangle at level 0, with every edge hard.
    pub fn build(positions: Vec<Point3>, triangles: Vec<[VertexId; 3]>) -> Self {
        let topology = Topology::build(&triangles);
        let nets = triangles
            .iter()
            .map(|corners| {
                let [alpha, beta, gamma] = corners.map(|index| positions[index as usize]);
                SimplexNet::from_corners(alpha, beta, gamma)
            })
            .collect();
        let continuity = topology
            .edge_faces
            .keys()
            .map(|key| (*key, Continuity::Hard))
            .collect();
        Self {
            positions,
            triangles,
            topology,
            nets,
            continuity,
            soft: Vec::new(),
        }
    }

    pub fn positions(&self) -> &[Point3] {
        &self.positions
    }

    pub fn triangles(&self) -> &[[VertexId; 3]] {
        &self.triangles
    }

    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Every patch, as it stands after the last edit or enforcement.
    pub fn nets(&self) -> &[SimplexNet] {
        &self.nets
    }

    pub fn net(&self, face: FaceId) -> &SimplexNet {
        &self.nets[face as usize]
    }

    pub fn degree(&self) -> usize {
        self.nets.first().map_or(0, SimplexNet::degree)
    }

    pub fn continuity(&self, edge: &EdgeKey) -> Option<&Continuity> {
        self.continuity.get(edge)
    }

    /// The soft edges, in the order they were made soft.
    pub fn soft_edges(&self) -> &[SoftEdge] {
        &self.soft
    }

    /// Raises every patch one degree without moving the surface.
    ///
    /// Soft edges gain a boundary position each, and their `mu` grows with them. The soft edges'
    /// interior rows are then re-derived, which is why this ends in an enforcement pass rather
    /// than doing the work twice: the slave's interior row is derived state, not stored intent.
    pub fn elevate(&mut self) {
        for net in &mut self.nets {
            *net = net.elevate();
        }
        let degree = self.degree();
        for edge in &mut self.soft {
            // The new position has no slope of its own yet. C1 is the right default because it is
            // the join that looks smooth; the caller moves it afterwards if they want.
            edge.mu.resize(degree + 1, 1.0);
        }
        self.enforce();
    }

    /// Makes an edge soft at `mu = 1` throughout, which is C1.
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
        let mu = vec![1.0; self.degree() + 1];
        self.soft.retain(|entry| entry.edge != edge);
        self.soft.push(SoftEdge {
            edge,
            master,
            slave,
            mu: mu.clone(),
        });
        self.continuity.insert(edge, Continuity::Soft(mu));
        self.enforce();
        true
    }

    /// Makes an edge hard again, leaving the boundary shared and the interiors as they are.
    pub fn set_hard(&mut self, edge: EdgeKey) {
        self.soft.retain(|entry| entry.edge != edge);
        self.continuity.insert(edge, Continuity::Hard);
        self.enforce();
    }

    /// Edits the nets directly, then re-establishes every continuity tag.
    ///
    /// This bypasses the seam routing in [`PatchMesh::set_control_point`], so a write that lands on
    /// a shared boundary or on a slave's interior row will be undone by the enforcement that
    /// follows. It exists for bulk work — refining, rebuilding, loading — where the caller knows
    /// the whole model. Single-point editing should go through `set_control_point`.
    pub fn edit(&mut self, edit: impl FnOnce(&mut [SimplexNet])) {
        edit(&mut self.nets);
        self.enforce();
    }

    /// Moves one control point and re-establishes every edge tag.
    ///
    /// This is the editing primitive, and it exists because a shared boundary has two faces: a
    /// point written on one of them is a point on the edge, and writing only one side is how a
    /// crease becomes a crack. Two things are routed rather than written where they are:
    ///
    /// - a point on a boundary row is written to **both** faces, so a seam drags as one curve;
    /// - a point one step inside a soft edge is the **master's** to decide, so writing a slave's
    ///   row inverts the invariant and moves the master instead of being undone by enforcement.
    ///
    /// Returns [`Edit::Overconstrained`] and changes nothing when two or more soft edges decide the
    /// same point — see that variant for why degree 3 makes this common.
    pub fn set_control_point(&mut self, face: FaceId, i: usize, j: usize, point: Point3) -> Edit {
        let degree = self.degree();
        let corners = self.triangles[face as usize];
        let ends_here = [
            edge_endpoints(corners, EdgeSlot::OppositeAlpha),
            edge_endpoints(corners, EdgeSlot::OppositeBeta),
            edge_endpoints(corners, EdgeSlot::OppositeGamma),
        ];

        // Which soft edges decide this point. Counted before anything is written, so a refusal
        // leaves the model untouched.
        let deciding: Vec<SoftEdge> = self
            .soft
            .iter()
            .filter(|edge| edge.slave.0 == face && decides_point(degree, edge.slave.1, i, j))
            .cloned()
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
                let other_ends = edge_endpoints(self.triangles[other as usize], other_slot);
                let other_t = mirrored_position(degree, ends, other_ends, t);
                let (oi, oj) = boundary_index(degree, other_slot, other_t);
                self.nets[other as usize].set_point(oi, oj, point);
            }
        }

        if !self.nets[face as usize].set_point(i, j, point) {
            return Edit::OutOfRange;
        }

        for edge in deciding {
            let master_ends = edge_endpoints(self.triangles[edge.master.0 as usize], edge.master.1);
            for t in 0..degree {
                let (li, lj) = interior_index(degree, edge.slave.1, t);
                if (li, lj) != (i, j) {
                    continue;
                }
                let master_t =
                    mirrored_position(degree, ends_here[edge.slave.1 as usize], master_ends, t);
                let (bi, bj) = boundary_index(degree, edge.master.1, master_t);
                let boundary = self.nets[edge.master.0 as usize][(bi, bj)];
                let mu = edge.mu[master_t];
                if mu.abs() < f64::EPSILON {
                    // The invariant cannot be inverted at mu == 0. The write stands on this face
                    // and enforcement puts the master's row back, which is the honest outcome.
                    continue;
                }
                // slave = Q + mu (Q - master)  =>  master = Q - (slave - Q) / mu
                let master_interior = boundary - (point - boundary) / mu;
                let (mi, mj) = interior_index(degree, edge.master.1, master_t);
                self.nets[edge.master.0 as usize].set_point(mi, mj, master_interior);
            }
        }

        self.enforce();
        Edit::Applied
    }

    /// The continuity parameters of a soft edge, indexed by the **master's** local position.
    ///
    /// Writing here takes effect on the next [`PatchMesh::enforce`]; use
    /// [`PatchMesh::set_mu`] to write and apply in one step.
    pub fn mu_mut(&mut self, edge: &EdgeKey) -> Option<&mut Vec<f64>> {
        let soft = self.soft.iter_mut().find(|entry| entry.edge == *edge)?;
        Some(&mut soft.mu)
    }

    /// Changes one position's slope on a soft edge and re-establishes it.
    ///
    /// `1` is C1, `0` puts the slave's row on the boundary, and anything else is G1 with a
    /// reparameterization across the seam.
    pub fn set_mu(&mut self, edge: EdgeKey, position: usize, mu: f64) -> bool {
        let Some(slot) = self.soft.iter_mut().find(|entry| entry.edge == edge) else {
            return false;
        };
        if position >= slot.mu.len() {
            return false;
        }
        slot.mu[position] = mu;
        self.enforce();
        true
    }

    /// Re-establishes every edge tag: soft edges get their interior rows derived, and every
    /// shared boundary is made identical on both sides.
    ///
    /// Idempotent, and correct after any edit. Hard edges need only the boundary copy, because a
    /// crease is a shared curve with independent interiors — dropping that copy would make it a
    /// crack instead.
    pub fn enforce(&mut self) {
        // Hard edges first, soft edges last. At degree 3 and below a face's adjacent row for one
        // edge lies on the boundary row of its other edges, so deriving a soft edge first and
        // sharing hard edges afterwards would let a hard copy overwrite the derived value. Order
        // is the whole correctness argument here, and it is why this is not a set iteration.
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

    /// The shared boundary curve of an edge, read from the face that owns it.
    pub fn boundary_points(&self, edge: &EdgeKey) -> Option<Vec<Point3>> {
        let faces = self.topology.edge_faces.get(edge)?;
        let (face, slot) = *faces.first()?;
        let degree = self.degree();
        Some(
            (0..=degree)
                .map(|t| {
                    let (i, j) = boundary_index(degree, slot, t);
                    self.nets[face as usize][(i, j)]
                })
                .collect(),
        )
    }

    fn enforce_soft(&mut self, edge: &SoftEdge) {
        let degree = self.degree();
        let master_ends = edge_endpoints(self.triangles[edge.master.0 as usize], edge.master.1);
        let slave_ends = edge_endpoints(self.triangles[edge.slave.0 as usize], edge.slave.1);
        let master = edge.master.0 as usize;
        let slave = edge.slave.0 as usize;

        // The boundary is shared and has `degree + 1` positions.
        for t in 0..=degree {
            let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
            let (mi, mj) = boundary_index(degree, edge.master.1, t);
            let (si, sj) = boundary_index(degree, edge.slave.1, slave_t);
            let boundary = self.nets[master][(mi, mj)];
            self.nets[slave].set_point(si, sj, boundary);
        }

        // The interior row beside it is one shorter, so the last boundary position has no row
        // next to it. Deriving the boundary first is what makes this correct: the invariant reads
        // the shared curve, not the slave's own stale copy of it.
        //
        // When the two faces run along the edge in opposite directions the mirror of master
        // position 0 lands on the slave's position `degree`, which is past the end of its interior
        // row. That position is left untied rather than clamped: the edge covers `degree`
        // interior positions on one side and `degree - 1` on the other, and no assignment makes
        // them one to one.
        for t in 0..degree {
            let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
            if slave_t >= degree {
                continue;
            }
            let (mi, mj) = boundary_index(degree, edge.master.1, t);
            let (ai, aj) = interior_index(degree, edge.master.1, t);
            let (si, sj) = interior_index(degree, edge.slave.1, slave_t);
            let boundary = self.nets[master][(mi, mj)];
            let master_interior = self.nets[master][(ai, aj)];
            self.nets[slave].set_point(
                si,
                sj,
                slave_interior(boundary, master_interior, edge.mu[t]),
            );
        }
    }

    fn share_boundary(
        &mut self,
        key: &EdgeKey,
        owner: (FaceId, EdgeSlot),
        follower: (FaceId, EdgeSlot),
    ) {
        let degree = self.degree();
        let owner_corners = self.triangles[owner.0 as usize];
        let follower_corners = self.triangles[follower.0 as usize];
        let owner_ends = edge_endpoints(owner_corners, owner.1);
        let follower_ends = edge_endpoints(follower_corners, follower.1);

        for t in 0..=degree {
            let follower_t = mirrored_position(degree, owner_ends, follower_ends, t);
            let (oi, oj) = boundary_index(degree, owner.1, t);
            let (fi, fj) = boundary_index(degree, follower.1, follower_t);
            let point = self.nets[owner.0 as usize][(oi, oj)];
            self.nets[follower.0 as usize].set_point(fi, fj, point);
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
            .triangles
            .iter()
            .enumerate()
            .filter(|(_, corners)| corners.contains(&vertex))
            .map(|(face, _)| face as FaceId)
            .collect();
        for face in incident {
            let corners = self.triangles[face as usize];
            for slot in EdgeSlot::ALL {
                let ends = edge_endpoints(corners, slot);
                for (t, end) in [(0usize, ends[0]), (degree, ends[1])] {
                    if end != vertex {
                        continue;
                    }
                    let (i, j) = boundary_index(degree, slot, t);
                    self.nets[face as usize].set_point(i, j, point);
                }
            }
        }
    }
}

/// The boundary position `t` of this face's edge, if `(i, j)` lies on its boundary row.
fn boundary_position_of(degree: usize, slot: EdgeSlot, i: usize, j: usize) -> Option<usize> {
    (0..=degree).find(|t| boundary_index(degree, slot, *t) == (i, j))
}

/// Whether this face's edge, as a slave, decides the control point at `(i, j)`.
fn decides_point(degree: usize, slot: EdgeSlot, i: usize, j: usize) -> bool {
    (0..degree).any(|t| interior_index(degree, slot, t) == (i, j))
}
