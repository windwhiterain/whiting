//! The curve, the sweep and the mesh predicates every geometry crate shares.
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
//!
//! # What it is not
//!
//! There is no mesh type here, and that is deliberate rather than an omission: `mountain`'s faces
//! are keyed by a **belt** and `building`'s by an **element**, and reusing either would mean writing
//! one document's provenance onto the other's surfaces. What is shared is the arithmetic and — when
//! it lands — the predicates over a triangle list.
//!
//! There is no scene, no document and no renderer. Nothing here knows what a frame is.

pub mod curve;
pub mod math;

pub use curve::{Bezier, Curve, CurveError, Frame, Jet, Station};
pub use math::{Vec2, Vec3};
