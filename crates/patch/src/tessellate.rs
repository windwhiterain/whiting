//! Sampling a mesh of patches into an indexed mesh, with every shared edge sampled once.
//!
//! # Why this is a module and not three lines at the call site
//!
//! A face's patch evaluates at any parameter, so a caller could sample each face's own grid and be
//! done. That is the wrong way twice over.
//!
//! The **wrongness** is that two faces sampling the same boundary independently produce two sets of
//! numbers that are equal in exact arithmetic and need not be equal in `f64`. The rows are mirrored,
//! so the two faces sum the same Bernstein terms in the opposite order, and the rounding differs.
//! How much that differs is a measurement (`tessellation_probe`), not an assumption.
//!
//! The **cost** is that a mesh with `F` faces and `E` shared edges stores a copy of every shared
//! curve per side, which is exactly the duplication a rasterizer then has to reconcile at draw time.
//!
//! So the boundary is sampled once, in the edge's canonical direction, and both faces are handed the
//! same indices into the same points. Two faces cannot then disagree about where a shared edge is,
//! because there is only one set of numbers to disagree about.
//!
//! # What the indices mean
//!
//! A [`FaceGrid`] is `side` by `side` samples with `i` running over `u` and `j` over `v`, so grid
//! `(0, 0)` is the patch's `(0, 0)` corner and `(side - 1, side - 1)` is `(1, 1)`. Samples are laid
//! out that way rather than as triangles, so a caller tessellates to whichever primitive it draws.

use std::collections::BTreeMap;

use crate::patch_mesh::PatchMesh;
use crate::point::Point3;
use crate::topology::{EdgeKey, EdgeSlot, FaceId, VertexId, canonical, edge_endpoints};

/// A whole surface sampled into points that faces share.
#[derive(Clone, Debug, PartialEq)]
pub struct Tessellation {
    points: Vec<Point3>,
    faces: Vec<FaceGrid>,
    side: usize,
}

impl Tessellation {
    /// Every sampled point. One per shared boundary position, however many faces meet on it.
    pub fn points(&self) -> &[Point3] {
        &self.points
    }

    /// How many faces the surface has.
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Samples per edge, so each face is `(side - 1)` by `(side - 1)` quads.
    pub fn side(&self) -> usize {
        self.side
    }

    /// The grid of one face, or `None` when there is no such face.
    pub fn face(&self, face: FaceId) -> Option<&FaceGrid> {
        self.faces.get(face as usize)
    }

    /// Every face's grid, in face order.
    pub fn faces(&self) -> &[FaceGrid] {
        &self.faces
    }

    /// How many faces use each point, indexed by point.
    ///
    /// A point used by two faces is a shared boundary sample, and a point used by one is either an
    /// interior sample or a mesh vertex nothing else reached. This is how a caller checks that the
    /// sharing happened rather than assuming it: **the count is structural, not geometric**, so no
    /// tolerance is involved.
    pub fn uses_per_point(&self) -> Vec<usize> {
        let mut uses = vec![0usize; self.points.len()];
        for face in &self.faces {
            for index in &face.points {
                uses[*index] += 1;
            }
        }
        uses
    }
}

/// One face's patch, sampled on a `side` by `side` grid of indices into [`Tessellation::points`].
#[derive(Clone, Debug, PartialEq)]
pub struct FaceGrid {
    side: usize,
    points: Vec<usize>,
}

impl FaceGrid {
    /// Samples per edge.
    pub fn side(&self) -> usize {
        self.side
    }

    /// The indices along one of this face's four edges, first end first.
    ///
    /// A rasteriser needs this to close a face's boundary loop, and a caller checking that two faces
    /// share an edge needs it to see that both hold the *same indices*, in one order or the other.
    /// That is a fact about integers rather than about coordinates, so nothing here is a tolerance.
    pub fn edge(&self, slot: EdgeSlot) -> Vec<usize> {
        (0..self.side)
            .map(|k| {
                let (i, j) = grid_position(self.side, slot, k);
                self.points[i * self.side + j]
            })
            .collect()
    }

    /// Every index, row-major in `(i, j)` with `i` running over `u`.
    pub fn points(&self) -> &[usize] {
        &self.points
    }

    /// The index of the sample at grid `(i, j)`, or `None` when the grid does not reach it.
    pub fn at(&self, i: usize, j: usize) -> Option<usize> {
        if i >= self.side || j >= self.side {
            return None;
        }
        self.points.get(i * self.side + j).copied()
    }
}

impl PatchMesh {
    /// Samples the whole surface into an indexed mesh of `side` by `side` grids.
    ///
    /// Every shared boundary is sampled once and both faces are handed the same points, so the
    /// result has no crack a caller could find by comparing numbers — the comparison is between
    /// indices, not between coordinates.
    ///
    /// # Panics
    ///
    /// If `side` is below two. A face with one sample across it is a point rather than a face, and
    /// the alternative would be a silently empty mesh.
    ///
    /// Nothing constrains `side` against the patch's degree. Sampling is by **parameter**, not by
    /// control index, so a coarse patch can be sampled finely and a fine one coarsely, and the
    /// samples a caller gets do not move when the degree does.
    pub fn tessellate(&self, side: usize) -> Tessellation {
        assert!(
            side >= 2,
            "a tessellation needs at least two samples across a face"
        );
        let mut points: Vec<Point3> = Vec::new();
        let mut vertices: BTreeMap<VertexId, usize> = BTreeMap::new();
        let mut edges: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();

        // Every distinct edge, once, in canonical order. Canonical rather than per-face so that two
        // faces meeting on it are looking at the same list and not at two lists that ought to
        // match.
        for (key, faces) in &self.topology().edge_faces {
            let &(face, slot) = faces
                .first()
                .expect("every edge is held by at least one face");
            let ends = edge_endpoints(self.quads()[face as usize], slot);
            let wound_forwards = ends == [key.0, key.1];
            let indices: Vec<usize> = (0..side)
                .map(|k| {
                    // Canonical position `k` is the parameter `k / (side - 1)` along the edge, which
                    // is the same parameter whichever face and whichever degree is looking at it.
                    let along = k as f64 / (side - 1) as f64;
                    let across = if wound_forwards { along } else { 1.0 - along };

                    // The two ends of an edge are **mesh vertices**, and a vertex belongs to every
                    // face around it rather than to one edge. Sampling it per edge would leave each
                    // face arriving at the same point by a different route, which is agreement in
                    // value and not in place — and a rasteriser cannot tell the difference. So a
                    // vertex is sampled once for the whole mesh, and enforcement has already given
                    // every face's copy of it the same number.
                    let vertex = match k {
                        0 => Some(key.0),
                        _ if k == side - 1 => Some(key.1),
                        _ => None,
                    };
                    let index = match vertex.and_then(|id| vertices.get(&id).copied()) {
                        Some(shared) => shared,
                        None => {
                            points.push(self.sample_edge_position(face, slot, across));
                            points.len() - 1
                        }
                    };
                    if let Some(id) = vertex {
                        vertices.insert(id, index);
                    }
                    index
                })
                .collect();
            edges.insert(*key, indices);
        }

        let mut grids = Vec::with_capacity(self.quads().len());
        for (index, corners) in self.quads().iter().enumerate() {
            let face = index as FaceId;
            let mut grid = vec![usize::MAX; side * side];
            for slot in EdgeSlot::ALL {
                let ends = edge_endpoints(*corners, slot);
                let key = canonical(ends);
                let samples = edges
                    .get(&key)
                    .expect("an edge of a face was sampled when the mesh was walked");
                // The face's own winding runs the other way when its first end is not the
                // canonical one, which is the only difference between the two faces' views of it.
                let wound_forwards = ends == [key.0, key.1];
                let along = (0..side).map(|k| {
                    if wound_forwards {
                        samples[k]
                    } else {
                        samples[side - 1 - k]
                    }
                });
                for (k, sample) in along.enumerate() {
                    let (i, j) = grid_position(side, slot, k);
                    let cell = &mut grid[i * side + j];
                    // A corner belongs to two of this face's own edges. Filling it once keeps it one
                    // index rather than two that happen to hold the same number.
                    if *cell == usize::MAX {
                        *cell = sample;
                    }
                }
            }

            // The interior belongs to this face alone, so it is evaluated here and nowhere else.
            for i in 1..side - 1 {
                for j in 1..side - 1 {
                    let u = i as f64 / (side - 1) as f64;
                    let v = j as f64 / (side - 1) as f64;
                    grid[i * side + j] = points.len();
                    points.push(self.evaluate(face, u, v));
                }
            }
            debug_assert!(grid.iter().all(|cell| *cell != usize::MAX));
            grids.push(FaceGrid { side, points: grid });
        }

        Tessellation {
            points,
            faces: grids,
            side,
        }
    }

    /// The surface at edge fraction `across` of `face`'s `slot` row, `across` running `0` to `1`.
    ///
    /// By parameter rather than by control index, so the answer does not depend on the degree and a
    /// caller comparing two tessellations of the same surface is comparing the same points.
    fn sample_edge_position(&self, face: FaceId, slot: EdgeSlot, across: f64) -> Point3 {
        match slot {
            EdgeSlot::MinU => self.evaluate(face, 0.0, across),
            EdgeSlot::MaxU => self.evaluate(face, 1.0, across),
            EdgeSlot::MinV => self.evaluate(face, across, 0.0),
            EdgeSlot::MaxV => self.evaluate(face, across, 1.0),
        }
    }
}

/// Where sample `k` of `side` along `slot` lands in a face's grid, whose `i` runs over `u`.
///
/// The four cases are not special: each edge fixes one of the two parameters and runs the other
/// from `0` at its first end to `1` at its second.
fn grid_position(side: usize, slot: EdgeSlot, k: usize) -> (usize, usize) {
    debug_assert!(side >= 2);
    let last = side - 1;
    match slot {
        EdgeSlot::MinU => (0, k),
        EdgeSlot::MaxU => (last, k),
        EdgeSlot::MinV => (k, 0),
        EdgeSlot::MaxV => (k, last),
    }
}
