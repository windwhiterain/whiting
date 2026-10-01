//! The three packed runs a clustered pass reads, and the shadow page pool beside them.
//!
//! # What is here
//!
//! One record per light, one count per cluster, and one row of the cap per cluster, packed
//! against schemas built here. The bytes and the layout cannot disagree about a stride because
//! both come from the same schema: the run is packed with the layout of the schema the shader
//! was handed.
//!
//! The page pool is the second half of the same story. A light record names the shadow page it
//! was given, and a light with no page is lit by the cheap approximation instead — which is why
//! the record says which of the two it is rather than leaving a page index of zero to mean both
//! "the first page" and "no page".
//!
//! # Invariants
//!
//! - The three cluster buffers are read only, and a buffer is created when something binds it
//!   rather than because the instance holds it, so an entry a later stage does not read costs
//!   one packed run in the document.
//! - A cluster's row is exactly the cap wide, so a fragment can index it without a prefix
//!   sum of the counts. A cluster with fewer lights than the cap is padded with the first light
//!   rather than with a sentinel, and the count beside the row is what says how many entries
//!   are real.
//! - A page index of minus one is a light with no page. A page index of zero is the first page.
//!   The two cannot be told apart by a shader unless the compiler says which is which, so the
//!   value that means "none" is one no real page can carry.

use definition::{
    PackedBuilder, PackedError, PackedLayout, PackedList, Path, Schema, SchemaKind, Value,
};

use graphics::binders::{DATA, RESOURCE, SCHEMA, SIZE_BYTES, SOURCE, USAGE};

use crate::CompileError;
use crate::CompileNote;
use crate::standard::FRAME;
use crate::standard::math::{IDENTITY, Matrix};

use super::assignment::{Candidate, ClusterLight, Clustering, Overflow, contribution};

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

/// Field of the light record naming which shadow page the light was given.
///
/// # Invariants
///
/// Minus one is the value that means "this light has no page", and no real page carries it.
/// A page index of zero is the first page and is a page, so the two cannot be told apart by
/// magnitude and the compiler writes a distinct value for the second.
pub const LIGHT_PAGE_FIELD: &str = "page";

/// Field of the light record naming the light's own view projection.
///
/// # Invariants
///
/// Sixteen floats, column major, which is the order a `mat4x4<f32>` is read in. A fragment
/// takes its world position through this matrix to find the page the light rendered, so a
/// record without it is a record a shadow cannot be looked up from — which is why it is in
/// the record and not beside the page: an index has to resolve to everything the shader
/// needs about that light in one read.
pub const LIGHT_SHADOW_PROJECTION_FIELD: &str = "shadow_projection";

/// How many floats one light record is wide.
///
/// # Invariants
///
/// It is derived from [`light_record_schema`] rather than written down, because a stride the
/// schema and the packing could disagree about is a stride that reads the wrong light's
/// colour. The count is `3 + 1 + 3 + 1 + 16`: colour, page, position, radius and the
/// projection.
pub fn light_record_stride() -> usize {
    24
}

/// Field of the cluster count record naming the count.
pub const CLUSTER_COUNT_FIELD: &str = "count";

/// Field of the cluster list record naming the light.
pub const CLUSTER_LIGHT_FIELD: &str = "light";

/// The page index a light carries when it was given no page.
///
/// # Invariants
///
/// This is a value in the record's own type rather than a separate flag, because a flag beside
/// an index is a second thing a shader can disagree with. A shader tests the index once and
/// knows which of the two paths to take. The value is below every legal page index, so a
/// record that was never written also reads as "no page" rather than as "the first page".
pub const NO_PAGE: f64 = -1.0;

/// How many dropped lights a report names before it counts the rest instead.
///
/// The count is always the whole truth; only the naming is bounded, because one cluster can
/// drop thousands of lights and a report that printed every label of every one of them is a
/// report nobody reads to the end.
const NAMED_LIGHTS: usize = 8;

/// The three buffers one assignment produced, and what it had to drop.
///
/// # Invariants
///
/// The two lists are read together: `counts` is one per cluster, `indices` is
/// `clusters * lights_per_cluster` of them, and the lights of cluster `c` are the first
/// `counts[c]` of the row starting at `c * lights_per_cluster`.
pub struct Clusters {
    /// One record per light, packed from [`light_record_schema`].
    pub records: PackedList,
    /// One count per cluster, packed from [`cluster_count_schema`].
    pub counts: PackedList,
    /// One slot per cluster per cap, packed from [`cluster_list_schema`].
    pub indices: PackedList,
    /// The page each light was given, one per light, indexed as the records are.
    pub pages: Vec<usize>,
    /// Whether each light reached at least one cluster, one per light.
    ///
    /// # Invariants
    ///
    /// This is the difference between a light the pool turned away and a light that was never
    /// a candidate. A light behind the eye reaches nothing, so it is lit by nothing and no
    /// page was spent on it; counting it as dropped would point an author at a pool that was
    /// never the problem.
    pub reached: Vec<bool>,
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
    /// same way whatever scene it draws.
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

    /// Returns the three binder declarations a clustered shading pass carries, in the same
    /// order as [`Clusters::entries`].
    ///
    /// # Invariants
    ///
    /// A declaration's `size_bytes` is the size of the entry it names, and the two are built
    /// from the same [`PackedList`], so a pass cannot declare a buffer at one size and bind
    /// the bytes of another. This is the counterpart of [`Clusters::entries`] and belongs
    /// beside it for that reason: the payload and the declaration are two views of one run,
    /// and a rule expressed twice is a rule that can be applied once.
    pub fn declarations(&self) -> [(&'static str, Value); 3] {
        [
            (
                LIGHT_RECORDS_ENTRY,
                declaration(LIGHT_RECORDS_ENTRY, self.records.bytes().len() as i64),
            ),
            (
                CLUSTER_COUNTS_ENTRY,
                declaration(CLUSTER_COUNTS_ENTRY, self.counts.bytes().len() as i64),
            ),
            (
                CLUSTER_INDICES_ENTRY,
                declaration(CLUSTER_INDICES_ENTRY, self.indices.bytes().len() as i64),
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
        let cap = path.key(super::assignment::LIGHTS_PER_CLUSTER);
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
                super::assignment::LIGHTS_PER_CLUSTER,
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

    /// Returns the page one light was given, or nothing when it was given none.
    pub fn page_of(&self, light: usize) -> Option<usize> {
        self.pages
            .get(light)
            .copied()
            .filter(|page| *page != NO_PAGE_USIZE)
    }
}

/// The page count [`Clusters::page_of`] filters against, which is [`NO_PAGE`] as a count.
const NO_PAGE_USIZE: usize = usize::MAX;

/// Names a cluster's dropped lights, counting the ones past the naming limit.
fn dropped_names(labels: &[String]) -> String {
    let (named, rest) = labels.split_at(labels.len().min(NAMED_LIGHTS));
    let names: Vec<&str> = named.iter().map(String::as_str).collect();
    if rest.is_empty() {
        return names.join(", ");
    }
    format!("{} and {} more", names.join(", "), rest.len())
}

/// Packs the three buffers, assigns the shadow pages, and reports what the cap had to drop.
///
/// # Invariants
///
/// The pages are assigned here rather than by the caller because a page index is only
/// meaningful beside the record it belongs to, and a caller that assigned them would have to
/// know the record order as well as the clustering. A light with no page contributes
/// [`NO_PAGE`], which is the value the shading pass tests for.
pub(super) fn pack(
    clustering: &Clustering,
    lights: &[ClusterLight],
    pool: &LightPool,
    mut candidates: Vec<Vec<Candidate>>,
) -> Result<Clusters, CompileError> {
    let cap = clustering.lights_per_cluster;
    let mut counts = PackedBuilder::new(one_number(CLUSTER_COUNT_FIELD));
    let mut indices = PackedBuilder::new(one_number(CLUSTER_LIGHT_FIELD));
    indices.reserve(candidates.len() * cap);
    let mut overflows = Vec::new();
    let mut lights_dropped = 0usize;

    // Which lights reached at least one cluster, read off the walk before the rows below
    // consume it. A light that reached none is lit by nothing, so a page for it would be a
    // page rendering nothing and would take the place of one that does.
    let reached: Vec<bool> = {
        let mut seen = vec![false; lights.len()];
        for reaching in &candidates {
            for candidate in reaching {
                seen[candidate.light] = true;
            }
        }
        seen
    };

    for (cluster, reaching) in candidates.iter_mut().enumerate() {
        // Highest contribution first, and the light's own order for a tie, so that one scene
        // compiles to one set of bytes however the walk above happened to find its lights.
        reaching.sort_by(|first, second| {
            second
                .weight
                .total_cmp(&first.weight)
                .then(first.light.cmp(&second.light))
        });
        let kept = reaching.len().min(cap);
        if reaching.len() > cap {
            overflows.push(Overflow {
                cluster,
                reached: reaching.len(),
                dropped: reaching[cap..]
                    .iter()
                    .map(|candidate| lights[candidate.light].label.clone())
                    .collect(),
            });
            lights_dropped += reaching.len() - cap;
        }
        counts
            .field(CLUSTER_COUNT_FIELD, &[kept as f64])
            .map_err(|error| unpacked(CLUSTER_COUNTS_ENTRY, error))?;
        for slot in 0..cap {
            let light = reaching
                .get(slot)
                .map_or(0.0, |candidate| candidate.light as f64);
            indices
                .field(CLUSTER_LIGHT_FIELD, &[light])
                .map_err(|error| unpacked(CLUSTER_INDICES_ENTRY, error))?;
        }
    }

    let pages = assign_pages(lights, pool, &reached);

    Ok(Clusters {
        records: light_records(lights, &pages)?,
        counts: counts
            .finish()
            .map_err(|error| unpacked(CLUSTER_COUNTS_ENTRY, error))?,
        indices: indices
            .finish()
            .map_err(|error| unpacked(CLUSTER_INDICES_ENTRY, error))?,
        pages,
        reached,
        overflows,
        lights_dropped,
    })
}

/// Gives the pages of a pool to the lights that contribute most, by the same ranking the
/// per cluster cap uses.
///
/// # Invariants
///
/// The rank is [`contribution`] over the distance to the whole view, which is a different
/// question from the one a cluster asks: a cluster asks how bright this light is *there*, and
/// the pool asks how much of the image this light reaches at all. That is why a light can be
/// kept by a cluster and dropped by the pool, and why the two are ranked rather than one
/// derived from the other.
///
/// A light with no page is not a light the pool ranked and rejected for want of room. It is a
/// light that reaches no cluster, a light of a kind that casts no shadow, and a light that is
/// already unshadowed because it is covered by the view. A page for any of those is a page
/// rendering nothing.
fn assign_pages(lights: &[ClusterLight], pool: &LightPool, reached: &[bool]) -> Vec<usize> {
    let mut pages: Vec<usize> = vec![NO_PAGE_USIZE; lights.len()];
    let mut ranked: Vec<(f64, usize)> = Vec::with_capacity(lights.len());
    for (index, light) in lights.iter().enumerate() {
        if !light.clustered || !reached[index] {
            continue;
        }
        ranked.push((
            contribution(light.brightness(), light.radius * light.radius),
            index,
        ));
    }
    // Highest contribution first, and the light's own order for a tie, so that one scene
    // compiles to one set of bytes however the ranking walk happened to find its lights.
    ranked.sort_by(|(first, first_light), (second, second_light)| {
        second.total_cmp(first).then(first_light.cmp(second_light))
    });
    for (page, (_, light)) in ranked.into_iter().take(pool.pages).enumerate() {
        pages[light] = page;
    }
    pages
}

/// How a scene's shadow pages are shaped, as a scene document decides it.
///
/// # Invariants
///
/// A page size and a page count are **fields on the scene**, not constants in this file. A
/// number in the source is a value someone chose once; a number in a document is a value an
/// author chooses per scene, and this project's rule is that a schema can change without
/// recompiling rust. A pool of 512 square `depth32float` pages is a mebibyte each and a pool
/// of 1024 square pages is four, and which one a scene wants is the author's to decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightPool {
    /// The width of one page in pixels.
    pub page_width: i64,
    /// The height of one page in pixels.
    pub page_height: i64,
    /// How many pages the pool holds, which is the depth of the texture array.
    ///
    /// # Invariants
    ///
    /// This is refused above [`MAX_TEXTURE_ARRAY_LAYERS`], which is a floor this compiler
    /// enforces rather than a ceiling the hardware imposes: a scene that asks for more is
    /// refused rather than given a pool it cannot have, because a pool silently smaller than
    /// the one the document asked for is a scene that silently lost the lights it did not
    /// have room for.
    pub pages: usize,
}

impl LightPool {
    /// Returns how many bytes the whole pool occupies at the given page size.
    pub fn total_bytes(&self) -> i64 {
        // `depth32float` is four bytes a texel, which is the format the standard pipeline
        // renders its depth in and the one a page is a page of.
        self.page_width * self.page_height * 4 * self.pages as i64
    }
}

/// Reads the shadow page pool a scene declares, falling back to the convention.
///
/// # Invariants
///
/// The three counts are read out of the **frame** block, beside the width and height of the
/// image and the depth buffer that share them, because a page's size and count are properties
/// of the frame being rendered rather than of the cluster grid a light is sorted into.
///
/// Every count is at least one, for the same reason the cluster grid's are: a pool of nothing
/// is a scene whose every light is lit by the approximation, which is a wrong image rather
/// than a fast one. The page count is additionally refused above the array limit, and the
/// refusal names the number rather than truncating it.
pub fn read_light_pool(scene: &Value) -> Result<LightPool, CompileError> {
    let path = Path::root().key(FRAME);
    Ok(LightPool {
        page_width: pool_count(scene, SHADOW_PAGE_WIDTH, 1024, &path)?,
        page_height: pool_count(scene, SHADOW_PAGE_HEIGHT, 1024, &path)?,
        pages: pool_count(scene, SHADOW_PAGES, 64, &path)? as usize,
    })
}

/// Field holding the width of one shadow page.
pub const SHADOW_PAGE_WIDTH: &str = "shadow_page_width";

/// Field holding the height of one shadow page.
pub const SHADOW_PAGE_HEIGHT: &str = "shadow_page_height";

/// Field holding how many shadow pages the pool holds.
pub const SHADOW_PAGES: &str = "shadow_pages";

/// The deepest texture array a scene is allowed to ask for.
///
/// # Invariants
///
/// This is a **floor this compiler enforces**, not a statement about the hardware. The device
/// requests the adapter's own limits, so `max_texture_array_layers` is whatever the adapter
/// reports and is not this number; what this number does is refuse a document that names a
/// pool deeper than a chain of reasons this project has agreed to stand behind. A scene refused
/// here would be refused on a machine that could have held it, which is the honest failure
/// rather than a texture array that fails to create on the next machine.
pub const MAX_TEXTURE_ARRAY_LAYERS: usize = 256;

/// Reads one count of the pool, refusing a count of nothing.
fn pool_count(
    scene: &Value,
    name: &str,
    convention: i64,
    path: &Path,
) -> Result<i64, CompileError> {
    let Some(declared) = scene
        .as_field_map()
        .and_then(|fields| fields.get(&FRAME.to_owned()))
    else {
        return Ok(convention);
    };
    let Some(fields) = declared.as_field_map() else {
        return Ok(convention);
    };
    let Some(value) = fields.get(name) else {
        return Ok(convention);
    };
    let field = path.key(name);
    let Some(count) = value.as_int() else {
        return Err(CompileError::MalformedScene {
            path: field,
            expected: "a whole number of pixels or pages",
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
    if name == SHADOW_PAGES && count > MAX_TEXTURE_ARRAY_LAYERS as i64 {
        return Err(CompileError::PoolTooDeep {
            path: field,
            pages: count,
            limit: MAX_TEXTURE_ARRAY_LAYERS,
        });
    }
    Ok(count)
}

/// Returns what a host is told about a pool that could not hold every candidate.
///
/// # Invariants
///
/// The note is emitted only for a light that **reached the view and still got no page**. A
/// light that reaches no cluster is not in this count: nothing dropped it, and sending an
/// author to raise a pool that was never the problem is worse than saying nothing. A scene
/// drawn with fewer real shadows than it has candidate lights is a wrong image, and the count
/// of what went missing is the only thing an author can act on.
pub fn pool_notes(
    lights: &[ClusterLight],
    pool: &LightPool,
    assignment: &Clusters,
) -> Vec<CompileNote> {
    let candidates: Vec<usize> = lights
        .iter()
        .enumerate()
        .filter(|(index, light)| light.clustered && assignment.reached[*index])
        .map(|(index, _)| index)
        .collect();
    let without: Vec<&str> = candidates
        .iter()
        .filter(|index| assignment.pages[**index] == NO_PAGE_USIZE)
        .map(|index| lights[*index].label.as_str())
        .collect();
    if without.is_empty() {
        return Vec::new();
    }
    let path = Path::root().key(FRAME).key(SHADOW_PAGES);
    let given = candidates.len() - without.len();
    vec![CompileNote {
        path,
        message: format!(
            "the shadow page pool holds {} pages of {} by {} pixels, which is {} bytes in \
             total, and it was given {given} of the {} lights that reach this view. The other \
             {} are lit by the approximation rather than by a real shadow map, because a page \
             is worth rendering once and only the {given} brightest lights were worth one. Raise \
             `{}` if the scene wants more of them to cast one.",
            pool.pages,
            pool.page_width,
            pool.page_height,
            pool.total_bytes(),
            candidates.len(),
            without.len(),
            SHADOW_PAGES,
        ),
    }]
}

/// Packs one record per light, in the order the scene lists them, which is the order the
/// cluster lists index into.
///
/// # Invariants
///
/// A scene naming **no** light at all still gets one record, written by [`write_record`] from
/// the placeholder values. The buffer is bound by every lighting pass — the binding point is
/// declared unconditionally, so a scene with nothing in the cluster lists still has the
/// binding to satisfy — and the shader declares `array<LightRecord>`, so a buffer of no
/// records is four bytes of alignment where a binding wants a whole ninety six byte record.
/// wgpu refuses that draw with a validation error naming a buffer size, which is a scene of
/// pure environment light failing to render for want of a placeholder.
fn light_records(lights: &[ClusterLight], pages: &[usize]) -> Result<PackedList, CompileError> {
    let layout = graphics::layout::packed_layout(&light_record_schema()).map_err(|error| {
        CompileError::Unpackable {
            path: Path::root().key(graphics::ENTRIES).key(LIGHT_RECORDS_ENTRY),
            reason: error.to_string(),
        }
    })?;
    let mut records = PackedBuilder::new(layout);
    records.reserve(lights.len().max(1));
    for (index, light) in lights.iter().enumerate() {
        write_record(
            &mut records,
            light.colour,
            page_value(pages[index]),
            light.position,
            light.radius,
            light.shadow_projection,
        )?;
    }
    if lights.is_empty() {
        // The placeholder, and why it is the answer it is. A **zero colour** and a **zero
        // radius** at the origin: a light that reaches nothing and lights nothing, so a record
        // that somehow came to be read would add nothing to a fragment rather than carrying a
        // brightness nothing in the document asked for. `NO_PAGE` and the identity are the
        // answers an environment light's **own** record already gives, for the same reason it
        // is never read — it is in no cluster list, so no fragment looks a page up through it.
        // What makes this one unreadable is structural rather than numerical: with an empty
        // light list every cluster's count is zero, so the loop that would load it never runs.
        write_record(&mut records, [0.0; 3], NO_PAGE, [0.0; 3], 0.0, IDENTITY)?;
    }
    records
        .finish()
        .map_err(|error| unpacked(LIGHT_RECORDS_ENTRY, error))
}

/// Writes one record's five fields into a run under construction.
///
/// # Invariants
///
/// The fields are written in the layout's order — `color`, `page`, `position`, `radius`,
/// `shadow_projection` — because the builder refuses a field written out of order, which is
/// what makes the run mean what its layout says. `page` sorts before `position`, so the sorted
/// order and the writing order are the same run.
fn write_record(
    records: &mut PackedBuilder,
    colour: [f64; 3],
    page: f64,
    position: [f64; 3],
    radius: f64,
    projection: Matrix,
) -> Result<(), CompileError> {
    records
        .field(LIGHT_COLOR_FIELD, &colour)
        .and_then(|builder| builder.field(LIGHT_PAGE_FIELD, &[page]))
        .and_then(|builder| builder.field(LIGHT_POSITION_FIELD, &position))
        .and_then(|builder| builder.field(LIGHT_RADIUS_FIELD, &[radius]))
        .and_then(|builder| builder.field(LIGHT_SHADOW_PROJECTION_FIELD, &projection))
        .map_err(|error| unpacked(LIGHT_RECORDS_ENTRY, error))?;
    Ok(())
}

/// Returns the page a record carries, which is minus one for a light with no page.
fn page_value(page: usize) -> f64 {
    if page == NO_PAGE_USIZE {
        NO_PAGE
    } else {
        page as f64
    }
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

/// Builds the binder declaration of a storage buffer holding so many bytes.
///
/// # Invariants
///
/// The declaration carries **no** usage word, because a usage word is a property of the entry
/// in the instance and not of one pass's view of it: the same bytes are a read only storage
/// buffer here whatever else binds them. The entry's own `usage` is what the device reads, and
/// a pass that named a different level than the entry declared would be a pass contradicting
/// the document it binds.
pub fn declaration(entry: &str, size_bytes: i64) -> Value {
    Value::variant(
        "Resource",
        [(
            RESOURCE.to_owned(),
            Value::variant(
                "Buffer",
                [
                    (SOURCE.to_owned(), Value::String(entry.to_owned())),
                    (SIZE_BYTES.to_owned(), Value::Int(size_bytes)),
                ],
            ),
        )],
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
/// The record is a point light's: where it is, what colour it is, the radius it was assigned
/// by, the shadow page it was given, and the projection that finds the page. The fields sort
/// as `color`, `page`, `position`, `radius`, `shadow_projection`, which is the order the
/// packed layout derives from the schema, so the bytes and the layout cannot disagree about
/// a stride.
///
/// The page is in the record because that is the only place a shading loop can reach it from:
/// the loop is over a cluster's light indices, and an index has to resolve to everything the
/// shader needs about that light in one read.
///
/// **The tightly packed record is also the layout a `WGSL` `var<storage>` struct computes,
/// and that is not a coincidence to be relied on by eye.** A three component vector is
/// sixteen byte aligned and twelve bytes long, so the field after it must start four bytes
/// later for the next one to be aligned — and each of this record's two vectors is followed
/// by exactly one `f32`, which is precisely that four bytes. The projection is a matrix at
/// offset thirty two, which is aligned, and the record is ninety six bytes, a whole number of
/// sixteen byte blocks. A field added to this schema that does not keep that property would
/// be read by a shader at the wrong offset, and the packed layout is the one place the bytes
/// are described, so this is where the claim is made.
pub fn light_record_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            LIGHT_COLOR_FIELD.to_owned(),
            graphics::layout::components_schema(3)
                .with_documentation("Colour of the light, scaled by its intensity."),
        ),
        (
            LIGHT_PAGE_FIELD.to_owned(),
            Schema::new(SchemaKind::Float).with_documentation(
                "The shadow page this light was given, or minus one when it was given none and \
                 is lit by the approximation instead.",
            ),
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
        (
            LIGHT_SHADOW_PROJECTION_FIELD.to_owned(),
            graphics::layout::components_schema(16).with_documentation(
                "The light's own view projection, sixteen floats in column major order. A \
                 fragment carries its world position through this matrix to find the light's \
                 page, which is what a shadow lookup is.",
            ),
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
