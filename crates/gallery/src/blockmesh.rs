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

use std::collections::BTreeMap;

use mountain::facet::{BlockFace, Face, FacetMesh};
use mountain::tessellation::BeltRef;
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

/// Builds one Hermite surface per **belt** rather than per block.
///
/// **The same construction over a different set of faces**, and the difference is what the belt's
/// domain buys. A block's faces carry `(u, v)` local to that block, so a block's covered region is a
/// ragged subset of its own bounding box — 266 of the grid points this crate lays at `side` 4 fall
/// outside their block's triangles. **A belt's faces carry `(u, v)` over the whole belt**, so the
/// region is a rectangle by construction and the grid lands on the rock rather than beside it.
///
/// That is the whole argument for the belt, and it is the same argument `Face::uv` makes about
/// itself: `u` is "the fraction of the belt's upper boundary already walked", not "the fraction of
/// this block's".
///
/// Blocks within a belt become cells of one surface, so **the ribs between them stop being creases**
/// — see the note on [`BlockSurfaces`] for whether that is a loss.
pub fn from_belts(facet: &FacetMesh, side: usize) -> BeltSurfaces {
    // Group by belt, in the order the facets name them, so the result does not depend on a hash map's
    // iteration order.
    let mut order: Vec<BeltRef> = Vec::new();
    let mut grouped: BTreeMap<BeltRef, Vec<&Face>> = BTreeMap::new();
    for face in &facet.faces {
        if !order.contains(&face.attributes.belt) {
            order.push(face.attributes.belt);
        }
        grouped.entry(face.attributes.belt).or_default().push(face);
    }
    order.sort_by_key(|belt| (belt.valley.0, belt.segment, belt.side));

    let mut belts = Vec::with_capacity(order.len());
    for (index, belt) in order.iter().enumerate() {
        let Some(faces) = grouped.get(belt) else {
            continue;
        };
        let Some(surface) = surface_from_faces(facet, faces, side) else {
            continue;
        };
        belts.push(BeltSurface {
            surface,
            source: index as u32,
            belt: *belt,
        });
    }
    BeltSurfaces {
        belts,
        side,
        blocks: facet.blocks.len(),
    }
}

/// One belt's Hermite surface, and which belt it is.
#[derive(Clone, Debug)]
pub struct BeltSurface {
    /// The surface, smooth everywhere inside the belt.
    pub surface: HermiteSurface,
    /// Which belt of [`BeltSurfaces::belts`] this is, by list position.
    pub source: u32,
    /// The massif's own name for the belt, so a caller can find it again after a round trip.
    pub belt: BeltRef,
}

/// Every belt of a massif, as a Hermite surface.
#[derive(Clone, Debug)]
pub struct BeltSurfaces {
    belts: Vec<BeltSurface>,
    side: usize,
    blocks: usize,
}

impl BeltSurfaces {
    pub fn len(&self) -> usize {
        self.belts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.belts.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&BeltSurface> {
        self.belts.get(index)
    }

    pub fn belts(&self) -> &[BeltSurface] {
        &self.belts
    }

    /// The grid resolution every belt was built at.
    pub fn side(&self) -> usize {
        self.side
    }

    /// How many patches in total.
    pub fn patch_count(&self) -> usize {
        self.belts
            .iter()
            .map(|belt| belt.surface.face_count())
            .sum()
    }

    /// How many blocks the massif has, so a caller can say what a belt absorbed.
    pub fn blocks(&self) -> usize {
        self.blocks
    }
}

/// One grid over any set of faces that share a domain, or `None` when they cover no area.
fn surface_from_faces(facet: &FacetMesh, faces: &[&Face], side: usize) -> Option<HermiteSurface> {
    if faces.is_empty() || side < 2 {
        return None;
    }

    // The covered domain. **Not the unit square**, because a caller may hand this a set whose `uv`
    // runs over a sub-range; the unit square would then put the whole grid off the rock.
    let (mut low_u, mut high_u) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut low_v, mut high_v) = (f32::INFINITY, f32::NEG_INFINITY);
    for face in faces {
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
            positions.push(sample_block(facet, faces, u, v)?);
        }
    }

    // The winding: `(0,0) (1,0) (1,1) (0,1)` per cell, which is the order `HermiteSurface` and every
    // quad in this crate uses.
    //
    // **A cell whose corners are not four distinct points is left out, and only that cell.**
    // Projection can put two grid points on the same place — a corner that fell outside the region
    // is sent to the nearest point of it, and near a cusp or a thin arm two corners can be sent to
    // the same one — and `HermiteSurface::build` refuses a quad with a repeated corner, which would
    // take the **whole block** with it. Measured, keeping every quad cost 87 of the massif's 182
    // blocks at `side` 4 and 181 of them at `side` 8, so a front end that fixed its corners and lost
    // four fifths of the rock was not a fix. Leaving the cell out keeps the block.
    let mut quads = Vec::with_capacity((side - 1) * (side - 1));
    for row in 0..side - 1 {
        for column in 0..side - 1 {
            let near = (row * side + column) as VertexId;
            let corners = [
                near,
                near + 1,
                near + side as VertexId + 1,
                near + side as VertexId,
            ];
            let distinct = corners.iter().all(|first| {
                corners.iter().all(|second| {
                    first == second
                        || positions[*first as usize].distance(positions[*second as usize]) > 0.0
                })
            });
            if distinct {
                quads.push(corners);
            }
        }
    }
    if quads.is_empty() {
        return None;
    }

    HermiteSurface::build(positions, quads).ok()
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
    let surface = surface_from_faces(facet, &faces, side)?;
    Some(BlockSurface { surface, source })
}

/// The block's own surface at `(u, v)`, averaged over every triangle of the block that covers it.
///
/// **Averaged, not first-found**, because a point on a triangle boundary belongs to two of them and
/// picking one makes the sampled grid depend on triangle order.
///
/// **A parameter point no triangle covers is projected onto the region the block does cover**
/// rather than snapped to a vertex. `Face::uv`'s own documentation makes `u` the fraction of the
/// *belt's* upper boundary walked, so a block's triangles cover a ragged subset of their own
/// bounding box and a uniform grid over that box puts corners off the rock at every resolution
/// above the coarsest. What used to happen there was [`nearest_vertex`]: the corner was replaced by
/// one of the block's mesh **vertices**, chosen for being near in parameter space and therefore
/// arbitrarily far in space, which drags a whole cell with it. Measured on the massif, cells in
/// blocks that had something snapped departed `3.6e-1` at the median against `1.2e-1` for the
/// blocks that had nothing snapped, and the worst went from `1.6e0` to `9.7e0`.
///
/// The replacement moves the corner to the **nearest point of the covered region**, so the corner
/// still lies on the rock — the distance it moves is bounded by how far outside the region the box
/// reaches, and no vertex is invented. Every block therefore still builds and the grid still covers
/// the block, which dropping the uncovered cells would not: a cell with one corner off the rock is
/// missing, and the missing cells are the rock's border.
fn sample_block(facet: &FacetMesh, faces: &[&Face], u: f32, v: f32) -> Option<Point3> {
    if let Some(point) = covered(facet, faces, u, v) {
        return Some(point);
    }
    let [u, v] = nearest_covered(faces, u, v);
    // **`None` rather than a position invented here.** `nearest_covered` answers a point lying on a
    // triangle of this block, so the second `covered` succeeds whenever the block has one triangle
    // with an area in its parameter plane. A block that does not is one whose every face is
    // degenerate, and for it there is no rock to sample: the old code answered such a block with
    // the first vertex it walked, which is a position no triangle of the block's own parameter
    // plane has any claim to.
    covered(facet, faces, u, v)
}

/// The block's own surface at `(u, v)`, or `None` when no triangle of the block covers it.
fn covered(facet: &FacetMesh, faces: &[&Face], u: f32, v: f32) -> Option<Point3> {
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
    if count == 0.0 {
        return None;
    }
    Some(Point3::new(sum[0] / count, sum[1] / count, sum[2] / count))
}

/// The point of the block's covered region nearest to `(u, v)`, in the parameter plane.
///
/// **A search over triangles, not over vertices**, because the region is what the grid has to land
/// on and its boundary is made of triangle edges. The point found is then moved a little way **into**
/// the triangle it belongs to, because the nearest point on a triangle is usually *on* one of its
/// edges and a point placed on an edge by arithmetic in `f32` lands just outside it — far enough to
/// fail [`barycentric`]'s `-1e-9` tolerance, which reads as "no triangle covers this" and takes the
/// block with it. Inside by a thousandth of the way to the centroid is past any rounding and still on
/// the rock.
fn nearest_covered(faces: &[&Face], u: f32, v: f32) -> [f32; 2] {
    const INSIDE: f32 = 1.0e-3;
    let mut best = [u, v];
    let mut inside = [u, v];
    let mut best_distance = f64::INFINITY;
    for face in faces {
        let Some(point) = closest_on_triangle(face.uv, [u, v]) else {
            continue;
        };
        let distance = f64::from(point[0] - u) * f64::from(point[0] - u)
            + f64::from(point[1] - v) * f64::from(point[1] - v);
        if distance < best_distance {
            best_distance = distance;
            best = point;
            inside = [
                (face.uv[0][0] + face.uv[1][0] + face.uv[2][0]) / 3.0,
                (face.uv[0][1] + face.uv[1][1] + face.uv[2][1]) / 3.0,
            ];
        }
    }
    [
        best[0] + (inside[0] - best[0]) * INSIDE,
        best[1] + (inside[1] - best[1]) * INSIDE,
    ]
}

/// The point of a triangle nearest to `point`, or `None` for a degenerate one.
///
/// The standard region test: inside, else the nearest of the three edges, else the nearest vertex —
/// which is what a triangle's own boundary is made of, so a caller that gets a point back always
/// gets one the triangle covers.
fn closest_on_triangle(corners: [[f32; 2]; 3], point: [f32; 2]) -> Option<[f32; 2]> {
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
    if first >= EPSILON && second >= EPSILON && third >= EPSILON {
        return Some(point);
    }
    let mut best = a;
    let mut best_distance = f64::INFINITY;
    for (from, to) in [(a, b), (b, c), (c, a)] {
        let Some(on) = closest_on_segment(from, to, point) else {
            continue;
        };
        let distance = f64::from(on[0] - point[0]) * f64::from(on[0] - point[0])
            + f64::from(on[1] - point[1]) * f64::from(on[1] - point[1]);
        if distance < best_distance {
            best_distance = distance;
            best = on;
        }
    }
    Some(best)
}

/// The point of the segment `from`–`to` nearest to `point`, or `None` when the segment is a point.
fn closest_on_segment(from: [f32; 2], to: [f32; 2], point: [f32; 2]) -> Option<[f32; 2]> {
    let along = [to[0] - from[0], to[1] - from[1]];
    let length = along[0] * along[0] + along[1] * along[1];
    if length <= f32::EPSILON {
        return Some(from);
    }
    let share = ((point[0] - from[0]) * along[0] + (point[1] - from[1]) * along[1]) / length;
    let share = share.clamp(0.0, 1.0);
    Some([from[0] + share * along[0], from[1] + share * along[1]])
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
