//! Instance transforms: our `transform` as a Cycles `<transform>` scope.
//!
//! # The convention difference
//!
//! Our document stores a translation, a rotation as three Euler angles in degrees, and a
//! scale. The loader takes a translation, a scale, and a rotation as **an axis and an angle
//! in degrees** — `rotate="20 1 0 0"` is twenty degrees about the vector `(1, 0, 0)`.
//!
//! Euler angles and an axis-angle pair are different parameterisations of the same
//! rotation, so this is a conversion rather than a mapping, and it has to happen here or
//! every instance in the scene comes out rotated wrongly.
//!
//! The order the loader composes in is fixed and is the reverse of the usual reading order:
//! it applies `matrix`, then `translate`, then `rotate`, then `scale`, each multiplied onto
//! the enclosing scope. So one scope carries one transform and nesting is the only way to
//! compose, which is what an `Instance` is.

use super::error::EmitResult;
use super::read;
use definition::Value;

/// One instance's transform, in the document's parameterisation.
#[derive(Clone, Copy)]
pub struct Transform {
    pub translation: [f64; 3],
    /// Euler angles in degrees, applied X then Y then Z.
    pub rotation: [f64; 3],
    pub scale: [f64; 3],
}

impl Transform {
    /// The identity, for an instance that writes no transform.
    pub fn identity() -> Self {
        Self {
            translation: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

/// Reads an instance's `transform`, defaulting to the identity.
///
/// A missing transform is the identity rather than an error: an instance with no transform
/// is a mesh at the origin, which is a thing authors write.
pub fn read_transform(value: Option<&Value>, context: &str) -> EmitResult<Transform> {
    let Some(value) = value else {
        return Ok(Transform::identity());
    };
    let fields = value
        .as_field_map()
        .ok_or_else(|| super::error::EmitError::Malformed {
            detail: format!("{context}'s transform is not a structure"),
        })?;
    Ok(Transform {
        translation: triple_or(fields, "translation", [0.0; 3])?,
        rotation: triple_or(fields, "rotation", [0.0; 3])?,
        scale: triple_or(fields, "scale", [1.0; 3])?,
    })
}

fn triple_or(fields: &definition::FieldMap, name: &str, default: [f64; 3]) -> EmitResult<[f64; 3]> {
    match read::field(fields, name) {
        Some(value) => read::triple(value, &format!("{name}")),
        None => Ok(default),
    }
}

/// Converts Euler angles in degrees to the axis and angle the loader wants.
///
/// # Why this exists
///
/// The loader's `rotate` attribute is one axis and one angle, and the `transform_rotate` it
/// calls applies a single rotation about that axis. Three Euler angles are three rotations,
/// so the sum of the three rotation matrices has to be reduced to the single axis-angle pair
/// that produces the same orientation.
///
/// The quaternion route is the one that does not need case analysis. Three Euler angles
/// become a quaternion by composition, and a quaternion becomes an axis and an angle by
/// normalisation. Both steps are exact, so the emitted scene places instances where the
/// document said they go.
///
/// # What it rotates decides how the basis change reaches it
///
/// The angles are in this engine's axes and the axis and angle are Cycles', so the basis
/// change has to be applied — and a rotation of **points** and a rotation that carries a
/// **frame** take it differently. See [`super::axes::Rotation`]; this is the only place the
/// two meet, so there is nowhere else for them to drift apart.
///
/// A zero rotation has no axis to name. It is written as no rotation at all rather than as
/// some arbitrary axis, because a zero-length axis is a division by zero in the loader and
/// an arbitrary one is a lie about the scene.
pub fn axis_angle(rotation: [f64; 3], kind: super::axes::Rotation) -> Option<([f64; 3], f64)> {
    let quaternion = quaternion_from_euler(
        rotation[0].to_radians(),
        rotation[1].to_radians(),
        rotation[2].to_radians(),
    );
    let quaternion = match kind {
        super::axes::Rotation::Points => super::axes::rotation_points(quaternion),
        super::axes::Rotation::Frame => super::axes::rotation_frame(quaternion),
    };
    let sin_half = (quaternion[1].powi(2) + quaternion[2].powi(2) + quaternion[3].powi(2)).sqrt();
    if sin_half < 1e-9 {
        return None;
    }
    let angle = 2.0 * sin_half.atan2(quaternion[0]);
    let axis = [
        quaternion[1] / sin_half,
        quaternion[2] / sin_half,
        quaternion[3] / sin_half,
    ];
    Some((axis, angle.to_degrees()))
}

/// Composes three Euler rotations, X then Y then Z, into one quaternion.
///
/// The composition order matches the reading order of the document's `rotation` field,
/// which is also the order the rasterised pipeline applies them in.
///
/// # Two of these four terms had the wrong sign
///
/// This was `w = cx*cy*cz - sx*sy*sz`, which is the scalar part of a **conjugate**, not of a
/// composition. It is wrong for every rotation that turns about more than one axis, and the
/// scalar part is the one that decides the *magnitude* of the rotation: a negative `w` takes the
/// angle out through `atan2`, so a half turn comes back as 360 minus it.
///
/// The measured symptom: a camera at `(0, 0.7, 2.6)` looking at `(0, 0.1, -8)` is a half turn
/// about `Y` followed by a small pitch. Composed correctly that is 180 degrees about an axis
/// almost parallel to `Y`. This function produced `w = -0.1248`, and
/// `2 * atan2(sin_half, w)` turned that into **353.5 degrees about an axis parallel to `X`** —
/// a rotation that points the camera at nothing. The render came back black, with no error, no
/// warning and a full sample count, because the document it was given was a scene with the
/// camera aimed away from every surface in it.
///
/// The composition of three rotations about X, then Y, then Z, with `w` first, is:
///
/// ```text
/// w = cx*cy*cz + sx*sy*sz
/// x = sx*cy*cz - cx*sy*sz
/// y = cx*sy*cz + sx*cy*sz
/// z = cx*cy*sz - sx*sy*cz
/// ```
fn quaternion_from_euler(x: f64, y: f64, z: f64) -> [f64; 4] {
    // Halving the **argument** is what produces the half-angle. Writing it as `sin(x) * 0.5`
    // instead — which is what this did — is not the same thing: it is only equal for the sine by
    // the double-angle identity applied backwards, and it is not equal for the cosine at all.
    // `cos(0)` became `0.5` rather than `1`, so a rotation about one axis was scaled by a factor
    // that depended on how the other two happened to be zero.
    let (sx, cx) = ((x * 0.5).sin(), (x * 0.5).cos());
    let (sy, cy) = ((y * 0.5).sin(), (y * 0.5).cos());
    let (sz, cz) = ((z * 0.5).sin(), (z * 0.5).cos());
    [
        cx * cy * cz + sx * sy * sz,
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
    ]
}
