//! The ground a plant stands on: an authored function, and the two questions a seated plant asks of it.
//!
//! # The one idea
//!
//! **Seating takes `impl Fn([f32; 2]) -> f32`**, so a caller with a `terrain::HeightField` passes its sampler and this
//! crate stays terrain-independent — `building`'s D8 rule adopted verbatim, and for the same reason: `terrain`
//! produces numbers and never geometry, and this crate produces geometry and never erosion.
//!
//! # What "on the ground" means
//!
//! A plant's base is the disc of its root, so a base is on the ground when the surface passes within the base's own
//! radius of the point it stands at. That is what makes *"a plant whose base is not on the ground"* a violation rather
//! than a picture of a floating plant: a root sunk half a metre into a courtyard and a root hanging a metre above it
//! are the same defect seen from two sides, and [`footing`] reports it as one signed number.

use crate::plant::Placement;

/// A placement moved onto the ground: its base is at the surface's own height under it.
///
/// # Invariants
///
/// **Only the height changes.** The plan position and the turn a caller chose are theirs, so seating a plant cannot
/// move it sideways or turn it — a rule that could would make a seam of two plants a thing nobody could predict. A
/// placement already on a flat ground is returned unchanged, which is what lets a scene add a ground and keep the
/// picture it had.
pub fn seat(placement: Placement, ground: impl Fn([f32; 2]) -> f32) -> Placement {
    let height = ground([placement.origin[0], placement.origin[2]]);
    Placement {
        origin: [placement.origin[0], height, placement.origin[2]],
        ..placement
    }
}

/// How far a base stands above the ground: positive floating, negative sunk, zero on it.
pub fn footing(placement: &Placement, ground: impl Fn([f32; 2]) -> f32) -> f32 {
    placement.origin[1] - ground([placement.origin[0], placement.origin[2]])
}

/// Whether a base of `radius` is on the ground: the surface passes within its own radius of where it stands.
///
/// # Invariants
///
/// The tolerance is the base's **own** radius and not a number this crate picks, so a culm of two centimetres and a
/// shrub of half a metre are judged by the same rule and neither is asked for an accuracy its own geometry does not
/// have.
pub fn on_the_ground(placement: &Placement, radius: f32, ground: impl Fn([f32; 2]) -> f32) -> bool {
    footing(placement, ground).abs() <= radius
}
