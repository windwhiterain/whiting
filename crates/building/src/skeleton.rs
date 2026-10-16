//! The authored document, and what an agent asks of it.
//!
//! This is the value an agent edits. Everything downstream — expansion, validation — reads it and
//! never writes to it, so an edit is always a change to a value the author can see whole.

use crate::expand::{BuildError, build};
use crate::grid::{Bay, FaceGrid, FaceRef, Grids, face_extent, face_length};
use crate::mass::{Mass, MassId};
use crate::mesh::BuildingMesh;
use crate::roof::{Roof, RoofId};
use crate::validate::{Violation, document, mesh};

/// The control structure of a building: its masses, its roofs, and the rhythms on its faces.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildingSkeleton {
    /// The masses, body first by convention but not by rule.
    pub masses: Vec<Mass>,
    /// The roofs, each naming the mass it sits on.
    pub roofs: Vec<Roof>,
    /// The grids, one per face that has been given a rhythm. A face with no entry has no rhythm,
    /// which is a different state from a rhythm with no bays — see [`FaceGrid::is_empty`].
    pub grids: Grids,
}

impl BuildingSkeleton {
    /// Builds a skeleton from its masses and its roofs, with no face yet given a rhythm.
    pub fn new(masses: Vec<Mass>, roofs: Vec<Roof>) -> Self {
        Self {
            masses,
            roofs,
            grids: Grids::new(),
        }
    }

    /// Returns one face's rhythm, or `None` when that face has none.
    pub fn grid(&self, face: FaceRef) -> Option<&FaceGrid> {
        self.grids.get(face)
    }

    /// Returns one bay by its slot, or `None` when either the face has no rhythm or the slot is off
    /// it. This is the lookup a placement uses, so it answers for the whole path and not just for the
    /// grid: a caller never has to hold a face's rhythm to ask about a cell of it.
    pub fn bay(&self, face: FaceRef, row: usize, column: usize) -> Option<Bay> {
        self.grids.get(face)?.bay(face, row, column)
    }

    /// Returns the plan length of a face's edge, or `None` when there is no such mass or edge.
    pub fn face_length(&self, face: FaceRef) -> Option<f32> {
        face_length(self.mass(face.mass)?, face.edge)
    }

    /// Returns the vertical extent of a face: the lowest ground under it, and the wall top.
    pub fn face_extent(&self, face: FaceRef) -> Option<(f32, f32)> {
        face_extent(self.mass(face.mass)?, face.edge)
    }

    /// Returns every face of a mass, in edge order — the faces a rhythm can be authored on.
    pub fn faces_of(&self, mass: MassId) -> Vec<FaceRef> {
        match self.mass(mass) {
            Some(held) => (0..held.outline.len())
                .map(|edge| FaceRef::new(mass, edge))
                .collect(),
            None => Vec::new(),
        }
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
