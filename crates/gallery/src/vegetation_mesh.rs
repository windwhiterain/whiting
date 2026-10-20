//! A plant's wood, and the blade every leaf instances, as a scene draws them.
//!
//! This is the one place this crate joins `vegetation` to a scene, the way [`building_mesh`] is the one
//! place it joins `building` to one: neither crate may know about the other, so the conversion lives
//! above them.
//!
//! # What is converted, and what is not
//!
//! A plant arrives as two lists of numbers: one [`Wood`] per node, each a [`shape::Loft`] already in
//! the world, and one [`LeafGroup`] per node a field put leaves on, each a list of rigid placements of
//! one blade. **The wood is baked and the blade is instanced**, because that is the division the
//! vegetation crate states (see [`vegetation::expand::Expansion`]): a branch's shape is the document and
//! a leaf's is a template, so `k` leaves of one blade cost the template's own triangles once and `k`
//! transforms.
//!
//! # Invariants
//!
//! - **The whole plant's wood is one packed list, and each solid's points carry that node's own id.**
//!   A branch is a closed solid of its own, so the mesh is not indexed and the id is written at every
//!   corner rather than interpolated across a face — the same rule [`crate::building_identity`] states
//!   for a face's element. A pixel on a culm and a pixel on a twig then answer different handles out of
//!   the same image.
//! - **The id is the node's position in [`vegetation::plant::Plant::walk`]'s order, counted from one.**
//!   Zero is the value a surface that is not an authored element writes (see
//!   [`document::naming::NOT_AUTHORED`]), so the walk's first node is one and the two answers cannot
//!   collide. The numbering is taken from the walk rather than from the position in [`Wood`] so that a
//!   caller passing a filtered list of solids cannot renumber the plant under itself.
//! - **The blade is packed once, in the blade's own frame.** [`vegetation::leaf::LeafTemplate::loft`]
//!   is the space the instance transform expects: the frame's origin becomes the instance's translation
//!   and its axes the instance's rotation, so the blade is what every leaf instances and nothing bakes
//!   it twice. See [`crate::instancing`] for the other half of that contract.
//! - **Every corner's id is the base the instanced offset is measured from.** A blade is one mesh drawn
//!   by many instances, so its own points cannot name the node each leaf hangs from; the number a leaf
//!   pixel carries is [`LEAF_BASE_ID`] plus the `authored_offset` its row carries, which is what makes a
//!   leaf resolve to its twig. It is a **sum** rather than a second baked copy of the geometry, which is
//!   the whole reason `kit::AUTHORED_OFFSET` exists.
//! - **Wood alone creases, and only above [`WOOD_CREASE`].** A section whose sharpest corner is under
//!   the crease draws exactly the corners it drew before — positions, texture coordinates, order and
//!   count alike — so the change is an *identity* on every round-section plant rather than a small
//!   change to all of them, and a culm that shades round today shades round because nothing was
//!   altered that could have stopped it. A blade and a decorator are smooth by nature and are left on
//!   [`loft_corners`] outright.
//!
//! [`building_mesh`]: crate::building_mesh

use std::collections::BTreeMap;

use definition::{PackedBuilder, PackedList};
use vegetation::expand::Wood;
use vegetation::handle::Handle;
use vegetation::leaf::LeafTemplate;
use vegetation::plant::{Placement, Plant};

use document::building_identity::building_extras;
use document::geometry::{Corner, loft_corners, loft_corners_creased_at};
use document::naming::ID_OUTPUT;

/// The id every corner of a blade's own mesh carries.
///
/// The base an instance's `authored_offset` is measured from: a leaf drawn by the row whose offset is
/// `node − LEAF_BASE_ID` writes the id of the node it hangs from, and a leaf drawn by a row that names
/// no offset writes this one. One for every leaf, so the blade's own geometry is genuinely one mesh and
/// not one mesh per node that carries leaves.
pub const LEAF_BASE_ID: u32 = 1;

/// The angle of turn above which a wood section's corner is creased rather than smoothed.
///
/// # Why this number
///
/// **Thirty five degrees sits between the roundest section the wood is swept at and the next one
/// down.** [`vegetation::expand::Detail::DEFAULT`] sweeps every solid at `around = 12`, and a
/// regular twelve-gon turns `360 / 12 = 30` degrees at each of its corners — so a culm, a bough and a
/// twig are all shaded by the smooth average at this crease, which is what they were shaded by before.
/// A regular decagon turns 36 degrees and a flat ribboned plank far more, so a section drawn *to* be
/// faceted still is.
///
/// **Thirty five and not thirty one.** The number has to clear the twelve-gon's 30 degrees by a
/// margin wider than the rounding of a normalised dot product, or a culm would facet on a machine
/// this one and not on the next; five degrees is that margin without coming near the 36 degrees at
/// which a barely-coarser circle starts to read as a decagon's edges.
///
/// The alternative reading — an interior angle rather than a turn — would put the same threshold at
/// 145 degrees, and is stated in [`document::geometry::loft_corners_creased_at`] as the one it is not.
pub const WOOD_CREASE: f32 = 0.61;

/// The id of every node of a plant: its position in [`vegetation::plant::Plant::walk`]'s order, counted
/// from one.
///
/// # Invariants
///
/// **One walk produces both the numbers a frame writes and the table a reader looks them up in.** The
/// table is a [`BTreeMap`] and is built here once, so a mesh and a sidecar that were handed the same
/// plant agree by construction rather than by two orderings that have not diverged yet.
pub fn node_ids(plant: &Plant) -> BTreeMap<Handle, u32> {
    node_ids_from(plant, LEAF_BASE_ID)
}

/// The id of every node of a plant, counted from `base` instead of from one.
///
/// # Invariants
///
/// **A planting numbers its plants apart by its base**, so two plants of one scene never write the same id and a
/// frame can answer *which plant and which node* with one number. The stride between two bases must exceed every
/// plant's node count, and [`crate::vegetation_picture::PLANTING_STRIDE`] is the number the grove scene uses: a
/// plant of more than sixty-five thousand nodes is not one a picture draws.
pub fn node_ids_from(plant: &Plant, base: u32) -> BTreeMap<Handle, u32> {
    plant
        .walk()
        .enumerate()
        .map(|(index, (handle, _))| (handle, base + index as u32))
        .collect()
}

/// Returns a plant's whole wood as one mesh: every node's solid, each corner carrying that node's id.
///
/// # Invariants
///
/// - **One mesh, and one material for it.** The plant is a single object standing on a floor, so a
///   material per branch would be six materials and six passes over one tree; the branches differ in
///   their geometry, which is already in the points.
/// - **A solid's points carry that solid's own id at all three of every triangle's corners**, read
///   through the layout the renderer's own `packed_layout` derives from the extras' point schema. Writing
///   the field order here would be a second copy of the rule that a field's position is its name's
///   position.
pub fn wood_points(wood: &[Wood], ids: &BTreeMap<Handle, u32>) -> PackedList {
    wood_points_at(wood, ids, &Placement::IDENTITY)
}

/// Returns a plant's whole wood as one mesh, moved into the world by `placement`.
///
/// # Invariants
///
/// **The placement moves the points and not the ids**: a plant standing somewhere is the same plant, so its wood's
/// corners carry the ids its own walk gave them and only the positions and normals change. That is what keeps a
/// scene's two plants answerable: the id names the plant through its base and the node through its offset.
pub fn wood_points_at(
    wood: &[Wood],
    ids: &BTreeMap<Handle, u32>,
    placement: &Placement,
) -> PackedList {
    wood_points_of(std::iter::once((wood, ids, placement)))
}

/// Returns every planting's wood as **one** mesh, each corner still carrying the id its own planting numbered.
///
/// # Invariants
///
/// - **One mesh for a whole scene, however many plants it holds.** A grove of six plants is six woods and a meadow is
///   hundreds; a mesh per plant would be a pass per plant, because the emitter groups by shader, mesh and images. The
///   ids are in the corners, so merging the geometry does not merge the names: a pixel still answers *which plant and
///   which node*.
/// - **The order of the plantings is the order of the corners**, so a mesh is reproducible from its plantings.
/// - **Wood is the only path here that creases, and it is the only one that may.** A blade and a
///   decorator are leaves and seats, both of them smooth by nature, and a building carries a pinned
///   hash per mesh that a normal cannot move. Wood is what a fluted bole is made of, so wood is
///   where [`document::geometry::loft_corners_creased_at`] is asked for — at [`WOOD_CREASE`], which every
///   round section is under, so a culm drawn today is the culm drawn yesterday.
pub fn wood_points_of<'a>(
    plantings: impl IntoIterator<Item = (&'a [Wood], &'a BTreeMap<Handle, u32>, &'a Placement)>,
) -> PackedList {
    let mut corners = Vec::new();
    let mut triangles = 0;
    for (wood, ids, placement) in plantings {
        for solid in wood {
            let id = *ids
                .get(&solid.handle)
                .expect("every solid's handle is one the plant's own walk yielded");
            corners.extend(
                loft_corners_creased_at(&solid.loft, placement, WOOD_CREASE)
                    .into_iter()
                    .map(|corner| (corner, id)),
            );
            triangles += solid.loft.triangles().len();
        }
    }
    packed(corners.into_iter(), triangles)
}

/// Returns a blade's own mesh, in the blade's own frame: what every leaf of a field instances.
///
/// # Invariants
///
/// **Every corner carries [`LEAF_BASE_ID`], and the normal and the `(u, v)` are the loft's own** — the
/// same rules [`loft_corners`] states, so a blade shades as a blade here exactly as a swept tube shades
/// as a tube in [`crate::sweep_picture`].
pub fn template_points(template: &LeafTemplate) -> PackedList {
    let loft = template.loft();
    packed(
        loft_corners(&loft)
            .into_iter()
            .map(|corner| (corner, LEAF_BASE_ID)),
        loft.triangles().len(),
    )
}

/// Returns a decorator's own mesh, in the decorator's own frame: what every seat of a template instances.
///
/// # Invariants
///
/// **The same base id as a blade**, because a decorator is seated on a node and answers the same question a leaf
/// does: *which node is this pixel*. Its material's `authored_offset` carries the node, exactly as a leaf's does,
/// so a knot names the bough it grew out of and the id space needs no second channel.
pub fn decorator_points(template: &vegetation::decorator::Template) -> PackedList {
    let loft = template.loft();
    packed(
        loft_corners(&loft)
            .into_iter()
            .map(|corner| (corner, LEAF_BASE_ID)),
        loft.triangles().len(),
    )
}

/// Packs corners against the authored point schema, one id per corner.
///
/// The layout is the extras' own, read through the same `packed_layout` the renderer reads it with, so a
/// mesh whose fields move cannot leave a buffer laid out for the fields the schema used to have. It is
/// [`crate::building_identity`]'s extras rather than a schema written here, because the channel a plant
/// draws on is that one: one flat `authored_id` per point, summed with the row's offset, which is
/// exactly the question *"which node is this pixel"* and is not a second question per subsystem.
fn packed(corners: impl Iterator<Item = (Corner, u32)>, triangles: usize) -> PackedList {
    let schema = building_extras().point_schema();
    let layout = graphics::layout::packed_layout(&schema)
        .expect("the building's own point schema is a struct of components");
    let mut points = PackedBuilder::new(layout);
    points.reserve(triangles * 3);
    for (corner, id) in corners {
        let written = points
            .field(ID_OUTPUT, &[f64::from(id)])
            .and_then(|points| points.field("normal", &corner.normal))
            .and_then(|points| points.field("position", &corner.position))
            .and_then(|points| points.field("texture", &corner.texture));
        written.expect("a generator writes whole elements of its own layout");
    }
    points
        .finish()
        .expect("every generator writes whole elements of its own layout")
}
