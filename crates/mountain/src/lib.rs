//! Author a mountain massif as a ridge and valley polyline skeleton, and expand it into a
//! planar-facet triangle mesh.
//!
//! # What this is for
//!
//! The ridge distribution of a real massif is *planned*: it has hierarchy, direction and
//! rhythm, and no noise function has those. Noise cannot express a rock mass's block faces
//! either — a real flank reads as a set of planar facets meeting at sharp edges — and a
//! triangle is exactly one planar block face. So the ridge lines are authored as polylines, and
//! the polylines are expanded into a triangle mesh. See
//! [SPEC/mountain.md](../../../.agents/SPEC/mountain.md).
//!
//! # The one idea
//!
//! **The lines carry their correspondence.** A valley node names the point it faces on each of
//! its two sides ([`ValleyNode::left`], [`ValleyNode::right`]), so the surface between two
//! consecutive valley nodes is one belt bounded by known lines, and the mesh between them is a
//! strip that walks both. There is no triangulation step, no constraint to insert, and nothing
//! to search: the correspondence *is* the mesh's topology, written down by the author.
//!
//! Everything that is usually hard about expanding polylines into a mesh follows from that.
//! A spur junction is a corner in a belt's upper boundary, and its first row collapses to a
//! triangle. A summit and a ridge tip close the same way. Watertightness is structural rather
//! than the outcome of a search. What is left for the author is the thing that was always the
//! author's: getting the correspondence right, which [`validate`] measures.
//!
//! # What it is not
//!
//! There is no editor, no runtime service, no serialization and no renderer here. An agent
//! authors by writing rust against this api — [`ops`] for the edits, [`derive`] for a starting
//! point, [`validate`] for what is wrong, and [`Massif::build`] for the mesh.
//!
//! ```no_run
//! use whiting_mountain::Massif;
//! use whiting_mountain::derive::Starter;
//!
//! let massif = Massif::starter(7, Starter::default());
//! for violation in massif.validate() {
//!     println!("{violation:?}");
//! }
//! let mesh = massif.build().expect("the starter builds");
//! println!("{} faces, {} points", mesh.faces.len(), mesh.points.len());
//! ```

pub mod derive;
pub mod expand;
pub mod facet;
pub mod line;
pub mod math;
pub mod ops;
pub mod sampling;
pub mod skeleton;
pub mod tessellation;
pub mod validate;
pub mod zip;

pub use derive::{Starter, derive_valleys, starter};
pub use expand::{Belt, BuildError, Problem, build, enumerate};
pub use facet::{Face, FaceAttributes, FacetMesh};
pub use line::{LineRef, Node, Side, Station};
pub use sampling::{Sample, SampledLine, Sampling, Vertex};
pub use skeleton::{
    Attach, FootprintNode, Ridge, RidgeId, RidgeNode, Skeleton, Valley, ValleyId, ValleyNode,
};
pub use tessellation::{BeltRef, DiagonalOverride, DiagonalRule, Tessellation};
pub use validate::{Violation, validate};

/// A mountain massif: the control structure, and how finely it is expanded.
///
/// # Invariants
///
/// The document is the skeleton. The tessellation is not part of it in any sense that survives
/// an edit — changing the step changes the mesh and not the mountain — and it is kept beside
/// the skeleton only so that one value can be handed to `build`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Massif {
    /// The authored lines.
    pub skeleton: Skeleton,
    /// How finely they are expanded.
    pub tessellation: Tessellation,
}

impl Massif {
    /// Builds a massif from a skeleton, at the default sampling.
    pub fn new(skeleton: Skeleton) -> Self {
        Self {
            skeleton,
            tessellation: Tessellation::default(),
        }
    }

    /// Sets the sampling.
    pub fn with_tessellation(mut self, tessellation: Tessellation) -> Self {
        self.tessellation = tessellation;
        self
    }

    /// Expands the skeleton into a facet mesh.
    pub fn build(&self) -> Result<FacetMesh, BuildError> {
        build(&self.skeleton, self.tessellation.clone())
    }

    /// Reports what an edit has got wrong.
    pub fn validate(&self) -> Vec<Violation> {
        validate(&self.skeleton, self.tessellation.clone())
    }

    /// Returns the plan area the footprint encloses.
    ///
    /// # Invariants
    ///
    /// Its sign says which way the loop is wound, and that sign is the contract: the expansion
    /// winds every face so it looks up, so a footprint wound the other way encloses the same
    /// land and the mesh that covers it has the **opposite** area. Comparing the two is
    /// therefore a check of coverage and of orientation in one number, which is what
    /// [`FacetMesh::plan_area`] is for.
    pub fn footprint_area(&self) -> f32 {
        let outline = &self.skeleton.footprint;
        if outline.len() < 3 {
            return 0.0;
        }
        let mut total = 0.0;
        let mut previous = outline[outline.len() - 1].position;
        for node in outline {
            total += crate::math::cross(previous, node.position);
            previous = node.position;
        }
        total * 0.5
    }
}
