//! Plan and space arithmetic, taken from `mountain` rather than restated.
//!
//! # Invariants
//!
//! A plan point is `(x, z)` and a space point is `(x, y, z)`, so the plan is the space with `y`
//! dropped: `y` is up, and [`lift`] is the one place the two are joined. This module is a
//! re-export and not a copy — two crates that author geometry into one scene have to agree about
//! which axis is up, and the way to agree is to have one definition of it.

pub use mountain::math::{
    Vec2, Vec3, clamp, cross, cross3, difference, distance, dot, dot3, face_normal, length,
    length3, lerp, lift, normal, normalized, normalized3, offset, plan, signed_area, span,
    subtract,
};
