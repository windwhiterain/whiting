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

use crate::line::LineRef;
use crate::math::{Vec2, Vec3, distance};
use crate::tessellation::BeltRef;

/// One planar block face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    /// The three points, wound so the face looks up.
    pub indices: [u32; 3],
    /// What the face is, for whatever reads the mesh after this crate.
    pub attributes: FaceAttributes,
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
}
