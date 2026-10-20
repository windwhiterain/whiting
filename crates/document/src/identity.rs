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
use mountain::{Face, FacetMesh, Massif, Side};

use crate::naming::{ID_OUTPUT, NOT_AUTHORED, U_OUTPUT, V_OUTPUT};
use crate::shaders::{Authored, Extras};

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
/// - **The ids are a contiguous run starting at [`Identity::first_id`], with no gaps**, so an id
///   is a position in one list and the table is that list. A gap would make an id a range rather
///   than a position, and a range is a thing a reader has to reason about to notice a missing
///   row.
/// - **A run has to be stated rather than assumed to begin at one.** A scene drawing more than one
///   massif numbers them one after another, and each is told where its run begins — because two
///   tables that both started at one would hand the same number to two different flanks, and a
///   pixel would resolve to whichever of them a reader happened to ask about first. That is the
///   failure the reservation of zero above exists to make impossible, and starting every table at
///   one makes it possible again.
/// - **The table is a function of the skeleton alone**, so two builds of one skeleton give the
///   same ids — which is what lets an agent remember an id between two edits. What does *not*
///   survive an edit is the mesh, and nothing here depends on the mesh.
pub struct Identity {
    first_id: u32,
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
    ///
    /// `first_id` is the id of the first belt and the run is `first_id ..= first_id + belts`. A
    /// caller with one massif passes `1`; a caller with more numbers them one after another and
    /// hands each the number the one before it ended on.
    pub fn of(massif: &Massif, first_id: u32, path: impl Fn(BeltRef) -> String) -> Self {
        let mut rows = Vec::new();
        let mut by_belt = BTreeMap::new();
        for (index, belt) in belt_references(&massif.skeleton).into_iter().enumerate() {
            let id = first_id + index as u32;
            by_belt.insert(belt, id);
            rows.push(Row {
                id,
                path: path(belt),
                belt,
            });
        }
        Self {
            first_id,
            rows,
            by_belt,
        }
    }

    /// Returns the number this table's run of ids begins at.
    pub fn first_id(&self) -> u32 {
        self.first_id
    }

    /// Returns the id of a belt, or none when it is not one this massif has.
    pub fn id_of(&self, belt: BeltRef) -> Option<u32> {
        self.by_belt.get(&belt).copied()
    }

    /// Returns the row an id names, or none for the reserved zero, for an id below this table's
    /// own run, and for anything above the count.
    ///
    /// # Invariants
    ///
    /// **The subtraction is what refuses.** An id outside this run is a number another massif's
    /// row carries, and indexing with it would hand a reader the path of a flank standing
    /// somewhere else in the scene entirely.
    pub fn row(&self, id: u32) -> Option<&Row> {
        self.rows.get(id.checked_sub(self.first_id)? as usize)
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
/// - **The normal is read per corner, and the id and the belt coordinates at all three.** The id
///   and the coordinates are properties of a *face* rather than of a vertex, and this mesh is not
///   indexed, so a value written at one corner is a value a rasteriser interpolates across the
///   face and the other two corners would pull it somewhere else. The normal is the exception that
///   proves the rule: it is a property of a *smoothing group*, it is averaged over that group's
///   own faces ([`mountain::CornerNormals::of_grouped`]), and writing it three times the same is
///   exactly the hard edge the group exists to remove.
/// - **The smoothing group is passed in and this function has no opinion of its own.** Which edges
///   of a massif are creases is a statement about the scene rather than about a mesh, and the two
///   scenes that draw one disagree: see [`across_the_flank`] and [`across_the_belt`].
/// - **`(u, v)` is the face's own `uv`, copied.** The expander defines the belt's parameter
///   space once, in [`mountain::Face::uv`]; deriving it again here would be a second definition
///   that is wrong by a rotation of the triangle and wrong silently.
/// - **The id is the belt's**, not the face's, so two faces of one belt agree and a caller
///   reading either of them names the same authored surface.
pub fn faceted(mesh: &FacetMesh, identity: &Identity, smoothing: fn(&Face) -> u32) -> PackedList {
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
    let normals = mountain::CornerNormals::of_grouped(mesh, smoothing);
    for (index, face) in mesh.faces.iter().enumerate() {
        let [a, b, c] = mesh.triangle(face);
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
        // `a, c, b` rather than `a, b, c`, and this is the boundary where the massif crate's own
        // winding is translated into a document one. `mountain::math::face_normal` is
        // `normal(c - a, b - a)` with the arguments deliberately swapped — the massif's faces have
        // positive plan area in `(x, z)`, which in a y-up right-handed frame crosses to `-y` — so
        // the normals `CornerNormals` hands over are the negation of the right-handed normal of
        // the triangle as the crate winds it. The convention is that the **normal** is outward and
        // the winding obeys it, so the order written here is the one that agrees. Each corner
        // still carries its own normal, its own `uv` and its own position: only the order the three
        // are written in changes, and nothing that reads a winding on this side.
        for (corner, (point, uv)) in [a, c, b].into_iter().zip(face.uv).enumerate() {
            let position = point.map(|value| value as f64);
            let normal = normals.at(index, corner).map(|value| value as f64);
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

/// Returns the smoothing group that makes **one flank of one valley a single surface**.
///
/// # Invariants
///
/// - **It groups the strips of a flank and not the flank.** A flank is written as a run of belts,
///   one per segment along its crest, and two consecutive strips meet along a rib. The rib is an
///   edge of the correspondence and not a feature of the rock: the height field is continuous
///   across it and the two strips either side are the same slope of the same mountain. Averaging
///   per belt instead puts a crease on every one of them, and a massif of eleven strips to a flank
///   comes out as a fan of straw laid down the hill rather than as a face of rock.
///
/// - **It stops at the crest and at the floor.** A ridge is named by the two flanks that face it,
///   one from each side, so a crest vertex falls in two groups and keeps its crease — which is
///   what the crest is. The valley floor is shared the same way, and a floor that is a line is
///   what makes this a massif and not a lump.
///
/// - **Two groups never share an id**, which is the whole contract of a smoothing key: it is a
///   valley's number and then which of its two sides, packed into the one number the averaging can
///   compare.
pub fn across_the_flank(face: &Face) -> u32 {
    let belt = face.attributes.belt;
    2 * u32::try_from(belt.valley.0)
        .expect("a valley id counts valleys and a count of valleys is not a u64's worth")
        + match belt.side {
            Side::Left => 0,
            Side::Right => 1,
        }
}

/// Returns the smoothing group that makes **each belt its own surface**, every rib a crease.
///
/// # Invariants
///
/// **It is the belt's own block, read as it stands.** That is what [`mountain::CornerNormals::of`]
/// would do without being told, and naming it here is the point: the choice is a scene's, and a
/// scene whose rock is meant to read as cut into facets wants this one. It is the conservative
/// reading, because a facet boundary is a real edge in every surface that has one, and this
/// function is what a caller passes when it does not know better.
pub fn across_the_belt(face: &Face) -> u32 {
    face.block
}
