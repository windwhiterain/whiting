//! Standing one leaf's rigid frame as the scene instance that draws it.
//!
//! # What this is for
//!
//! `vegetation::leaf::LeafTemplate` is one blade's geometry, authored once in its own frame, and every leaf
//! of a field is that blade placed by a [`LeafInstance`]: a rigid [`Placement`] and one uniform scale. A
//! scene draws a mesh by a **translation, a rotation and a scale**, so something has to turn the frame
//! into those three numbers, and this is the one place that does it — the way [`crate::instancing`] is the
//! one place a building's part frame becomes an instance transform.
//!
//! # The order the three numbers are extracted in
//!
//! [`transform_matrix`](scene::standard::math::transform_matrix) composes **`Rx · Ry · Rz`**, applied to
//! the right of the point, and its own doc examples pin that convention: a quarter turn about `z` takes
//! `+x` to `+y`. A [`Placement`] is orthonormal and right-handed (`vegetation::plant::Placement` states
//! `z = x × y`), so its columns **are** that rotation's matrix, and the extraction is the standard one for
//! that order:
//!
//! ```text
//! ry = asin(M[0][2])                    rx = atan2(−M[1][2],  M[2][2])    rz = atan2(−M[0][1],  M[0][0])
//! ```
//!
//! read off the placement's own axes, so `M[row][column]` is `axis[column][row]` and
//! `M[0][2] = z[0]`, `M[1][2] = z[1]`, `M[2][2] = z[2]`, `M[0][1] = y[0]`, `M[0][0] = x[0]`.
//!
//! **The gimbal case is where `x` is world up**, which is `|M[0][2]| = |sin ry| = 1`: then `rz` is set to
//! zero and `rx = atan2(M[2][1], M[1][1]) = atan2(y[2], y[1])`, which is the same rotation written with
//! its first and last turns collapsed onto one axis. Both branches are stated here because the
//! extraction is the one conversion in this slice that cannot fail loudly: a wrong order or a wrong sign
//! still produces three numbers, and they still produce a frame — somewhere else. The probe
//! `plant_instancing_probe` asserts the round trip, which is the only thing that can see it.
//!
//! # What the scale is, and where it is applied
//!
//! `vegetation` states a template point reaches the world as `placement.place(point) * scale`: the frame
//! first, then one uniform multiplication. A scene's instance transform applies the **scale first** and
//! the translation last (`transform_matrix`'s own doc: *"Scale is applied first, so it does not scale the
//! translation"*), which is `scale · R · p + origin` rather than `scale · R · p + scale · origin`. The two
//! agree exactly when the scale is one, and they differ by `(scale − 1) · origin` otherwise — which is
//! measured, not assumed, by the probe's `place(p) · s` row, which reports every leaf that differs and
//! by how much. **The scene transform is the one that stands a leaf in the world**: a leaf's origin is a
//! point on its parent's skin, and scaling it toward the plant's root would drag every leaf off the twig
//! it hangs from, in proportion to how small the leaf is.

use scene::standard::math::transform_matrix;
use vegetation::leaf::LeafInstance;
use vegetation::plant::Placement;

/// One instance transform: where the blade stands, how it is turned, and what it is scaled by.
///
/// The three fields are exactly what a scene instance declares, in that order — the same alias
/// [`crate::instancing::Standing`] gives a building's part.
pub type Standing = ([f64; 3], [f64; 3], [f64; 3]);

/// The cosine of `ry` below which the frame is read as a gimbal lock rather than as two turns.
///
/// `1e-9` is a quarter of a degree's worth of cosine at this magnitude: far below any frame a leaf field
/// places on its own, and far above the rounding of a frame whose `x` is world up by accident.
const GIMBAL_COSINE: f64 = 1.0e-9;

/// Returns the three turns that take the blade's own `+x`, `+y`, `+z` onto the instance's axes.
///
/// # Invariants
///
/// - **The order is `Rx · Ry · Rz`, read in that order and written in that order** into the instance's
///   `rotation` field, which is what [`transform_matrix`] composes. The two halves are the whole
///   contract: a rotation extracted for `Rz · Ry · Rx` is a perfectly good rotation and puts every leaf
///   somewhere other than the twig it hangs from.
/// - **`ry` is the principal turn, so `cos ry ≥ 0`**, which is what makes the other two unique. A frame
///   whose `x` is world up has no such `ry` and takes the gimbal branch: `rz = 0` and `rx` carries the
///   whole turn, which is the same rotation.
/// - **The frame is not renormalised.** A [`Placement`] states that `(x, y, z)` is orthonormal, and a
///   renormalising extraction would silently repair a frame that is not — and this is the one conversion
///   whose failure is a picture rather than an error.
pub fn euler(placement: &Placement) -> [f64; 3] {
    let (x, y, z) = (placement.x, placement.y, placement.z);
    let sin_y = f64::from(z[0]);
    // `M[0][2] = sin ry`, so the principal turn is its arcsine and the other two are read off the axes
    // the two remaining rows name. The clamp is the domain of `asin`, not a repair: a frame that is
    // orthonormal gives `|sin ry| ≤ 1` up to the rounding of `f32`, and a rounding past it is not a
    // different frame.
    let ry = sin_y.clamp(-1.0, 1.0).asin();
    let cosine = (1.0 - sin_y * sin_y).sqrt();
    if cosine > GIMBAL_COSINE {
        [
            f64::from(-z[1]).atan2(f64::from(z[2])),
            ry,
            f64::from(-y[0]).atan2(f64::from(x[0])),
        ]
    } else {
        [f64::from(y[2]).atan2(f64::from(y[1])), ry, 0.0]
    }
}

/// Returns the instance transform that draws the blade where a leaf instance puts it.
///
/// # Invariants
///
/// - **`position` is `placement.origin` and `scale` is `[instance.scale; 3]`**, both as `f64`. The scale
///   is uniform because [`vegetation::leaf::LeafInstance`] states the blade's size is one number, and a
///   non-uniform scale here would be a second authoring of the same number.
/// - **The rotation is [`euler`]'s, and nothing else turns the frame.** A scene's transform is
///   translation, rotation, scale, so the order is fixed by the document and only the extraction is a
///   choice.
pub fn standing(leaf: &LeafInstance) -> Standing {
    let (translation, rotation, scale) = (
        leaf.placement.origin.map(f64::from),
        euler(&leaf.placement),
        [f64::from(leaf.scale); 3],
    );
    (translation, rotation, scale)
}

/// Returns the matrix [`standing`]'s transform stands as, which is what a scene compiles the instance
/// into.
///
/// # Invariants
///
/// **The matrix is built from the very three numbers [`standing`] returns**, so a probe measuring the
/// round trip measures the document's own convention rather than a second derivation of it.
pub fn matrix(leaf: &LeafInstance) -> scene::standard::math::Matrix {
    let (translation, rotation, scale) = standing(leaf);
    transform_matrix(translation, rotation, scale)
}
