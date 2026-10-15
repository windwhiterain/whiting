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
pub fn from_belts(facet: &FacetMesh, side: usize) -> Option<BeltSurfaces> {
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

    // **Every belt's grid goes into one surface value**, and the belts meet along their valley
    // rows at vertices that are separate but coincident: each belt carries its own copy of each
    // station of the shared run, and the copies agree exactly because both are the same arclength
    // resample of the same run. The surface is therefore a set of components that touch, which is
    // what a creased massif is — a valley is a crease because two flanks leave it in different
    // directions, not because an edge was tagged — and it is why no `Join` is set here any more:
    // a shared vertex would have forced one `dv` to serve both flanks, and the averaged
    // cross-boundary derivative pinched every seam cell into the dark diamonds this arrangement
    // removes. See [`grid_points`].
    let mut positions: Vec<Point3> = Vec::new();
    let mut quads: Vec<[VertexId; 4]> = Vec::new();
    let mut belts = Vec::with_capacity(order.len());

    for belt in order.iter() {
        let Some(faces) = grouped.get(belt) else {
            continue;
        };
        let Some(grid) = grid_points(facet, faces, side, &mut positions) else {
            continue;
        };
        let first_face = quads.len() as u32;
        push_quads(&positions, &grid, side, &mut quads);
        belts.push(BeltRecord {
            belt: *belt,
            first_face,
            face_count: quads.len() as u32 - first_face,
        });
    }

    let Ok(surface) = HermiteSurface::build(positions, quads) else {
        return None;
    };

    Some(BeltSurfaces {
        surface,
        belts,
        side,
        blocks: facet.blocks.len(),
    })
}

/// A belt's place in the one surface, so a caller can reach its patches.
#[derive(Clone, Copy, Debug)]
pub struct BeltRecord {
    pub belt: BeltRef,
    pub first_face: u32,
    pub face_count: u32,
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

/// Every belt of a massif, as **one** Hermite surface value.
///
/// The belts are components of it that touch along coincident valley vertices rather than cells
/// sharing edges, so each carries its own derivative field and the valleys stay creases. See
/// [`from_belts`].
#[derive(Clone, Debug)]
pub struct BeltSurfaces {
    surface: HermiteSurface,
    belts: Vec<BeltRecord>,
    side: usize,
    blocks: usize,
}

impl BeltSurfaces {
    /// The whole massif, as one surface whose belts touch at coincident valley vertices.
    pub fn surface(&self) -> &HermiteSurface {
        &self.surface
    }

    pub fn surface_mut(&mut self) -> &mut HermiteSurface {
        &mut self.surface
    }

    /// Where each belt sits in the surface.
    pub fn belts(&self) -> &[BeltRecord] {
        &self.belts
    }

    /// How many belts the massif has.
    pub fn len(&self) -> usize {
        self.belts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.belts.is_empty()
    }

    /// The grid resolution every belt was built at.
    pub fn side(&self) -> usize {
        self.side
    }

    /// How many patches in total.
    pub fn patch_count(&self) -> usize {
        self.surface.face_count()
    }

    /// How many blocks the massif has, so a caller can say what a belt absorbed.
    pub fn blocks(&self) -> usize {
        self.blocks
    }

    /// The belt a face belongs to.
    pub fn belt_of_face(&self, face: u32) -> Option<BeltRef> {
        self.belts
            .iter()
            .find(|belt| face >= belt.first_face && face < belt.first_face + belt.face_count)
            .map(|belt| belt.belt)
    }
}

/// The mesh vertices on a belt's **valley** row, in order, deduped.
///
/// `Face::uv` puts `v = 1` on the valley, so these are the vertices of the faces whose `uv` reaches
/// the top of the belt's own `v` range. `expand.rs`'s `valley_run` hands a valley's Left and Right
/// belts the **same** run, so both belts resample the same stations — which is what makes their
/// duplicated valley rows coincident point for point.
fn valley_vertices(faces: &[&Face]) -> Vec<u32> {
    let mut high = f32::NEG_INFINITY;
    for face in faces {
        for corner in face.uv {
            high = high.max(corner[1]);
        }
    }
    let mut found: BTreeMap<u32, f32> = BTreeMap::new();
    for face in faces {
        for (index, corner) in face.uv.iter().enumerate() {
            if (corner[1] - high).abs() > 1.0e-5 {
                continue;
            }
            let vertex = face.indices[index];
            let slot = found.entry(vertex).or_insert(f32::INFINITY);
            *slot = slot.min(corner[0]);
        }
    }
    let mut ordered: Vec<(f32, u32)> = found.into_iter().map(|(id, u)| (u, id)).collect();
    ordered.sort_by(|left, right| left.0.total_cmp(&right.0));
    ordered.into_iter().map(|(_, id)| id).collect()
}

/// A belt's grid of vertex indices into `positions`, with its valley row shared with its partner.
fn grid_points(
    facet: &FacetMesh,
    faces: &[&Face],
    side: usize,
    positions: &mut Vec<Point3>,
) -> Option<Vec<VertexId>> {
    if faces.is_empty() || side < 2 {
        return None;
    }
    // A belt whose faces cover no area in their own parameter plane has no surface to sample.
    covered_domain(faces)?;

    // The valley row, resampled off the run this belt's own faces name. A valley's Left and Right
    // belts carry the same run, so resampling it by arc length gives both of them the same
    // stations — the same points, in opposite orders — which is what makes their duplicated valley
    // rows coincident. Sampling at uniform `u` instead — which is what each belt would do on its
    // own — parameterises the two from different upper runs and lands them `2.412e-02` apart on a
    // curved valley: not a crack, and not C0 either.
    let run = valley_vertices(faces);
    let run = resample_by_arclength(facet, &run, side);

    // **The grid's columns and rows are the covered region's own, not its bounding box's.**
    //
    // A belt's `uv` runs over a rectangle by construction but the block inside it need not fill
    // that rectangle, and a uniform grid over the box therefore lands corners where no triangle
    // is. Every one of those corners is answered by [`sample_block`] projecting onto the nearest
    // covered point, and a cell with an invented corner **folds**: measured on the massif, 2554 of
    // the 29484 drawn triangles were shaded by a normal a whole pi out, which is a fold and not a
    // shading fault — an emissive backdrop behind the massif showed every dark patch stayed dark,
    // so they are not holes either. They are surface.
    //
    // **By rank rather than by value.** Taking the quantiles of the covered region's own `u` and
    // `v` puts each column and row where the rock actually is, so the grid needs no invented corner
    // at all. Dropping the cells that had one was measured and it works — the folds become holes,
    // which is the same picture with the same cost of 17.6% of the surface — so this is the way to
    // have the folds not exist rather than to remove them afterwards.
    let Some((columns, rows)) = covered_quantiles(faces, side - 1) else {
        return None;
    };

    let mut grid = Vec::with_capacity(side * side);
    for row in 0..side {
        for column in 0..side {
            if row == side - 1 {
                // **The valley row is duplicated per belt rather than shared by vertex id**
                // (2026-10-08; it used to be one vertex keyed by the run and the place along it).
                // Both belts resample the same run at the same arc-length stations, so the two
                // copies of each station are the same point to the last bit — the crack the old
                // uniform-`u` sampling measured at `2.412e-02` is what the arclength resample
                // already removed, not what the sharing removed.
                //
                // What sharing did cost is the reason it is gone: a Hermite surface holds one
                // `dv` per vertex, and at a shared valley vertex that one vector has to serve two
                // flanks that leave the valley in different directions, so it serves neither —
                // the averaged cross-boundary derivative pinched the patches of both belts at
                // every seam cell, which is the row of dark diamonds down every valley in the
                // render. Separate vertices at coincident positions keep the surface C0 exactly,
                // keep the valley a crease, and let each belt carry its own derivative field.
                let id = positions.len() as VertexId;
                positions.push(run[column]);
                grid.push(id);
                continue;
            }
            let v = rows[row];
            let u = columns[column];
            let id = positions.len() as VertexId;
            positions.push(sample_block(facet, faces, u, v)?);
            grid.push(id);
        }
    }
    Some(grid)
}

/// `side` points along a run of mesh vertices, spaced by arc length.
/// `side` points along a run of mesh vertices, spaced by arc length.
///
/// # Invariants
///
/// - **`points[0]` and `points[len-1]` are returned exactly.** The two belts that share this run
///   have to agree on where it begins and ends, and a rounded interior point that drifted past
///   either end would put their shared edge somewhere neither of them has rock for.
/// - **The result is inside the run.** Every point is on the polyline itself, never extrapolated
///   past either end.
///
/// The first version located the segment with `starts.iter().position(|start| *start + 1.0 < target)`
/// — a unit-less epsilon compared against an arc length — and fell through to the **last** segment
/// for any target below the second segment's start. It therefore answered `points[last + 1]` for
/// the run's whole first stretch, putting every early sample on the run's far end. Several seam
/// points landed on one place, `push_quads` dropped the cells whose corners were not distinct, and
/// the massif rendered with holes through it. The `+ 1.0` was the bug and it is gone rather than
/// tuned: the comparison that selects a segment is against the segment's own length, so it cannot
/// be right by accident on a long run and wrong on a short one.
fn resample_by_arclength(facet: &FacetMesh, run: &[u32], side: usize) -> Vec<Point3> {
    let points: Vec<Point3> = run
        .iter()
        .map(|index| {
            let point = facet.points[*index as usize];
            Point3::new(point[0] as f64, point[1] as f64, point[2] as f64)
        })
        .collect();
    if points.is_empty() || side == 0 {
        return Vec::new();
    }
    if points.len() == 1 {
        return vec![points[0]; side];
    }

    // The length of each segment, in the order they appear, and their total.
    let spans: Vec<f64> = points
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .collect();
    let total: f64 = spans.iter().sum();
    if total <= 0.0 {
        // A run whose every vertex is in the same place has no length to divide.
        return vec![points[0]; side];
    }

    (0..side)
        .map(|step| {
            let target = total * step as f64 / (side - 1).max(1) as f64;
            // Walk the segments rather than searching for one: the run is short and this is linear
            // in a number that is already small, and a search here is where the first version went
            // wrong.
            let mut walked = 0.0f64;
            let mut at = spans.len() - 1;
            for (index, span) in spans.iter().enumerate() {
                if target < walked + *span {
                    at = index;
                    break;
                }
                walked += *span;
            }
            let span = spans[at].max(f64::MIN_POSITIVE);
            let share = ((target - walked) / span).clamp(0.0, 1.0);
            points[at] + (points[at + 1] - points[at]) * share
        })
        .collect()
}

/// Appends a belt's cells, skipping any whose corners collapse onto one place.
fn push_quads(
    positions: &[Point3],
    grid: &[VertexId],
    side: usize,
    quads: &mut Vec<[VertexId; 4]>,
) {
    for row in 0..side - 1 {
        for column in 0..side - 1 {
            let near = (row * side + column) as usize;
            let corners = [
                grid[near],
                grid[near + 1],
                grid[near + side + 1],
                grid[near + side],
            ];
            // **A cell is kept when its four corners are four distinct places.**
            //
            // A size test used to stand beside this one — a cell whose shortest edge was under
            // `SLIVER` of its longest was dropped as undrawable — and it was the dark diamonds in
            // the render, twice over: every dropped cell is a hole in the surface, and a hole
            // shows the lit inside of the rock behind it, which reads as a black or dark-grey
            // diamond exactly the size of one cell. The folds the test was chasing were never the
            // thin quad's; they were the tangent seed's (see [`patch::HermiteSurface`]), and with
            // a seed scaled to its chord a thin quad is a thin patch and draws as one. What
            // genuinely cannot be drawn here is a quad with two corners in one place, which is a
            // triangle, and this surface type has no triangle: that alone is dropped.
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
    let Some((low_u, high_u, low_v, high_v)) = covered_domain(&faces) else {
        return None;
    };
    let mut positions = Vec::with_capacity(side * side);
    // A block has no partner to share a seam with, so every point of its grid is sampled.
    for row in 0..side {
        let v = low_v + (high_v - low_v) * row as f32 / (side - 1) as f32;
        for column in 0..side {
            let u = low_u + (high_u - low_u) * column as f32 / (side - 1) as f32;
            positions.push(sample_block(facet, &faces, u, v)?);
        }
    }
    let grid: Vec<VertexId> = (0..positions.len() as VertexId).collect();
    let surface = surface_from_grid(&positions, &grid, side)?;
    Some(BlockSurface { surface, source })
}

/// The `uv` box a set of faces covers, or `None` when it covers no area.
///
/// **Not the unit square**, because a caller may hand this a set whose `uv` runs over a sub-range.
fn covered_domain(faces: &[&Face]) -> Option<(f32, f32, f32, f32)> {
    if faces.is_empty() {
        return None;
    }
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
    Some((low_u, high_u, low_v, high_v))
}

/// Builds a surface from a grid of vertex indices, dropping any cell whose corners collapse.
fn surface_from_grid(
    positions: &[Point3],
    grid: &[VertexId],
    side: usize,
) -> Option<HermiteSurface> {
    let mut quads = Vec::with_capacity((side - 1) * (side - 1));
    push_quads(positions, grid, side, &mut quads);
    if quads.is_empty() {
        return None;
    }
    HermiteSurface::build(positions.to_vec(), quads).ok()
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

/// `count` column positions and `count` row positions, taken as quantiles of the **covered**
/// region's own `u` and `v` rather than as fractions of its bounding box.
///
/// # Invariants
///
/// - **Every position returned is one a triangle of this set covers**, or as near as the sampling
///   grid below resolves. A position off the rock is answered by a projection, and a cell built
///   from a projected corner folds — measured on the massif as 2554 of 29484 drawn triangles shaded
///   by a normal a whole pi out.
/// - **The first and last are the region's own ends**, so the grid still reaches the rock's edges
///   rather than stopping short of them.
///
/// The probe grid is coarse on purpose: it is a measure of *where the rock is*, and a finer one
/// costs more without placing a column better. It is `PROBE` squares a side regardless of how many
/// triangles the set has, so the cost is bounded and the placement does not depend on the
/// tessellation.
fn covered_quantiles(faces: &[&Face], count: usize) -> Option<(Vec<f32>, Vec<f32>)> {
    if faces.is_empty() || count == 0 {
        return None;
    }
    let (low_u, high_u, low_v, high_v) = covered_domain(faces)?;
    const PROBE: usize = 64;
    let mut us: Vec<f32> = Vec::new();
    let mut vs: Vec<f32> = Vec::new();
    for row in 0..PROBE {
        let v = low_v + (high_v - low_v) * row as f32 / (PROBE - 1) as f32;
        for column in 0..PROBE {
            let u = low_u + (high_u - low_u) * column as f32 / (PROBE - 1) as f32;
            if covers(faces, u, v) {
                us.push(u);
                vs.push(v);
            }
        }
    }
    if us.is_empty() {
        return None;
    }
    us.sort_by(f32::total_cmp);
    vs.sort_by(f32::total_cmp);
    Some((quantiles(&us, count), quantiles(&vs, count)))
}

/// `count` positions spread evenly by rank over a sorted run, first and last included.
fn quantiles(sorted: &[f32], count: usize) -> Vec<f32> {
    let last = sorted.len() - 1;
    (0..=count)
        .map(|step| sorted[(last as f64 * step as f64 / count as f64).round() as usize])
        .collect()
}

/// Whether any triangle of the set covers the parameter point.
fn covers(faces: &[&Face], u: f32, v: f32) -> bool {
    faces
        .iter()
        .any(|face| barycentric(face.uv, [u, v]).is_some())
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
