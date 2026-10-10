//! This engine's axes, and the ones Cycles has.
//!
//! # The difference
//!
//! This engine is **y-up**. Cycles is **z-up**. Nothing else about the two agrees — not the
//! order of the components, not which one carries the height — and the format has no element
//! that says which way up a scene is. So every coordinate the emitter writes has to be put
//! into Cycles' axes, or the scene is read turned.
//!
//! Measured, and the symptom is quiet. `foldyard`'s camera is emitted with a rotation almost
//! entirely about Y, which leaves its up on Cycles' `+Y`, and a plane of constant `y` therefore
//! reads as a floor: the geometry came out looking right. The **sky** did not, because
//! Cycles' environment lookup is `v = acos(dir.z) / pi` — z-up — and so an image whose rows
//! run zenith to nadir was sampled across an axis lying sideways in the frame. An axis ramp
//! put in as the world showed the image's own row running from 0.83 to 0.51 along a single
//! horizontal line of the render, which no equirectangular lookup can produce.
//!
//! A pitch arrived as a roll for the same reason: `camera_rotation` turns about this engine's
//! `X`, which for a camera looking along `+Z` is a turn about its own forward axis.
//!
//! See `.agents/investigate/cycles-world-axes.md` for the measurements and their sources.
//!
//! # The basis change
//!
//! ```text
//! (x, y, z)  ->  (x, -z, y)
//! ```
//!
//! This engine's `+y` goes to Cycles' `+z` and `+z` goes to `-y`, which keeps the basis
//! right-handed: the determinant of `[[1, 0, 0], [0, 0, -1], [0, 1, 0]]` is 1. It is also a
//! quarter turn about `+X`, which is what makes [`rotation_euler`]'s addition a single term
//! rather than a composition.
//!
//! # Invariants
//!
//! Everything written in world coordinates goes through this module, and it is the only
//! place that knows the mapping. A coordinate written by hand is a scene that is turned by
//! however much the two worlds happen to disagree.

/// Returns one position in Cycles' axes.
pub fn position(point: [f64; 3]) -> [f64; 3] {
    [point[0], -point[2], point[1]]
}

/// Returns one scale in Cycles' axes.
///
/// A scale is a diagonal matrix, and the basis change on both sides of it permutes the
/// diagonal. It is here because leaving it out is silent: an instance scaled unevenly comes
/// out with two of its three axes exchanged, which is a shape rather than an error.
pub fn scale(axes: [f64; 3]) -> [f64; 3] {
    [axes[0], axes[2], axes[1]]
}

/// What the rotation in a transform is a rotation **of**.
///
/// The basis change reaches a rotation differently in the two cases, and the difference is
/// the whole of it.
pub enum Rotation {
    /// The rotation moves **points**: a mesh's vertices, an instance's translation. The basis
    /// change is on both sides — the point is being placed in a world that has moved — so the
    /// rotation is conjugated by it.
    Points,
    /// The rotation carries a **local frame both renderers already agree on**: a camera's view
    /// axis, a light's beam. Cycles reads its own camera's local frame the way this engine
    /// reads its own — forward along `+Z`, up along `+Y` — so the local side is unchanged and
    /// only the world side takes the basis change.
    ///
    /// Conjugating one of these is the mistake this distinguishes: the result is a rotation
    /// that is still valid and still unit length, and it aims the camera at a place that is
    /// `90` degrees off from the one the document named.
    Frame,
}

/// The basis change as a quaternion: a quarter turn about `+X`.
fn quarter_turn() -> [f64; 4] {
    let half = std::f64::consts::FRAC_PI_4;
    [half.cos(), half.sin(), 0.0, 0.0]
}

/// Returns a rotation in Cycles' axes, for [`Rotation::Frame`].
///
/// Both renderers read their camera and their lights through the same local frame — forward
/// along `+Z`, up along `+Y` — so the local side of the rotation is unchanged and the basis
/// change is applied on the **left** of it.
///
/// It is applied to the quaternion rather than to the Euler angles because the document's
/// three angles compose into `Rz Ry Rx`, so a turn put on the left does not land next to the
/// turn about `X` it shares an axis with and cannot be folded into one of the three. Measured:
/// adding ninety to the first angle gave a camera aimed at a different part of the sky.
pub fn rotation_frame(quaternion: [f64; 4]) -> [f64; 4] {
    let turn = quarter_turn();
    let (w, x, y, z) = (turn[0], turn[1], turn[2], turn[3]);
    let (qw, qx, qy, qz) = (quaternion[0], quaternion[1], quaternion[2], quaternion[3]);
    [
        w * qw - x * qx,
        w * qx + x * qw + y * qz - z * qy,
        w * qy - x * qz + y * qw + z * qx,
        w * qz + x * qy - y * qx + z * qw,
    ]
}

/// Returns a rotation in Cycles' axes, for [`Rotation::Points`].
///
/// Conjugation `A R A^-1` maps a quaternion's **vector** part by the basis change and leaves
/// its scalar part alone, because a conjugation by `A` is `A R A^T` and the transpose is the
/// inverse. So this is the permutation `(qx, qy, qz) -> (qx, -qz, qy)` and nothing else.
///
/// Written against the composition `R = Rz(c) Ry(b) Rx(a)`: conjugation by `Rx(90)` carries a
/// turn about `X` to a turn about `X`, one about `Y` to a turn about `Z`, and one about `Z` to
/// a turn about `-Y`, so the answer is `Rx(a) Rz(b) Ry(-c)`.
pub fn rotation_points(quaternion: [f64; 4]) -> [f64; 4] {
    [quaternion[0], quaternion[1], -quaternion[3], quaternion[2]]
}
