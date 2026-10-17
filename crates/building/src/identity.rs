//! Identity: which authored element of a building a face — and so a pixel — shows.
//!
//! # What this is for
//!
//! The same job [`gallery::identity`] does for a massif, for a building: an agent looking at a picture
//! has nothing to look at but the picture, and *"the third window's reveal is too deep"* is a sentence
//! about a frame. This module hands the frame a number per authored element and hands the agent the
//! table that turns the number back into a name — so a report can say **"part 1, its left reveal"**
//! instead of *"one of the 3 400 faces"*.
//!
//! The four hops are the sibling's, and so is who owns each: a pixel is read off the rendered image by
//! the frame, an id becomes a name here, a face becomes an element through
//! [`BuildingFace::element`](crate::mesh::BuildingFace), and an element becomes an opening, a bay or a
//! template through the document. **No id is baked into the image by this module**; the render half
//! (an `id` channel the mesh shader writes) is the device layer's, exactly as it is for the mountains.
//!
//! # Invariants
//!
//! - **One walk produces both the numbers and the table.** [`Identity::of`] reads the mesh's own
//!   elements once and returns both, so the number a face would write and the row a reader looks up
//!   cannot disagree — there is no second function that returns one without the other.
//! - **The ids are a contiguous run from `first_id`, with no gaps**, so an id is a position in one
//!   list and a gap would be a range a reader has to reason about.
//! - **Zero is reserved and never allocated** ([`NOT_AUTHORED`]): it is what a surface that is not part
//!   of the building writes — the ground, the sky — so "is this pixel the building at all" is
//!   answerable from the same image rather than by comparing two.
//! - **The numbering is a function of the *document and the mesh*, not of the document alone** — and
//!   that is the one place this module diverges from its sibling, deliberately. A massif's belts can
//!   be enumerated from its skeleton (`mountain::expand::belt_references`), so its table is a function
//!   of the skeleton. A building's surfaces cannot: a roof's fascia is numbered by its position in the
//!   **skin's own boundary edge list**, and the count of those edges is a fact the expansion proves
//!   rather than a number the document holds. Re-deriving that count here would be a second
//!   derivation of the skin's boundary — the exact failure the crate has paid for twice — so this
//!   module asks the mesh instead. The practical property the sibling's invariant exists for survives:
//!   ids are stable across edits that do not change a surface's own boundary, which is what lets an
//!   agent remember "part 1's left reveal" between two edits.
//!
//! # What it costs
//!
//! `O(faces log faces)`: one pass to gather the distinct elements and one sort. The table is what a
//! caller keeps; asking it is `O(log n)` by element and `O(1)` by id.

use std::collections::{BTreeMap, BTreeSet};

use crate::mesh::{BuildingMesh, Element, MassSurface, PartSurface, RoofSurface};
use crate::roof::RoofId;

/// The id a surface that is not an authored element of the building writes.
///
/// Zero is **not allocated by [`Identity::of`]**: a reader can tell "this pixel is the ground" from
/// the same image that says which element the rest of the frame shows.
pub const NOT_AUTHORED: u32 = 0;

/// One authored element, and the number the frame carries for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The number a fragment writes.
    pub id: u32,
    /// The element the number names.
    pub element: Element,
    /// The name a reader is given, in the document's own words.
    pub path: String,
}

/// The ids of every authored element of a building, and the table that names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    first_id: u32,
    rows: Vec<Row>,
    by_element: BTreeMap<Element, u32>,
}

impl Identity {
    /// Numbers every element the mesh's faces name, and writes the table in the same pass.
    ///
    /// # Invariants
    ///
    /// The walk is over **the mesh's own elements**, gathered into a set, so "a face whose element has
    /// no id" is not a case that can arise — it is the same set, read twice. The order is the derived
    /// order of [`Element`], which is a total order over variant, id and surface, so two runs of one
    /// mesh number the elements identically and an agent can compare two tables by eye.
    ///
    /// A caller with one building passes `1`; a caller drawing two numbers the second from where the
    /// first ended, and [`first_id`](Self::first_id) is how it asks.
    pub fn of(built: &BuildingMesh, first_id: u32) -> Self {
        let elements: BTreeSet<Element> = built.faces.iter().map(|face| face.element).collect();
        let mut rows = Vec::with_capacity(elements.len());
        let mut by_element = BTreeMap::new();
        for (index, element) in elements.into_iter().enumerate() {
            let id = first_id + index as u32;
            by_element.insert(element, id);
            rows.push(Row {
                id,
                element,
                path: describe(element),
            });
        }
        Self {
            first_id,
            rows,
            by_element,
        }
    }

    /// Returns the id of the first element, which is the run's own start.
    pub fn first_id(&self) -> u32 {
        self.first_id
    }

    /// Returns the number of elements, and therefore the last id of the run.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns whether the building has no element at all.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the table, in id order.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Returns the id of an element, or `None` when it is not one the mesh names.
    ///
    /// `O(log n)`, and the only way a caller gets an id: the table is built by the same walk, so an
    /// answer here is always a row a reader can look up.
    pub fn id_of(&self, element: Element) -> Option<u32> {
        self.by_element.get(&element).copied()
    }

    /// Returns the row one id names, or `None` when the id is outside the run or is
    /// [`NOT_AUTHORED`].
    ///
    /// `O(1)`, because an id is a position in the table and the run starts at `first_id`.
    pub fn row(&self, id: u32) -> Option<&Row> {
        let offset = id.checked_sub(self.first_id)? as usize;
        self.rows.get(offset)
    }

    /// Returns the row a face shows, which is the join from the frame to the document.
    pub fn of_face(&self, built: &BuildingMesh, face: usize) -> Option<&Row> {
        self.row(self.id_of(built.faces.get(face)?.element)?)
    }
}

/// Returns the name a reader is given for one element.
///
/// # Invariants
///
/// **The name is the document's words and not the crate's types**: "mass 0, outer wall over edge 2",
/// "part 1, its left reveal", "roof 0, skin of plane 1". A report that said
/// `Element::Mass(MassId(0), OuterWall { edge: 2 })` would be a report an author has to translate, and
/// the translation is the part they are worst at.
fn describe(element: Element) -> String {
    match element {
        Element::Mass(id, surface) => format!("mass {}, {}", id.0, mass_surface(surface)),
        Element::Roof(id, surface) => format!("{}, {}", roof_title(id), roof_surface(surface)),
        Element::Wall(id, surface) => format!("wall {}, {}", id.0, wall_surface(surface)),
        Element::Part { placement, surface } => match surface {
            PartSurface::Mass(id, surface) => format!(
                "part {placement}, template mass {}, {}",
                id.0,
                mass_surface(surface)
            ),
            PartSurface::Roof(id, surface) => format!(
                "part {placement}, template {}, {}",
                roof_title(id),
                roof_surface(surface)
            ),
            PartSurface::Wall(id, surface) => format!(
                "part {placement}, template wall {}, {}",
                id.0,
                wall_surface(surface)
            ),
        },
    }
}

/// Returns the sentence for one of a wall's surfaces.
fn wall_surface(surface: crate::wall::WallSurface) -> String {
    match surface {
        crate::wall::WallSurface::Side { left: true } => "its left side, looking out".to_owned(),
        crate::wall::WallSurface::Side { left: false } => "its right side".to_owned(),
        crate::wall::WallSurface::End { start: true } => "its start end".to_owned(),
        crate::wall::WallSurface::End { start: false } => "its far end".to_owned(),
        crate::wall::WallSurface::Bottom => "its underside".to_owned(),
        crate::wall::WallSurface::Top => "its top".to_owned(),
    }
}

/// Returns what a roof is called in a name.
fn roof_title(id: RoofId) -> String {
    format!("roof {}", id.0)
}

/// Returns the sentence for one of a mass's surfaces.
fn mass_surface(surface: MassSurface) -> String {
    match surface {
        MassSurface::OuterWall { edge } => format!("outer wall over edge {edge}"),
        MassSurface::InnerWall { edge } => format!("inner wall over edge {edge}"),
        MassSurface::WallTop { edge } => format!("wall top over edge {edge}"),
        MassSurface::FloorTop => "its floor".to_owned(),
        MassSurface::SlabSoffit => "its slab's soffit".to_owned(),
        MassSurface::Reveal {
            edge,
            opening,
            side,
        } => format!(
            "{} reveal of opening {opening} in edge {edge}",
            match side {
                crate::openings::RevealSide::Left => "the left",
                crate::openings::RevealSide::Right => "the right",
                crate::openings::RevealSide::Head => "the head",
                crate::openings::RevealSide::Sill => "the sill",
            }
        ),
    }
}

/// Returns the sentence for one of a roof's surfaces.
fn roof_surface(surface: RoofSurface) -> String {
    match surface {
        RoofSurface::Skin { plane } => format!("the skin of plane {plane}"),
        RoofSurface::Soffit { plane } => format!("the soffit under plane {plane}"),
        RoofSurface::Fascia { edge } => {
            format!("the fascia over the skin's boundary edge {edge}")
        }
    }
}
