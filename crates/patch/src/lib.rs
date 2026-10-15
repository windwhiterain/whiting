//! Freeform patches over a mesh: one editable patch per face, one continuity tag per edge.
//!
//! Two representations live here, and they are not variants of one another.
//!
//! - [`PatchMesh`] is the **per-patch Bernstein** model. Each face carries its own rectangular
//!   control net and a shared edge is repaired by an enforcement pass that mirrors one side's
//!   adjacent row into the other. It is what [`gallery::nurbs`](../../../crates/gallery/src/nurbs.rs)
//!   draws today.
//! - [`HermiteSurface`] is the **Hermite** model. The corner jets are shared rather than
//!   reconciled, so continuity is structural and there is no pass to run. It is what
//!   `.agents/handoffs/hermite-surface.md` measures.
//!
//! They are kept side by side while [`HermiteSurface`] is being measured against real input; the
//! Hermite model replaces [`PatchMesh`] rather than sitting beside it, and the removal of
//! [`enforce`] is part of that.

pub mod bernstein;
pub mod continuity;
pub mod edit;
pub mod enforce;
pub mod hermite;
pub mod hermite_surface;
pub mod level;
pub mod patch_mesh;
pub mod point;
pub mod rect;
pub mod simplex;
pub mod subdivide;
pub mod tessellate;
pub mod topology;

pub use continuity::{Continuity, SoftEdge, slave_interior};
pub use edit::{Edit as SurfaceEdit, Editor};
pub use enforce::Rules;
pub use hermite::{HermitePatch, Jet};
pub use hermite_surface::{HermiteSurface, Join, SurfaceError};
pub use level::{Face, Level};
pub use patch_mesh::{Edit, PatchMesh};
pub use point::Point3;
pub use rect::{RectNet, RectNetError};
pub use simplex::{SimplexNet, SimplexNetError, count};
pub use subdivide::{Origin, QuadMesh, subdivide};
pub use tessellate::{FaceGrid, Tessellation};
pub use topology::{EdgeKey, EdgeSlot, FaceId, Topology, VertexId};
