//! The curve, the section, the sweep and the mesh predicates every geometry crate shares.
//!
//! # What this is for
//!
//! Four crates need one definition of which axis is up, and two of them have already paid for a
//! copy each: `building` takes `mountain` for `math` alone and says so in its own manifest, and
//! `terrain` calls `mountain::math` directly from two files. The rules that decide what belongs
//! here are in [`.agents/proposals/shape-primitives.md`](../../../.agents/proposals/shape-primitives.md).
//!
//! - [`math`] — plan and space arithmetic. The one definition of the frame: `x` east, `y` up, `z`
//!   south.
//! - [`curve`] — a cubic Hermite polyline. Every spine, every leaf midrib and every envelope in this
//!   workspace is one, and every exporter needs it as cubic Beziers, which [`curve::Curve::segments`]
//!   returns.
//! - [`scalar`] — one number along a spine: a taper, a crown's reach, a twist. It is the same
//!   polynomial as [`curve`] with the space taken out, and its `t` is the **spine's** arc-length
//!   fraction rather than its own — which is why it is a separate type and not a flag on the curve.
//! - [`section`] — the closed loop a sweep draws in the plane of the spine's frame: an ellipse at any
//!   aspect ratio, or an authored outline. A circle and a blade are one shape at different aspects, so
//!   two arms carry the four names revision 1 of the proposal listed.
//! - [`sweep`] — a spine, a section, and the two laws that place the section at every station. A trunk,
//!   a blade and an envelope are one call here, and refining an envelope is a change of section rather
//!   than a new type.
//! - [`loft`] — what a sweep returns: positions, triangles, and the one definition of `(u, v)`.
//! - [`mesh`] — the predicates over a bare triangle list: boundary loops, manifoldness, Euler's
//!   characteristic, and the signed volume that turns an inside-out solid into a named reading.
//!
//! # What it is not
//!
//! There is no mesh type here, and that is deliberate rather than an omission: `mountain`'s faces
//! are keyed by a **belt** and `building`'s by an **element**, and reusing either would mean writing
//! one document's provenance onto the other's surfaces. What is shared is the arithmetic, the walk and
//! the predicates over a triangle list — a shape's, not a document's.
//!
//! There is no scene, no document and no renderer. Nothing here knows what a frame is.

pub mod curve;
mod hermite;
pub mod loft;
pub mod math;
pub mod mesh;
pub mod scalar;
pub mod section;
pub mod sweep;

pub use curve::{
    Bezier, CLOSEST_SAMPLES, CLOSEST_STEPS, Closest, Curve, CurveError, Frame, Jet, Station,
};
pub use loft::Loft;
pub use math::{Vec2, Vec3};
pub use scalar::{Scalar, ScalarError};
pub use section::{Outline, Section, SectionError};
pub use sweep::{Part, Sweep};
