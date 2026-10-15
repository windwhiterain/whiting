//! The massif as Hermite surfaces: **one surface per authored block**, creased where blocks meet.
//!
//! # Why per block and not per quad
//!
//! [`crate::hermite_surface::HermiteSurface`] needs a quad mesh laid out like a grid: every shared
//! edge must be the *same kind* of edge on both its faces, or the two faces disagree about which way
//! `u` runs and a single per-vertex derivative field cannot serve both. Measured on the massif's own
//! Doo-Sabin mesh, `examples/hermite_massif_probe.rs`, **13746 of 13834 shared edges fail that test** —
//! so the massif's current front end cannot be given to a Hermite surface at all, and the reason is
//! that a Doo-Sabin mesh has no consistent `(u, v)` frame.
//!
//! `mountain` authors **blocks**, not that. `BlockFace` carries the block's own two runs — crest and
//! valley floor — and `Face::uv` carries each triangle's place in the block's own belt
//! parameterization: `u` along the crest by distance, `v` down the flank. That is a **structured
//! domain**, and a regular grid over it is a grid.
//!
//! # What this is and is not
//!
//! **It is the crease model `mountain` already asks for**, and that is worth quoting rather than
//! asserting: `mountain::facet::Face::block`'s documentation says *"A block boundary is a crease. Every
//! vertex a block shares with another block is still one vertex of the mesh, but its normal is read per
//! block … the ribs and the crest lines are the hard edges of the rock."*
//!
//! So each block is one surface, smooth **inside itself**, and the boundary between two of them is a
//! crease — which here is structural rather than tagged, because they are two surfaces that happen to
//! agree on the vertices they share. No edge is C0 by accident and none is C1 by accident.
//!
//! **It is not a fit.** Nothing here interpolates a target shape: the grid samples the block's own
//! triangles, so the surface passes through the rock the expander actually built, and how far off the
//! mesh it runs is measured by `examples/block_surface_probe.rs` rather than argued here.

use mountain::facet::{BlockFace, Face, FacetMesh};
use patch::topology::VertexId;
use patch::{HermiteSurface, Point3};

/// How many samples a side of a block's grid carries.
///
/// **Fixed, and the number is a measured trade rather than a preference.** A block's own surface is
/// walked at a resolution the tessellation chose; sampling it on a coarser grid loses the walk's
/// detail and a finer one costs patches to carry detail the block does not have. `side` is exposed so
/// a caller can sweep it — the probe does exactly that.
pub const SIDE: usize = 4;

/// One block's Hermite surface, and where it came from.
#[derive(Clone, Debug)]
pub struct BlockSurface {
    /// The surface. Every face is smooth to its own block's boundary.
    pub surface: HermiteSurface,
    /// Which of `FacetMesh::blocks` this came from.
    ///
    /// **Carried rather than implied by position**, because a block the sampler cannot cover is
    /// skipped and the list is therefore shorter than the massif's. A caller pairing a surface back
    /// to its rock by counting from zero gets the wrong block — which is a wrong answer that looks
    /// right, and the first version of the probe measured `2.5e+02` of disagreement for exactly that
    /// reason.
    pub source: u32,
}

/// Every block of a massif, as a Hermite surface.
#[derive(Clone, Debug)]
pub struct BlockSurfaces {
    blocks: Vec<BlockSurface>,
    side: usize,
}

impl BlockSurfaces {
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&BlockSurface> {
        self.blocks.get(index)
    }

    pub fn blocks(&self) -> &[BlockSurface] {
        &self.blocks
    }

    /// The grid resolution every block was built at.
    pub fn side(&self) -> usize {
        self.side
    }

    /// How many patches in total, which is the number the Doo-Sabin front end is measured against.
    pub fn patch_count(&self) -> usize {
        self.blocks
            .iter()
            .map(|block| block.surface.face_count())
            .sum()
    }
}

/// Builds one Hermite surface per authored block.
///
/// Returns only the blocks it could build, and **a block it cannot build is not an error here**: a
/// block whose own surface covers no area in its belt domain — a strip whose every row collapsed at
/// a summit — has nothing to sample and is skipped, which is the same rule `mountain`'s own expander
/// applies when it declines to make a block. The count of what was skipped is in
/// [`BlockSurfaces::len`] against the massif's own block count.
pub fn from_facet(facet: &FacetMesh, side: usize) -> BlockSurfaces {
    let mut blocks = Vec::with_capacity(facet.blocks.len());
    for (index, block) in facet.blocks.iter().enumerate() {
        if let Some(surface) = block_surface(facet, block, side, index as u32) {
            blocks.push(surface);
        }
    }
    BlockSurfaces { blocks, side }
}

/// One block's grid, or `None` when the block has no surface to sample at all.
fn block_surface(
    facet: &FacetMesh,
    block: &BlockFace,
    side: usize,
    source: u32,
) -> Option<BlockSurface> {
    let faces: Vec<&Face> = facet.faces[block.faces.start as usize..block.faces.end as usize]
        .iter()
        .collect();
    if faces.is_empty() {
        return None;
    }

    // The block's own domain. **Not the unit square**: `Face::uv`'s documentation records that 89 of
    // 182 blocks leave part of their own `uv` box uncovered, and sampling across a gap extrapolates
    // off the rock into a derivative field nothing measured.
    let (mut low_u, mut high_u) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut low_v, mut high_v) = (f32::INFINITY, f32::NEG_INFINITY);
    for face in &faces {
        for corner in face.uv {
            low_u = low_u.min(corner[0]);
            high_u = high_u.max(corner[0]);
            low_v = low_v.min(corner[1]);
            high_v = high_v.max(corner[1]);
        }
    }
    if high_u <= low_u || high_v <= low_v {
        return None;
    }

    let mut positions = Vec::with_capacity(side * side);
    for row in 0..side {
        let v = low_v + (high_v - low_v) * row as f32 / (side - 1) as f32;
        for column in 0..side {
            let u = low_u + (high_u - low_u) * column as f32 / (side - 1) as f32;
            positions.push(sample_block(facet, &faces, u, v));
        }
    }

    // The winding: `(0,0) (1,0) (1,1) (0,1)` per cell, which is the order `HermiteSurface` and every
    // quad in this crate uses.
    let mut quads = Vec::with_capacity((side - 1) * (side - 1));
    for row in 0..side - 1 {
        for column in 0..side - 1 {
            let near = (row * side + column) as VertexId;
            quads.push([
                near,
                near + 1,
                near + side as VertexId + 1,
                near + side as VertexId,
            ]);
        }
    }

    let surface = HermiteSurface::build(positions, quads).ok()?;
    Some(BlockSurface { surface, source })
}

/// The block's own surface at `(u, v)`, averaged over every triangle of the block that covers it.
///
/// **Averaged, not first-found**, because a point on a triangle boundary belongs to two of them and
/// picking one makes the sampled grid depend on triangle order.
///
/// **A point no triangle covers falls back to the nearest vertex of the block.** `Face::uv`'s own
/// documentation records that 89 of 182 blocks leave part of their `uv` box uncovered — a block's
/// belt domain is not always a rectangle — so a grid corner at the corner of that box can be off the
/// rock. Skipping the block for it would throw away half the massif; snapping to the nearest vertex
/// keeps the grid well formed and moves only the corner that was already off the surface. The count
/// of blocks that needed it is not reported here because the snapping is a deformation of the domain
/// rather than an error, and `examples/block_surface_probe.rs` measures the result against the rock.
fn sample_block(facet: &FacetMesh, faces: &[&Face], u: f32, v: f32) -> Point3 {
    let mut sum = [0.0f64; 3];
    let mut count = 0.0f64;
    for face in faces {
        let Some(weights) = barycentric(face.uv, [u, v]) else {
            continue;
        };
        for (index, share) in weights.into_iter().enumerate() {
            let point = facet.points[face.indices[index] as usize];
            sum[0] += point[0] as f64 * share;
            sum[1] += point[1] as f64 * share;
            sum[2] += point[2] as f64 * share;
        }
        count += 1.0;
    }
    if count > 0.0 {
        return Point3::new(sum[0] / count, sum[1] / count, sum[2] / count);
    }
    nearest_vertex(facet, faces, u, v)
}

/// The vertex of the block's own triangles whose parameter position is nearest `(u, v)`.
fn nearest_vertex(facet: &FacetMesh, faces: &[&Face], u: f32, v: f32) -> Point3 {
    let mut best = Point3::ZERO;
    let mut best_distance = f32::INFINITY;
    for face in faces {
        for (index, corner) in face.uv.iter().enumerate() {
            let distance = (corner[0] - u) * (corner[0] - u) + (corner[1] - v) * (corner[1] - v);
            if distance < best_distance {
                best_distance = distance;
                let point = facet.points[face.indices[index] as usize];
                best = Point3::new(point[0] as f64, point[1] as f64, point[2] as f64);
            }
        }
    }
    best
}

/// The barycentric weights of `point` over a triangle given in a parameter plane.
///
/// `None` outside it, and the boundary included — a point exactly on an edge is on the triangle and
/// excluding it would make every triangle boundary a hole in the sampled grid.
fn barycentric(corners: [[f32; 2]; 3], point: [f32; 2]) -> Option<[f64; 3]> {
    let [a, b, c] = corners;
    let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
    if area.abs() < f32::EPSILON {
        return None;
    }
    let first =
        ((b[0] - point[0]) * (c[1] - point[1]) - (c[0] - point[0]) * (b[1] - point[1])) / area;
    let second =
        ((c[0] - point[0]) * (a[1] - point[1]) - (a[0] - point[0]) * (c[1] - point[1])) / area;
    let (first, second) = (first as f64, second as f64);
    let third = 1.0 - first - second;
    const EPSILON: f64 = -1.0e-9;
    if first < EPSILON || second < EPSILON || third < EPSILON {
        return None;
    }
    Some([first, second, third])
}
