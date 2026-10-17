//! The authored document, and what an agent asks of it.
//!
//! This is the value an agent edits. Everything downstream — expansion, validation — reads it and
//! never writes to it, so an edit is always a change to a value the author can see whole.

use crate::expand::{BuildError, build};
use crate::grid::{Bay, FaceGrid, FaceRef, Grids, face_extent, face_length};
use crate::mass::{Mass, MassId};
use crate::mesh::BuildingMesh;
use crate::openings::{Extents, Openings};
use crate::parts::{Anchor, Bounds, Parts, Pin, Placement, Template};
use crate::roof::{Roof, RoofId};
use crate::slab::{Slab, SlabId};
use crate::validate::{Violation, document, mesh};
use crate::wall::{Wall, WallId};

/// Everything a placement resolves to: what stands, where, and where it lands.
///
/// # Invariants
///
/// It is produced in one place ([`BuildingSkeleton::placed`]) and then read by the expander, the
/// validator and any probe, so "where the part is" is one answer rather than three readings of the same
/// numbers. The box is in the bay's own frame and the pin is in the same frame, which is why a mounted
/// part's `at` is the *resolved* origin and not the pin the author would have written.
#[derive(Debug, Clone, Copy)]
pub struct Placed<'a> {
    /// The template that stands.
    pub template: &'a Template,
    /// The bay it stands in.
    pub bay: Bay,
    /// The origin its own frame is placed at, in the bay's frame.
    pub at: Pin,
    /// The box it then occupies, in the bay's frame.
    pub reach: Bounds,
}

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
    /// The openings: holes cut through the walls, each naming a bay by slot.
    pub openings: Openings,
    /// The parts: the templates a document authors and the places they stand.
    pub parts: Parts,
    /// The walls: the site's enclosures, one straight run each.
    pub walls: Vec<Wall>,
    /// The slabs: level prisms over plan polygons, which is what a flat roof is.
    pub slabs: Vec<Slab>,
}

impl BuildingSkeleton {
    /// Builds a skeleton from its masses and its roofs, with no rhythm authored, no wall cut, and
    /// nothing placed.
    pub fn new(masses: Vec<Mass>, roofs: Vec<Roof>) -> Self {
        Self {
            masses,
            roofs,
            grids: Grids::new(),
            openings: Openings::new(),
            parts: Parts::new(),
            walls: Vec::new(),
            slabs: Vec::new(),
        }
    }

    /// Returns everything a placement resolves to: its template, its bay, the pin it lands on, and the
    /// box it then occupies in that bay's own frame.
    ///
    /// # Invariants
    ///
    /// **One place resolves a placement**, so the expander that stands the part and the validator that
    /// measures it cannot disagree — the same rule [`aperture`](Self::aperture) follows for a hole. A
    /// template index the document does not hold, a bay slot the grid does not hold, a socket the
    /// template does not name, an anchor whose opening is not in that wall, or a template with no mass
    /// all answer `None`, which is exactly what [`unresolved_placements`](Self::unresolved_placements)
    /// names. `O(log faces)` for the bay and `O(1)` for the rest.
    pub fn placed(&self, index: usize) -> Option<Placed<'_>> {
        let placement = self.parts.placement(index)?;
        let template = self.parts.template(placement.template)?;
        let bay = self.bay(placement.face, placement.row, placement.column)?;
        let at = self.pin_of(placement)?;
        let reach = placement.reach(template, at)?;
        Some(Placed {
            template,
            bay,
            at,
            reach,
        })
    }

    /// Returns the pin a placement lands on, whether it was written or hangs from an anchor.
    ///
    /// # Invariants
    ///
    /// **A joint is one subtraction, not a transform**: every part is authored in the placement's frame,
    /// so mating a socket to an anchor only moves the template's origin, and the origin that puts the
    /// socket on the anchor is `anchor − socket`. No rotation is involved and none can be got wrong —
    /// see [`Socket`](crate::parts::Socket) for why that is the whole of the design.
    ///
    /// A free placement returns its own [`Pin`]; a mounted one whose socket or anchor does not resolve
    /// answers `None`.
    pub fn pin_of(&self, placement: &Placement) -> Option<Pin> {
        let Some(joint) = &placement.joint else {
            return Some(placement.at);
        };
        let template = self.parts.template(placement.template)?;
        let socket = template.socket(&joint.socket)?;
        let bay = self.bay(placement.face, placement.row, placement.column)?;
        let (along, up) = self.anchor(placement.face, bay, joint.anchor)?;
        Some(Pin::new(along - socket.at.0, up - socket.at.1))
    }

    /// Returns the point an anchor names, in its bay's own frame: along the edge from the bay's start,
    /// and up from its sill.
    ///
    /// # Invariants
    ///
    /// **The point is read out of the document**, so a joint follows an edit: an opening's head is "the
    /// middle of the hole across, the hole's own head" and nothing else, which is why moving the door's
    /// head moves every part hung on it. A bay's own anchors need no lookup at all. `O(1)` for a bay's
    /// and `O(log faces)` for an opening's, which is what [`aperture`](Self::aperture) costs.
    pub fn anchor(&self, face: FaceRef, bay: Bay, anchor: Anchor) -> Option<(f32, f32)> {
        let point = match anchor {
            Anchor::BaySill => (bay.width() * 0.5, 0.0),
            Anchor::BayHead => (bay.width() * 0.5, bay.height()),
            Anchor::BayStart => (0.0, bay.height() * 0.5),
            Anchor::BayEnd => (bay.width(), bay.height() * 0.5),
            Anchor::OpeningHead { opening } | Anchor::OpeningSill { opening } => {
                let held = self.openings.get(opening)?;
                if held.face != face {
                    return None;
                }
                let (_, hole) = self.aperture(opening)?;
                let middle = (hole.from + hole.to) * 0.5;
                let elevation = match anchor {
                    Anchor::OpeningHead { .. } => hole.high,
                    _ => hole.low,
                };
                (middle - bay.from, elevation - bay.low)
            }
        };
        Some(point)
    }

    /// Returns the index of every placement that **cannot be resolved** in one wall.
    ///
    /// # Invariants
    ///
    /// The companion of [`placed`](Self::placed), and it exists because a lookup that answers `None`
    /// is a lookup that **filters**: an expander reading only the resolvable placements would stand a
    /// wall's other parts and say nothing about the one whose slot the grid no longer holds — a mesh
    /// quietly disagreeing with its document. Both go through `placed`, so there is one definition of
    /// "resolved" rather than one per caller. `O(placements of that face)`.
    pub fn unresolved_placements(&self, face: FaceRef) -> Vec<usize> {
        self.parts
            .of_face(face)
            .into_iter()
            .filter_map(|index| self.placed(index).is_none().then_some(index))
            .collect()
    }

    /// Returns the rectangle an opening cuts, and the wall it is cut into.
    ///
    /// # Invariants
    ///
    /// **One place computes a hole's extents, so the expander that cuts it and the validator that
    /// measures it cannot disagree.** The bay comes from the document's own grid — a slot the grid no
    /// longer holds answers `None`, which is how a re-split that orphaned a placement becomes visible
    /// rather than silently relocating it — and the insets come from the opening. `O(log faces)` for
    /// the grid and `O(1)` for the arithmetic.
    pub fn aperture(&self, index: usize) -> Option<(FaceRef, Extents)> {
        let opening = self.openings.get(index)?;
        let bay = self.bay(opening.face, opening.row, opening.column)?;
        let extents = opening.extents(bay)?;
        Some((opening.face, extents))
    }

    /// Returns the index of every opening in one wall that **cannot be resolved**.
    ///
    /// # Invariants
    ///
    /// This is the companion of [`apertures_of`](Self::apertures_of), and it exists because that
    /// lookup **filters**: a hole whose slot the grid no longer holds, or whose insets leave it no area,
    /// is simply absent from what it returns — which is right for a walk that asks "what can I cut"
    /// and wrong for one that must not cut silently. An expander reading only `apertures_of` would
    /// build a wall with a hole missing and no complaint, so the honest pair is these two: what can be
    /// cut, and what could not be. Both go through [`aperture`](Self::aperture), so there is one
    /// definition of "resolved" rather than one per caller. `O(openings of that face)`.
    pub fn unresolved_openings(&self, face: FaceRef) -> Vec<usize> {
        self.openings
            .of_face(face)
            .into_iter()
            .filter_map(|(index, _)| self.aperture(index).is_none().then_some(index))
            .collect()
    }

    /// Returns the index and the rectangle of every hole in one wall, in the order they were authored.
    ///
    /// A hole whose slot the grid no longer holds is **absent from this answer** rather than reported
    /// here: this is the walk the expander cuts from, and a slot that vanished is the validator's to
    /// report (it can name the opening), not a hole to cut somewhere else.
    pub fn apertures_of(&self, face: FaceRef) -> Vec<(usize, Extents)> {
        self.openings
            .of_face(face)
            .into_iter()
            .filter_map(|(index, _)| {
                self.aperture(index)
                    .and_then(|(at, extents)| (at == face).then_some((index, extents)))
            })
            .collect()
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

    /// Returns one wall by id.
    pub fn wall(&self, id: WallId) -> Option<&Wall> {
        self.walls.get(id.0)
    }

    /// Returns every wall's id, in the document's own order.
    pub fn wall_ids(&self) -> Vec<WallId> {
        (0..self.walls.len()).map(WallId).collect()
    }

    /// Returns one slab by id.
    pub fn slab(&self, id: SlabId) -> Option<&Slab> {
        self.slabs.get(id.0)
    }

    /// Returns every slab's id, in the document's own order.
    pub fn slab_ids(&self) -> Vec<SlabId> {
        (0..self.slabs.len()).map(SlabId).collect()
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
