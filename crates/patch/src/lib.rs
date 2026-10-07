//! Freeform patches over a mesh: one editable patch per face, one continuity tag per edge.
//!
//! The design this implements is
//! [`.agents/SPEC.align/hierarchical-nurbs-multipatch.md`](../../../.agents/SPEC.align/hierarchical-nurbs-multipatch.md).
//! In brief: a mesh face carries a tensor-product patch, a shared edge is either a hard crease
//! (the two sides move independently) or soft (the tangent plane is continuous across it), and
//! refinement is a layered linear sum in which an empty layer contributes nothing.
//!
//! Nothing here fits or approximates a mesh. A freshly built model reproduces its mesh exactly.

pub mod bernstein;
pub mod continuity;
pub mod patch_mesh;
pub mod rect;
pub mod simplex;
pub mod subdivide;
pub mod topology;

pub use continuity::{Continuity, SoftEdge, slave_interior};
pub use patch_mesh::{Edit, PatchMesh};
pub use rect::{RectNet, RectNetError};
pub use simplex::{Point3, SimplexNet, SimplexNetError, count};
pub use subdivide::{Origin, QuadMesh, subdivide};
pub use topology::{EdgeKey, EdgeSlot, FaceId, Topology, VertexId};
