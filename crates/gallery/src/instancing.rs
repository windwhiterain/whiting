//! Standing one template's mesh in many places: the scene side of a building's parts.
//!
//! # What this is for
//!
//! `building::expand` bakes every part's geometry, so a facade with twelve identical windows uploads twelve
//! copies of the same 28 faces. The crate has handed out the two pieces that make that unnecessary —
//! [`Template::mesh`](building::parts::Template::mesh), one expansion in the template's own frame, and
//! [`PartFrame`], where a placement stands it — and this module is the join a *scene* needs: a frame turned
//! into the translation and rotation a scene instance carries.
//!
//! # Invariants
//!
//! - **A wall-mounted frame is a yaw about `y` and a translation, and nothing else.** Its `along` and
//!   `inward` lie in the plan and its `up` is world up (`(0, 1, 0)`) by construction, so the rotation is one
//!   angle: a scene's instance transform can carry it exactly rather than approximately.
//! - **The angle is read from the frame's own axes, not from a wall's.** `yaw = atan2(inward.x, inward.z)`
//!   is the angle that takes local `+x` to `along` and local `+z` to `inward` under
//!   [`transform_matrix`](scene::standard::math::transform_matrix)'s Euler convention, and it is
//!   derived from the frame so a second derivation cannot disagree with the first about which way a wall
//!   faces.
//! - **The scale is one.** A template's mesh is already in the size it will be drawn at; a scale here would
//!   be a second authoring of the same number.

use building::parts::PartFrame;

/// One placement's instance transform: where it stands, how it is turned, and what it is scaled by.
///
/// The three fields are exactly what a scene instance declares, in that order.
pub type Standing = ([f64; 3], [f64; 3], [f64; 3]);

/// Returns the instance transform that stands a template's own mesh where a placement puts it.
///
/// # Invariants
///
/// See the module's: the rotation is a yaw about `y`, read from the frame's `inward`, and the scale is one.
pub fn standing(frame: &PartFrame) -> Standing {
    let yaw = f64::from(frame.inward[0]).atan2(f64::from(frame.inward[2]));
    (
        [
            f64::from(frame.origin[0]),
            f64::from(frame.origin[1]),
            f64::from(frame.origin[2]),
        ],
        [0.0, yaw, 0.0],
        [1.0; 3],
    )
}

/// Returns the points of one template's own mesh, ready to be drawn once and instanced.
///
/// # Invariants
///
/// **The points are in the template's frame**, which is the space the instance transform expects: the
/// frame's origin becomes the instance's translation and its axes the instance's rotation, so a point at
/// the template's own origin lands at the placement's origin.
pub fn template_points(
    template: &building::parts::Template,
) -> Result<definition::PackedList, Vec<building::expand::Problem>> {
    let held = template.mesh()?;
    Ok(crate::building_mesh::points_where(&held, |_| true))
}
