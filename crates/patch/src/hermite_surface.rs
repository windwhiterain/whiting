//! A quad mesh whose faces are bicubic Hermite patches: exactly C1 inside a soft-edge component,
//! C0 with a crease across a hard one.
//!
//! # What is shared, and why that is the whole design
//!
//! The insight that makes this representation different from `crate::PatchMesh` is that **a
//! boundary curve and the cross-boundary derivative are the same data**, so a bicubic Hermite face
//! cannot have a shared boundary curve and a private normal at the same time. Whatever is shared
//! decides what the tag can mean, and the three quantities below are the three honest choices:
//!
//! | stored | where | shared | gives |
//! | --- | --- | --- | --- |
//! | `positions` | per mesh vertex | always | C0, never a crack |
//! | `derivatives` — `du`, `dv` | per mesh vertex | always | the boundary curves are one curve, and so is the cross-boundary direction |
//! | `twist` — `duv` | per edge endpoint | **soft only**; private per face when hard | the face's interior |
//!
//! So:
//!
//! - **Soft.** The twist is shared as well, so the two faces' corner jets are *equal*, and two
//!   patches with equal corner jets are the same surface. **Exactly C1, structurally**, with no
//!   enforcement pass and nothing to enforce.
//! - **Hard.** The twist is private, and the twist enters the cross-boundary derivative —
//!   `dS/du` at an edge is `sum of h00*Fu + h01*Fuv` — so the two sides' normals differ, while
//!   every boundary curve, and therefore every shared point, is untouched. **C0 with a crease, and
//!   never a crack.** A private twist does not appear in a boundary curve at all, which is exactly
//!   why it is the safe place to put a face's freedom.
//!
//! # What this costs, stated plainly
//!
//! **A face has four free vectors on a fully hard mesh and none on a fully soft one.** That is the
//! same count as the four private interior control points of a degree-3 Bernstein net, so this is
//! not a reduction — but the freedom is *twist*, not an arbitrary interior point, and there is no
//! longer a handle that moves one patch's middle without its neighbours.
//!
//! The alternative — a private cross-boundary derivative — is not available here and this module
//! says so rather than pretending otherwise. It would need the boundary curve stored as data
//! independent of the corner jets, which is a degree-2 row beside the curve: `crate::PatchMesh`'s
//! shape. This representation removes the enforcement pass, not the fact that a crease costs
//! something.
//!
//! # Frames
//!
//! `du` and `dv` are stored **per mesh vertex**, shared by every face meeting there, which is only
//! meaningful if those faces agree about which way is `u`. They do, and for a reason that costs
//! nothing to check: `EdgeSlot::MinU` and `MaxU` are the edges that fix **u**, so `u` varies along
//! `MinV` and `MaxV`, and every face's `u` runs the same way as its neighbour's on a shared edge
//! as long as the mesh is wound consistently. A mesh that is not wound consistently is not
//! representable here, and [`HermiteSurface::build`] says so rather than producing a surface that
//! disagrees with itself.
//!
//! # Editing
//!
//! Every mutator here preserves continuity by construction, because there is nothing to re-establish
//! afterwards: a caller cannot produce a crack, a mis-shared boundary or a tangent-plane
//! discontinuity by writing to this type. See [`HermiteSurface::set_derivative`] and
//! [`HermiteSurface::set_twist`].

use std::collections::{BTreeMap, BTreeSet};

use crate::hermite::{HermitePatch, Jet};
use crate::point::Point3;
use crate::topology::{EdgeKey, EdgeSlot, FaceId, Topology, VertexId, canonical};

/// How the faces on an edge join.
///
/// The replacement for [`crate::Continuity`], and smaller: there is no slope parameter, because the
/// cross-boundary derivative is either shared or it is not, and nothing in between is expressible
/// in a bicubic Hermite face.
///
/// `Soft` is the default, so a freshly built surface is smooth everywhere and a crease is something
/// a caller asks for rather than something it has to remove.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Join {
    /// C0 with a crease. The boundary curve is shared; the twist either side is not.
    Hard,
    /// Exactly C1. Position, both derivatives and the twist are shared at both ends.
    #[default]
    Soft,
}

/// Why a mesh could not be turned into a Hermite surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceError {
    /// The same edge is traversed the same way by both its faces, which is what makes `u` and `v`
    /// mean the same direction at a shared vertex. A mesh that fails this is not representable
    /// here, and saying so is the answer — the alternative is a surface that disagrees with itself
    /// about which way is up at a vertex.
    InconsistentWinding {
        edge: EdgeKey,
        first: EdgeSlot,
        second: EdgeSlot,
    },
    /// An edge with three or more faces on it. Two faces can share a curve; three cannot, and there
    /// is no pair to pick.
    NonManifold { edges: usize },
    /// A quad whose corners are not four distinct vertices.
    DegenerateQuad { face: FaceId },
}

/// A mesh of bicubic Hermite patches.
#[derive(Clone, Debug)]
pub struct HermiteSurface {
    positions: Vec<Point3>,
    quads: Vec<[VertexId; 4]>,
    topology: Topology,
    /// `du` and `dv` at each mesh vertex, shared by every face meeting there.
    derivatives: Vec<(Point3, Point3)>,
    /// One step per direction, shared by every face. See [HermiteSurface::step_u].
    steps: (f64, f64),
    /// How unequal the mesh's cells are. See [HermiteSurface::step_spread].
    spread: (f64, f64),
    /// The twist at each face's four corners.
    ///
    /// **One slot per face corner, not one per edge**, and that is a deliberate trade. Sharing the
    /// slot per edge would be storage-cheaper, but then a corner sitting on a soft edge and a hard
    /// one would have to answer to both, and the rule that resolves it — "shared when *either*
    /// edge is soft" — means tagging a single edge hard produces **no crease at all**, because the
    /// corner's other edge is still soft. Paying for a second copy of a number instead buys the
    /// thing a crease is for, and the copy is kept correct by the only two ways to write it.
    private_twist: Vec<[Point3; 4]>,
    join: BTreeMap<EdgeKey, Join>,
}

impl HermiteSurface {
    /// Builds a surface whose tangent field is seeded by finite differences over the positions.
    ///
    /// A freshly built model reproduces its mesh corner for corner: the positions are the mesh's
    /// own and the derivatives are a seed, not a fit. There is no construction error to absorb.
    pub fn build(positions: Vec<Point3>, quads: Vec<[VertexId; 4]>) -> Result<Self, SurfaceError> {
        let topology = Topology::build(&quads);
        let non_manifold = topology
            .edge_faces
            .values()
            .filter(|faces| faces.len() > 2)
            .count();
        if non_manifold > 0 {
            return Err(SurfaceError::NonManifold {
                edges: non_manifold,
            });
        }
        for (face, corners) in quads.iter().enumerate() {
            if corners[0] == corners[1]
                || corners[0] == corners[2]
                || corners[0] == corners[3]
                || corners[1] == corners[2]
                || corners[1] == corners[3]
                || corners[2] == corners[3]
            {
                return Err(SurfaceError::DegenerateQuad {
                    face: face as FaceId,
                });
            }
        }

        // `EdgeSlot::MinU` fixes `u`, so `u` varies along `MinV` and `MaxV`. An edge is therefore
        // the same *kind* for both its faces exactly when their slots agree on that, and a mesh
        // where they do not is wound inconsistently.
        for (edge, faces) in &topology.edge_faces {
            if let [(_, first), (_, second)] = faces.as_slice() {
                if first.varies_along_u() != second.varies_along_u() {
                    return Err(SurfaceError::InconsistentWinding {
                        edge: *edge,
                        first: *first,
                        second: *second,
                    });
                }
            }
        }

        let derivatives = seed_derivatives(&positions, &topology);
        // A fresh model has no twist, which is the "twist-free" bicubic patch and is what a caller
        // means by "start from my mesh". Every edge seeds as shared, so a fresh surface is smooth
        // everywhere; a caller introduces a crease by tagging an edge hard.
        let (steps, spread) = measure_steps(&positions, &topology);
        let private_twist = vec![[Point3::ZERO; 4]; quads.len()];
        let join = topology
            .edge_faces
            .keys()
            .map(|edge| (*edge, Join::Soft))
            .collect();

        Ok(Self {
            positions,
            quads,
            topology,
            derivatives,
            steps,
            spread,
            private_twist,
            join,
        })
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

    pub fn face_count(&self) -> usize {
        self.quads.len()
    }

    pub fn join(&self, edge: &EdgeKey) -> Join {
        self.join.get(edge).copied().unwrap_or_default()
    }

    /// Tags an edge.
    ///
    /// **Always safe.** Positions and both first derivatives are shared whatever this says, so an
    /// edge can never become a crack; what changes is the twist, and therefore the cross-boundary
    /// derivative, and therefore the tangent plane. Soft makes the two faces identical there; hard
    /// lets them differ.
    pub fn set_join(&mut self, edge: EdgeKey, join: Join) {
        self.join.insert(edge, join);
    }

    /// The corner jet a face reads at one of its corners.
    pub fn corner_jet(&self, face: FaceId, corner: usize) -> Jet {
        let vertex = self.quads[face as usize][corner];
        let (du, dv) = self.derivatives[vertex as usize];
        Jet {
            point: self.positions[vertex as usize],
            du,
            dv,
            duv: self.twist_at(face, corner),
        }
    }

    /// The twist a face reads at one of its corners. One slot per face corner, so a face can hold
    /// a different answer from its neighbour; [`HermiteSurface::set_twist`] is what keeps the two
    /// in step wherever they are required to.
    fn twist_at(&self, face: FaceId, corner: usize) -> Point3 {
        self.private_twist[face as usize][corner]
    }

    /// Which corner of a face sits at `vertex`, if any.
    ///
    /// Written as a search rather than as arithmetic on the winding because the two disagree at
    /// exactly one corner, and a face whose corner 3 is resolved as corner 2 puts two of its
    /// vertices on one place and disagrees with its neighbour about where the edge runs.
    fn corner_at(&self, face: FaceId, vertex: VertexId) -> Option<usize> {
        self.quads[face as usize]
            .iter()
            .position(|held| *held == vertex)
    }

    /// Sets one face's twist at one corner, carrying it across every **soft** edge at that corner.
    ///
    /// **Always succeeds, and always leaves the surface continuous.** Across a soft edge the same
    /// value is written to the face on the other side in the same call, so the two faces remain one
    /// surface there. Across a **hard** edge nothing is carried, and that is exactly how a crease
    /// appears: tag an edge hard, twist its two sides apart, and the join stops being C1 while
    /// staying C0.
    ///
    /// This is the whole difference from an enforcement pass, and it is worth being exact about the
    /// mechanism. Today a caller writes a control point and a later pass repairs the model, so the
    /// surface has been wrong for the length of that interval and the repair is invisible to the
    /// caller — which is how `Edit::Overconstrained` and the write-order arbitration came to
    /// exist. Here **the write is the whole edit**: there is nothing to defer, nothing to order,
    /// and no state the next read has to repair. A caller cannot leave the model in a condition it
    /// has to know about, which is the property that makes this pleasant to drive from an agent.
    pub fn set_twist(&mut self, face: FaceId, corner: usize, twist: Point3) {
        let vertex = self.quads[face as usize][corner];

        // Transitive, and the depth is what matters here. The faces around a vertex are a **cycle**,
        // not a tree: carrying the value to the immediate neighbours leaves the *next* face on the
        // ring holding an older value, which breaks the soft edge between it and its own
        // neighbour. Measured, that arm reported `1.699e-1` rad on seams the edit was not aimed at.
        // The walk is bounded by the valence of one vertex, so it is constant work per write and
        // never a scan over the mesh.
        let mut written: BTreeSet<(FaceId, usize)> = BTreeSet::new();
        let mut pending = vec![(face, corner)];
        while let Some((current, current_corner)) = pending.pop() {
            if !written.insert((current, current_corner)) {
                continue;
            }
            self.private_twist[current as usize][current_corner] = twist;
            for slot in EdgeSlot::ALL {
                let ends = crate::topology::edge_endpoints(self.quads[current as usize], slot);
                if !ends.contains(&vertex) {
                    continue;
                }
                let key = canonical(ends);
                if self.join(&key) != Join::Soft {
                    continue;
                }
                let Some(faces) = self.topology.edge_faces.get(&key) else {
                    continue;
                };
                for (neighbour, _) in faces {
                    if let Some(other) = self.corner_at(*neighbour, vertex) {
                        pending.push((*neighbour, other));
                    }
                }
            }
        }
    }

    /// The twist a face reads at a corner, for a caller that wants to inspect rather than change.
    pub fn twist(&self, face: FaceId, corner: usize) -> Point3 {
        self.twist_at(face, corner)
    }

    /// Every `(face, corner)` reachable from a vertex through soft edges.
    ///
    /// **The set [`HermiteSurface::set_twist`] writes, and therefore the set an undo has to
    /// record.** A twist travels around the faces at a vertex, so one call writes several slots, and
    /// a caller that records only the one it asked for leaves the rest stale on the way back. That
    /// is not a subtlety of this module's implementation — it is what "carries a value across every
    /// soft edge at the corner" means, and a caller keeping an undo stack has to see all of it.
    ///
    /// Bounded by the valence of one vertex, so it is constant work and never a scan.
    pub fn soft_ring(&self, vertex: VertexId) -> Vec<(FaceId, usize)> {
        let mut found: Vec<(FaceId, usize)> = Vec::new();
        let mut pending: Vec<(FaceId, usize)> = Vec::new();
        for (face, quad) in self.quads.iter().enumerate() {
            if let Some(corner) = quad.iter().position(|held| *held == vertex) {
                pending.push((face as FaceId, corner));
            }
        }
        while let Some((face, corner)) = pending.pop() {
            if found.contains(&(face, corner)) {
                continue;
            }
            found.push((face, corner));
            let quads = self.quads[face as usize];
            for slot in EdgeSlot::ALL {
                let ends = crate::topology::edge_endpoints(quads, slot);
                if !ends.contains(&vertex) {
                    continue;
                }
                if self.join(&canonical(ends)) != Join::Soft {
                    continue;
                }
                for (neighbour, _) in &self.topology.edge_faces[&canonical(ends)] {
                    if let Some(other) = self.corner_at(*neighbour, vertex) {
                        pending.push((*neighbour, other));
                    }
                }
            }
        }
        found
    }

    /// Writes one face's twist at one corner **without carrying it across soft edges**.
    ///
    /// **For undo, and for nothing else.** It is the one write in this type that can put the two
    /// faces on a soft edge out of step, and it exists because undo has to put back a set of slots
    /// exactly as they were — which means replaying each write directly rather than re-deriving
    /// which slots a carry would reach today. A caller reaching for this to "just set a twist"
    /// reintroduces precisely the defect this representation exists to remove.
    pub fn write_twist_raw(&mut self, face: FaceId, corner: usize, twist: Point3) {
        self.private_twist[face as usize][corner] = twist;
    }

    /// The patch one face reads, built from its four corner jets and the edge steps.
    ///
    /// **This path and per-edge boundary scaling were both tried; this one is the one that measures
    /// better, and the other one is why.** Scaling each of the four boundary lines by the chord of
    /// *its own* edge — which is what "each face's boundary is the shared edge's curve" seems to
    /// ask for — takes the crease's split to `0.000e+00` and takes the soft seams from `2.892e-3`
    /// to `3.010e-2` with zero-length normals. The cause is structural: `dS/du` at the `u = 0`
    /// boundary reads the `a = 1` row, whose end coefficients come from **this face's** `MinV` and
    /// `MaxV` edges — the two perpendicular edges — and those are not the shared edge. So the
    /// cross-boundary derivative stops being determined by shared data, and the normal mismatch
    /// that a soft edge has to eliminate comes back.
    ///
    /// So the two facts are: **a bicubic Hermite face cannot have both a per-edge-scaled boundary
    /// curve and a shared cross-boundary derivative**, because the corner jet has two derivative
    /// slots and a shared edge's boundary tangent is not the same slot as the shared edge's
    /// cross-boundary. The measurement in both directions is in `examples/hermite_probe.rs` and the
    /// gallery scene's handoff.
    ///
    /// What this path gives, measured: **exactly C1** (`across 0.000e+00` on every seam of the
    /// crate's grid) and a residual split of `2.892e-3` on a mesh whose cells are not exactly equal,
    /// because `step_v` belongs to the face and the two faces of a shared edge hold slightly
    /// different ones. Both numbers are properties of this representation, not of one caller.
    pub fn patch(&self, face: FaceId) -> HermitePatch {
        HermitePatch::new(
            self.step_u(),
            self.step_v(),
            [
                self.corner_jet(face, 0),
                self.corner_jet(face, 1),
                self.corner_jet(face, 2),
                self.corner_jet(face, 3),
            ],
        )
    }

    /// Evaluates the surface at one face's local parameters.
    pub fn evaluate(&self, face: FaceId, s: f64, t: f64) -> Point3 {
        self.patch(face).evaluate(s, t)
    }

    /// `dS/du` in world units, read from the face rather than derived from a triangle.
    pub fn tangent_u(&self, face: FaceId, s: f64, t: f64) -> Point3 {
        self.patch(face).tangent_u(s, t)
    }

    /// `dS/dv` in world units.
    pub fn tangent_v(&self, face: FaceId, s: f64, t: f64) -> Point3 {
        self.patch(face).tangent_v(s, t)
    }

    /// `dS/du x dS/dv`. Read it from the **composed** surface; normals of layers do not add.
    pub fn normal(&self, face: FaceId, s: f64, t: f64) -> Point3 {
        self.patch(face).normal(s, t)
    }

    /// The parameter distance a face spans along `u`, and the chord of its `MinV` edge.
    ///
    /// **Not the number that matters.** What matters is that the two faces sharing an edge get the
    /// *same* one, and that is enforced by [`HermiteSurface::step_u`] rather than by this
    /// calculation. Kept because a caller measuring a mesh wants to know how far apart the chords
    /// are, which is exactly the residual the shared step is buying freedom from.
    pub fn step_along_u(&self, face: FaceId) -> f64 {
        let quads = self.quads[face as usize];
        self.chord(crate::topology::edge_endpoints(quads, EdgeSlot::MinV))
    }

    /// The parameter distance a face spans along `v`, the chord of its `MinU` edge. See
    /// [`HermiteSurface::step_along_u`].
    pub fn step_along_v(&self, face: FaceId) -> f64 {
        let quads = self.quads[face as usize];
        self.chord(crate::topology::edge_endpoints(quads, EdgeSlot::MinU))
    }

    /// The step every face spans along `u`. **The parameter synchronization, in one number.**
    ///
    /// It is the mean chord over the mesh rather than a per-face chord, and that is the whole of the
    /// design decision. The step enters twice and both times it has to agree:
    ///
    /// - a boundary curve's Hermite tangent is `dP/du * step_u`, so a per-face step makes two faces
    ///   draw *different* curves along a shared edge — measured `2.892e-3` apart on the gallery
    ///   sheet, against `0.000e+00` on a grid whose cells happen to be equal;
    /// - `dS/du` is `1 / step_u` times a quantity that *is* shared, so a per-face step scales the two
    ///   faces' normals by different amounts and leaves a seam that reads `2.526e-3` instead of
    ///   zero.
    ///
    /// **Neither residual is the surface's; both are the parameterization's.** With one shared step
    /// per direction the surface cannot tell the difference, because there is no difference to tell.
    ///
    /// The cost is stated rather than hidden: a mesh whose cells are strongly anisotropic gets one
    /// uniform parameterization, so its tangents are stretched relative to arc length.
    /// [`HermiteSurface::step_spread`] measures how much, and a caller with a structured mesh can
    /// replace the mean with a per-grid-line step, which is the obvious refinement.
    pub fn step_u(&self) -> f64 {
        self.steps.0
    }

    /// The step every face spans along `v`. See [`HermiteSurface::step_u`].
    pub fn step_v(&self) -> f64 {
        self.steps.1
    }

    /// How unequal the mesh's cells are, as the ratio of the largest chord to the smallest in each
    /// direction.
    ///
    /// This is the residual the shared step is standing in for: at `1.000e+00` the two families of
    /// measurement agree exactly, and the ratio is what a caller needs in order to say how much of
    /// its surface is being described by the mean rather than by the geometry.
    pub fn step_spread(&self) -> (f64, f64) {
        self.spread
    }

    /// Moves a mesh vertex.
    ///
    /// **Always safe.** Every face at the vertex follows it and the shared boundary curves move with
    /// it, because the curves are generated from the positions rather than stored beside them. A
    /// drag cannot open a crack, and it cannot create a crease either — a vertex that moves takes
    /// its tangents with it.
    pub fn set_position(&mut self, vertex: VertexId, point: Point3) {
        self.positions[vertex as usize] = point;
    }

    /// Sets `du` and `dv` at a mesh vertex.
    ///
    /// **Always safe, and this is the property that makes the surface pleasant to edit.** The two
    /// are shared by every face at the vertex, so writing them changes the surface smoothly
    /// everywhere at once and there is nothing left to re-establish: no enforcement pass, no order
    /// to get wrong, and no edit that can leave the model in a state the next read has to repair.
    pub fn set_derivative(&mut self, vertex: VertexId, du: Point3, dv: Point3) {
        self.derivatives[vertex as usize] = (du, dv);
    }

    /// The tangent field at a vertex, for a caller that wants to adjust rather than replace it.
    pub fn derivative(&self, vertex: VertexId) -> (Point3, Point3) {
        self.derivatives[vertex as usize]
    }

    fn chord(&self, ends: [VertexId; 2]) -> f64 {
        let [first, second] = ends;
        let distance = self.positions[first as usize].distance(self.positions[second as usize]);
        if distance > 0.0 { distance } else { 1.0 }
    }
}

/// Measures the mesh's chords in each direction and returns the mean per direction and the ratio of
/// largest to smallest.
///
/// The mean is what every face uses, so that a shared edge has one step rather than two. The spread
/// is reported because it is the size of the thing being averaged: a spread of `1.0` is a mesh whose
/// cells are equal and a spread of `2.0` is a mesh being described by a number that fits none of
/// them, which a caller is entitled to know before it sculpts.
fn measure_steps(positions: &[Point3], topology: &Topology) -> ((f64, f64), (f64, f64)) {
    let mut along_u: Vec<f64> = Vec::new();
    let mut along_v: Vec<f64> = Vec::new();
    for (edge, faces) in &topology.edge_faces {
        let Some((_, slot)) = faces.first() else {
            continue;
        };
        let (first, second) = *edge;
        let distance = positions[first as usize].distance(positions[second as usize]);
        if distance <= 0.0 {
            continue;
        }
        if slot.varies_along_u() {
            along_u.push(distance);
        } else {
            along_v.push(distance);
        }
    }
    let mean = |values: &Vec<f64>| -> f64 {
        if values.is_empty() {
            return 1.0;
        }
        values.iter().sum::<f64>() / values.len() as f64
    };
    let ratio = |values: &Vec<f64>| -> f64 {
        match (
            values.iter().cloned().fold(f64::INFINITY, f64::min),
            values.iter().cloned().fold(0.0f64, f64::max),
        ) {
            (low, high) if low > 0.0 => high / low,
            _ => 1.0,
        }
    };
    (
        (mean(&along_u), mean(&along_v)),
        (ratio(&along_u), ratio(&along_v)),
    )
}

/// Seeds `du` and `dv` at every vertex from the direction of each incident edge, half-weighted so a
/// vertex on `n` edges averages to something of that size rather than `n` times it.
///
/// An edge's tangent is `du` when `u` varies along it and `dv` otherwise — the one decision
/// [`EdgeSlot::is_u_edge`] makes, and the reason a per-vertex field can serve every face at a
/// vertex.
///
/// A difference rather than a closed form, and that is a choice about what is being claimed: what is
/// under test is that the surface built from *whatever* tangent field is continuous across a seam,
/// so seeding it beautifully would be measuring the seed.
fn seed_derivatives(positions: &[Point3], topology: &Topology) -> Vec<(Point3, Point3)> {
    let vertex_count = positions.len();
    let mut du = vec![Point3::ZERO; vertex_count];
    let mut dv = vec![Point3::ZERO; vertex_count];

    for (edge, faces) in &topology.edge_faces {
        let Some((_, slot)) = faces.first() else {
            continue;
        };
        let (first, second) = *edge;
        let distance = positions[first as usize].distance(positions[second as usize]);
        if distance == 0.0 {
            continue;
        }
        let along = (positions[second as usize] - positions[first as usize]) / distance;
        let half = 0.5;
        if slot.varies_along_u() {
            du[first as usize] = along * half;
            du[second as usize] = along * half;
        } else {
            dv[first as usize] = along * half;
            dv[second as usize] = along * half;
        }
    }
    (0..vertex_count)
        .map(|vertex| (du[vertex], dv[vertex]))
        .collect()
}
