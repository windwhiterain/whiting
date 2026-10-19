//! Whiting's alpine-oak plant scene as six cacheable stages.
//!
//! # The vertical this crate is
//!
//! A lichen program draws a whole *Quercus aquifolioides* with six `@cache`
//! cells, one per stage, and a Whiting edit re-runs only the stages its own
//! parameter reaches. Nothing of Whiting is a native op inside the language: the
//! stages live in a generated `dylib`, the program reaches them through
//! [`lichen_language::link`]'s `External` operator, and every value that crosses
//! the boundary is a `usize`.
//!
//! | stage | reads | produces |
//! |---|---|---|
//! | [`tree`] | `seed` | the grown skeleton |
//! | [`dressed`] | `tree`, `envelope` | the fluted bole under the crown's guard |
//! | [`wood`] | `dressed`, `stations`, `around`, `spacing` | one closed solid per node |
//! | [`leaves`] | `dressed`, `density` | one group per leafed node |
//! | [`drawn`] | `wood`, `leaves`, `index` | the expansion, the ids, the decorators |
//! | [`document`] | `drawn`, key-light position | the scene document |
//!
//! # Why an opaque intermediate is a `usize`
//!
//! `lichen_highlevel::ValueExt` is `Debug + Copy + PartialEq`, so a lichen value
//! cannot hold an `Arc`, a `Box` or any other handle: a lichen value is copied by
//! the engine on every read and compared by value on every key. A `Plant` is
//! megabytes of curves and a `Drawn` holds a whole expansion, so neither can be a
//! lichen value. **The boundary therefore carries an id and the slab carries the
//! object**, and that is the whole of the design's one hard constraint.
//!
//! # The slab, and the requirement it imposes
//!
//! [`slab::intern`] is content-addressed: the key is the stage tag, the parent
//! ids and every parameter bit, and the same key answers the same id for as long
//! as the process lives. **This is not an optimisation — it is what makes the
//! vertical work.** A stage's id is the only name the next stage is given, so the
//! next stage's key is a function of it. An id that moved between two compiles of
//! the same source would move every downstream key with it, every `@cache` cell
//! would miss, and nothing downstream of the first recompute would ever be
//! reused. The id is stable because the slab is never reordered and never
//! renumbered: an id is a position in a vector that only ever grows at the end.
//!
//! # What the design gives up
//!
//! - **No persistence across processes.** The slab is a `static` inside the loaded
//!   dylib, so a second process starts empty and re-runs every stage. That is
//!   sound — interning is content-addressed, so a cold process reaches the same
//!   *answers* — but it is not a cache: nothing is carried between runs.
//! - **The slab only grows.** An edit that lowers `stations` leaves the old wood
//!   interned under its own key forever, because a key that some other cell may
//!   still name cannot be dropped. [`slab::reset`] exists so a probe can measure
//!   a cold build, and nothing else should call it: it invalidates every id
//!   already handed out.
//! - **A stage is atomic to the cache.** `wood` at `stations: 9` is one key, so a
//!   cell that depends on it is rebuilt whole; there is no partial reuse inside a
//!   stage, and there cannot be, because the stage's output is a `Vec<Wood>` the
//!   next stage names as a whole.
//! - **Two processes are two worlds.** An id means nothing outside the process
//!   that interned it, so a host may compare ids only within one process and
//!   must compare [`fingerprint`]s across two.
//!
//! # The library this crate generates
//!
//! [`library`] is the [`LibrarySource`] the generator renders into
//! `lichen-whiting-link`: the wrapper's lichen source, the bridge over the
//! argument array, and one entry per family. See docs/notes/native-link-library.md
//! §15.

pub mod fingerprint;
pub mod library;
pub mod slab;
pub mod stages;

pub use slab::Held;
pub use stages::{document, drawn, dressed, leaves, tree, wood};

/// The oak's whole-tree plate, as the six stages compose it.
///
/// # Invariant
///
/// The identity holds only for the **unedited** oak: `alpine_oak(SEED)` is
/// `alpine_oak_species(dress(tree(SEED)))`, and any stage parameter that differs
/// from its baseline is a different plant by construction.
pub const COMPOSITION: &str =
    "alpine_oak(seed) == alpine_oak_species(alpine_oak_dress(alpine_oak_tree(seed)))";
