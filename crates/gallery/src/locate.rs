//! Answers "which authored part of the scene is this pixel?", from what a frame declared.
//!
//! # What this is
//!
//! The four hops of [`.agents/proposals/agent-content-identity.md`](../../../.agents/proposals/agent-content-identity.md),
//! composed into one call. A caller reads the three values off a rendered frame and gets a line
//! of code:
//!
//! ```text
//! landscape/valley_3/left_flank
//!   id 137  face 1804  u 0.612  v 0.831
//!   top    Ridge(RidgeId(5)), reached between the stations of valley 3 nodes 7 and 8
//!   the edit: massif.skeleton.valleys[3].nodes[7].elevation  (or node 8)
//! ```
//!
//! **The point of composing them here is that nobody can compose them wrongly.** Each hop is
//! owned by whatever knows it — the frame for the values, this module's table for the path, the
//! mountain crate for the belt, the skeleton for the lines — and a caller that chained them itself
//! would be able to chain them wrong, and would be wrong in the middle, where the answer looks
//! like an answer.
//!
//! # Why it takes three numbers and not a frame
//!
//! **The dependency runs device → gallery, and a frame is the device's type.** Taking the three
//! values keeps that true: this crate asks nothing of the renderer, and a caller that has the
//! values from anywhere — a readback, a sidecar, a test — can ask the same question. It also
//! keeps the arithmetic here honest about what it actually needs, which is three floats.
//!
//! # What it refuses
//!
//! - A scene it has no naming for, and a value of zero — which is the plain, and is what the
//!   reserved zero is for.
//! - Values from a frame and a mesh that are from different builds. The id arrives in pixels and
//!   the mesh comes from a fresh build, and the two agreeing is a fact about history that no
//!   single call can establish; the caller's manifest is what says so, and this says nothing
//!   rather than naming another element's path.

use crate::identity::{ID_OUTPUT, U_OUTPUT, V_OUTPUT};

/// Resolves the three values a frame declared at one pixel to the authored part they name.
///
/// # Invariants
///
/// - **A value of zero is answered as itself, not as "unknown".** It is a real answer about a
///   real pixel — this one is the plain — and a caller told "not an authored element" knows not
///   to go looking for a crest there. `None` is reserved for a scene with no naming, which is a
///   different thing entirely.
/// - **An id above the count is reported, not clamped.** A frame and a mesh from different builds
///   produce exactly that, and the number in the message is what tells the two apart.
pub fn locate(scene: &str, values: &[(String, f32)]) -> Option<String> {
    let one = |name: &str| {
        values
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, value)| *value)
    };
    // **A building's frame answers through its own id**, and the two domains are asked in the order their
    // channels exist: a building scene declares `authored_id` and nothing else, the landscape declares the id
    // and a belt coordinate. Asking both and taking whichever answers is what keeps this one function rather
    // than one per subsystem — and neither answers for a scene it does not know.
    if let Some(id) = one(ID_OUTPUT) {
        if let Some(answer) = crate::building_identity::locate_in(scene, id.round() as u32) {
            return Some(answer);
        }
        // **A plant's frame answers through the same channel**, and its answer is the handle that names
        // the node rather than a building element's path: the id is the node's position in the plant's own
        // walk, and a leaf drawn by the blade mesh carries its twig's. The reserved zero is answered as
        // itself, because a pixel of the ground a plant stands on is a real answer about a real pixel.
        if scene == crate::vegetation_picture::SCENE
            || scene == crate::vegetation_picture::WINDY
            || scene == crate::vegetation_picture::CROWNED
            || scene == crate::vegetation_picture::KNOTTED
        {
            let id = id.round() as u32;
            return Some(match crate::vegetation_picture::handle_of(id) {
                Some(handle) => {
                    format!("{handle}\n  authored_id {id}: this pixel is that node of the plant, and a \
                             leaf drawn by the blade mesh carries its twig's")
                }
                None if id == crate::identity::NOT_AUTHORED => {
                    "not an authored element: this pixel is the ground the plant stands on, or the sky \
                     above it"
                        .to_owned()
                }
                None => format!("authored_id {id} is not one of this plant's nodes"),
            });
        }
    }
    crate::identity::locate(
        scene,
        one(ID_OUTPUT).map(|value| [value, 0.0, 0.0, 0.0]),
        one(U_OUTPUT).unwrap_or(0.0),
        one(V_OUTPUT).unwrap_or(0.0),
    )
}
