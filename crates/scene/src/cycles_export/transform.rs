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
/// A zero rotation has no axis to name. It is written as no rotation at all rather than as
/// some arbitrary axis, because a zero-length axis is a division by zero in the loader and
/// an arbitrary one is a lie about the scene.
pub fn axis_angle(rotation: [f64; 3]) -> Option<([f64; 3], f64)> {
    let [rx, ry, rz] = rotation.map(f64::to_radians);
    let quaternion = quaternion_from_euler(rx, ry, rz);
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
fn quaternion_from_euler(x: f64, y: f64, z: f64) -> [f64; 4] {
    let (sx, cx) = (x.sin() * 0.5, x.cos() * 0.5);
    let (sy, cy) = (y.sin() * 0.5, y.cos() * 0.5);
    let (sz, cz) = (z.sin() * 0.5, z.cos() * 0.5);
    [
        cx * cy * cz - sx * sy * sz,
        sx * cy * cz + cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
    ]
}
