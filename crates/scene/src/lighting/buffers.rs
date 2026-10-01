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

use graphics::binders::{DATA, SCHEMA, SIZE_BYTES, USAGE};

use crate::CompileError;
use crate::CompileNote;

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

    let pages = assign_pages(lights, pool);

    Ok(Clusters {
        records: light_records(lights, &pages)?,
        counts: counts
            .finish()
            .map_err(|error| unpacked(CLUSTER_COUNTS_ENTRY, error))?,
        indices: indices
            .finish()
            .map_err(|error| unpacked(CLUSTER_INDICES_ENTRY, error))?,
        pages,
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
fn assign_pages(lights: &[ClusterLight], pool: &LightPool) -> Vec<usize> {
    let mut pages: Vec<usize> = vec![NO_PAGE_USIZE; lights.len()];
    let mut ranked: Vec<(f64, usize)> = Vec::with_capacity(lights.len());
    for (index, light) in lights.iter().enumerate() {
        if !light.clustered {
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
    /// This is bounded by `max_texture_array_layers` of 256 on the downlevel limits this
    /// renderer requests. A scene that asks for more is refused rather than given a pool it
    /// cannot have, because a pool silently smaller than the one the document asked for is a
    /// scene that silently lost the lights it did not have room for.
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
/// Every count is at least one, for the same reason the cluster grid's are: a pool of nothing
/// is a scene whose every light is lit by the approximation, which is a wrong image rather
/// than a fast one. The page count is additionally refused above the array limit, and the
/// refusal names the number rather than truncating it.
pub fn read_light_pool(scene: &Value) -> Result<LightPool, CompileError> {
    let path = Path::root().key(super::assignment::CLUSTERING);
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

/// The deepest texture array the downlevel limits this renderer requests allow.
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
        .and_then(|fields| fields.get(&super::assignment::CLUSTERING.to_owned()))
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
/// The note is emitted whenever a light that reaches the view was given no page, whether
/// because the pool filled or because the light reached nothing. A scene drawn with fewer
/// real shadows than it has lights is a wrong image, and the count of what went missing is
/// the only thing an author can act on.
pub fn pool_notes(lights: &[ClusterLight], pool: &LightPool, pages: &[usize]) -> Vec<CompileNote> {
    let without: Vec<&str> = lights
        .iter()
        .enumerate()
        .filter(|(index, light)| light.clustered && pages[*index] == NO_PAGE_USIZE)
        .map(|(_, light)| light.label.as_str())
        .collect();
    if without.is_empty() {
        return Vec::new();
    }
    let path = Path::root()
        .key(super::assignment::CLUSTERING)
        .key(SHADOW_PAGES);
    let given = lights.len() - without.len();
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
            lights.len(),
            without.len(),
            SHADOW_PAGES,
        ),
    }]
}

/// Packs one record per light, in the order the scene lists them, which is the order the
/// cluster lists index into.
fn light_records(lights: &[ClusterLight], pages: &[usize]) -> Result<PackedList, CompileError> {
    let layout = graphics::layout::packed_layout(&light_record_schema()).map_err(|error| {
        CompileError::Unpackable {
            path: Path::root().key(graphics::ENTRIES).key(LIGHT_RECORDS_ENTRY),
            reason: error.to_string(),
        }
    })?;
    let mut records = PackedBuilder::new(layout);
    records.reserve(lights.len());
    for (index, light) in lights.iter().enumerate() {
        // Sorted by name, so the colour is written first and the page second: the builder
        // refuses a field written out of the layout's order, which is what makes the run mean
        // what its layout says. `color`, `page`, `position`, `radius` is that order, because
        // `page` sorts before `position`.
        records
            .field(LIGHT_COLOR_FIELD, &light.colour)
            .and_then(|builder| builder.field(LIGHT_PAGE_FIELD, &[page_value(pages[index])]))
            .and_then(|builder| builder.field(LIGHT_POSITION_FIELD, &light.position))
            .and_then(|builder| builder.field(LIGHT_RADIUS_FIELD, &[light.radius]))
            .map_err(|error| unpacked(LIGHT_RECORDS_ENTRY, error))?;
    }
    records
        .finish()
        .map_err(|error| unpacked(LIGHT_RECORDS_ENTRY, error))
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
/// The record is a point light's: where it is, what colour it is, the radius it was assigned by,
/// and the shadow page it was given. The fields sort as `color`, `page`, `position`, `radius`,
/// which is the order the packed layout derives from the schema, so the bytes and the layout
/// cannot disagree about a stride.
///
/// The page is in the record because that is the only place a shading loop can reach it from:
/// the loop is over a cluster's light indices, and an index has to resolve to everything the
/// shader needs about that light in one read.
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
