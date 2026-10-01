//! Which lights reach which parts of the view, decided here rather than in a pass.
//!
//! # What this is for
//!
//! One pass per light per group is what stops a scene holding thousands of lights: the pass
//! count is `O(lights * groups)`, and a thousand lights is a thousand shadow maps. A clustered
//! path replaces both with data. The view is divided into a grid of boxes, every box holds the
//! lights that reach it, and a shading pass that draws one box's worth of lights per fragment
//! costs one pass however many lights there are.
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
//! - The three buffers are read only. Nothing here writes a buffer, because there is no pass to
//!   write one and no dispatch to write it with, and `max_storage_buffers_per_shader_stage` is
//!   four: light records, cluster counts and cluster lists are all of the budget stage 3b has,
//!   with one spare.
//! - Every light is listed in every cluster its volume reaches, up to the cap. A light that is
//!   not listed is a light no shading pass can find, so the cap is the only place one goes
//!   missing, and the cap reports what it missed.
//! - A kind that covers the whole view is listed nowhere and takes no cluster slot. A
//!   directional light and an environment light are in every box by definition, so listing them
//!   would spend the budget on lights the culling was never going to remove; they are in the
//!   record buffer and in no cluster list, which is what gives stage 3b its own path for them.
//!   A scene whose lights are only of those two kinds therefore has empty cluster lists, which
//!   is the correct answer rather than a missing one.

use definition::{
    FieldMap, PackedBuilder, PackedError, PackedLayout, PackedList, Path, Schema, SchemaKind, Value,
};

use graphics::binders::{DATA, SCHEMA, SIZE_BYTES, USAGE};

use crate::standard::camera::Projection;
use crate::standard::math::{dot, subtract};
use crate::standard::scene_schema;
use crate::{CompileError, CompileNote};

/// How many clusters the frame is divided into across, for a scene that says nothing.
pub const DEFAULT_WIDTH_EXTENTS: usize = 16;

/// How many clusters the frame is divided into up, for a scene that says nothing.
pub const DEFAULT_HEIGHT_EXTENTS: usize = 9;

/// How many slices the frustum is divided into, for a scene that says nothing.
pub const DEFAULT_DEPTH_EXTENTS: usize = 24;

/// How many lights one cluster keeps, for a scene that says nothing.
pub const DEFAULT_LIGHTS_PER_CLUSTER: usize = 64;

/// Name of the instance entry holding one packed record per light.
pub const LIGHT_RECORDS_ENTRY: &str = "light_records";

/// Name of the instance entry holding how many lights each cluster kept.
pub const CLUSTER_COUNTS_ENTRY: &str = "cluster_light_counts";

/// Name of the instance entry holding the light of every slot of every cluster.
pub const CLUSTER_INDICES_ENTRY: &str = "cluster_light_indices";

/// Field of the light record naming what colour the light is.
pub const LIGHT_COLOR_FIELD: &str = "color";

/// Field of the light record naming where the light is.
pub const LIGHT_POSITION_FIELD: &str = "position";

/// Field of the light record naming how far the light reaches.
pub const LIGHT_RADIUS_FIELD: &str = "radius";

/// Field of the cluster count record naming the count.
pub const CLUSTER_COUNT_FIELD: &str = "count";

/// Field of the cluster list record naming the light.
pub const CLUSTER_LIGHT_FIELD: &str = "light";

/// How many dropped lights a report names before it counts the rest instead.
///
/// The count is always the whole truth; only the naming is bounded, because one cluster can
/// drop thousands of lights and a report that printed every label of every one of them is a
/// report nobody reads to the end.
const NAMED_LIGHTS: usize = 8;

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
        let path = Path::root().key(scene_schema::CLUSTERING);
        let Some(declared) = scene
            .as_field_map()
            .and_then(|fields| fields.get(&scene_schema::CLUSTERING.to_owned()))
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
            width_extents: extent(
                fields,
                scene_schema::WIDTH_EXTENTS,
                convention.width_extents,
                &path,
            )?,
            height_extents: extent(
                fields,
                scene_schema::HEIGHT_EXTENTS,
                convention.height_extents,
                &path,
            )?,
            depth_extents: extent(
                fields,
                scene_schema::DEPTH_EXTENTS,
                convention.depth_extents,
                &path,
            )?,
            lights_per_cluster: extent(
                fields,
                scene_schema::LIGHTS_PER_CLUSTER,
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

/// The three buffers one assignment produced, and what it had to drop.
///
/// # Invariants
///
/// The two lists are read together: `counts` is one per cluster, `indices` is
/// `clusters * lights_per_cluster` of them, and the lights of cluster `c` are the first
/// `counts[c]` of the row starting at `c * lights_per_cluster`. The row is a fixed width
/// rather than packed end to end, because a fragment reading one cluster's lights computes
/// that one multiplication, and a list packed end to end would need a prefix sum of the
/// counts to find the row — which is the sum this stage exists to avoid computing.
pub struct Clusters {
    /// One record per light, packed from [`light_record_schema`].
    pub records: PackedList,
    /// One count per cluster, packed from [`cluster_count_schema`].
    pub counts: PackedList,
    /// One slot per cluster per cap, packed from [`cluster_list_schema`].
    pub indices: PackedList,
    /// The clusters that had more lights reach them than the cap keeps, in cluster order.
    pub overflows: Vec<Overflow>,
    /// How many lights were dropped, counted over every cluster.
    ///
    /// # Invariants
    ///
    /// One light is counted once per cluster it was dropped from, so this is a count of
    /// (cluster, light) pairs missing rather than of distinct lights: a light dropped from
    /// four clusters was missing four times over, and the cost of that is four regions lit by
    /// nothing rather than one.
    pub lights_dropped: usize,
}

impl Clusters {
    /// Returns the three instance entries this assignment contributes, in a fixed order.
    ///
    /// # Invariants
    ///
    /// The names are constants rather than per scene names, so a shading pass names them the
    /// same way whatever scene it draws, and an entry a pass does not bind costs nothing: a
    /// buffer is created when something binds it, not because the instance holds it.
    pub fn entries(&self) -> [(&'static str, Value); 3] {
        [
            (
                LIGHT_RECORDS_ENTRY,
                storage(self.records.bytes(), &light_record_schema()),
            ),
            (
                CLUSTER_COUNTS_ENTRY,
                storage(self.counts.bytes(), &cluster_count_schema()),
            ),
            (
                CLUSTER_INDICES_ENTRY,
                storage(self.indices.bytes(), &cluster_list_schema()),
            ),
        ]
    }

    /// Returns what a host is told about an assignment that dropped lights.
    ///
    /// # Invariants
    ///
    /// Nothing is dropped without a note. A scene lit less than it was drawn is a wrong image
    /// nobody sees until the render finishes, so the report carries a count for every
    /// overflowing cluster rather than a sample of them, and the labels of one cluster are
    /// named up to [`NAMED_LIGHTS`] and counted beyond that.
    pub fn notes(&self, clustering: &Clustering, path: &Path) -> Vec<CompileNote> {
        if self.overflows.is_empty() {
            return Vec::new();
        }
        let cap = path.key(scene_schema::LIGHTS_PER_CLUSTER);
        let mut notes = vec![CompileNote {
            path: cap.clone(),
            message: format!(
                "{} of the {} clusters of this view had more lights reach them than the cap of {} \
                 keeps, and {} of the light entries in their lists are missing. The lights that \
                 contribute most are the ones kept, so this view is lit less than the scene was \
                 drawn: raise `{}`, or the grid, or move the lights.",
                self.overflows.len(),
                clustering.cluster_count(),
                clustering.lights_per_cluster,
                self.lights_dropped,
                scene_schema::LIGHTS_PER_CLUSTER,
            ),
        }];
        for overflow in &self.overflows {
            notes.push(CompileNote {
                path: cap.clone(),
                message: format!(
                    "cluster {} kept {} of the {} lights that reached it and dropped {}.",
                    overflow.cluster,
                    overflow.reached - overflow.dropped.len(),
                    overflow.reached,
                    dropped_names(&overflow.dropped),
                ),
            });
        }
        notes
    }
}

/// Names a cluster's dropped lights, counting the ones past the naming limit.
fn dropped_names(labels: &[String]) -> String {
    let (named, rest) = labels.split_at(labels.len().min(NAMED_LIGHTS));
    let names: Vec<&str> = named.iter().map(String::as_str).collect();
    if rest.is_empty() {
        return names.join(", ");
    }
    format!("{} and {} more", names.join(", "), rest.len())
}

/// Assigns every light to the clusters its volume reaches.
///
/// # Invariants
///
/// - A light is assigned by its volume, which is a sphere of the radius the document decides,
///   and never by its position alone: a light at the centre of a cluster lights the whole of
///   it, and a light beyond its radius lights none of it.
/// - A light is listed at most once per cluster, and a cluster's list is at most the cap.
/// - A light that takes no cluster slots is in no list at all rather than in every one, and
///   its record is still emitted: whether an exempt light is applied is stage 3b's business,
///   and a record the emitter dropped would be a light stage 3b could not find.
pub fn assign(
    clustering: &Clustering,
    lights: &[ClusterLight],
    projection: &Projection,
) -> Result<Clusters, CompileError> {
    let view = View::read(clustering, projection);
    let cluster_count = clustering.cluster_count();
    let mut candidates: Vec<Vec<(usize, f64)>> = vec![Vec::new(); cluster_count];

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
                    candidates[view.index(column, row, slice)]
                        .push((index, contribution(light.brightness(), distance_squared)));
                }
            }
        }
    }

    pack(clustering, lights, candidates)
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
fn contribution(brightness: f64, distance_squared: f64) -> f64 {
    brightness / (1.0 + distance_squared)
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
    fn read(light: &ClusterLight, projection: &Projection, view: &View) -> Option<Self> {
        let offset = subtract(light.position, projection.eye);
        let centre = [
            dot(offset, projection.right),
            dot(offset, projection.up),
            dot(offset, projection.forward),
        ];
        let nearest = (centre[2] - light.radius).max(projection.near);
        let furthest = (centre[2] + light.radius).min(projection.far);
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

/// One cluster, as a box in view space.
#[derive(Debug, Clone, Copy)]
struct ClusterBox {
    /// Across, measured from the centre of the frame.
    across: [f64; 2],
    /// Up, measured from the centre of the frame.
    up: [f64; 2],
    /// How far in front of the eye.
    depth: [f64; 2],
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
    fn read(clustering: &Clustering, projection: &Projection) -> Self {
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

/// Packs the three buffers and reports what the cap had to drop.
///
/// # Invariants
///
/// Every cluster's row is exactly the cap wide, so a fragment can index it without a prefix
/// sum. A cluster with fewer lights than the cap is padded with the first light rather than
/// with a sentinel, and the count beside the row is what says how many of the entries are
/// real: a padding entry is only ever read by a fragment that ignored its own cluster's
/// count, and a sentinel would be a value a shader had to know the meaning of.
fn pack(
    clustering: &Clustering,
    lights: &[ClusterLight],
    mut candidates: Vec<Vec<(usize, f64)>>,
) -> Result<Clusters, CompileError> {
    let cap = clustering.lights_per_cluster;
    let mut counts = PackedBuilder::new(one_number(CLUSTER_COUNT_FIELD));
    let mut indices = PackedBuilder::new(one_number(CLUSTER_LIGHT_FIELD));
    indices.reserve(candidates.len() * cap);
    let mut overflows = Vec::new();
    let mut lights_dropped = 0usize;

    for (cluster, reaching) in candidates.iter_mut().enumerate() {
        // Highest contribution first, and the light's own order for a tie, so that one scene
        // compiles to one set of bytes however the walk above happened to find its lights.
        reaching.sort_by(|(first, first_weight), (second, second_weight)| {
            second_weight
                .total_cmp(first_weight)
                .then(first.cmp(second))
        });
        let kept = reaching.len().min(cap);
        if reaching.len() > cap {
            overflows.push(Overflow {
                cluster,
                reached: reaching.len(),
                dropped: reaching[cap..]
                    .iter()
                    .map(|(index, _)| lights[*index].label.clone())
                    .collect(),
            });
            lights_dropped += reaching.len() - cap;
        }
        counts
            .field(CLUSTER_COUNT_FIELD, &[kept as f64])
            .map_err(|error| unpacked(CLUSTER_COUNTS_ENTRY, error))?;
        for slot in 0..cap {
            let light = reaching.get(slot).map_or(0.0, |(index, _)| *index as f64);
            indices
                .field(CLUSTER_LIGHT_FIELD, &[light])
                .map_err(|error| unpacked(CLUSTER_INDICES_ENTRY, error))?;
        }
    }

    Ok(Clusters {
        records: light_records(lights)?,
        counts: counts
            .finish()
            .map_err(|error| unpacked(CLUSTER_COUNTS_ENTRY, error))?,
        indices: indices
            .finish()
            .map_err(|error| unpacked(CLUSTER_INDICES_ENTRY, error))?,
        overflows,
        lights_dropped,
    })
}

/// Packs one record per light, in the order the scene lists them, which is the order the
/// cluster lists index into.
fn light_records(lights: &[ClusterLight]) -> Result<PackedList, CompileError> {
    let layout = graphics::layout::packed_layout(&light_record_schema()).map_err(|error| {
        CompileError::Unpackable {
            path: Path::root().key(graphics::ENTRIES).key(LIGHT_RECORDS_ENTRY),
            reason: error.to_string(),
        }
    })?;
    let mut records = PackedBuilder::new(layout);
    records.reserve(lights.len());
    for light in lights {
        // Sorted by name, so the colour is written first: the builder refuses a field written
        // out of the layout's order, which is what makes the run mean what its layout says.
        records
            .field(LIGHT_COLOR_FIELD, &light.colour)
            .and_then(|builder| builder.field(LIGHT_POSITION_FIELD, &light.position))
            .and_then(|builder| builder.field(LIGHT_RADIUS_FIELD, &[light.radius]))
            .map_err(|error| unpacked(LIGHT_RECORDS_ENTRY, error))?;
    }
    records
        .finish()
        .map_err(|error| unpacked(LIGHT_RECORDS_ENTRY, error))
}

/// Returns a layout of one number, which is what a count and an index are: a run whose every
/// record holds one value.
fn one_number(field: &str) -> PackedLayout {
    PackedLayout::new([(field.to_owned(), 1)])
}

/// Builds the payload of a read only storage buffer holding a packed run.
fn storage(bytes: &[u8], schema: &Schema) -> Value {
    Value::variant(
        "Buffer",
        [
            (SIZE_BYTES.to_owned(), Value::Int(bytes.len() as i64)),
            (
                USAGE.to_owned(),
                Value::List(vec![Value::String("storage".to_owned())]),
            ),
            (SCHEMA.to_owned(), schema.to_value()),
            (DATA.to_owned(), Value::Bytes(bytes.to_vec())),
        ],
    )
}

/// Reports a run that could not be built as a compile error naming the entry it was for.
fn unpacked(entry: &str, error: PackedError) -> CompileError {
    CompileError::Unpackable {
        path: Path::root().key(graphics::ENTRIES).key(entry),
        reason: error.to_string(),
    }
}

/// Builds the schema of one light's record, which is what a clustered shading pass reads its
/// lights out of.
///
/// # Invariants
///
/// The record is a point light's: where it is, what colour it is, and the radius it was
/// assigned by. The fields sort as `color`, `position`, `radius`, which is the order the
/// packed layout derives from the schema, so the bytes and the layout cannot disagree about a
/// stride.
///
/// A shadow projection is not in the record, and its absence is a decision rather than an
/// oversight: what a light's shadow map is looked up through belongs to the shadow stage, and
/// stage 4 replaces that addressing with a page pool. A record carrying a projection matrix
/// would be sixteen floats per light that this stage has no reader for.
pub fn light_record_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            LIGHT_COLOR_FIELD.to_owned(),
            graphics::layout::components_schema(3)
                .with_documentation("Colour of the light, scaled by its intensity."),
        ),
        (
            LIGHT_POSITION_FIELD.to_owned(),
            graphics::layout::components_schema(3)
                .with_documentation("Where the light is, in world space."),
        ),
        (
            LIGHT_RADIUS_FIELD.to_owned(),
            Schema::new(SchemaKind::Float)
                .with_documentation("How far the light reaches, as a sphere around its position."),
        ),
    ]))
    .with_documentation("One light, as a clustered shading pass reads it.")
}

/// Builds the schema of one cluster's count, which is how many lights the cluster kept.
pub fn cluster_count_schema() -> Schema {
    one_number_schema(CLUSTER_COUNT_FIELD, "How many lights this cluster kept.")
}

/// Builds the schema of one slot of one cluster, which is the light in it.
pub fn cluster_list_schema() -> Schema {
    one_number_schema(
        CLUSTER_LIGHT_FIELD,
        "The light in this slot, as an index into the light record buffer.",
    )
}

/// Builds the schema of a run whose every record is one number.
fn one_number_schema(field: &str, documentation: &str) -> Schema {
    Schema::new(SchemaKind::structure([(
        field.to_owned(),
        Schema::new(SchemaKind::Float).with_documentation(documentation),
    )]))
    .with_documentation("One record of a run of numbers, four bytes each.")
}
