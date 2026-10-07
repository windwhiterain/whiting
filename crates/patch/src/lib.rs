//! Freeform patches over a mesh: one editable patch per face, one continuity tag per edge.
//!
//! The design this implements is
//! [`.agents/SPEC.align/hierarchical-nurbs-multipatch.md`](../../../.agents/SPEC.align/hierarchical-nurbs-multipatch.md).
//! In brief: a mesh face carries a tensor-product patch, a shared edge is either a hard crease
//! (the two sides move independently) or soft (the tangent plane is continuous across it), and
//! refinement is a layered linear sum in which an empty layer contributes nothing.
//!
//! Nothing here fits or approximates a mesh. A freshly built model reproduces its mesh exactly.

pub mod simplex;

pub use simplex::{Point3, SimplexNet, SimplexNetError, count};
