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

/// How finely a belt is subdivided along its length.
///
/// The only number left in this file, because it is the one thing about the massif that is not part of
/// its shape: it is how much resolution the facets get, and the spec's block faces are only as small as
/// this makes them.
const STEP: f32 = 46.0;

/// Builds the massif.
pub fn massif() -> mountain::Massif {
    mountain::Massif::new(authored::skeleton())
        .with_tessellation(Tessellation::default().with_step(STEP))
}
