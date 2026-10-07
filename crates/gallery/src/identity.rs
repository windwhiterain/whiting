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
        (U_OUTPUT.to_owned(), 2, coordinate(U_OUTPUT)),
        (V_OUTPUT.to_owned(), 3, coordinate(V_OUTPUT)),
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

/// Resolves a pixel of a rendered frame to the authored part of the landscape it shows.
///
/// Returns nothing for a scene this module has no naming for, and for a pixel the scene drew no
/// authored element on — which is the plain, and is the answer `0` was reserved to give.
///
/// # Invariants
///
/// - **The four hops happen here and nowhere else.** The frame is read for its `id`, `u` and `v`;
///   the `id` names a row of [`Identity::of`]'s table; the mesh turns that face and the `(u, v)`
///   into a belt; and the skeleton names the lines and stations. A caller that chained them
///   itself would be able to chain them wrong, and would be wrong in the middle, where the answer
///   looks like an answer.
/// - **It refuses rather than guessing when the two sides disagree.** The id comes from pixels and
///   the mesh from a build, and a caller holding a frame from one build and a mesh from another
///   would be told the id is unknown rather than handed another element's path.
pub fn locate(scene: &str, id: Option<[f32; 4]>, u: f32, v: f32) -> Option<String> {
    if scene != "landscape" {
        return None;
    }
    let massif = crate::landscape::massif();
    let mesh = massif.build().ok()?;
    let identity = Identity::of(&massif, crate::landscape::belt_path);

    // The value is the **first** component, which is where a one component image puts it: an
    // `r32float` target is a single channel and the readback pads the row out to four floats.
    // Taking the second would be a number that is always zero, and a scene whose ids all start
    // at one would answer "not an authored element" for every pixel of itself.
    let texel = id?;
    let id = texel[0];
    // **An id of no value is the reserved zero and is answered as itself**, not as "unknown": it
    // is a real answer about a real pixel — this one is the plain — and a caller that is told
    // "not an authored element" knows not to go looking for a crest there.
    let Some(row) = identity.row(id as u32) else {
        if id == NOT_AUTHORED as f32 {
            return Some("not an authored element: the pixel is the plain".to_owned());
        }
        return Some(format!(
            "id {id:.0} is not one of this massif's {} elements. The frame and the mesh are \
             probably from different builds",
            identity.rows().len()
        ));
    };
    // The face is the one of the belt's own faces whose uv triangle holds the point. **It is
    // found by the round trip, not by a containment test written here**: `locate` inverts the
    // rasteriser's interpolation, so a point inside a triangle comes back as itself, and a point
    // outside every one of them does not. That makes the containment rule the crate's rather
    // than a second one, and a belt's faces are a handful so the walk is free.
    let point = mesh
        .faces
        .iter()
        .filter(|face| face.attributes.belt == row.belt)
        .find_map(|face| {
            let found = mesh.locate(face, [u, v]);
            let close = (found.uv[0] - u).abs() < 1.0e-3 && (found.uv[1] - v).abs() < 1.0e-3;
            close.then_some((face, found))
        });
    let Some((face, point)) = point else {
        return Some(format!(
            "{}  id {}  belt valley {} {:?} segment {}  u {:.3}  v {:.3}  — no face of that belt \
             covers it, so the frame is probably from another build",
            row.path, row.id, row.belt.valley.0, row.belt.side, row.belt.segment, u, v,
        ));
    };

    // **The two valley nodes are the actionable part.** A belt is the surface between two
    // consecutive nodes of one valley, and moving either of them is the edit an author makes;
    // the crest it runs along is reached through the stations those two nodes carry. Naming the
    // belt and its parameter would be true and would not answer "what do I change".
    let valley = &massif.skeleton.valleys[row.belt.valley.0];
    let segment = row.belt.segment;
    let finish = segment + 1;
    Some(format!(
        "{}\n  \
         id {}  face {}  u {:.3}  v {:.3}\n  \
         top    {:?}, reached between the stations of valley {} nodes {segment} and {finish}\n  \
         bottom valley {} nodes {segment} and {finish}, at elevation {:.1}\n  \
         the edit: massif.skeleton.valleys[{}].nodes[{segment}].elevation  (or node {finish})",
        row.path,
        row.id,
        face.indices[0],
        point.uv[0],
        point.uv[1],
        face.attributes.top,
        row.belt.valley.0,
        row.belt.valley.0,
        valley.nodes[segment].elevation,
        row.belt.valley.0,
    ))
}

fn field(name: &str) -> Authored {
    Authored::Field {
        name: name.to_owned(),
        components: 1,
        flat: true,
    }
}

/// Returns the two belt coordinates, which are **not** flat.
///
/// # Invariants
///
/// They are a place on a surface, and a place is exactly the thing interpolation computes: a
/// pixel between two corners is between their parameters. Carried flat they would quantise the
/// belt to its triangles, and a caller asking "where on this flank" would be answered with the
/// nearest vertex rather than with the pixel they pointed at.
fn coordinate(name: &str) -> Authored {
    Authored::Field {
        name: name.to_owned(),
        components: 1,
        flat: false,
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
