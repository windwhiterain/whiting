//! The naming a scene answers with: the table of ids and handles, and the words around a handle.
//!
//! # What this is
//!
//! A frame carries a number per pixel — the `authored_id` — and the number is only useful if
//! something can turn it back into a name. This module is that something, and it is about
//! **documents**: the table it reads is the one a document carries, so a caller holding a
//! document and a frame drawn from it can say what a pixel is without knowing which scene made
//! the document.
//!
//! # Where the table comes from
//!
//! **The document carries it, and the document is the only place it is read from.** A table
//! built by the content at the moment it is asked for answers about the content *now*, which is
//! a different question from the one a frame asks: a document written before an edit and a
//! frame drawn from it are one picture, and the name that goes with the picture is the name the
//! document carried when it was written. A table that is rebuilt on every question cannot be
//! that.
//!
//! # The words
//!
//! **A handle alone is not the answer, and the sentences around it are part of the naming.**
//! "this pixel is that node of the oak, and a leaf drawn by the blade mesh carries its shoot's"
//! says what the number means *in this scene*; the handle says which node it is. The two are
//! read from the same place and are asked by the same caller, so they cannot drift apart.

use std::collections::BTreeMap;

use definition::{FieldMap, Value};
use scene::standard::ELEMENTS;

/// The id a surface that is not an authored element writes.
///
/// Zero is **not allocated by any table**: it is what the ground and the sky write, so "this
/// pixel is not part of the scene" is answerable from the same image that says which part it
/// is.
pub const NOT_AUTHORED: u32 = 0;

/// The name a caller looks the id image up under.
pub const ID_OUTPUT: &str = "authored_id";

/// The name a caller looks the belt's distance-along-the-crest image up under.
pub const U_OUTPUT: &str = "authored_u";

/// The name a caller looks the belt's distance-across-the-strip image up under.
pub const V_OUTPUT: &str = "authored_v";

/// The ids of a scene's elements and the handles that name them.
///
/// # Invariants
///
/// - **The rows are in id order and there are no gaps in the order itself.** A lookup is a
///   binary search, so a caller asking about a pixel costs `O(log n)` rather than a walk of
///   every element the scene has.
/// - **An id this table does not hold answers `None`**, and so does the reserved zero: "no such
///   element" and "not an authored element" are different answers, and a caller that conflated
///   them would read a typo as an element that happened to be off screen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    rows: Vec<(u32, String)>,
}

impl Table {
    /// Builds a table from the rows a scene's walk produced.
    ///
    /// # Invariants
    ///
    /// **The rows are sorted by id, and the sort is stable in the id.** Two rows with one id
    /// would be two names for one number, and the first of them is the one a lookup answers
    /// with — so the table refuses the second rather than picking one.
    pub fn of(rows: impl IntoIterator<Item = (u32, String)>) -> Self {
        let mut rows: Vec<(u32, String)> = rows.into_iter().collect();
        rows.sort_by(|left, right| left.0.cmp(&right.0));
        rows.dedup_by(|left, right| left.0 == right.0);
        Self { rows }
    }

    /// Returns the handle one id names, or none for an id this table does not hold.
    ///
    /// `O(log n)`, because the rows are in id order.
    pub fn handle(&self, id: u32) -> Option<&str> {
        self.rows
            .binary_search_by_key(&id, |(held, _)| *held)
            .ok()
            .map(|at| self.rows[at].1.as_str())
    }

    /// Returns every row, in id order.
    pub fn rows(&self) -> &[(u32, String)] {
        &self.rows
    }

    /// Returns the id one handle names, or none when no handle says that.
    ///
    /// # Invariants
    ///
    /// **The first row that says it wins, and the rows are in id order.** Two rows with one
    /// handle would be two names for one id, and the table is built by refusing that, so the
    /// answer here is the one the table holds.
    ///
    /// `O(rows)`, which is what a tool costs and a frame never pays: the caller asks once per
    /// query and a scene asks nothing.
    pub fn id_named(&self, handle: &str) -> Option<u32> {
        self.rows
            .iter()
            .find(|(_, held)| held == handle)
            .map(|(id, _)| *id)
    }

    /// Returns whether the table holds no row at all.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// A scene's own answer to *"which authored element is this id?"*.
///
/// # Invariants
///
/// **The note and the zero sentence are the scene's own words, and the table is the scene's own
/// ids.** They are read from one place — the document — so a caller that has the document and a
/// frame drawn from it is reading the two from the same picture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Naming {
    /// What an id means here, as one clause: the sentence an asker puts after the handle.
    pub note: String,
    /// What the reserved zero means here, as one sentence.
    pub zero: String,
    /// The ids and the handles that name them.
    pub handles: Table,
}

impl Naming {
    /// Returns the handle one id names, or none for an id this naming does not hold.
    pub fn handle(&self, id: u32) -> Option<&str> {
        self.handles.handle(id)
    }
}

/// The naming a document carries, read back out of it.
pub type Elements = Naming;

/// Writes the naming a document carries.
///
/// # Invariants
///
/// **The handles are keyed by the id, as the id is spelled.** The document's own field map is
/// keyed by name, so the id is a key and not a position: a scene whose ids are a stride apart
/// would otherwise have to carry every id between two of them.
pub fn elements_field(naming: &Naming) -> Value {
    let handles: FieldMap = naming
        .handles
        .rows()
        .iter()
        .map(|(id, handle)| (id.to_string(), Value::String(handle.clone())))
        .collect();
    Value::structure([
        ("note".to_owned(), Value::String(naming.note.clone())),
        ("zero".to_owned(), Value::String(naming.zero.clone())),
        ("handles".to_owned(), Value::Dict(handles)),
    ])
}

/// Reads the naming a document carries, or none when the document carries none.
///
/// # Invariants
///
/// **A document without the field names nothing.** There is no default naming and no guessed
/// one: a scene that names its own elements says so in its document, and a scene that does not
/// is answered in the caller's own words.
pub fn elements_of(document: &Value) -> Option<Elements> {
    let fields = document.as_field_map()?;
    let elements = fields.get(ELEMENTS)?;
    let fields = elements.as_field_map()?;
    let text = |name: &str| {
        fields
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let handles = fields
        .get("handles")
        .and_then(Value::as_dict)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(id, handle)| {
                    let id = id.parse::<u32>().ok()?;
                    let handle = handle.as_str()?.to_owned();
                    Some((id, handle))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(Naming {
        note: text("note"),
        zero: text("zero"),
        handles: Table::of(handles),
    })
}

/// Resolves an `authored_id` into the handle of the element it shows.
///
/// # Invariants
///
/// - **The reserved zero has no handle, and says so rather than naming the first element.** It
///   is not an authored element and no table allocates it; the sentence about it belongs to
///   [`locate`].
/// - **A scene with no naming answers `None`**, and so does an id no table holds. "This document
///   names nothing" and "this id names nothing" are different answers.
pub fn handle_in(naming: Option<&Naming>, id: u32) -> Option<String> {
    naming?.handle(id).map(str::to_owned)
}

/// Resolves the three values a frame declared at one pixel to the authored part they name.
///
/// # Invariants
///
/// - **A value of zero is answered as itself, not as "unknown".** It is a real answer about a
///   real pixel — this one is the ground — and a caller told "not an authored element" knows
///   not to go looking for a crest there. `None` is reserved for a document whose content names
///   nothing at all, which is a different thing entirely.
/// - **An id above the count is reported, not clamped.** A frame and a document from different
///   builds produce exactly that, and the number in the message is what tells the two apart.
/// - **The first line of an answer is its handle, and [`handle_in`] gives that line alone.** The
///   lines beneath it are about a pixel, and a caller holding only an id asks [`handle_in`]
///   rather than being handed a sentence with a pixel it does not have in it.
pub fn locate(naming: Option<&Naming>, values: &[(String, f32)]) -> Option<String> {
    let one = |name: &str| {
        values
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, value)| *value)
    };
    let Some(naming) = naming else {
        return None;
    };
    let Some(authored) = one(ID_OUTPUT) else {
        return None;
    };
    // **The value is the first component**, which is where a one component image puts it: an
    // `r32float` target is a single channel and the readback pads the row out to four floats.
    // Taking the second would be a number that is always zero, and a scene whose ids all start
    // at one would answer "not an authored element" for every pixel of itself.
    let id = authored.round() as u32;
    if id == NOT_AUTHORED {
        return Some(if naming.zero.is_empty() {
            "not an authored element: the ground the scene stands on, or the sky above it"
                .to_owned()
        } else {
            naming.zero.clone()
        });
    }
    match naming.handle(id) {
        Some(handle) => Some(if naming.note.is_empty() {
            handle.to_owned()
        } else {
            format!("{handle}\n  authored_id {id}: {}", naming.note)
        }),
        None => Some(format!(
            "authored_id {id} is not one of this scene's elements"
        )),
    }
}

/// Returns the ids a table holds, keyed by the id, for a caller that writes the table out.
pub fn rows_by_id(table: &Table) -> BTreeMap<u32, String> {
    table.rows().iter().cloned().collect()
}
