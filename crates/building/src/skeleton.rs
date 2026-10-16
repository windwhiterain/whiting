//! The authored document, and what an agent asks of it.
//!
//! This is the value an agent edits. Everything downstream — expansion, validation — reads it and
//! never writes to it, so an edit is always a change to a value the author can see whole.

use crate::expand::{BuildError, build};
use crate::mass::{Mass, MassId};
use crate::mesh::BuildingMesh;
use crate::roof::{Roof, RoofId};
use crate::validate::{Violation, document, mesh};

/// The control structure of a building: its masses and its roofs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildingSkeleton {
    /// The masses, body first by convention but not by rule.
    pub masses: Vec<Mass>,
    /// The roofs, each naming the mass it sits on.
    pub roofs: Vec<Roof>,
}

impl BuildingSkeleton {
    /// Builds a skeleton from its two parts.
    pub fn new(masses: Vec<Mass>, roofs: Vec<Roof>) -> Self {
        Self { masses, roofs }
    }

    /// Returns the id of every mass, in list order.
    pub fn mass_ids(&self) -> Vec<MassId> {
        (0..self.masses.len()).map(MassId).collect()
    }

    /// Returns the id of every roof, in list order.
    pub fn roof_ids(&self) -> Vec<RoofId> {
        (0..self.roofs.len()).map(RoofId).collect()
    }

    /// Returns one mass.
    pub fn mass(&self, id: MassId) -> Option<&Mass> {
        self.masses.get(id.0)
    }

    /// Returns one roof.
    pub fn roof(&self, id: RoofId) -> Option<&Roof> {
        self.roofs.get(id.0)
    }

    /// Returns the roofs that name a mass as their host.
    pub fn roofs_of(&self, mass: MassId) -> Vec<RoofId> {
        self.roofs
            .iter()
            .enumerate()
            .filter_map(|(index, roof)| (roof.host == mass).then_some(RoofId(index)))
            .collect()
    }

    /// Returns the masses that name a mass as their host.
    pub fn wings_of(&self, host: MassId) -> Vec<MassId> {
        self.masses
            .iter()
            .enumerate()
            .filter_map(|(index, mass)| {
                (mass.attachment.map(|attachment| attachment.host) == Some(host))
                    .then_some(MassId(index))
            })
            .collect()
    }
}

/// A building: the control structure, and how finely it is expanded.
///
/// # Invariants
///
/// The document is the skeleton. There is no rule tree above it and no derivation state beside it:
/// every rhythm, every attachment and every roof plane is a value in the skeleton, which is what
/// makes an edit a change to something the author can see whole
/// (`.agents/proposals/agentic-building-generation.md` D10).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Building {
    /// The authored masses and roofs.
    pub skeleton: BuildingSkeleton,
}

impl Building {
    /// Builds a building from a skeleton.
    pub fn new(skeleton: BuildingSkeleton) -> Self {
        Self { skeleton }
    }

    /// Expands the skeleton into closed planar-facet solids.
    pub fn build(&self) -> Result<BuildingMesh, BuildError> {
        build(&self.skeleton)
    }

    /// Reports everything wrong with this building, document and mesh alike.
    ///
    /// # Invariants
    ///
    /// Document violations are reported whether or not the skeleton expands, because they are
    /// facts about what the author wrote; mesh violations are reported only for a skeleton that
    /// expands, because a mesh that does not exist has no closure to measure. **A skeleton that
    /// does not expand is itself reported** — [`Violation::DidNotExpand`], carrying the expander's
    /// own problems — so the "cheap inner loop" an agent calls while editing cannot read a skeleton
    /// that can never expand as a clean one. All of them are returned in a deterministic order, so
    /// two runs diff — the contract `mountain`'s `validate` established, kept here for the same
    /// reason: a check that stops at the first fault turns a fix into a sequence of runs.
    pub fn validate(&self) -> Vec<Violation> {
        let mut violations = document(&self.skeleton);
        match self.build() {
            Ok(built) => violations.extend(mesh(&self.skeleton, &built)),
            Err(error) => violations.push(Violation::DidNotExpand {
                problems: error.problems,
            }),
        }
        violations
    }
}
