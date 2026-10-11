//! The massif this scene is of: **written down**, in [`authored`].
//!
//! # Why there is nothing here but a call
//!
//! There used to be a great deal here — a walk of the ridge graph's faces, a search for each basin's
//! head, a floor laid out along a bisector, a rim woven from the basins in the order they were found.
//! All of it derived the valleys from the ridges, and it was the wrong thing to build: the control
//! structure of a massif is the ridge polylines, the valley polylines and the correspondence between
//! them, and that structure is **meant to be written directly**. Deriving it is a generator, and a
//! generator here goes wrong in the way generators do — the derived floor was a fan, and a fan is the
//! right shape for the one summit-centred cone that the derivation was written for and the wrong shape
//! for everything else, so the massif could not become a dissected upland without the derivation being
//! rewritten underneath it.
//!
//! What is left is one call. If this module grows again, it should be because the *authoring* needs
//! something — a helper that places a junction so the author does not repeat a coordinate — and not
//! because some part of the structure wants to be inferred.

mod authored;

use mountain::Tessellation;

/// How wide a facet is allowed to be, as a fraction of the massif's own extent across the plan.
///
/// The only number left in this file, and it is a **ceiling** rather than a spacing: a stretch of
/// line between two anchors — a node or a station a valley named — is never cut finer than one
/// facet unless it is longer than this. The anchors are the ribs, so a facet is about as wide as
/// the flank is deep and the cells come out square instead of the combs a fixed spacing cut them
/// into. Twelve is the number that reads as blocks rather than as drapes on a massif this size.
const FACET_FRACTION: f64 = 12.0;

/// Builds the massif.
pub fn massif() -> mountain::Massif {
    let skeleton = authored::skeleton();
    let step = facet_ceiling(&skeleton) as f32;
    mountain::Massif::new(skeleton).with_tessellation(Tessellation::default().with_step(step))
}

/// Returns how wide a facet may be, read from the skeleton's own extent across the plan.
fn facet_ceiling(skeleton: &mountain::Skeleton) -> f64 {
    let (mut low, mut high) = ([f64::MAX, f64::MAX], [f64::MIN, f64::MIN]);
    for ridge in &skeleton.ridges {
        for node in &ridge.nodes {
            for axis in 0..2 {
                low[axis] = low[axis].min(node.position[axis] as f64);
                high[axis] = high[axis].max(node.position[axis] as f64);
            }
        }
    }
    let extent = ((high[0] - low[0]).powi(2) + (high[1] - low[1]).powi(2)).sqrt();
    extent / FACET_FRACTION
}
