//! The process-global slab every stage interns into.
//!
//! # Invariant
//!
//! [`intern`] answers the **same id for the same key** for the slab's whole life.
//! A stage's id is the only thing the next stage is named by, so an id that
//! moved between two compiles would change every downstream key and reuse
//! nothing.

use std::collections::HashMap;
use std::sync::{LazyLock, PoisonError, RwLock};

/// What a stage produces, held under its id.
pub enum Held {
    Tree(vegetation::plant::Plant),
    Dressed(vegetation::plant::Plant),
    Wood(Vec<vegetation::expand::Wood>),
    Leaves(Vec<vegetation::leaf::LeafGroup>),
    Drawn(gallery::vegetation_picture::Drawn),
    Document(definition::Value),
}

/// One stage's identity: which stage, which parents, which parameter bits.
///
/// # Invariant
///
/// A key is the stage tag, the parent ids and **every** parameter that can change
/// what the stage computes. A stage that left one bit out would answer a stale
/// value for an edit that bit carries, and no downstream key would move to show
/// it.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Key {
    stage: u8,
    parents: Vec<u64>,
    parameters: Vec<u64>,
}

/// The stage tags, one per stage function.
pub mod stage {
    /// [`crate::tree`].
    pub const TREE: u8 = 1;
    /// [`crate::dressed`].
    pub const DRESSED: u8 = 2;
    /// [`crate::wood`].
    pub const WOOD: u8 = 3;
    /// [`crate::leaves`].
    pub const LEAVES: u8 = 4;
    /// [`crate::drawn`].
    pub const DRAWN: u8 = 5;
    /// [`crate::document`].
    pub const DOCUMENT: u8 = 6;
}

/// The one slab: its index, and each slot's own key beside its value.
struct Slab {
    index: HashMap<Key, u64>,
    held: Vec<(Key, Held)>,
}

/// # Invariant
///
/// One slab per process: the dylib is loaded once and every entry reads this
/// static, so an id means the same value on both sides of the `link` boundary.
static SLAB: LazyLock<RwLock<Slab>> = LazyLock::new(|| {
    RwLock::new(Slab {
        index: HashMap::new(),
        held: Vec::new(),
    })
});

/// The key one stage interns under.
pub fn key(stage: u8, parents: &[u64], parameters: &[u64]) -> Key {
    Key {
        stage,
        parents: parents.to_vec(),
        parameters: parameters.to_vec(),
    }
}

/// The value `stage` interned under `id`, cloned.
///
/// # Invariant
///
/// A dressed id and a tree id are both `usize`, so the stage tag is the only
/// thing that tells them apart; a caller naming the wrong stage is answered
/// `None` rather than another stage's value.
pub fn get(stage: u8, id: u64) -> Option<Held> {
    let slab = SLAB.read().unwrap_or_else(PoisonError::into_inner);
    let (key, held) = slab.held.get(id as usize)?;
    if key.stage != stage {
        return None;
    }
    Some(match held {
        Held::Tree(plant) => Held::Tree(plant.clone()),
        Held::Dressed(plant) => Held::Dressed(plant.clone()),
        Held::Wood(wood) => Held::Wood(wood.clone()),
        Held::Leaves(leaves) => Held::Leaves(leaves.clone()),
        Held::Drawn(drawn) => Held::Drawn(drawn.clone()),
        Held::Document(document) => Held::Document(document.clone()),
    })
}

/// The parent ids the value under `id` was interned from.
pub fn parents(id: u64) -> Vec<u64> {
    let slab = SLAB.read().unwrap_or_else(PoisonError::into_inner);
    slab.held
        .get(id as usize)
        .map_or_else(Vec::new, |(key, _)| key.parents.clone())
}

/// The parameter words the value under `id` was interned with.
pub fn parameters(id: u64) -> Vec<u64> {
    let slab = SLAB.read().unwrap_or_else(PoisonError::into_inner);
    slab.held
        .get(id as usize)
        .map_or_else(Vec::new, |(key, _)| key.parameters.clone())
}

/// Intern `make`'s value under `key`, or answer the id already interned.
///
/// # Invariant
///
/// `make` runs **only** when the key is absent, and it never re-enters the slab:
/// a stage reads its parents with [`get`] and interns its own result with
/// [`intern`], and those are two separate calls.
pub fn intern(key: Key, make: impl FnOnce() -> Held) -> u64 {
    if let Some(id) = lookup(&key) {
        return id;
    }
    let held = make();
    let mut slab = SLAB.write().unwrap_or_else(PoisonError::into_inner);
    // Two callers may have raced the same key; the first id is the one that stands.
    if let Some(id) = slab.index.get(&key) {
        return *id;
    }
    let id = slab.held.len() as u64;
    slab.index.insert(key.clone(), id);
    slab.held.push((key, held));
    id
}

/// The id `key` is interned under, when it is interned.
pub fn lookup(key: &Key) -> Option<u64> {
    let slab = SLAB.read().unwrap_or_else(PoisonError::into_inner);
    slab.index.get(key).copied()
}

/// The structural fingerprint of the document interned under `id`.
///
/// # Invariant
///
/// `0` is what an id that names no document answers, and no live document
/// fingerprints as `0`.
pub fn fingerprint(id: u64) -> u64 {
    match get(stage::DOCUMENT, id) {
        Some(Held::Document(document)) => crate::fingerprint::of(&document),
        _ => 0,
    }
}

/// The document interned under `id`.
pub fn document(id: u64) -> Option<definition::Value> {
    match get(stage::DOCUMENT, id) {
        Some(Held::Document(document)) => Some(document),
        _ => None,
    }
}

/// Empty the slab; the one thing that makes it shrink.
///
/// # Invariant
///
/// Every id handed out before this call is meaningless afterwards, so a host
/// reads its answers before it resets and keeps no id across it.
pub fn reset() {
    let mut slab = SLAB.write().unwrap_or_else(PoisonError::into_inner);
    slab.index.clear();
    slab.held.clear();
}

/// How many values the slab holds.
pub fn len() -> usize {
    SLAB.read()
        .unwrap_or_else(PoisonError::into_inner)
        .held
        .len()
}

/// Whether the slab holds nothing.
pub fn is_empty() -> bool {
    len() == 0
}

/// The key word one `f32` parameter contributes.
pub fn bits(value: f32) -> u64 {
    u64::from(value.to_bits())
}

/// The key word one `usize` parameter contributes.
pub fn count(value: usize) -> u64 {
    value as u64
}
