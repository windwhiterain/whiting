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
use crate::DeviceLimits;
use crate::limits::clamp_to_ceiling;
use crate::standard::FRAME;
use crate::standard::math::{IDENTITY, Matrix};

use super::assignment::{Candidate, ClusterLight, Clustering, Overflow, contribution};

/// Name of the instance entry holding one packed record per light.
pub const LIGHT_RECORDS_ENTRY: &str = "light_records";

/// Name of the instance entry holding how many lights each cluster kept.
pub const CLUSTER_COUNTS_ENTRY: &str = "cluster_light_counts";

/// Name of the instance entry holding the light of every slot of every cluster.
pub const CLUSTER_INDICES_ENTRY: &str = "cluster_light_indices";

/// Name of the instance entry holding the cone of every light.
///
/// # Invariants
///
/// It is a run of its own rather than three more fields of the light record, because the record
/// is read by every fragment of every clustered pass and a point light is most of what a scene
/// draws: thirty two bytes a light that most scenes never read is a cost every scene pays. The
/// cone is the one thing about a light that a fragment reads only when the light has one.
pub const SPOT_CONES_ENTRY: &str = "spot_cones";

/// Field of the cone record naming the axis the light shines along.
pub const CONE_AXIS_FIELD: &str = "axis";

/// Field of the cone record naming the cone's two limits.
pub const CONE_LIMITS_FIELD: &str = "limits";

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

/// Field of the light record naming where a light's **virtual** shadow pages live.
///
/// # Invariants
///
/// It is a `vec4` and not two `f32`s for the reason [`light_record_schema`] states: the record
/// has to stay a whole number of sixteen byte blocks, and the two numbers a virtual lookup
/// needs — which section of the page table is this light's, and which layer of the atlas holds
/// its pages — are eight bytes with eight more of pad beside them whether they are wanted or
/// not. `.x` is the word offset, `.y` the layer, and the last two components are that pad, so
/// they carry [`NO_PAGE`] too rather than zero: a shader that reads one of them by mistake gets
/// the sentinel rather than the first page.
///
/// A light drawn on the **pool** rather than on an atlas — the kinds whose virtual grid has not
/// landed yet — carries [`NO_PAGE`] in both, which is the same sentinel `page` already says and
/// means the same thing here: this record is not to be looked up that way.
pub const LIGHT_VIRTUAL_FIELD: &str = "virtual_page";

/// How many floats one light record is wide.
///
/// # Invariants
///
/// It is derived from [`light_record_schema`] rather than written down, because a stride the
/// schema and the packing could disagree about is a stride that reads the wrong light's
/// colour. The count is `3 + 1 + 3 + 1 + 16 + 4`: colour, page, position, radius, the
/// projection and the virtual page address.
pub fn light_record_stride() -> usize {
    28
}

/// The word a shadow page table begins at, in the buffer [`Clusters`] packs.
///
/// # Invariants
///
/// A record carries its light's table offset **absolute**, because a shader that had to add a
/// base of its own would have to be told it and the whole point of an index inside a record is
/// that nothing beside it has to be true. That base is the length of the record run, which the
/// emitter has to know before the run exists — the offset is a field of a record — so this is
/// the same arithmetic done from the one number that decides it, and
/// [`Clusters::attach_shadow_table`] checks the two answers against each other.
pub fn record_table_base(lights: usize) -> u32 {
    (lights.max(1) * light_record_stride()) as u32
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
    /// How many of the leading records are **global** lights.
    ///
    /// # Invariants
    ///
    /// A global light is one every fragment evaluates whether or not its cluster holds it, so
    /// a shading pass reaches it by **index** rather than through a cluster list — and an
    /// index has to mean the same thing to the compiler and to the shader, which is what
    /// writing the global records first buys: the first `globals` records of the run are the
    /// global ones, in the order the compiler built the list, and no cluster entry and no
    /// shadow page can disagree with that because they are all indexed by the same number.
    ///
    /// It is counted here rather than passed in because the run is written here: this is the
    /// one place that knows how many records there are, and a count handed to it could be a
    /// count of something other than what it packed.
    pub globals: usize,
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
    /// The lights' **shadow page table**, appended to the record run rather than bound beside it.
    ///
    /// # Why it shares a buffer with the records
    ///
    /// A clustered shading pass already binds four storage buffers — records, counts, indices,
    /// cones — and [`crate::lighting`] states that four is the whole budget a downlevel device
    /// admits to, with the cone run being the one that spent it. A fifth would be a document
    /// that validates against a schema and fails on a driver's limit, and the page table is
    /// read by precisely the pass with no room left in it. So it rides on the end of the records
    /// it indexes into, and a record's `virtual_page` is a word offset into that one buffer
    /// rather than into a table of its own.
    ///
    /// Empty for a scene no light of which is drawn on an atlas, which leaves the record run the
    /// size it always was.
    pub table: Vec<u8>,
}

impl Clusters {
    /// Puts a shadow page table at the end of the record run and returns the word it begins at.
    ///
    /// # Invariants
    ///
    /// The base is read off the records that were actually packed rather than recomputed from
    /// the light count, because the offset a record carries and the bytes the shader walks have
    /// to agree to the word, and the one place that knows where the records end is the run
    /// itself. A caller that has to write an offset **before** packing — which is the only kind
    /// of caller there is, since the offset is a field of a record — is given
    /// [`record_table_base`] to ask the same question in advance, and this is where the two
    /// answers are checked against each other.
    pub fn attach_shadow_table(&mut self, words: &[u32], base: u32) {
        debug_assert_eq!(
            self.records.bytes().len() / 4,
            base as usize,
            "the page table was told to start at word {base}, which is not where the records \
             it indexes into end"
        );
        self.table = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    }

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
                storage(&self.record_bytes(), &light_record_schema()),
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

    /// Returns the record run with the shadow page table on the end of it.
    ///
    /// One function owns the concatenation because the entry and the declaration beside it are
    /// two views of the same bytes, and a size that came from one and not the other is a pass
    /// that binds a buffer of a length the instance never had.
    fn record_bytes(&self) -> Vec<u8> {
        let mut bytes = self.records.bytes().to_vec();
        bytes.extend_from_slice(&self.table);
        bytes
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
                declaration(LIGHT_RECORDS_ENTRY, self.record_bytes().len() as i64),
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
    /// Nothing is dropped without being **named**. A scene lit less than it was drawn is a
    /// wrong image nobody sees until the render finishes, and a count alone is not enough to
    /// act on: an author told that six thousand entries are missing cannot tell which of their
    /// fifty lights is the one to move. So the note carries three things — how many clusters
    /// overflowed, how many entries are missing, and which lights are behind all of it.
    ///
    /// It is **one** note rather than one per overflowing cluster, and the reason is that a
    /// cluster is not something the author can act on. The cluster index is an interior of the
    /// grid they configured, and they can recompute it; the light's **label** is the one string
    /// in the document they can go and edit. A grid of sixteen by nine by twenty four has three
    /// thousand four hundred and fifty six clusters, and a cap low enough for them all to
    /// overflow put a note per cluster — thousands of lines, nearly all of them naming the same
    /// two or three lights, which is a report that buries the fact it exists to state.
    ///
    /// The labels are the **distinct** ones across every cluster, sorted, and bounded by
    /// [`NAMED_LIGHTS`] with the rest counted: a scene that overflows is a scene where a few
    /// lights are crowded out everywhere, so the distinct set is small, and it is small by
    /// exactly the mechanism that produced the overflow.
    pub fn notes(&self, clustering: &Clustering, path: &Path) -> Vec<CompileNote> {
        if self.overflows.is_empty() {
            return Vec::new();
        }
        let cap = path.key(super::assignment::LIGHTS_PER_CLUSTER);
        // The distinct labels, sorted so that one scene compiles to one note. The order they
        // are first seen in is the order the clusters happen to be walked, and two clusters
        // that dropped the same light would put it in two different places otherwise.
        let mut labels: Vec<&str> = self
            .overflows
            .iter()
            .flat_map(|overflow| overflow.dropped.iter().map(String::as_str))
            .collect();
        labels.sort_unstable();
        labels.dedup();
        vec![CompileNote {
            path: cap.clone(),
            message: format!(
                "{} of the {} clusters of this view had more lights reach them than the cap of {} \
                 keeps, and {} of the light entries in their lists are missing. The lights that \
                 contribute most are the ones kept, so this view is lit less than the scene was \
                 drawn. The lights that never made it in, {} of them, are {}: raise `{}`, or the \
                 grid, or move the lights.",
                self.overflows.len(),
                clustering.cluster_count(),
                clustering.lights_per_cluster,
                self.lights_dropped,
                labels.len(),
                names_of(&labels),
                super::assignment::LIGHTS_PER_CLUSTER,
            ),
        }]
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

/// Names the lights that never made it in, counting the ones past the naming limit.
///
/// # Invariants
///
/// Each name is in backticks because the whole point of putting it in the message is that it is
/// a string the author can find in their document, and a bare word in a sentence of prose is
/// not a string they can go and edit. They arrive already **distinct** and sorted, which is why
/// this takes borrowed names and not a run of owned ones.
fn names_of(labels: &[&str]) -> String {
    let (named, rest) = labels.split_at(labels.len().min(NAMED_LIGHTS));
    let names: Vec<String> = named.iter().map(|label| format!("`{label}`")).collect();
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
    // The global lights are the leading run, and the count is how long it is. A global light
    // after a clustered one would be a record a shading pass never reaches, and a clustered
    // light inside the global range would be a light counted twice — one by the unconditional
    // loop and once by every cluster it reaches. The order is the compiler's to keep; this is
    // the one place both facts are read, so the assertion is where a compiler that broke it
    // is caught rather than in a frame.
    let globals = lights.iter().take_while(|light| light.global).count();
    debug_assert!(
        lights[globals..].iter().all(|light| !light.global),
        "the global lights are the first of the run"
    );

    Ok(Clusters {
        records: light_records(lights, &pages)?,
        globals,
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
        table: Vec::new(),
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
/// A light with no page is not a light the pool ranked and rejected for want of room. It is
/// a light that reaches no cluster, or a light of a kind that casts no shadow.
///
/// A **global** light is ranked as though it stood at the view itself, because that is what a
/// global light is: it names no reach, so its record carries a radius of zero, and a zero
/// distance is the brightest the ranking can answer. A light that named a reach and was
/// promoted for containing the whole scene keeps the radius it named, so it is ranked by the
/// distance it would have been culled at — the same number as before it was promoted, which
/// is what keeps a promotion from moving a page between two lights.
///
/// The two are asked apart by [`ClusterLight::wants_page`], and the second half of that is the
/// one a directional light turns on: it is exempt from every cluster, so the walk never
/// records it as having reached one, and a test that only looked at the walk would hand a sun
/// no page and the scene would render with no shadow in it and nothing to say so.
fn assign_pages(lights: &[ClusterLight], pool: &LightPool, reached: &[bool]) -> Vec<usize> {
    let mut pages: Vec<usize> = vec![NO_PAGE_USIZE; lights.len()];
    let mut ranked: Vec<(f64, usize)> = Vec::with_capacity(lights.len());
    for (index, light) in lights.iter().enumerate() {
        if !light.wants_page(reached[index]) {
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
    /// This is the depth the pool was **given**, which is not always the depth the document
    /// asked for: a pool deeper than [`DeviceLimits::max_texture_array_layers`] is given the
    /// ceiling and a [`CompileNote`] naming the loss. The note is what makes a smaller frame
    /// different from a wrong one, so a host that ignores the notes is rendering a scene that
    /// is quietly missing the lights it could not give a page.
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

/// Reads the shadow page pool a scene declares, bounding it to what the machine can hold.
///
/// # Invariants
///
/// The three counts are read out of the **frame** block, beside the width and height of the
/// image and the depth buffer that share them, because a page's size and count are properties
/// of the frame being rendered rather than of the cluster grid a light is sorted into.
///
/// Every count is at least one, for the same reason the cluster grid's are: a pool of nothing
/// is a scene whose every light is lit by the approximation, which is a wrong image rather
/// than a fast one, and it is wrong on every machine. A count **above** what the machine has
/// is the opposite case and is bounded rather than refused: the pool is given the ceiling and
/// a [`CompileNote`] names the number that was lost, so the frame still draws and the author
/// is told what it drew without.
///
/// The notes are returned beside the pool rather than pushed into a pool that holds numbers,
/// because a note is a fact about the compile and the pool is a fact about the resources.
pub fn read_light_pool(
    scene: &Value,
    limits: DeviceLimits,
) -> Result<(LightPool, Vec<CompileNote>), CompileError> {
    let path = Path::root().key(FRAME);
    let mut notes = Vec::new();
    let side = limits.texture_side_ceiling();
    let layers = limits.texture_layer_ceiling();
    let page_width = pool_count(
        scene,
        SHADOW_PAGE_WIDTH,
        1024,
        side,
        PAGE_SIDE_CONSEQUENCE,
        &path,
        &mut notes,
    )?;
    let page_height = pool_count(
        scene,
        SHADOW_PAGE_HEIGHT,
        1024,
        side,
        PAGE_SIDE_CONSEQUENCE,
        &path,
        &mut notes,
    )?;
    let pages = pool_count(
        scene,
        SHADOW_PAGES,
        64,
        layers,
        PAGE_COUNT_CONSEQUENCE,
        &path,
        &mut notes,
    )?;
    Ok((
        LightPool {
            page_width,
            page_height,
            pages: pages as usize,
        },
        notes,
    ))
}

/// How large the atlas a light's virtual shadow pages are drawn into may be, as a scene
/// document decides it.
///
/// # Invariants
///
/// These are two **ceilings** rather than two sizes. A page is `PAGE_SIZE` texels and a light
/// is given exactly the pages its casters ask for, so the atlas is filled to whatever the scene
/// needs and no more; what these two numbers bound is the point at which a scene's ask stops
/// being something this machine can hold, which is a refusal rather than a clamp. That is the
/// opposite of the pool's rule, and deliberately so: a pool that is too small still draws every
/// light it has room for, whereas an atlas that is too small would have to drop pages, and a
/// dropped page is a shadow that silently disappears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowAtlas {
    /// How many pages wide and tall one layer may be.
    pub side_pages: i64,
    /// How many lights the atlas may hold layers for.
    pub layers: i64,
}

impl ShadowAtlas {
    /// The texel side of one layer, which is what the depth array is created at.
    pub fn side_texels(&self) -> i64 {
        self.side_pages * crate::vshadow::PAGE_SIZE as i64
    }
}

/// What a scene that names no atlas is given, in pages.
///
/// # Why sixteen rather than eight
///
/// A light at the density the pool used to give it — one page of a thousand texels across the
/// column it shadows — is sized to a grid of **eight pages per side**, and a caster that fills that
/// grid is drawn into its own level and every coarser one, which is `64 + 16 + 4 + 1` pages. A
/// ceiling of eight would refuse the very first light the replacement was built for, at exactly
/// the resolution the scene already had, and the whole point of a ceiling is that a scene only
/// meets it by asking for more than it had.
const ATLAS_SIDE_PAGES: i64 = 16;

/// How many lights an atlas that names no count holds layers for.
const ATLAS_LAYERS: i64 = 8;

/// Reads the shadow atlas a scene declares, bounding it to what the machine can hold.
///
/// # Invariants
///
/// The side is bounded in **pages** rather than in texels, because the number an author raises
/// is the one the allocator compares against, and a ceiling that arrives already divided by a
/// page size cannot disagree with the divide the allocator does.
///
/// Both counts are bounded rather than refused, which is the one half of the atlas that is not
/// a refusal: a scene asking for a layer deeper than any adapter can make is asking for a
/// number on the wrong machine, and the note says so. A scene whose *pages* do not fit inside
/// these bounds is refused by [`crate::vshadow`], because that is a document no machine could
/// draw rather than a machine too small for this one.
pub fn read_shadow_atlas(
    scene: &Value,
    limits: DeviceLimits,
) -> Result<(ShadowAtlas, Vec<CompileNote>), CompileError> {
    let path = Path::root().key(FRAME);
    let mut notes = Vec::new();
    let page = crate::vshadow::PAGE_SIZE as i64;
    let side_pages = pool_count(
        scene,
        SHADOW_ATLAS_SIDE_PAGES,
        ATLAS_SIDE_PAGES,
        (limits.texture_side_ceiling() / page).max(1),
        "Every page of a light whose atlas would not fit at this size is refused rather than \
         dropped, and the compile error names the light whose demand passed it.",
        &path,
        &mut notes,
    )?;
    let layers = pool_count(
        scene,
        SHADOW_ATLAS_LAYERS,
        ATLAS_LAYERS,
        limits.texture_layer_ceiling(),
        "A spot light past the last layer is refused by name, so the scene does not draw a \
         shadowless lamp and say nothing about it.",
        &path,
        &mut notes,
    )?;
    Ok((ShadowAtlas { side_pages, layers }, notes))
}

/// What a page drawn at the size given rather than the size asked for costs a frame.
const PAGE_SIDE_CONSEQUENCE: &str = "Every page in the pool is rendered at the size given rather than the size asked for, so \
     every shadow in the frame is coarser than this document says.";
/// What a page count given rather than asked for costs a frame.
const PAGE_COUNT_CONSEQUENCE: &str = "A light that got no page is lit by the approximation rather than by a real shadow map, \
     and the pool overflow note names which of them those are.";

/// Field holding the width of one shadow page.
pub const SHADOW_PAGE_WIDTH: &str = "shadow_page_width";

/// Field holding the height of one shadow page.
pub const SHADOW_PAGE_HEIGHT: &str = "shadow_page_height";

/// Field holding how many shadow pages the pool holds.
pub const SHADOW_PAGES: &str = "shadow_pages";

/// Field holding how many pages wide the shadow atlas is.
pub const SHADOW_ATLAS_SIDE_PAGES: &str = "shadow_atlas_side_pages";

/// Field holding how many atlas layers the machine is asked for.
pub const SHADOW_ATLAS_LAYERS: &str = "shadow_atlas_layers";

/// Reads one count of the pool, refusing a count of nothing and bounding it to the machine.
fn pool_count(
    scene: &Value,
    name: &str,
    convention: i64,
    ceiling: i64,
    consequence: &str,
    path: &Path,
    notes: &mut Vec<CompileNote>,
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
    Ok(clamp_to_ceiling(
        count,
        ceiling,
        name,
        field,
        consequence,
        notes,
    ))
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
///
/// A light drawn as **virtual pages** is out of the count as well, for the same reason in the
/// other direction: it was refused a pool page on purpose, and a note telling the reader to
/// raise `shadow_pages` for it points at a number that would change nothing.
pub fn pool_notes(
    lights: &[ClusterLight],
    pool: &LightPool,
    assignment: &Clusters,
) -> Vec<CompileNote> {
    let candidates: Vec<usize> = lights
        .iter()
        .enumerate()
        .filter(|(index, light)| {
            light.clustered && !light.virtual_shadow && assignment.reached[*index]
        })
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
            light.virtual_page,
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
        write_record(
            &mut records,
            [0.0; 3],
            NO_PAGE,
            [0.0; 3],
            0.0,
            IDENTITY,
            [NO_PAGE; 4],
        )?;
    }
    records
        .finish()
        .map_err(|error| unpacked(LIGHT_RECORDS_ENTRY, error))
}

/// Writes one record's six fields into a run under construction.
///
/// # Invariants
///
/// The fields are written in the layout's order — `color`, `page`, `position`, `radius`,
/// `shadow_projection`, `virtual_page` — because the builder refuses a field written out of
/// order, which is what makes the run mean what its layout says. `page` sorts before
/// `position`, so the sorted order and the writing order are the same run.
fn write_record(
    records: &mut PackedBuilder,
    colour: [f64; 3],
    page: f64,
    position: [f64; 3],
    radius: f64,
    projection: Matrix,
    virtual_page: [f64; 4],
) -> Result<(), CompileError> {
    records
        .field(LIGHT_COLOR_FIELD, &colour)
        .and_then(|builder| builder.field(LIGHT_PAGE_FIELD, &[page]))
        .and_then(|builder| builder.field(LIGHT_POSITION_FIELD, &position))
        .and_then(|builder| builder.field(LIGHT_RADIUS_FIELD, &[radius]))
        .and_then(|builder| builder.field(LIGHT_SHADOW_PROJECTION_FIELD, &projection))
        .and_then(|builder| builder.field(LIGHT_VIRTUAL_FIELD, &virtual_page))
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
///
/// # Invariants
///
/// It is public because not every read only storage run belongs to an assignment: a run that
/// is a property of the light rather than of which clusters reach it is packed by the standard
/// scene compiler, which owns the document the light came from. What it is not allowed to be
/// is a **second** way of building one of these — the payload and the declaration beside it
/// are two views of the same run, and a rule written twice is a rule that can be applied once.
pub fn storage(bytes: &[u8], schema: &Schema) -> Value {
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
/// by, the shadow page it was given, the projection that finds the page, and where its
/// virtual pages are. The fields sort as `color`, `page`, `position`, `radius`,
/// `shadow_projection`, `virtual_page`, which is the order the packed layout derives from the
/// schema, so the bytes and the layout cannot disagree about a stride.
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
/// offset thirty two, which is aligned, and the record is then a hundred bytes with twelve of
/// pad to add; the virtual address beside it is a `vec4`, so the record is **one hundred and
/// twelve** bytes, a whole number of sixteen byte blocks. A field added to this schema that
/// does not keep that property would be read by a shader at the wrong offset, and the packed
/// layout is the one place the bytes are described, so this is where the claim is made.
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
        (
            LIGHT_VIRTUAL_FIELD.to_owned(),
            graphics::layout::components_schema(4).with_documentation(
                "Where this light's virtual shadow pages are: the word its page table section \
                 starts at, and the atlas layer its pages live in. Either is minus one for a \
                 light drawn on the pool rather than on an atlas.",
            ),
        ),
    ]))
    .with_documentation("One light, as a clustered shading pass reads it.")
}

/// One light's cone, as a document states it.
///
/// # Invariants
///
/// Every light has one, so the run has one record per light whatever the kinds are: a kind
/// that has no cone carries the whole sphere rather than an absent field, because a cluster
/// index is a plain index into this run and a run with a hole in it is a run a fragment has
/// to test before it can read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotCone {
    /// The unit direction the light shines along, in world space.
    pub axis: [f64; 3],
    /// The half angle of the cone the light is at full strength within, in radians.
    pub inner_angle: f64,
    /// The half angle of the cone the light has fallen to nothing at, in radians.
    pub outer_angle: f64,
}

impl SpotCone {
    /// Returns the cone of a light that shines in every direction, which is a point light.
    ///
    /// Both angles are a half turn, so the two cosines the shader compares are equal and the
    /// run answers "the whole sphere" for every direction. The shader recognises that shape
    /// by the equality rather than by the value, so the answer does not rest on a cosine of
    /// a half turn being exactly minus one.
    pub fn whole_sphere() -> Self {
        Self {
            // The axis of a cone that is the whole sphere is never compared against, because
            // the two limits are equal and the shader answers before it reads one.
            axis: [0.0, 0.0, 1.0],
            inner_angle: std::f64::consts::PI,
            outer_angle: std::f64::consts::PI,
        }
    }

    /// Returns how far from the light a cone of this shape reaches into a scene whose shadowed
    /// volume is of half width `shadow_extent`.
    ///
    /// # Invariants
    ///
    /// This is the distance to the **rim** of the volume's base, and it is the radius the
    /// cluster assignment has to cull a spot light by. A cone of half angle `θ` reaches a disc
    /// of half width `e` at a distance `e / tan(θ)` along its axis, and the rim of that disc
    /// is `e / sin(θ)` away — the hypotenuse, which is strictly further than the depth and is
    /// the only radius whose sphere **contains** the volume.
    ///
    /// Containment is the whole requirement, and it is why this is not the depth. A sphere of
    /// the depth alone would drop the light from the clusters at the edge of its own cone,
    /// which is a light that stops lighting where it is still shining.
    pub fn reach(&self, shadow_extent: f64) -> f64 {
        (shadow_extent.max(0.0) / self.outer_angle.sin().max(1e-6)).max(shadow_extent)
    }
}

/// Builds the schema of one light's cone, which is what a clustered shading pass scales that
/// light by.
///
/// # Invariants
///
/// The fields sort as `axis` then `limits`, which is the order the packed layout derives from
/// the schema and the order the WGSL struct is written in, so the two agree about every byte.
///
/// **Two vectors of four components is the shape this record has to be, and the reason is the
/// same one the light record's own documentation gives.** A `WGSL` `array<T>` in a storage
/// buffer lays its elements out at `roundUp(align_of_T, size_of_T)`, and a struct holding a
/// vector is aligned to sixteen bytes — so a record of the three components of a direction and
/// two angles is twenty bytes packed and **thirty two** to a shader, and a shader reading the
/// tightly packed run would read the second light's direction as the first light's outer angle.
/// Thirty two bytes is the smallest record that is a whole number of sixteen byte blocks, and
/// these two vectors are thirty two: `axis` carries the unit direction with its fourth
/// component reserved, and `limits` carries the cosine of the inner half angle and the cosine
/// of the outer one with two components reserved.
///
/// The two limits are cosines rather than the angles themselves because a shader compares a
/// dot product against them and needs no inverse cosine to do it, and because the falloff
/// between the two is then a straight line in the number being compared. An angle and its
/// cosine are not both stored: a record carrying a number and the number derived from it is a
/// record with two answers to one question, which is the shape this project's `NO_PAGE`
/// sentinel exists to avoid.
pub fn spot_cone_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            CONE_AXIS_FIELD.to_owned(),
            graphics::layout::components_schema(4).with_documentation(
                "The unit direction the light shines along, in xyz. The fourth component is \
                 reserved, and is what puts `limits` on its own sixteen byte boundary.",
            ),
        ),
        (
            CONE_LIMITS_FIELD.to_owned(),
            graphics::layout::components_schema(4).with_documentation(
                "The cosine of the inner half angle in x and the cosine of the outer one in y, \
                 so a fragment scales the light by how its direction falls between them. Two \
                 equal cosines are the whole sphere, which is what a light that is not a spot \
                 light carries. The last two components are reserved.",
            ),
        ),
    ]))
    .with_documentation("One light's cone, as a clustered shading pass reads it.")
}

/// Packs one cone per light, in the order the scene lists them, which is the order the light
/// record buffer is indexed by.
///
/// # Invariants
///
/// The two runs have **one record each per light, in the same order**, so a cluster's index
/// is an index into both and a fragment reads a light's record and its cone without a second
/// lookup or a test on whether one exists.
///
/// A scene naming **no** light at all still gets one cone, by the same argument
/// [`light_records`] gives for its placeholder: the buffer is bound by every lighting pass,
/// and the shader declares `array<SpotCone>`, so a run of no records is a binding wgpu
/// refuses for want of a whole thirty two byte element.
pub fn spot_cones(lights: &[SpotCone]) -> Result<PackedList, CompileError> {
    let layout = graphics::layout::packed_layout(&spot_cone_schema()).map_err(|error| {
        CompileError::Unpackable {
            path: Path::root().key(graphics::ENTRIES).key(SPOT_CONES_ENTRY),
            reason: error.to_string(),
        }
    })?;
    let mut cones = PackedBuilder::new(layout);
    cones.reserve(lights.len().max(1));
    for cone in lights {
        write_cone(&mut cones, cone)?;
    }
    if lights.is_empty() {
        write_cone(&mut cones, &SpotCone::whole_sphere())?;
    }
    cones
        .finish()
        .map_err(|error| unpacked(SPOT_CONES_ENTRY, error))
}

/// Writes one cone's two fields into a run under construction.
///
/// # Invariants
///
/// The fields are written in the layout's order — `axis` then `limits` — because the builder
/// refuses a field written out of order, which is what makes the run mean what its layout
/// says. The three reserved components are written rather than skipped, so the run is as long
/// as the layout says it is rather than as long as the real data is.
fn write_cone(cones: &mut PackedBuilder, cone: &SpotCone) -> Result<(), CompileError> {
    let axis = [cone.axis[0], cone.axis[1], cone.axis[2], 0.0];
    let limits = [cone.inner_angle.cos(), cone.outer_angle.cos(), 0.0, 0.0];
    cones
        .field(CONE_AXIS_FIELD, &axis)
        .and_then(|builder| builder.field(CONE_LIMITS_FIELD, &limits))
        .map_err(|error| unpacked(SPOT_CONES_ENTRY, error))?;
    Ok(())
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
