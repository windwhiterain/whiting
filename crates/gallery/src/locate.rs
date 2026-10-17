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
//! # The registry
//!
//! **One row per scene, and the row is data rather than a branch.** A scene that names its own elements registers
//! a [`Naming`]: the resolver that turns an id into a handle, and the one clause that says what an id means in that
//! scene. `--locate` and `--region` ask the same table and want different sentences, so the table answers handles and
//! the asker writes the prose — which is also why a scene absent from it is refused rather than guessed at.
//!
//! # Why it takes three numbers and not a frame
//!
//! **The dependency runs device → gallery, and a frame is the device's type.** Taking the three
//! values keeps that true: this crate asks nothing of the renderer, and a caller that has the
//! values from anywhere — a readback, a sidecar, a test — can ask the same question. It also
//! keeps the arithmetic here honest about what it actually needs, which is three floats.

use crate::identity::{ID_OUTPUT, NOT_AUTHORED, U_OUTPUT, V_OUTPUT};

/// A scene's own answer to *"which authored element is this id?"*.
pub type Resolver = fn(u32) -> Option<String>;

/// What a scene says about its own ids: how to resolve one, and what one means.
#[derive(Debug, Clone, Copy)]
pub struct Naming {
    /// The scene's name, which is the name a document carries and a frame is asked about.
    pub scene: &'static str,
    /// The id's handle, or `None` when the id names nothing in this scene's content.
    pub resolve: Resolver,
    /// What an id means here, as one clause: the sentence an asker puts after the handle.
    pub note: &'static str,
}

/// Every scene that names its own elements, keyed by name.
///
/// # Invariants
///
/// - **A scene appears once.** A name in this table is a scene whose pixels can be answered; a scene absent from it
///   falls through to [`crate::identity::locate`]'s own answer, which is the landscape's, and is refused by a caller
///   that has no naming for it.
/// - **The resolver answers the handle and nothing else**, so the two askers — a pixel and a rectangle — cannot
///   disagree about what an id means: they read one function.
/// - **The four still scenes share one resolver**, because they draw one plant; the sculpted and grove scenes have
///   their own because their trees differ from the reference tree by at least one node.
pub const NAMINGS: &[Naming] = &[
    Naming {
        scene: crate::vegetation_picture::SCENE,
        resolve: crate::vegetation_picture::handle_of,
        note: "this pixel is that node of the plant, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::vegetation_picture::WINDY,
        resolve: crate::vegetation_picture::handle_of,
        note: "this pixel is that node of the plant under its wind, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::vegetation_picture::CROWNED,
        resolve: crate::vegetation_picture::handle_of,
        note: "this pixel is that node of the crowned plant, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::vegetation_picture::KNOTTED,
        resolve: crate::vegetation_picture::handle_of,
        note: "this pixel is that node of the knotted plant, and a leaf or a knot drawn by an instanced mesh carries its node's",
    },
    Naming {
        scene: crate::vegetation_picture::SCULPTED,
        resolve: crate::vegetation_picture::sculpted_handle_of,
        note: "this pixel is that node of the sculpted plant, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::vegetation_picture::SEGMENTATION,
        resolve: crate::vegetation_picture::handle_of,
        note: "one colour per element: this pixel is that node of the plant, and a leaf wears the colour of the twig it grew from",
    },
    Naming {
        scene: crate::vegetation_picture::GROVE,
        resolve: crate::vegetation_picture::grove_handle_of,
        note: "the planting is the id's quotient by the stride and the node is its remainder",
    },
];

/// The row a scene registered, or `None` when it names nothing.
pub fn naming(scene: &str) -> Option<&'static Naming> {
    NAMINGS.iter().find(|naming| naming.scene == scene)
}

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
        let id = id.round() as u32;
        if let Some(answer) = crate::building_identity::locate_in(scene, id) {
            return Some(answer);
        }
        if let Some(naming) = naming(scene) {
            return Some(match (naming.resolve)(id) {
                Some(handle) => format!("{handle}\n  authored_id {id}: {}", naming.note),
                None if id == NOT_AUTHORED => {
                    "not an authored element: this pixel is the ground the plant stands on, or the sky \
                     above it"
                        .to_owned()
                }
                None => format!("authored_id {id} is not one of this scene's elements"),
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
