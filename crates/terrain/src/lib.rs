//! Turn a mountain massif's facet mesh into a height field, and erode it.
//!
//! # What this is for
//!
//! [`mountain`] expands an authored ridge and valley skeleton into a mesh of planar block
//! faces, and that is the right surface for a rock face and the wrong one for a *landscape*:
//! a real mountain flank is not a fan of flat facets meeting at sharp edges, it is a surface
//! that gullies, that carries the water off its crest, and that leaves the talus where the
//! talus falls. Those are statements about a height field, so this crate turns the mesh into
//! one — exactly, because the mesh is exactly planar — and then runs an erosion solve over
//! it.
//!
//! It is a **separate crate** rather than a module of `mountain` because
//! [SPEC/mountain.md](../../../.agents/SPEC/mountain.md) R6 puts erosion in a downstream
//! pipeline and out of that crate's scope, and a module added there would make the spec
//! false rather than extend it.
//!
//! # The two steps
//!
//! - [`rasterize`] reads a [`mountain::FacetMesh`] onto a rectangular lattice. There is no
//!   error term: a cell inside a face reads that face's own plane.
//! - [`erode`] runs the solve, and [`erode_traced`] hands back the books as well so a caller
//!   can check that the mass line closed rather than assume it did.
//!
//! # What it is not
//!
//! There is no geometry output. A [`HeightField`] is numbers and a lattice, and what is drawn
//! from them — a triangle mesh, a scene's point buffer, a picture — belongs to whoever draws.
//! That is why the solver has no opinion about winding, normals or shading, and why a caller
//! reading [`HeightField::sample`] is reading the same surface the solver moved.
//!
//! ```
//! use mountain::derive::Starter;
//! use mountain::Massif;
//! use whiting_terrain::{ErodeParams, Hydraulic, erode_traced, rasterize};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let massif = Massif::starter(7, Starter::default());
//! let mesh = massif.build()?;
//!
//! // Four unit cells across the massif's own plan bounds.
//! let read = rasterize(&mesh, 4.0)?;
//! let (eroded, books) = erode_traced(&ErodeParams::default(), &read.field, &Hydraulic)?;
//!
//! // The mass line closed: what left the massif in load is what is gone from it in rock.
//! assert!((books.eroded - books.deposited - books.suspended - books.spilled).abs() < 1.0e-3);
//! // And it moved rock — deposition is half of an erosion, so neither end of the relief is
//! // a direction it travels in.
//! let moved = read
//!     .field
//!     .heights()
//!     .iter()
//!     .zip(eroded.heights())
//!     .filter(|(from, to)| from != to)
//!     .count();
//! assert!(moved > 0);
//! # Ok(())
//! # }
//! ```

pub mod erode;
pub mod heightfield;
pub mod rasterize;

pub use erode::{
    Books, D8, ErodeParams, Hydraulic, INV_STEP, Transport, by_height_descending, delta_of, erode,
    erode_traced,
};
pub use heightfield::{FieldError, HeightField};
pub use rasterize::{NO_FACE, Provenance, Rasterize, Rasterized, rasterize};
