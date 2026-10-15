//! Reading a [`FacetMesh`] onto a lattice: the conversion from an arbitrary footprint to a
//! rectangle, done by exact lookup rather than by approximation.
//!
//! # Invariants
//!
//! - **The lookup is exact, because the mesh is.** Every face of a [`FacetMesh`] is planar
//!   ([`mountain::facet`]), so the elevation over a face is the barycentric interpolation of
//!   its three corners. Nothing is resampled, averaged or smoothed, and a cell that lands
//!   inside a face reads that face and not a neighbour of it.
//! - **A cell is inside the massif exactly when a face covers its centre.** There is no
//!   separate point-in-footprint test to disagree with the mesh: the massif *is* its faces,
//!   so the mask is read off the same surface the mesh draws rather than a second opinion
//!   about where it ends.
//! - **The first face found wins, and faces are visited in mesh order.** The mesh covers its
//!   footprint exactly once, so two faces can only both answer for a point on the edge they
//!   share — and along a shared edge the two interpolate identically, because that edge's two
//!   corners are the same two points. The order is fixed anyway, because a lattice that
//!   changed its answer when a face was merely re-indexed would be a lattice an agent could
//!   not diff.
//!
//! # The index, and why there is one
//!
//! Asking every cell about every face is `cells × faces`, which at a 512² lattice over a
//! three thousand face massif is eight hundred million point-in-triangle tests — minutes for
//! a conversion whose arithmetic is otherwise a multiply and an add. The faces are therefore
//! bucketed once over the plan, each face going into every bucket its plan bounding box
//! touches, and a cell reads only its own bucket. The build is linear in the faces and the
//! query is a short scan of one bucket, so the conversion is linear in both.

use mountain::math::{Vec2, signed_area};
use mountain::{Face, FacetMesh};

use crate::heightfield::{FieldError, HeightField};

/// How loosely a point may fall outside a face and still read it.
///
/// # Invariants
///
/// It exists because a cell centre landing exactly on an edge, a crest or a rim is a
/// measure-zero event that arithmetic reaches anyway: the three subtriangle areas then sum to
/// the whole one with rounding, and a strict test would drop a cell that is genuinely on the
/// face. It is a fraction of the face's own area rather than a distance, so it is the same
/// slack on a two unit facet and on a two hundred unit one.
const CONTAINMENT_SLACK: f32 = 1.0e-6;

/// Reads a mesh onto a lattice whose cells are `cell` plan units square.
///
/// # Invariants
///
/// - **The lattice is square and centred on the mesh's own plan bounds**, and its spacing is
///   the one asked for rounded *up* to whatever covers both extents. A lattice narrower than
///   the mesh would leave the outermost faces off it, and a massif whose rim is off the
///   lattice is a massif with a straight cut along one side of its own picture.
/// - **`base` is the lowest elevation under any cell**, so a caller drawing the lattice gets
///   ground continuing at the foot of the massif rather than at a number it invented.
/// - **The provenance is read in the same pass as the height**, from the same containment test
///   on the same point. A caller asking which face a cell stands on is therefore answered by
///   the face the height came from, and cannot be handed a second answer found by looking
///   again — which for a boundary cell is a different face.
pub fn rasterize(mesh: &FacetMesh, cell: f32) -> Result<Rasterized, Rasterize> {
    if cell <= 0.0 || cell.is_nan() {
        return Err(Rasterize::Field(FieldError::NotASpacing { cell }));
    }
    let (low, high) = plan_bounds(mesh);
    let extent = [high[0] - low[0], high[1] - low[1]];
    // One square spacing covering both axes: the wider of the two steps each axis would
    // need, which is the only value at which neither axis is left short.
    let columns = ((extent[0] / cell).ceil() as usize).max(1);
    let rows = ((extent[1] / cell).ceil() as usize).max(1);
    let step = (extent[0] / columns as f32).max(extent[1] / rows as f32);
    let origin = [
        (low[0] + high[0]) * 0.5 - (columns as f32 - 1.0) * 0.5 * step,
        (low[1] + high[1]) * 0.5 - (rows as f32 - 1.0) * 0.5 * step,
    ];

    let index = BucketIndex::of(mesh, low, extent);
    let mut heights = vec![0.0; columns * rows];
    let mut inside = vec![false; columns * rows];
    let mut provenance = Provenance {
        faces: vec![NO_FACE; columns * rows],
        weights: vec![[0.0; 3]; columns * rows],
    };
    let mut lowest = f32::INFINITY;
    for row in 0..rows {
        for column in 0..columns {
            let plan = [
                origin[0] + column as f32 * step,
                origin[1] + row as f32 * step,
            ];
            let at = row * columns + column;
            if let Some((face, weights)) = index.lookup(mesh, plan) {
                heights[at] = weights
                    .into_iter()
                    .zip(corners_of(mesh, face))
                    .fold(0.0, |height, (weight, corner)| {
                        height + weight * mesh.points[corner as usize][1]
                    });
                inside[at] = true;
                provenance.faces[at] = face;
                provenance.weights[at] = weights;
                lowest = lowest.min(heights[at]);
            }
        }
    }
    let field = HeightField::of(
        origin,
        step,
        columns,
        rows,
        if lowest.is_finite() { lowest } else { 0.0 },
        heights,
        inside,
    )
    .map_err(Rasterize::Field)?;
    if field.covered() == 0 {
        return Err(Rasterize::NoRock {
            cells: columns * rows,
            cell,
        });
    }
    Ok(Rasterized { field, provenance })
}

/// Returns the three vertex indices of one face, in its own corner order.
fn corners_of(mesh: &FacetMesh, face: u32) -> [u32; 3] {
    mesh.faces[face as usize].indices
}

/// The face a cell with no face under it names.
pub const NO_FACE: u32 = u32::MAX;

/// Which face each cell was read from, and where in it.
///
/// # Invariants
///
/// - **One entry per cell, in lattice order**, the same order as the field's own buffers, so
///   an index that means a cell means the same cell in all three.
/// - **The weights are barycentric in the face's own plan triangle**, in the face's corner
///   order, so a caller interpolating anything the face carries at that corners — a texture
///   coordinate, a value per corner — does it with the same three numbers the height was
///   built from. Weights for [`NO_FACE`] are zero rather than left over, so a caller that
///   ignores the face index and reads them anyway gets a point at the first corner instead of
///   a NaN.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    /// The face each cell was read from, or [`NO_FACE`].
    pub faces: Vec<u32>,
    /// Each cell's barycentric coordinates in that face.
    pub weights: Vec<[f32; 3]>,
}

/// A mesh read onto a lattice: the field, and the mesh each cell of it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Rasterized {
    /// The elevations.
    pub field: HeightField,
    /// Which face each cell was read from.
    pub provenance: Provenance,
}

/// Why a mesh could not be read onto a lattice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rasterize {
    /// The lattice itself could not be built.
    Field(FieldError),
    /// No cell centre fell inside the mesh.
    ///
    /// # Invariants
    ///
    /// It is a **report about the spacing rather than about the mesh**, and it is checked for
    /// because the alternative is a field that is quietly empty: every cell reads as base,
    /// the solve erodes nothing, and the books balance perfectly on a landscape that was
    /// never sampled. A spacing coarser than the massif is the usual cause, and the one
    /// answer that fixes it.
    NoRock {
        /// How many cells the lattice had.
        cells: usize,
        /// The plan spacing that was asked for.
        cell: f32,
    },
}

impl std::fmt::Display for Rasterize {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rasterize::Field(error) => write!(formatter, "{error}"),
            Rasterize::NoRock { cells, cell } => write!(
                formatter,
                "no cell centre of a {cells} cell lattice at {cell} units fell inside the mesh"
            ),
        }
    }
}

impl std::error::Error for Rasterize {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Rasterize::Field(error) => Some(error),
            Rasterize::NoRock { .. } => None,
        }
    }
}

impl From<FieldError> for Rasterize {
    fn from(error: FieldError) -> Self {
        Rasterize::Field(error)
    }
}

/// Returns the plan bounding box of every vertex the mesh has.
fn plan_bounds(mesh: &FacetMesh) -> (Vec2, Vec2) {
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for point in &mesh.points {
        let point = mountain::math::plan(*point);
        for axis in 0..2 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    if low[0] > high[0] {
        return ([0.0, 0.0], [0.0, 0.0]);
    }
    (low, high)
}

/// The bucket span of one face over the plan, as half open column and row ranges.
type Span = (usize, usize, usize, usize);

/// The faces of a mesh, bucketed over the plan.
///
/// # Invariants
///
/// The buckets are a **counted run rather than a vector per bucket**, in two passes: count
/// what lands in each bucket, prefix-sum, fill. A vector per bucket is the shorter code and
/// the more expensive one — a 512² lattice is a quarter of a million heap headers for what
/// is usually a few faces apiece — and the run is one allocation for the whole index.
struct BucketIndex {
    /// The bucket lattice's own extent.
    columns: usize,
    rows: usize,
    /// The plan position of the first bucket's origin.
    origin: Vec2,
    /// The plan span of one bucket, on each axis.
    span: Vec2,
    /// Where each bucket's faces begin, one entry per bucket plus a closing entry.
    starts: Vec<u32>,
    /// The faces, grouped by bucket.
    faces: Vec<u32>,
}

impl BucketIndex {
    /// Buckets every face that covers plan area, over the mesh's own bounding box.
    ///
    /// The bucket lattice is sized to the face count rather than to the lattice being
    /// filled, because the two are independent: a fine lattice over a coarse mesh would
    /// otherwise pay for a quarter of a million buckets holding one face between them, and a
    /// coarse lattice over a fine mesh would put every face in one bucket and make the
    /// lookup a search again.
    fn of(mesh: &FacetMesh, low: Vec2, extent: Vec2) -> BucketIndex {
        // About two faces to a bucket, and a bucket lattice of the mesh's own aspect so the
        // buckets are square in the plan rather than stretched along the long axis.
        let aspect = if extent[1] > 0.0 {
            (extent[0] / extent[1]).max(1.0e-3)
        } else {
            1.0
        };
        let wanted = (mesh.faces.len() / 2).max(1) as f32;
        let columns = (wanted * aspect).sqrt().round().max(1.0) as usize;
        let rows = (wanted / aspect).sqrt().round().max(1.0) as usize;
        // A span of zero divides by nothing, which happens when the mesh's plan is a line —
        // a footprint collapsed onto a segment. The smallest normal float keeps the index
        // total without special-casing the query.
        let span = [
            (extent[0] / columns as f32).max(f32::MIN_POSITIVE),
            (extent[1] / rows as f32).max(f32::MIN_POSITIVE),
        ];

        let buckets = columns * rows;
        let spans: Vec<Option<Span>> = mesh
            .faces
            .iter()
            .map(|face| bucket_span(mesh, face, low, span, columns, rows))
            .collect();

        let mut counts = vec![0_u32; buckets + 1];
        for span in spans.iter().flatten() {
            for_each_bucket(*span, columns, |bucket| counts[bucket + 1] += 1);
        }
        let mut starts = counts;
        for bucket in 0..buckets {
            starts[bucket + 1] += starts[bucket];
        }
        let mut faces = vec![0_u32; starts[buckets] as usize];
        let mut cursor = starts.clone();
        for (face, span) in spans.iter().enumerate() {
            if let Some(span) = *span {
                for_each_bucket(span, columns, |bucket| {
                    faces[cursor[bucket] as usize] = face as u32;
                    cursor[bucket] += 1;
                });
            }
        }
        BucketIndex {
            columns,
            rows,
            origin: low,
            span,
            starts,
            faces,
        }
    }

    /// Returns the face covering a plan position, and the point's place in its plan triangle.
    fn lookup(&self, mesh: &FacetMesh, plan: Vec2) -> Option<(u32, [f32; 3])> {
        let bucket = self.bucket_of(plan);
        self.faces[self.starts[bucket] as usize..self.starts[bucket + 1] as usize]
            .iter()
            .find_map(|face| cover(mesh, *face, plan))
    }

    /// Returns which bucket a plan position falls in, clamped to the index.
    fn bucket_of(&self, plan: Vec2) -> usize {
        let column = (((plan[0] - self.origin[0]) / self.span[0]).floor().max(0.0) as usize)
            .min(self.columns - 1);
        let row = (((plan[1] - self.origin[1]) / self.span[1]).floor().max(0.0) as usize)
            .min(self.rows - 1);
        row * self.columns + column
    }
}

/// Runs a body once per bucket a span covers, columns before rows.
fn for_each_bucket(span: Span, columns: usize, mut body: impl FnMut(usize)) {
    for column in span.0..span.2 {
        for row in span.1..span.3 {
            body(row * columns + column);
        }
    }
}

/// Returns the bucket range a face's plan bounding box covers.
///
/// A face with no plan area returns nothing: it encloses no point, so it can never be the
/// face a cell reads, and giving it a bucket would only lengthen every scan in that one.
fn bucket_span(
    mesh: &FacetMesh,
    face: &Face,
    low: Vec2,
    span: Vec2,
    columns: usize,
    rows: usize,
) -> Option<Span> {
    let mut near = [f32::INFINITY; 2];
    let mut far = [f32::NEG_INFINITY; 2];
    for corner in face.indices {
        let point = mountain::math::plan(mesh.points[corner as usize]);
        for axis in 0..2 {
            near[axis] = near[axis].min(point[axis]);
            far[axis] = far[axis].max(point[axis]);
        }
    }
    if near[0] > far[0] || near[1] > far[1] {
        return None;
    }
    let column =
        |value: f32| (((value - low[0]) / span[0]).floor().max(0.0) as usize).min(columns - 1);
    let row = |value: f32| (((value - low[1]) / span[1]).floor().max(0.0) as usize).min(rows - 1);
    Some((
        column(near[0]),
        row(near[1]),
        column(far[0]) + 1,
        row(far[1]) + 1,
    ))
}

/// Returns the face covering a plan position, and the position's place in its plan triangle.
///
/// The three subtriangle areas are the barycentric coordinates themselves, which is why the
/// test needs no assumption about the winding: the coordinates sum to one whichever way the
/// face is wound, so dividing by the whole signed area yields the same three weights. They
/// are returned as well as consumed, because a caller that knows which face a cell came from
/// needs to know where in that face the cell stood.
fn cover(mesh: &FacetMesh, index: u32, plan: Vec2) -> Option<(u32, [f32; 3])> {
    let face = &mesh.faces[index as usize];
    let flat: [Vec2; 3] = std::array::from_fn(|corner| {
        mountain::math::plan(mesh.points[face.indices[corner] as usize])
    });
    let whole = signed_area(flat[0], flat[1], flat[2]);
    if whole == 0.0 {
        return None;
    }
    let weights = [
        signed_area(plan, flat[1], flat[2]) / whole,
        signed_area(flat[0], plan, flat[2]) / whole,
        signed_area(flat[0], flat[1], plan) / whole,
    ];
    if weights.iter().any(|weight| *weight < -CONTAINMENT_SLACK) {
        return None;
    }
    Some((index, weights))
}
