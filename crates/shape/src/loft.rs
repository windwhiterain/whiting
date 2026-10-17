//! A ring grid written as triangles, with the parameter of every vertex in it.
//!
//! # The one idea
//!
//! **`(u, v)` is owned here, not by a consumer.** `u` is the station's arc-length fraction along the
//! spine and `v` is `i / around`, the arc-length fraction around the section's loop — and both are
//! read from the same array as the positions, so a consumer that packs point data cannot invent a
//! second table. That is `mountain`'s `belt_uv` discipline taken one type down: one function, one
//! definition, one consumer.
//!
//! # What is not here
//!
//! No normals (a consumer's shading chooses its own), no material, no scene. **No quads**: the scene
//! layer's `Topology` has `Triangles` and `Segments` and nothing else, so the grid is triangulated by
//! the walk that produced it and not by its consumer.
//!
//! # The layout a consumer indexes by
//!
//! `positions.len() == rings * around` and the vertices are **ring-major**: the vertex at ring `r`
//! and slot `j` is `r * around + j`, which is [`Loft::vertex`]. The triangles are wound
//! counter-clockwise as seen from outside the swept solid, which is what makes
//! [`crate::mesh::signed_volume`] positive on a closed sweep.

use crate::math::{Vec2, Vec3};

/// A band of rings, triangulated, with the parameter of every vertex.
#[derive(Debug, Clone, PartialEq)]
pub struct Loft {
    positions: Vec<Vec3>,
    uvs: Vec<Vec2>,
    triangles: Vec<[u32; 3]>,
    rings: usize,
    around: usize,
}

impl Loft {
    pub(crate) fn new(
        positions: Vec<Vec3>,
        uvs: Vec<Vec2>,
        triangles: Vec<[u32; 3]>,
        rings: usize,
        around: usize,
    ) -> Self {
        debug_assert_eq!(positions.len(), rings * around);
        debug_assert_eq!(uvs.len(), positions.len());
        Self {
            positions,
            uvs,
            triangles,
            rings,
            around,
        }
    }

    /// The positions, ring-major.
    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }

    /// The `(u, v)` of each position, in the same order.
    pub fn uvs(&self) -> &[Vec2] {
        &self.uvs
    }

    /// The triangles, wound counter-clockwise seen from outside the solid.
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }

    /// How many rings this loft has.
    pub fn rings(&self) -> usize {
        self.rings
    }

    /// How many vertices each ring has.
    pub fn around(&self) -> usize {
        self.around
    }

    /// The position and the `(u, v)` of the vertex at a ring and a slot in it.
    pub fn vertex(&self, ring: usize, slot: usize) -> (Vec3, Vec2) {
        assert!(
            ring < self.rings && slot < self.around,
            "a loft's vertex is inside its grid"
        );
        let index = ring * self.around + slot;
        (self.positions[index], self.uvs[index])
    }

    /// Every vertex with its `(u, v)`.
    pub fn vertices(&self) -> impl Iterator<Item = (Vec3, Vec2)> + '_ {
        self.positions.iter().copied().zip(self.uvs.iter().copied())
    }

    /// The loft read as a triangle list, for the predicates in [`crate::mesh`].
    pub fn view(&self) -> crate::mesh::Triangles<'_> {
        crate::mesh::Triangles {
            positions: &self.positions,
            corners: &self.triangles,
        }
    }
}
