//! The six stages, each one an intern over the slab.
//!
//! # Invariant
//!
//! A stage's key carries **every** input that can change its output and nothing
//! else. A stage that keyed on a parameter it did not read would answer a stale
//! value under an id every downstream cell still believes.

use gallery::southwest_oak as oak;
use gallery::vegetation_mesh::{LEAF_BASE_ID, node_ids_from};
use gallery::vegetation_picture::{Drawn, Planting, View, plant_scene_drawn};
use vegetation::decorator::placed;
use vegetation::envelope::trim;
use vegetation::expand::{Detail, Expansion};
use vegetation::leaf::LeafGroup;
use vegetation::plant::Plant;

use crate::slab::{self, Held};

/// The oak's own detail dial at `stations`, `around` and `spacing`.
fn oak_detail(stations: usize, around: usize, spacing: f32) -> Detail {
    Detail {
        stations,
        around,
        spacing,
    }
}

/// The detail dial the wood under `id` was expanded at.
///
/// # Invariant
///
/// The dial is read back out of the wood's **own key**, so the document stage
/// builds its planting under the same dial the wood was drawn at without the
/// caller repeating it and having to keep the two consistent.
fn detail_of(id: u64) -> Option<Detail> {
    match slab::parameters(id).as_slice() {
        [stations, around, spacing] => Some(oak_detail(
            *stations as usize,
            *around as usize,
            *spacing as f32,
        )),
        _ => None,
    }
}

/// A dressed plant, or `None` when the slab does not hold the id.
fn dressed_of(id: u64) -> Option<Plant> {
    match slab::get(slab::stage::DRESSED, id) {
        Some(Held::Dressed(plant)) => Some(plant),
        _ => None,
    }
}

/// Grow the oak's skeleton from `seed`.
///
/// # Invariant
///
/// `seed` is the only parameter: the rule is the oak's own, and the growth is a
/// function of the rule and the seed alone.
pub fn tree(seed: u64) -> u64 {
    let key = slab::key(slab::stage::TREE, &[], &[seed]);
    slab::intern(key, || Held::Tree(oak::alpine_oak_tree(seed)))
}

/// Dress a grown skeleton: the fluted bole, then the crown's envelope.
///
/// # Invariant
///
/// `envelope` shifts the crown's radius law by so many metres, so `0.0` is
/// exactly [`oak::crown_envelope`] and the committed composition is the baseline.
pub fn dressed(tree: u64, envelope: f32) -> Option<u64> {
    let key = slab::key(slab::stage::DRESSED, &[tree], &[slab::bits(envelope)]);
    let mut plant = match slab::get(slab::stage::TREE, tree) {
        Some(Held::Tree(plant)) => plant,
        _ => return None,
    };
    oak::alpine_oak_dress(&mut plant);
    trim(&mut plant, &crown(envelope));
    Some(slab::intern(key, || Held::Dressed(plant)))
}

/// The oak's crown envelope with its radius law shifted by `envelope` metres.
fn crown(envelope: f32) -> vegetation::envelope::Envelope {
    let authored = oak::crown_envelope();
    vegetation::envelope::Envelope {
        radius: authored.radius.offset(envelope),
        shape: authored.shape,
    }
}

/// Expand a dressed plant's wood at the detail dial.
///
/// # Invariant
///
/// Every dial component enters the key as bits, so `stations: 8` and
/// `stations: 9` are different wood and a recompute at one dial is one id.
pub fn wood(dressed: u64, stations: usize, around: usize, spacing: f32) -> Option<u64> {
    let key = slab::key(
        slab::stage::WOOD,
        &[dressed],
        &[
            slab::count(stations),
            slab::count(around),
            slab::bits(spacing),
        ],
    );
    let plant = dressed_of(dressed)?;
    Some(slab::intern(key, || {
        Held::Wood(vegetation::expand::wood(
            &plant,
            oak_detail(stations, around, spacing),
        ))
    }))
}

/// Every leaf group of a dressed plant, at the density dial.
///
/// # Invariant
///
/// `density` is an **offset** on each field's own density law, which keeps that
/// law's knots and its slopes; `0.0` is the oak's own leafing exactly.
pub fn leaves(dressed: u64, density: f32) -> Option<u64> {
    let key = slab::key(slab::stage::LEAVES, &[dressed], &[slab::bits(density)]);
    let plant = dressed_of(dressed)?;
    Some(slab::intern(key, || {
        // The two halves of what `expand` does internally: the species' own
        // fields, each read off the same plant the wood is expanded from.
        let species = oak::alpine_oak_species(plant.clone());
        let mut groups: Vec<LeafGroup> = Vec::new();
        for authored in &species.fields {
            let mut field = authored.clone();
            field.density = field.density.offset(density);
            groups.extend(field.groups(&plant));
        }
        Held::Leaves(groups)
    }))
}

/// Draw one planting: the expansion's two halves, its ids and its decorators.
///
/// # Invariant
///
/// `index` is the planting's rank in the scene, because its node ids are
/// allocated in that order; it enters the key so a second planting can never be
/// mistaken for the first.
pub fn drawn(wood: u64, leaves: u64, index: usize) -> Option<u64> {
    let key = slab::key(slab::stage::DRAWN, &[wood, leaves], &[slab::count(index)]);
    let (Some(Held::Wood(expansion)), Some(Held::Leaves(foliage))) = (
        slab::get(slab::stage::WOOD, wood),
        slab::get(slab::stage::LEAVES, leaves),
    ) else {
        return None;
    };
    // The two stages name the same dressed parent, or the drawing joins two
    // plants and the ids below would be of neither.
    let wood_parents = slab::parents(wood);
    let leaf_parents = slab::parents(leaves);
    if wood_parents != leaf_parents || wood_parents.len() != 1 {
        return None;
    }
    let plant = dressed_of(wood_parents[0])?;
    let seats = oak_fruit(&plant);
    let planting = Planting::new(oak::alpine_oak_species(plant.clone()))
        .with_seats(seats.clone())
        .with_palette(oak::oak_palette());
    Some(slab::intern(key, || {
        Held::Drawn(Drawn {
            expansion: Expansion {
                wood: expansion,
                leaves: foliage,
            },
            ids: node_ids_from(
                &plant,
                LEAF_BASE_ID + index as u32 * gallery::vegetation_picture::PLANTING_STRIDE,
            ),
            decorators: placed(&plant, &seats),
            placement: planting.placement,
        })
    }))
}

/// Assemble the scene document from a drawing already made.
///
/// # Invariant
///
/// The key names the drawing and the three key-light components and nothing else,
/// so a lighting edit re-runs the document alone while every geometry stage is
/// reused.
pub fn document(drawn: u64, key_position: [f32; 3]) -> Option<u64> {
    let key = slab::key(
        slab::stage::DOCUMENT,
        &[drawn],
        &[
            slab::bits(key_position[0]),
            slab::bits(key_position[1]),
            slab::bits(key_position[2]),
        ],
    );
    let Some(Held::Drawn(drawing)) = slab::get(slab::stage::DRAWN, drawn) else {
        return None;
    };
    let drawing_parents = slab::parents(drawn);
    let [wood, _leaves] = drawing_parents.as_slice() else {
        return None;
    };
    let detail = detail_of(*wood)?;
    let wood_parents = slab::parents(*wood);
    let [dressed] = wood_parents.as_slice() else {
        return None;
    };
    let plant = dressed_of(*dressed)?;
    let planting = Planting::new(oak::alpine_oak_species(plant.clone()))
        .with_seats(oak_fruit(&plant))
        .with_palette(oak::oak_palette())
        .with_detail(detail);
    let mut lighting = oak::oak_lighting();
    for (axis, offset) in lighting.key_position.iter_mut().zip(key_position) {
        *axis += f64::from(offset);
    }
    Some(slab::intern(key, || {
        Held::Document(plant_scene_drawn(
            &[drawing],
            &[planting],
            &oak::OAK_FRAME,
            oak::ALPINE_OAK,
            &View::PHOTOGRAPH,
            &lighting,
            &[],
        ))
    }))
}

/// The oak's fruit seats on a dressed plant.
///
/// # Invariant
///
/// The seats are the oak's own scatter at the oak's own shoot depth, so a drawing
/// and the document that carries it place the same ornaments on the same nodes.
pub fn oak_fruit(plant: &Plant) -> Vec<vegetation::decorator::Seat> {
    oak::oak_fruit(&oak::alpine_oak_species(plant.clone()), oak::SHOOTS)
}
