//! Author a building as skeletons that carry their correspondence, and expand them into closed
//! planar-facet solids.
//!
//! # What this is for
//!
//! A building is planned all the way down: masses have hierarchy, facades have rhythm, roofs have
//! structure, and every face belongs to a named element of that plan. The gallery's own
//! architectural forms (`gallery::built`) are assembled face by face instead — a wall run is five
//! faces, its outer normal is a formula the caller must get right, and the file has to warn that a
//! ring walked the wrong way is *an invisible building rather than a visible fault*. This crate is
//! the planning layer above those forms. See `.agents/proposals/agentic-building-generation.md`.
//!
//! # The one idea
//!
//! **The skeletons carry their correspondence.** A roof plane's rows name their crest point and
//! their eave station, a mass's attachment names the host it lands on, and expansion is a direct
//! walk of what the author wrote down: no plane intersection search, no boolean, and no two
//! constructions that have to agree by luck. Watertightness is structural — two faces meet along an
//! edge because they were built from the same authored row — and what is left to the author is the
//! correspondence itself, which [`Building::validate`] measures.
//!
//! # What it is not
//!
//! No editor, no runtime service, no serialization, no renderer. An agent authors by writing rust
//! against this api — [`ops`] for the edits, [`derive`] for a starting point, [`validate`] for what
//! is wrong, and [`Building::build`] for the mesh.
//!
//! ```no_run
//! use whiting_building::Building;
//! use whiting_building::derive::{self, RoofStyle, Starter};
//!
//! let building = derive::starter(&Starter {
//!     width: 6.0,
//!     depth: 9.0,
//!     height: 3.2,
//!     roof: RoofStyle::Hip,
//!     ..Starter::default()
//! });
//! for violation in building.validate() {
//!     println!("{violation:?}");
//! }
//! let mesh = building.build().expect("the starter builds");
//! println!("{} faces, {} points", mesh.faces.len(), mesh.points.len());
//! ```
//!
//! # Conventions
//!
//! A plan point is `(x, z)` and a space point is `(x, y, z)` with `y` up ([`math`]). Unlike
//! `mountain`, whose mesh is a height field over the plan and whose faces are therefore wound to
//! look up, a building has vertical walls: **a face here is wound so that its right-handed normal
//! points out of the solid** (`cross(b - a, c - a)`, in that order). An outward-facing mesh has a
//! **positive** signed volume, and [`Building::validate`] reports the opposite sign as an
//! inside-out solid rather than leaving it to be discovered in a frame.

pub mod clip;
pub mod derive;
pub mod expand;
pub mod fill;
pub mod grid;
pub mod identity;
pub mod mass;
pub mod math;
pub mod mesh;
pub mod openings;
pub mod ops;
pub mod parts;
pub mod roof;
pub mod site;
pub mod skeleton;
pub mod slab;
pub mod validate;
pub mod wall;

pub use expand::{BuildError, Problem, build};
pub use grid::{Bay, FaceGrid, FaceRef, GRID_TOLERANCE, Grids, Rhythm};
pub use mass::{Attachment, Mass, MassId, PlanNode};
pub use mesh::{
    BuildingFace, BuildingMesh, CornerNormals, Element, MassSurface, RoofSurface, SolidId,
};
pub use openings::{Extents, Inset, Opening, Openings, RevealSide};
pub use parts::{Bounds, Parts, Pin, Placement, Template};
pub use roof::{CrestPoint, EaveStation, Roof, RoofId, RoofPlane, RoofRow};
pub use skeleton::{Building, BuildingSkeleton};
pub use slab::{Slab, SlabId, SlabSurface};
pub use validate::Violation;
