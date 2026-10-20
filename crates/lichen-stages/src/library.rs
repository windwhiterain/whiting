//! The [`LibrarySource`] the generator renders into the dylib.
//!
//! # Three hands
//!
//! The **wrapper** is lichen source the loader splices or binds, the **bridge**
//! is the author's Rust that reads the call's argument array, and the **entries**
//! are one thin body per family. Everything structural — the ABI, the `item`
//! helper, the manifest, the program gate — is [`LibrarySource`]'s own, so a
//! generator author writes the mapping and nothing else.
//!
//! # Two families, one wrapper
//!
//! The wrapper is **one namespace for every family the dylib carries**, so a binding is named
//! after the scene it belongs to (`tree`, `dressed`, … for the oak; `hall_plan`, `hall_built`,
//! `hall_parts`, `hall_document` for the hall) rather than after its stage. The hall's `parts` and
//! `document` would otherwise be a second `wood` and a second `document` in one program.

use lichen_language::link::{Entry, LibrarySource};

/// The lichen wrapper the manifest carries: one binding per family.
///
/// # Invariant
///
/// **A binding is one line at column zero with spaces around its `=`**, because
/// the manifest's `bind` list is read from exactly that shape and a length
/// mismatch against the build's statements is refused rather than paired wrongly.
/// The multi-argument families are curried, because the language writes a
/// parameter's type in the parameter position and one parameter at a time.
pub const WRAPPER: &str = "\
tree = seed: Int => {r = $whiting_tree(seed); r: Int; r}
dressed = tree: Int => envelope: Float => {r = $whiting_dressed(tree, envelope); r: Int; r}
wood = wood: Int => stations: Int => around: Int => spacing: Float => {r = $whiting_wood(wood, stations, around, spacing); r: Int; r}
leaves = leaves: Int => density: Float => {r = $whiting_leaves(leaves, density); r: Int; r}
drawn = wood: Int => leaves: Int => index: Int => {r = $whiting_drawn(wood, leaves, index); r: Int; r}
document = drawn: Int => key_x: Float => key_y: Float => key_z: Float => {r = $whiting_document(drawn, key_x, key_y, key_z); r: Int; r}
hall_plan = width: Float => depth: Float => storeys: Int => {r = $whiting_hall_plan(width, depth, storeys); r: Int; r}
hall_built = plan: Int => {r = $whiting_hall_built(plan); r: Int; r}
hall_parts = built: Int => {r = $whiting_hall_parts(built); r: Int; r}
hall_document = parts: Int => lens: Float => aim_x: Float => aim_y: Float => aim_z: Float => {r = $whiting_hall_document(parts, lens, aim_x, aim_y, aim_z); r: Int; r}
publish = published: Int => {r = $whiting_publish(published); r: Int; r}
";

/// The bridge: the author's Rust, inserted verbatim into the generated crate.
///
/// # Invariant
///
/// A body may read only `module`, `operand` and `item`, so every conversion an
/// entry needs is a free function over those three and nothing else.
pub const BRIDGE: &str = r#"use std::sync::atomic::{AtomicU64, Ordering};

use lichen_language::program::LangProgram;
use lichen_stages as stages;

/// The id the last `publish` call carried.
static PUBLISHED: AtomicU64 = AtomicU64::new(u64::MAX);

/// `len` arguments from `from`, read as `Int`, or `None` when one is not.
///
/// # Invariant
///
/// Every index in the range must answer `USize`: a call that passed a `Float`
/// where an `Int` belongs reads as absent rather than as a truncated number.
pub fn counts(
    module: &mut Module<LangProgram>,
    operand: &Value,
    from: usize,
    len: usize,
) -> Option<Vec<usize>> {
    (from..from + len)
        .map(
            |index| match AsEnum::<LowValue>::as_enum(&item(module, operand, index)?) {
                Some(LowValue::USize(number)) => Some(number),
                _ => None,
            },
        )
        .collect()
}

/// `len` arguments from `from`, read as `Float`, or `None` when one is not.
pub fn scalars(
    module: &mut Module<LangProgram>,
    operand: &Value,
    from: usize,
    len: usize,
) -> Option<Vec<f32>> {
    (from..from + len)
        .map(
            |index| match AsEnum::<LowValue>::as_enum(&item(module, operand, index)?) {
                Some(LowValue::Float(number)) => Some(number),
                _ => None,
            },
        )
        .collect()
}

/// A stage's answer as a lichen value.
pub fn id_of(id: Option<u64>) -> Value {
    match id {
        Some(id) => Value::from(LowValue::USize(id as usize)),
        None => Value::from(LowValue::Error),
    }
}

/// A stage's answer, for a stage that always has one.
pub fn id(id: u64) -> Value {
    id_of(Some(id))
}

/// What a call that could not be read answers.
///
/// # Invariant
///
/// `Error` and never a number: an id that names nothing must not read downstream
/// as a real answer.
pub fn missed() -> Value {
    Value::from(LowValue::Error)
}

/// Record `id` as the program's published answer, and hand it back.
///
/// # Invariant
///
/// A `SessionReport` carries its build behind an `Arc` the session's own cache
/// also holds, so a program value cannot be read out of one; publishing the
/// argument of an explicit call is how a host reads what a program actually
/// evaluated.
pub fn publish(published: u64) -> Value {
    PUBLISHED.store(published, Ordering::Relaxed);
    id(published)
}

/// The slab's fingerprint of the document under `id`.
#[unsafe(no_mangle)]
pub extern "C" fn lichen_whiting_fingerprint(id: u64) -> u64 {
    stages::slab::fingerprint(id)
}

/// The kind census of the document under `id`, one word per kind.
///
/// # Safety
///
/// The caller promises `out` names `len` machine words to write into.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lichen_whiting_shape(id: u64, out: *mut u64, len: *mut usize) {
    let census: Vec<u64> = stages::slab::document(id)
        .map(|document| {
            stages::fingerprint::shape(&document)
                .into_iter()
                .map(|(_, count)| count)
                .collect()
        })
        .unwrap_or_default();
    // SAFETY: the caller promises `out` holds at least `census.len()` words.
    unsafe {
        std::ptr::copy_nonoverlapping(census.as_ptr(), out, census.len());
        *len = census.len();
    }
}

/// How many values the slab holds.
#[unsafe(no_mangle)]
pub extern "C" fn lichen_whiting_slab_len() -> u64 {
    stages::slab::len() as u64
}

/// The id the last `publish` call carried, or `u64::MAX` when none has.
#[unsafe(no_mangle)]
pub extern "C" fn lichen_whiting_published() -> u64 {
    PUBLISHED.load(Ordering::Relaxed)
}

/// Empty the slab, so a host can time a genuinely cold build.
///
/// # Safety
///
/// Every id handed out before this call is meaningless afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lichen_whiting_reset() {
    stages::slab::reset();
}
"#;

const BODY_TREE: &str = r#"match counts(module, operand, 0, 1) {
    Some(a) => id(stages::tree(a[0] as u64)),
    None => missed(),
}"#;

const BODY_DRESSED: &str = r#"match (counts(module, operand, 0, 1), scalars(module, operand, 1, 1)) {
    (Some(a), Some(b)) => id_of(stages::dressed(a[0] as u64, b[0])),
    _ => missed(),
}"#;

const BODY_WOOD: &str = r#"match (counts(module, operand, 0, 3), scalars(module, operand, 3, 1)) {
    (Some(a), Some(b)) => id_of(stages::wood(a[0] as u64, a[1], a[2], b[0])),
    _ => missed(),
}"#;

const BODY_LEAVES: &str = r#"match (counts(module, operand, 0, 1), scalars(module, operand, 1, 1)) {
    (Some(a), Some(b)) => id_of(stages::leaves(a[0] as u64, b[0])),
    _ => missed(),
}"#;

const BODY_DRAWN: &str = r#"match counts(module, operand, 0, 3) {
    Some(a) => id_of(stages::drawn(a[0] as u64, a[1] as u64, a[2])),
    None => missed(),
}"#;

const BODY_DOCUMENT: &str = r#"match (counts(module, operand, 0, 1), scalars(module, operand, 1, 3)) {
    (Some(a), Some(b)) => id_of(stages::document(a[0] as u64, [b[0], b[1], b[2]])),
    _ => missed(),
}"#;

const BODY_PUBLISH: &str = r#"match counts(module, operand, 0, 1) {
    Some(a) => publish(a[0] as u64),
    None => missed(),
}"#;

const BODY_HALL_PLAN: &str = r#"match (scalars(module, operand, 0, 2), counts(module, operand, 2, 1)) {
    (Some(a), Some(b)) => id(stages::hall::plan(a[0], a[1], b[0])),
    _ => missed(),
}"#;

const BODY_HALL_BUILT: &str = r#"match counts(module, operand, 0, 1) {
    Some(a) => id_of(stages::hall::built(a[0] as u64)),
    None => missed(),
}"#;

const BODY_HALL_PARTS: &str = r#"match counts(module, operand, 0, 1) {
    Some(a) => id_of(stages::hall::parts(a[0] as u64)),
    None => missed(),
}"#;

const BODY_HALL_DOCUMENT: &str = r#"match (counts(module, operand, 0, 1), scalars(module, operand, 1, 4)) {
    (Some(a), Some(b)) => id_of(stages::hall::document(a[0] as u64, b[0], [b[1], b[2], b[3]])),
    _ => missed(),
}"#;

/// The library this crate generates, for the program the toolchain runs.
///
/// # Invariant
///
/// Every entry's body reads **only** the arguments its stage's key names, so a
/// parameter that reaches the body also reaches the key.
pub fn source() -> LibrarySource {
    LibrarySource {
        items: BRIDGE.to_string(),
        wrapper: WRAPPER.to_string(),
        entries: vec![
            entry("whiting_tree", "lichen_whiting_tree", BODY_TREE),
            entry("whiting_dressed", "lichen_whiting_dressed", BODY_DRESSED),
            entry("whiting_wood", "lichen_whiting_wood", BODY_WOOD),
            entry("whiting_leaves", "lichen_whiting_leaves", BODY_LEAVES),
            entry("whiting_drawn", "lichen_whiting_drawn", BODY_DRAWN),
            entry("whiting_document", "lichen_whiting_document", BODY_DOCUMENT),
            entry(
                "whiting_hall_plan",
                "lichen_whiting_hall_plan",
                BODY_HALL_PLAN,
            ),
            entry(
                "whiting_hall_built",
                "lichen_whiting_hall_built",
                BODY_HALL_BUILT,
            ),
            entry(
                "whiting_hall_parts",
                "lichen_whiting_hall_parts",
                BODY_HALL_PARTS,
            ),
            entry(
                "whiting_hall_document",
                "lichen_whiting_hall_document",
                BODY_HALL_DOCUMENT,
            ),
            entry("whiting_publish", "lichen_whiting_publish", BODY_PUBLISH),
        ],
    }
}

/// One family entry: the lichen op name, the exported symbol, and its body.
fn entry(name: &str, symbol: &str, body: &str) -> Entry {
    Entry {
        name: name.to_string(),
        symbol: symbol.to_string(),
        body: body.to_string(),
    }
}
