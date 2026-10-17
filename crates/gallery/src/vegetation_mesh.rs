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
//!   [`crate::identity::NOT_AUTHORED`]), so the walk's first node is one and the two answers cannot
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
//!
//! [`building_mesh`]: crate::building_mesh

use std::collections::BTreeMap;

use definition::{PackedBuilder, PackedList};
use vegetation::expand::Wood;
use vegetation::handle::Handle;
use vegetation::leaf::LeafTemplate;
use vegetation::plant::Plant;

use crate::building_identity::building_extras;
use crate::geometry::{Corner, loft_corners};
use crate::identity::ID_OUTPUT;

/// The id every corner of a blade's own mesh carries.
///
/// The base an instance's `authored_offset` is measured from: a leaf drawn by the row whose offset is
/// `node − LEAF_BASE_ID` writes the id of the node it hangs from, and a leaf drawn by a row that names
/// no offset writes this one. One for every leaf, so the blade's own geometry is genuinely one mesh and
/// not one mesh per node that carries leaves.
pub const LEAF_BASE_ID: u32 = 1;

/// The id of every node of a plant: its position in [`vegetation::plant::Plant::walk`]'s order, counted
/// from one.
///
/// # Invariants
///
/// **One walk produces both the numbers a frame writes and the table a reader looks them up in.** The
/// table is a [`BTreeMap`] and is built here once, so a mesh and a sidecar that were handed the same
/// plant agree by construction rather than by two orderings that have not diverged yet.
pub fn node_ids(plant: &Plant) -> BTreeMap<Handle, u32> {
    plant
        .walk()
        .enumerate()
        .map(|(index, (handle, _))| (handle, index as u32 + 1))
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
    packed(
        wood.iter().flat_map(|solid| {
            let id = *ids
                .get(&solid.handle)
                .expect("every solid's handle is one the plant's own walk yielded");
            loft_corners(&solid.loft)
                .into_iter()
                .map(move |corner| (corner, id))
        }),
        wood.iter().map(|solid| solid.loft.triangles().len()).sum(),
    )
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
