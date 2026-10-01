//! Which light reaches which cluster, and the ranking that decides when a cluster is over
//! its cap.
//!
//! # Why the assignment is a cpu loop and not a compute pass
//!
//! `emit(scene, camera_index)` returns the instance and `Renderer::render(&self, graphics,
//! instance)` consumes an instance that is already built. For a given scene and a given camera
//! the frustum is fixed, the lights are fixed and so are the boxes, so the assignment is a pure
//! function of the documents — which is where a pure function belongs in this design, beside the
//! rest of the compilation.
//!
//! The textbook gpu path needs a clear dispatch, an `atomicAdd` count pass, a prefix sum for
//! the offsets and a scatter pass, over a grid that `max_compute_invocations_per_workgroup` of
//! 256 cannot cover in one workgroup. Here there is no clear, no counter, no prefix sum and no
//! dispatch, and — the reason this stage is cheaper here than the plan assumed — the per
//! cluster cap keeps exactly the lights that contribute most, because ranking a list of a few
//! dozen numbers is what a cpu is for. Truncating a cluster's list is the one thing a sparse
//! scheme must never do quietly: a light that is dropped is a surface lit by nothing.
//!
//! # How depth is distributed, and why
//!
//! The slices divide the projection's **depth**, which is uniform in one over distance, rather
//! than dividing the distance itself.
//!
//! - A slice even in depth is even on the frame. The frame covers a small angle at the near
//!   plane and a wide one at the far plane, so slices of equal *world* thickness pile up at the
//!   eye and vanish in the distance: a grid of them spends most of its boxes on the few
//!   clusters that fill the screen and none on the far ones. A cluster that is nearly empty in
//!   screen space but deep is still a cluster, and under an even-in-distance division it is a
//!   box no wider than a pixel.
//! - It is also the division a fragment can use for free. The depth is the number a fragment
//!   already carries, so a shading pass finds its cluster with a multiply and a cast and no
//!   arithmetic at all. Recovering a *distance* from a depth needs the near and far planes as
//!   well, and a shader reading one box's lights wants the box rather than the distance.
//!
//! The cost is stated rather than hidden. The slices are not even in world space: the nearest
//! is a hand's width in front of the eye and the furthest is most of the scene, so a light
//! spanning a slice boundary is listed in a box far larger than the part of the scene it
//! lights, and a light of a large radius is listed in many of the far slices where it
//! contributes least. Both are the price of a grid a fragment can index, and both are per scene
//! decisions, because the grid is.
//!
//! # What a cluster is
//!
//! One box in view space, a light's volume is a sphere around its position, and the test is the
//! two against each other: the squared distance from the sphere's centre to the box's nearest
//! point, against the squared radius. That is exact for a box, so the only slack in it is the
//! box itself. A slice of a perspective view is a slab that widens with depth, and the box is
//! the slab at its **far** end, which contains the whole of it: listing a light that only
//! grazes the slice's near corner costs work, and leaving one out is a hole in the image, so
//! the slack is spent in the direction that costs only work.
//!
//! # Invariants
//!
//! - The view is reached through [`ViewProjection`] and not through any one compiler's camera,
//!   so a compiler whose view answers these readings differently uses the same arithmetic.
//! - A light is assigned by its volume and never by its position alone, and a light is listed at
//!   most once per cluster, and a cluster's list is at most the cap.

use definition::{FieldMap, Path, Value};

use super::buffers::{Clusters, LightPool};
use crate::CompileError;

/// How many clusters the frame is divided into across, for a scene that says nothing.
pub const DEFAULT_WIDTH_EXTENTS: usize = 16;

/// How many clusters the frame is divided into up, for a scene that says nothing.
pub const DEFAULT_HEIGHT_EXTENTS: usize = 9;

/// How many slices the frustum is divided into, for a scene that says nothing.
pub const DEFAULT_DEPTH_EXTENTS: usize = 24;

/// How many lights one cluster keeps, for a scene that says nothing.
pub const DEFAULT_LIGHTS_PER_CLUSTER: usize = 64;

/// Field holding how the view frustum is divided for clustered lighting.
pub const CLUSTERING: &str = "clustering";

/// Field holding how many clusters the frame is divided into across.
pub const WIDTH_EXTENTS: &str = "width_extents";

/// Field holding how many clusters the frame is divided into up.
pub const HEIGHT_EXTENTS: &str = "height_extents";

/// Field holding how many slices the frustum is divided into, deep.
pub const DEPTH_EXTENTS: &str = "depth_extents";

/// Field holding how many lights one cluster keeps.
pub const LIGHTS_PER_CLUSTER: &str = "lights_per_cluster";

/// The readings of a view that measuring a grid into it needs.
///
/// # Why this is a trait and not a camera type
///
/// The grid is measured in the view's own space, against the view's depth, and what it needs
/// is the eye, the three axes, the two depth planes and the two mappings between depth and
/// distance. A compiler whose camera is a different type answers the same questions, and taking
/// them as a trait is what lets this arithmetic be a property of lighting rather than of the
/// standard scene's camera.
pub trait ViewProjection {
    /// Where the view is made from, in world space.
    fn eye(&self) -> [f64; 3];
    /// The direction the view looks along.
    fn forward(&self) -> [f64; 3];
    /// The axis the view's right runs along.
    fn right(&self) -> [f64; 3];
    /// The axis the view's up runs along.
    fn up(&self) -> [f64; 3];
    /// How far from the eye the near plane is.
    fn near(&self) -> f64;
    /// How far from the eye the far plane is.
    fn far(&self) -> f64;
    /// Returns the depth the view gives a distance in front of the eye, in zero to one.
    fn depth_at(&self, distance: f64) -> f64;
    /// Returns how far in front of the eye a depth is, which inverts [`ViewProjection::depth_at`].
    fn distance_at(&self, depth: f64) -> f64;
    /// Returns how far off centre the frame's edge is at a distance, across then up.
    fn half_width_at(&self, distance: f64) -> [f64; 2];
}

/// How the view is divided, as a scene document decides it.
///
/// # Invariants
///
/// Every extent and the cap is at least one, checked when the scene is read: a grid of
/// nothing is a scene whose lights reach no cluster, and that is a wrong image rather than a
/// fast one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clustering {
    /// How many clusters the frame is divided into across.
    pub width_extents: usize,
    /// How many clusters the frame is divided into up.
    pub height_extents: usize,
    /// How many slices the frustum is divided into, deep.
    pub depth_extents: usize,
    /// How many lights one cluster keeps.
    pub lights_per_cluster: usize,
}

impl Clustering {
    /// Returns the grid a scene that says nothing about clustering is drawn with.
    pub fn convention() -> Self {
        Self {
            width_extents: DEFAULT_WIDTH_EXTENTS,
            height_extents: DEFAULT_HEIGHT_EXTENTS,
            depth_extents: DEFAULT_DEPTH_EXTENTS,
            lights_per_cluster: DEFAULT_LIGHTS_PER_CLUSTER,
        }
    }

    /// Reads the grid a scene declares, falling back to the convention field by field.
    ///
    /// # Invariants
    ///
    /// Every field is read against the convention rather than against a default the author has
    /// to know, so a scene that names one extent and no others gets that one extent and the
    /// rest of the grid every scene gets.
    pub fn read(scene: &Value) -> Result<Self, CompileError> {
        let convention = Self::convention();
        let path = Path::root().key(CLUSTERING);
        let Some(declared) = scene
            .as_field_map()
            .and_then(|fields| fields.get(&CLUSTERING.to_owned()))
        else {
            return Ok(convention);
        };
        let Some(fields) = declared.as_field_map() else {
            return Err(CompileError::MalformedScene {
                path,
                expected: "the four counts of a cluster grid",
                found: declared.kind_name(),
            });
        };
        Ok(Self {
            width_extents: extent(fields, WIDTH_EXTENTS, convention.width_extents, &path)?,
            height_extents: extent(fields, HEIGHT_EXTENTS, convention.height_extents, &path)?,
            depth_extents: extent(fields, DEPTH_EXTENTS, convention.depth_extents, &path)?,
            lights_per_cluster: extent(
                fields,
                LIGHTS_PER_CLUSTER,
                convention.lights_per_cluster,
                &path,
            )?,
        })
    }

    /// Returns how many clusters the grid holds, which is the length of the count buffer and
    /// the number of rows the list buffer holds.
    pub fn cluster_count(&self) -> usize {
        self.width_extents * self.height_extents * self.depth_extents
    }
}

/// Reads one count of the grid, refusing a count of nothing.
fn extent(
    fields: &FieldMap,
    name: &str,
    convention: usize,
    path: &Path,
) -> Result<usize, CompileError> {
    let Some(value) = fields.get(name) else {
        return Ok(convention);
    };
    let field = path.key(name);
    let Some(count) = value.as_int() else {
        return Err(CompileError::MalformedScene {
            path: field,
            expected: "a whole number of clusters",
            found: value.kind_name(),
        });
    };
    if count < 1 {
        return Err(CompileError::EmptyClustering {
            path: field,
            field: name.to_owned(),
            found: count,
        });
    }
    Ok(count as usize)
}

/// One light as the assignment needs it, which is not the whole of a light.
///
/// # Invariants
///
/// `colour` is already scaled by the light's intensity, because that is the brightness a
/// surface is lit by and brightness is what the cap ranks by. A record carrying a colour and
/// an intensity separately would leave every reader of it a second answer to the same
/// question.
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterLight {
    /// What the light is called, which is how a report about it names it.
    pub label: String,
    /// Where the light is, in world space.
    pub position: [f64; 3],
    /// Its colour scaled by its intensity.
    pub colour: [f64; 3],
    /// How far it reaches, as a sphere around its position.
    pub radius: f64,
    /// The light's own view projection, sixteen floats in column major order.
    ///
    /// # Invariants
    ///
    /// This is **carried, not computed**: the arithmetic in this module never builds a
    /// projection, because a projection is a property of a view and this module has no view.
    /// The compiler that owns the light's camera fills it in, and a light of a kind that
    /// casts no shadow carries the identity rather than a matrix nothing looks through.
    ///
    /// It is here rather than beside the page in [`super::buffers`] because the record is
    /// packed from this one struct and a field reached from somewhere else would be a second
    /// array indexed the same way, which is a way for the two to disagree about which light
    /// a record is.
    pub shadow_projection: [f64; 16],
    /// Whether this kind of light takes cluster slots at all.
    ///
    /// A light that covers the whole view is in every cluster by definition, so listing it
    /// would spend the cap on a light the culling was never going to remove.
    pub clustered: bool,
}

impl ClusterLight {
    /// Returns the brightest the light can be anywhere, which is what the cap ranks by.
    ///
    /// # Invariants
    ///
    /// The channels are ranked by the largest rather than the sum: the cap is asking which
    /// light can make a surface look brightest, and a light that is red contributes no green
    /// however bright its sum is.
    pub fn brightness(&self) -> f64 {
        self.colour
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

/// One cluster that had more lights reach it than it keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overflow {
    /// Which cluster of the grid this was: depth slices count fastest, then rows, then columns.
    pub cluster: usize,
    /// How many lights reached it.
    pub reached: usize,
    /// The labels of the lights it did not keep.
    pub dropped: Vec<String>,
}

/// Assigns every light to the clusters its volume reaches, and gives the shadow pages to the
/// lights that contribute most.
///
/// # Invariants
///
/// - A light is assigned by its volume, which is a sphere of the radius the document decides,
///   and never by its position alone: a light at the centre of a cluster lights the whole of
///   it, and a light beyond its radius lights none of it.
/// - A light is listed at most once per cluster, and a cluster's list is at most the cap.
/// - A light that takes no cluster slots is in no list at all rather than in every one, and
///   its record is still emitted: whether an exempt light is applied is the shading pass's
///   business, and a record the emitter dropped would be a light nothing could find.
pub fn assign(
    clustering: &Clustering,
    lights: &[ClusterLight],
    projection: &impl ViewProjection,
    pool: &LightPool,
) -> Result<Clusters, CompileError> {
    let view = View::read(clustering, projection);
    let cluster_count = clustering.cluster_count();
    let mut candidates: Vec<Vec<Candidate>> = vec![Vec::new(); cluster_count];

    for (index, light) in lights.iter().enumerate() {
        if !light.clustered {
            continue;
        }
        let Some(volume) = Volume::read(light, projection, &view) else {
            continue;
        };
        for slice in volume.slices.0..=volume.slices.1 {
            for row in 0..clustering.height_extents {
                for column in 0..clustering.width_extents {
                    let box_of = view.box_of(column, row, slice);
                    let distance_squared = volume.nearest_squared(box_of);
                    if distance_squared > volume.radius_squared {
                        continue;
                    }
                    candidates[view.index(column, row, slice)].push(Candidate {
                        light: index,
                        weight: contribution(light.brightness(), distance_squared),
                    });
                }
            }
        }
    }

    super::buffers::pack(clustering, lights, pool, candidates)
}

/// How much a light can contribute to one cluster.
///
/// # Invariants
///
/// The ranking only has to be monotone in distance, and it is. A falloff of the shape
/// `1 / (a + b * d + c * d * d)` with positive coefficients decreases with distance whatever
/// its coefficients, so the lights nearest a cluster are the ones a scene's own falloff would
/// keep as well; a steeper falloff keeps the same lights more firmly. The one here is the
/// gentlest of that shape, with the `1` for the `a`.
///
/// No scene's falloff constants are written into this file, and that is the point: they are a
/// scene's decision, and a number chosen once in the source is a number no author can change.
///
/// The shadow page pool ranks by this same function, over a light's reach of the whole view
/// rather than of one cluster. Both are "how much does this light matter to a piece of the
/// image", and they differ in what the distance is measured to: a cluster is a box, so a light
/// just outside it still counts for a little, while a page is worth rendering once, so what
/// matters there is how much of the scene the light reaches at all. A light can therefore
/// matter to a cluster and still not be worth a page, which is the point of ranking twice
/// rather than once.
pub fn contribution(brightness: f64, distance_squared: f64) -> f64 {
    brightness / (1.0 + distance_squared)
}

/// One light's reach of one cluster, as an index and the weight it reached it at.
///
/// It is the compiler's own, rather than a tuple, because the packing reads it and the packing
/// is a different module: a type both halves name is the smallest thing that keeps one from
/// reaching into the other's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Candidate {
    /// Which light of the scene, which is the index the record buffer is written in.
    pub(super) light: usize,
    /// How much it contributes to the cluster it reached.
    pub(super) weight: f64,
}

/// One light's volume in view space, as the two things the test needs.
struct Volume {
    /// The sphere's centre: across, up, and how far in front of the eye.
    centre: [f64; 3],
    /// The squared radius, which is what the nearest distance is compared against.
    radius_squared: f64,
    /// The first and last slice the volume reaches, both inclusive.
    slices: (usize, usize),
}

impl Volume {
    /// Reads a light's volume, or nothing when the volume lies outside the depth range.
    ///
    /// # Invariants
    ///
    /// The slices come from the volume's own extent along the view, mapped through the
    /// projection's depth. A volume in front of the eye reaches a contiguous run of slices,
    /// because the slices partition the depth range and the volume is a contiguous interval
    /// along it; a volume beyond the far plane, in front of the near plane or behind the eye
    /// reaches none, which is why the test is on the interval rather than on the centre.
    fn read(light: &ClusterLight, projection: &impl ViewProjection, view: &View) -> Option<Self> {
        let offset = subtract(light.position, projection.eye());
        let centre = [
            dot(offset, projection.right()),
            dot(offset, projection.up()),
            dot(offset, projection.forward()),
        ];
        let nearest = (centre[2] - light.radius).max(projection.near());
        let furthest = (centre[2] + light.radius).min(projection.far());
        if nearest > furthest {
            return None;
        }
        Some(Self {
            centre,
            radius_squared: light.radius * light.radius,
            slices: view.slice_of(projection.depth_at(nearest), projection.depth_at(furthest)),
        })
    }

    /// Returns the squared distance from the centre to the nearest point of a box.
    ///
    /// # Invariants
    ///
    /// A light standing inside a box is at distance zero from it, so it is in that box at any
    /// radius at all: the falloff decides how bright it is there, and the assignment only
    /// decides whether it is found.
    fn nearest_squared(&self, box_of: ClusterBox) -> f64 {
        let across = outside(self.centre[0], box_of.across);
        let up = outside(self.centre[1], box_of.up);
        let depth = outside(self.centre[2], box_of.depth);
        across * across + up * up + depth * depth
    }
}

/// Returns how far a coordinate lies outside an interval, and zero when it is inside.
fn outside(coordinate: f64, interval: [f64; 2]) -> f64 {
    if coordinate < interval[0] {
        interval[0] - coordinate
    } else if coordinate > interval[1] {
        coordinate - interval[1]
    } else {
        0.0
    }
}

/// Subtracts one vector from another.
///
/// The arithmetic a view space measurement needs, kept here rather than taken from the
/// standard scene's matrix module: the two operations below are what this file uses, and a
/// general module that reaches into a compiler's private vector helpers is not a general one.
fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

/// Returns the dot product of two vectors.
fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

/// One cluster, as a box in view space.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClusterBox {
    /// Across, measured from the centre of the frame.
    pub(crate) across: [f64; 2],
    /// Up, measured from the centre of the frame.
    pub(crate) up: [f64; 2],
    /// How far in front of the eye.
    pub(crate) depth: [f64; 2],
}

/// The view, divided into the slices and boxes the assignment walks.
struct View {
    /// The distance at each slice boundary, one more than there are slices.
    boundaries: Vec<f64>,
    /// The frame's half width and half height at each slice's far boundary.
    widths: Vec<[f64; 2]>,
    width_extents: usize,
    height_extents: usize,
    depth_extents: usize,
}

impl View {
    /// Reads the divisions of a view.
    ///
    /// # Invariants
    ///
    /// The boundaries are the distances the projection gives each whole depth, so slice `i`
    /// lies between `boundaries[i]` and `boundaries[i + 1]`, the first is the near plane and
    /// the last the far plane. They are read through the projection rather than by dividing
    /// the near to far range, because a grid of even distances is not the grid a fragment
    /// indexes.
    fn read(clustering: &Clustering, projection: &impl ViewProjection) -> Self {
        let extents = clustering.depth_extents as f64;
        let boundaries: Vec<f64> = (0..=clustering.depth_extents)
            .map(|slice| projection.distance_at(slice as f64 / extents))
            .collect();
        let widths = boundaries
            .iter()
            .map(|distance| projection.half_width_at(*distance))
            .collect();
        Self {
            boundaries,
            widths,
            width_extents: clustering.width_extents,
            height_extents: clustering.height_extents,
            depth_extents: clustering.depth_extents,
        }
    }

    /// Returns the slices a volume between two depths reaches, clamped to the grid.
    ///
    /// # Invariants
    ///
    /// The range is one slice wider at each end than the depths alone call for, because which
    /// slice a volume's own end falls in is decided by a projection whose rounding puts a
    /// point exactly on a boundary in either of the two slices beside it, and a light missing
    /// from a cluster is a light nothing can shade. Being one slice wide costs two clusters a
    /// light at each end of its reach and can only add to the assignment: the box test
    /// decides membership, and a slice the volume does not reach fails it.
    fn slice_of(&self, from: f64, to: f64) -> (usize, usize) {
        (
            self.slice_at(from).saturating_sub(1),
            (self.slice_at(to) + 1).min(self.depth_extents - 1),
        )
    }

    /// Returns which slice a depth falls in, clamped to the grid.
    ///
    /// # Invariants
    ///
    /// A depth outside zero to one is clamped rather than refused: the projection's own
    /// rounding puts a point exactly on the far plane a hair past one, and a grid that
    /// refused to index it would lose a light to a rounding error.
    fn slice_at(&self, depth: f64) -> usize {
        let index = (depth * self.depth_extents as f64).floor();
        if index <= 0.0 {
            0
        } else {
            (index as usize).min(self.depth_extents - 1)
        }
    }

    /// Returns which cluster of the grid a column, row and slice are one cluster of.
    ///
    /// # Invariants
    ///
    /// Depth slices count fastest and rows next, so two clusters sharing a column and a row
    /// are `depth_extents` apart, and a fragment that knows its own three indices needs no
    /// arithmetic beyond these two products. The count buffer has one count per cluster and
    /// the list buffer one row per cluster, so the two have to agree on this number.
    fn index(&self, column: usize, row: usize, slice: usize) -> usize {
        slice * self.width_extents * self.height_extents + row * self.width_extents + column
    }

    /// Returns the box one column, row and slice of the view cover.
    ///
    /// # Invariants
    ///
    /// The box is the slab at the slice's **far** boundary, which is where a perspective view
    /// is widest, so the box contains the whole of the slab. The alternative — a box at the
    /// near boundary, which is the tighter fit — leaves out a light that reaches the slice's
    /// far half, and a light missing from a cluster is a light nothing can shade.
    fn box_of(&self, column: usize, row: usize, slice: usize) -> ClusterBox {
        let far = self.boundaries[slice + 1];
        let [half_across, half_up] = self.widths[slice + 1];
        ClusterBox {
            across: span(column, self.width_extents, half_across),
            up: span(row, self.height_extents, half_up),
            depth: [self.boundaries[slice], far],
        }
    }
}

/// Returns the two ends of one column or row of a frame, given its half width.
fn span(index: usize, extents: usize, half: f64) -> [f64; 2] {
    let step = 2.0 * half / extents as f64;
    let from = -half + step * index as f64;
    [from, from + step]
}
