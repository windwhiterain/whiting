//! Content identity: which authored part of the massif a pixel shows.
//!
//! # What this is for
//!
//! An agent looking at a picture has nothing else to look at. *"That crest is too rounded and
//! there is a shelf under it"* is a sentence about a picture, and the agent's only other surfaces
//! are a source file and a document. This module is the join: it hands the frame an id per
//! authored element, and hands the agent the table that turns that id back into a name.
//!
//! See [`.agents/proposals/agent-content-identity.md`](../../../.agents/proposals/agent-content-identity.md)
//! for the design and [`.agents/handoffs/agent-content-identity.md`](../../../.agents/handoffs/agent-content-identity.md)
//! for what is still owed.
//!
//! # The four hops, and who owns each
//!
//! | hop | owned by |
//! | --- | --- |
//! | a pixel → `id`, `u`, `v` | the frame, read off the rendered image |
//! | `id` → an authored path | **this module** |
//! | a face → a belt, and `(u, v)` → a place on it | `mountain`, in [`FacetMesh::locate`] |
//! | a belt → the two lines and stations that bound it | the skeleton, already |
//!
//! **The belt id is deliberately not in the image.** The mesh already carries
//! [`Face::attributes`](mountain::FacetMesh)'s belt, so the hop from a face to a belt is a lookup
//! in data the crate holds rather than a fifth number baked into every pixel. A number in the
//! image would be a second copy of a fact, and a second copy is a second thing to be wrong.
//!
//! # Invariants
//!
//! - **One walk produces both the ids and the table.** They are built together in
//!   [`Identity::of`] and there is no other way to get either, because the whole design rests on
//!   the two never disagreeing and a second walk would be a second thing to disagree.
//! - **The numbering comes from [`mountain::expand::belt_references`], not from a loop written
//!   here.** The expander enumerates belts in a fixed order and so does this; writing the order
//!   out a second time would be right today and wrong the first time it changed.
//! - **Zero is reserved and never allocated.** It is what a surface that is not an authored
//!   element writes — the plain this massif stands on — so "which pixel is not part of the
//!   massif" is answerable from the same image rather than by comparing it with another one.

use std::collections::BTreeMap;

use definition::{PackedBuilder, PackedList, Value};
use mountain::expand::belt_references;
use mountain::tessellation::BeltRef;
use mountain::{FacetMesh, Massif};

use crate::shaders::{Authored, Extras};

/// The name a caller looks the id image up under.
pub const ID_OUTPUT: &str = "authored_id";

/// The name a caller looks the belt's distance-along-the-crest image up under.
pub const U_OUTPUT: &str = "authored_u";

/// The name a caller looks the belt's distance-across-the-strip image up under.
pub const V_OUTPUT: &str = "authored_v";

/// The id a surface that is not an authored element writes.
///
/// Zero is **not allocated by [`Identity::of`]**, and the reservation is the whole point: a
/// reader can tell "this pixel is grass" from the same image that says which facet it is, with no
/// second render to compare against.
pub const NOT_AUTHORED: u32 = 0;

/// One authored element, and the number the frame carries for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The number a fragment writes.
    pub id: u32,
    /// The path the content gives this element, which is **the content's own naming and not the
    /// crate's**. `Massif` has no naming and no grouping, and SPEC/mountain.md is human-editable
    /// only; a path is a fact about the code that built the massif, not about the massif.
    pub path: String,
    /// Which belt this is, so a reader can check the id against the geometry rather than trust
    /// the table.
    pub belt: BeltRef,
}

/// The ids of every authored element, and the table that gives them their names.
///
/// # Invariants
///
/// - **The ids are `1..=belts` in [`belt_references`]' order, with no gaps**, so an id is a
///   position in one list and the table is that list. A gap would make an id a range rather than
///   a position, and a range is a thing a reader has to reason about to notice a missing row.
/// - **The table is a function of the skeleton alone**, so two builds of one skeleton give the
///   same ids — which is what lets an agent remember an id between two edits. What does *not*
///   survive an edit is the mesh, and nothing here depends on the mesh.
pub struct Identity {
    rows: Vec<Row>,
    by_belt: BTreeMap<BeltRef, u32>,
}

impl Identity {
    /// Assigns an id to every belt of a massif, and records the path the content gives it.
    ///
    /// # Invariants
    ///
    /// **The walk is [`belt_references`]'s and the table is written in the same pass**, so the
    /// number a fragment writes and the row a reader looks up are the same number by
    /// construction. There is no second function that returns one without the other.
    pub fn of(massif: &Massif, path: impl Fn(BeltRef) -> String) -> Self {
        let mut rows = Vec::new();
        let mut by_belt = BTreeMap::new();
        for (index, belt) in belt_references(&massif.skeleton).into_iter().enumerate() {
            let id = index as u32 + 1;
            by_belt.insert(belt, id);
            rows.push(Row {
                id,
                path: path(belt),
                belt,
            });
        }
        Self { rows, by_belt }
    }

    /// Returns the id of a belt, or none when it is not one this massif has.
    pub fn id_of(&self, belt: BeltRef) -> Option<u32> {
        self.by_belt.get(&belt).copied()
    }

    /// Returns the row an id names, or none for the reserved zero and for anything above the
    /// count.
    pub fn row(&self, id: u32) -> Option<&Row> {
        self.rows.get((id as usize).checked_sub(1)?)
    }

    /// Returns every row, in id order.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }
}

/// Returns the extra outputs the massif's own shader writes beside the frame.
///
/// # Invariants
///
/// - **All three are read from the points, never written as a constant.** A facet's identity is
///   a fact about that facet, and two faces of one flank are two different answers that only the
///   geometry knows apart.
pub fn massif_extras() -> Extras {
    Extras::new([
        (ID_OUTPUT.to_owned(), 1, field(ID_OUTPUT)),
        (U_OUTPUT.to_owned(), 2, field(U_OUTPUT)),
        (V_OUTPUT.to_owned(), 3, field(V_OUTPUT)),
    ])
}

/// Returns the extra outputs the **plain's** shader writes beside the frame.
///
/// # Invariants
///
/// **All three are the reserved zero, and none of them reads a point.** The plain is a height
/// field with no authored element of its own, and carrying three numbers that are the same at
/// every one of its vertices would put a field on every plain point in the gallery to say
/// nothing. This is the case that decided [`Authored::Constant`] existing: a scene's second
/// surface has to write the outputs too, because a stage that drew geometry and did not would
/// leave the image holding whatever an earlier fragment put there.
pub fn plain_extras() -> Extras {
    Extras::new([
        (
            ID_OUTPUT.to_owned(),
            1,
            Authored::Constant(f64::from(NOT_AUTHORED)),
        ),
        (U_OUTPUT.to_owned(), 2, Authored::Constant(0.0)),
        (V_OUTPUT.to_owned(), 3, Authored::Constant(0.0)),
    ])
}

/// Returns the scene's own `outputs` field, which is what the compiler builds the three targets
/// from.
///
/// # Invariants
///
/// **The locations are 1, 2, 3, and the type is `f32` for all three.** `f32` is `r32float`, and
/// an `rgba16float` is exact only to 2048 — so an id of 4093 in one of those would come back as
/// 4092 with nothing saying so. The frame is location zero and is the pipeline's, not declared
/// here.
pub fn outputs_field() -> Value {
    Value::dictionary([
        (ID_OUTPUT.to_owned(), declared(1)),
        (U_OUTPUT.to_owned(), declared(2)),
        (V_OUTPUT.to_owned(), declared(3)),
    ])
}

fn declared(location: u32) -> Value {
    Value::structure([
        ("location".to_owned(), Value::Int(i64::from(location))),
        ("type".to_owned(), Value::String("f32".to_owned())),
    ])
}

fn field(name: &str) -> Authored {
    Authored::Field {
        name: name.to_owned(),
        components: 1,
    }
}

/// Turns the massif's facets into the points the standard pipeline draws, each carrying the id
/// and the belt coordinate a fragment writes.
///
/// # Invariants
///
/// - **The normal, and the id and the belt coordinates, are written at all three corners.** All
///   four are properties of a *face* rather than of a vertex, and this mesh is not indexed, so a
///   value written at one corner is a value a rasteriser interpolates across the face and the
///   other two corners would pull it somewhere else.
/// - **`(u, v)` is the face's own `uv`, copied.** The expander defines the belt's parameter
///   space once, in [`mountain::Face::uv`]; deriving it again here would be a second definition
///   that is wrong by a rotation of the triangle and wrong silently.
/// - **The id is the belt's**, not the face's, so two faces of one belt agree and a caller
///   reading either of them names the same authored surface.
pub fn faceted(mesh: &FacetMesh, identity: &Identity) -> PackedList {
    let schema = massif_extras().point_schema();
    // The layout is **the schema's own**, through the same function the renderer reads it with.
    // Writing the field order out here would be a second copy of the rule that a field's
    // position is its name's position in the schema, and the two would agree until a field was
    // added whose name sorted somewhere else — which is a mesh whose vertices are read from the
    // wrong offsets and nothing that would say so.
    let layout = graphics::layout::packed_layout(&schema)
        .expect("the massif's own point schema is a struct of components");
    let mut points = PackedBuilder::new(layout);
    points.reserve(mesh.faces.len() * 3);
    for face in &mesh.faces {
        let [a, b, c] = mesh.triangle(face);
        let normal = mesh.normal(face).map(|value| value as f64);
        let texture = [
            face.attributes.ruggedness as f64,
            (face.attributes.dip_angle as f64 / std::f64::consts::FRAC_PI_2).clamp(0.0, 1.0),
        ];
        // A face whose belt is not in the table would be a face the expander made from a belt
        // the identity walk skipped, and writing a number nobody can look up would be a pixel
        // that answers "not an authored element" while showing a facet. It is refused instead.
        let id = identity
            .id_of(face.attributes.belt)
            .expect("every face's belt was numbered by the same walk that built the mesh")
            as f64;
        for (corner, uv) in [a, b, c].into_iter().zip(face.uv) {
            let position = corner.map(|value| value as f64);
            let written = points
                .field(ID_OUTPUT, &[id])
                .and_then(|points| points.field(U_OUTPUT, &[uv[0] as f64]))
                .and_then(|points| points.field(V_OUTPUT, &[uv[1] as f64]))
                .and_then(|points| points.field("normal", &normal))
                .and_then(|points| points.field("position", &position))
                .and_then(|points| points.field("texture", &texture));
            written.expect("a generator writes whole elements of its own layout");
        }
    }
    points
        .finish()
        .expect("every generator writes whole elements of its own layout")
}
