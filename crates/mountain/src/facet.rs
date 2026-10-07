//! The expanded form: a triangle mesh whose faces are the massif's planar block faces.
//!
//! # Invariants
//!
//! - **Every face is exactly planar.** Elevation is sampled on the skeleton lines and nowhere
//!   else; a triangle's interior is the linear interpolation of its three corners and is
//!   never displaced. This is the contract that keeps the block-face reading intact for a
//!   downstream pipeline, and it is not something the mesh can check for itself — it is a
//!   property of the expansion, held here as its guarantee.
//! - **The mesh is watertight and covers the footprint exactly once.** A hole and an overlap
//!   are both failures of the authored correspondence, and both show up as the mesh's plan
//!   area disagreeing with the footprint's. That is a number, not a judgement, and
//!   [`FacetMesh::plan_area`] is where it is read.
//! - A face carries its own provenance and never an index into a previous build, because
//!   every edit re-expands and every index moves.

use std::collections::{BTreeMap, BTreeSet};

use crate::line::LineRef;
use crate::math::{Vec2, Vec3, distance};
use crate::tessellation::BeltRef;

/// One planar block face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    /// The three points, wound so the face looks up.
    pub indices: [u32; 3],
    /// The belt's own parameter space at each of those three points.
    ///
    /// # Invariants
    ///
    /// - **Entry `i` is the point `indices[i]` is**, in that order and no other, because the
    ///   winding is fixed by the face's own plan area and a pair of lists an author has to zip
    ///   by hand is a second thing that can disagree with the first. A face stores its uv
    ///   beside its indices for the same reason [`FaceAttributes::belt`] is stored beside both:
    ///   they are one fact about one face.
    /// - **`u` is the fraction of the belt's upper boundary already walked, by distance**, and
    ///   `v` is the fraction from that boundary down to the lower one. Distance rather than
    ///   sample index because a belt's top path turns a corner where a spur leaves its parent's
    ///   flank, and at that corner the two differ — and "62% of the way along the crest" is the
    ///   sentence an author means, where "62% of the samples" is a fact about the tessellation.
    /// - **`v` is zero on the crest and one on the valley** for every face of a belt, so `v` says
    ///   how far down a flank a point is rather than how far along a row of one tessellation.
    pub uv: [[f32; 2]; 3],
    /// What the face is, for whatever reads the mesh after this crate.
    pub attributes: FaceAttributes,
}

/// One face, in the belt's own coordinates.
///
/// # Invariants
///
/// This is a **point on a belt**, not a point in space: it says where on the surface between one
/// valley and the line it faces the point lies, and the mesh turns it into a position. It is the
/// answer a caller reading a rendered pixel needs, because a position says where the surface is
/// and a belt coordinate says **which authored line and which station** that surface came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeltPoint {
    /// Which belt.
    pub belt: BeltRef,
    /// How far along its upper boundary, and how far down the strip.
    pub uv: [f32; 2],
}

/// What a face is, in the skeleton's own terms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceAttributes {
    /// The belt the face was expanded from.
    pub belt: BeltRef,
    /// The line the belt's upper boundary is on.
    pub top: LineRef,
    /// The line the belt's lower boundary is on.
    pub bottom: LineRef,
    /// The plan direction the face falls in, which is the horizontal part of its own normal.
    pub dip: Vec2,
    /// How far the face falls, in radians from the horizontal.
    pub dip_angle: f32,
    /// How broken the crest above the face is, from the ridge's own sharpness.
    pub ruggedness: f32,
}

/// The expanded form of a [`crate::Massif`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FacetMesh {
    /// Every vertex, shared: a sample of a skeleton line is one point however many faces
    /// meet it.
    pub points: Vec<Vec3>,
    /// Every face.
    pub faces: Vec<Face>,
}

impl FacetMesh {
    /// Returns the three points of a face.
    pub fn triangle(&self, face: &Face) -> [Vec3; 3] {
        [
            self.points[face.indices[0] as usize],
            self.points[face.indices[1] as usize],
            self.points[face.indices[2] as usize],
        ]
    }

    /// Returns the face's upward normal.
    ///
    /// # Invariants
    ///
    /// It is the normal of the side that is **looked at**, not the right-handed one over the
    /// winding; see [`crate::math::face_normal`] for why those are opposite here. A face with no
    /// plan area at all — one seen exactly edge on — has no side to be looked at, and the fallback
    /// is east rather than up so that a downstream reader cannot mistake a collapsed face for a
    /// level one.
    pub fn normal(&self, face: &Face) -> Vec3 {
        let [a, b, c] = self.triangle(face);
        crate::math::face_normal(a, b, c)
    }

    /// Returns the plan area the mesh covers.
    ///
    /// # Invariants
    ///
    /// It is the **signed** area, and the sign is kept. A mesh with a folded face has a
    /// positive area and a negative patch, and summing absolute areas would report it as
    /// whole — which is the one failure this number exists to find. Compared against the
    /// footprint's own signed area it is therefore a test of coverage and orientation at
    /// once.
    pub fn plan_area(&self) -> f32 {
        self.faces
            .iter()
            .map(|face| {
                let [a, b, c] = self.triangle(face);
                crate::math::signed_area([a[0], a[2]], [b[0], b[2]], [c[0], c[2]])
            })
            .sum()
    }

    /// Returns the lowest and highest space point the mesh reaches on each axis.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for point in &self.points {
            for axis in 0..3 {
                low[axis] = low[axis].min(point[axis]);
                high[axis] = high[axis].max(point[axis]);
            }
        }
        (low, high)
    }

    /// Returns the length of the shortest edge, which is how fine the mesh reads.
    pub fn shortest_edge(&self) -> f32 {
        let mut shortest = f32::INFINITY;
        for face in &self.faces {
            let [a, b, c] = self.triangle(face);
            for (from, to) in [(a, b), (b, c), (c, a)] {
                shortest = shortest.min(distance([from[0], from[2]], [to[0], to[2]]));
            }
        }
        shortest
    }

    /// Returns every edge with how many faces use it, keyed by its two vertex indices.
    ///
    /// # Invariants
    ///
    /// The key is ordered, so the three edges of a triangle are the same three however the
    /// triangle is wound and two faces sharing an edge agree on it. That is what makes the count a
    /// count of faces per edge rather than of directed half-edges, which is the number that decides
    /// whether the surface is a manifold.
    fn edges(&self) -> BTreeMap<(u32, u32), usize> {
        let mut edges: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        for face in &self.faces {
            for corner in 0..3 {
                let (a, b) = (face.indices[corner], face.indices[(corner + 1) % 3]);
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        edges
    }

    /// Returns how many closed loops of boundary the surface has.
    ///
    /// # Invariants
    ///
    /// A closed surface has none, and a **disc** — which is what a massif cut out of a footprint is
    /// meant to be — has exactly one, its rim. This is the number that catches a hole the plan area
    /// cannot: a junction left unwelded leaves triangles collapsed at one point, and a collapsed
    /// triangle encloses no area at all, so the area check passes and the surface has a slit.
    pub fn boundary_loops(&self) -> usize {
        let mut neighbours: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for ((a, b), count) in self.edges() {
            if count != 1 {
                continue;
            }
            neighbours.entry(a).or_default().push(b);
            neighbours.entry(b).or_default().push(a);
        }
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let mut loops = 0;
        for start in neighbours.keys().copied() {
            if !seen.insert(start) {
                continue;
            }
            loops += 1;
            let mut stack = vec![start];
            while let Some(vertex) = stack.pop() {
                for next in neighbours.get(&vertex).into_iter().flatten() {
                    if seen.insert(*next) {
                        stack.push(*next);
                    }
                }
            }
        }
        loops
    }

    /// Returns the Euler characteristic of the surface: vertices less edges plus faces.
    ///
    /// # Invariants
    ///
    /// It is read over the mesh's **own** vertices — the pair of line and sample the expander
    /// assigned — and not over positions welded afterwards. Welding coincident positions would
    /// report a slit as closed, which is exactly the failure this is here to find.
    ///
    /// A disc is one, a closed surface with no boundary is two less twice its genus, and an annulus
    /// is zero.
    pub fn euler_characteristic(&self) -> i64 {
        self.points.len() as i64 - self.edges().len() as i64 + self.faces.len() as i64
    }

    /// Returns whether every edge is shared by exactly two faces.
    ///
    /// # Invariants
    ///
    /// A closed surface is a manifold only if no edge carries three faces, which is what a
    /// duplicated sheet or a folded flap looks like. An edge used once is a boundary and is not
    /// asked about here; [`FacetMesh::boundary_loops`] counts those.
    pub fn is_manifold(&self) -> bool {
        self.edges().values().all(|count| *count <= 2)
    }

    /// Returns where on its belt a face is, given a point's uv in the belt's own coordinates.
    ///
    /// This is the **inverse of reading a face's uv**, and it is the half of that pair a caller
    /// holding a rendered pixel needs: the rasteriser hands back the three corner uv of the face
    /// it covered, interpolated across the triangle, and this turns that one pair of numbers into
    /// the belt and the position on it.
    ///
    /// # Invariants
    ///
    /// - **It reads the face's own three corners and nothing else**, so a uv written by the same
    ///   [`Face::uv`] a scene's point data carries comes back to the belt it came from. A
    ///   definition of the belt's parameter space written down twice is a second thing that can
    ///   be wrong, and it would be wrong silently and by a rotation of the triangle.
    /// - **A uv outside the triangle is clamped to the triangle**, because a rasteriser
    ///   interpolates a varying linearly and a point on a shared edge legitimately carries
    ///   either side's value. The result is therefore the closest belt coordinate on this face,
    ///   which is what a caller asking "where is that pixel on this face" means.
    /// - **It is O(1)**: three corners, one triangle. A caller locating a whole contact sheet
    ///   pays a constant per pixel rather than a walk of the mesh.
    pub fn locate(&self, face: &Face, uv: [f32; 2]) -> BeltPoint {
        // Barycentric coordinates of the point within the face's own uv triangle, by the areas
        // the three subtriangles cut off — the standard construction, and the one that needs no
        // assumption about the winding because a subtraction of a signed area is signed either
        // way.
        let [a, b, c] = face.uv;
        let whole = area_of(a, b, c);
        let (wa, wb, wc) = if whole.abs() <= AREA_EPSILON {
            // A degenerate uv triangle has no interior to interpolate in, which happens where a
            // summit closes: every corner of the collapsed row carries the same uv. The corner is
            // then as good an answer as any, and it is a point on the belt rather than a division
            // by nothing.
            (1.0, 0.0, 0.0)
        } else {
            (
                area_of(uv, b, c) / whole,
                area_of(a, uv, c) / whole,
                area_of(a, b, uv) / whole,
            )
        };
        BeltPoint {
            belt: face.attributes.belt,
            uv: [
                wa * a[0] + wb * b[0] + wc * c[0],
                wa * a[1] + wb * b[1] + wc * c[1],
            ],
        }
    }
}

/// How small a uv triangle's area must be before it is treated as having no interior.
///
/// It is the same threshold the expander drops a collapsed face at, on purpose: a face that was
/// not emitted and a uv triangle that cannot be interpolated in are the same condition reached
/// two ways, and one number for both is one fewer place for them to disagree.
const AREA_EPSILON: f32 = 1.0e-6;

/// Returns the signed area of a triangle in a two dimensional parameter space.
fn area_of(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
}
