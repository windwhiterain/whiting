//! What a plant touches: the places its wood stands inside something, reported and never repaired.
//!
//! # The one idea
//!
//! **A contact is a question asked of a function, not a solver.** The occupancy is a signed distance in the world
//! — negative inside, positive outside — and a contact is a sampled point of a spine that reads negative there. So
//! the crate never learns what a wall is: a caller with a `terrain::HeightField` passes its sampler, one with a
//! building passes its mesh's signed distance, and one with a neighbour passes the distance to that plant's own
//! solids. That is [`crate::field::Field::Obstacle`]'s occupancy read as a report rather than as a force, which is
//! why the two share a vocabulary instead of each inventing one.
//!
//! # Why the spine and not the skin
//!
//! A contact is measured on the spine, so a branch whose *skin* brushes a wall without its centreline entering is
//! not reported. That is the same approximation a sweep's tessellation already makes: a spine is the line the
//! taper is drawn around, and a report that measured the skin would need the sweep's resolution and would then
//! disagree with the geometry's own authoring. The doc says so rather than implying a tolerance nobody measured.
//!
//! # Report, never repair
//!
//! Nothing here moves a plant. The field is what moves it; this is what tells an author how much is left.

use shape::Vec3;

use crate::expand::Detail;
use crate::handle::Handle;
use crate::plant::Plant;

/// One place a plant's wood stands inside something.
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    /// The node whose spine is inside.
    pub handle: Handle,
    /// Where along that node's spine, as an arc-length fraction.
    pub station: f32,
    /// The world point, on the spine rather than on the skin.
    pub point: Vec3,
    /// How far inside, as the occupancy's own magnitude: always positive, because a point outside is not a contact.
    pub depth: f32,
}

/// Every place a plant's spines stand inside an obstacle, deepest first.
///
/// # Invariants
///
/// - **The sample is every live node's drawn spine at [`Detail::stations`]**, so a contact is a place the wood
///   actually is: a nudged, baked or field-bent spine is what is measured, and the count is the same one the
///   expansion draws at.
/// - **A contact is reported where the occupancy is negative**, and its depth is the magnitude, so the list is
///   ordered by how badly each point is buried and the first entry is the worst.
/// - **Nothing is repaired.** A plant is taken by reference and returned to unchanged; shaping it out of an
///   obstacle is [`crate::field::Field::Obstacle`]'s job and [`crate::envelope::trim`]'s for a crown.
pub fn collide(plant: &Plant, occupancy: impl Fn(Vec3) -> f32, detail: Detail) -> Vec<Contact> {
    let mut contacts = Vec::new();
    for (handle, _) in plant.live() {
        let Ok(spine) = plant.spine(&handle) else {
            continue;
        };
        for station in spine.stations(detail.stations.max(2)) {
            let depth = -occupancy(station.position);
            if depth > 0.0 {
                contacts.push(Contact {
                    handle: handle.clone(),
                    station: station.t,
                    point: station.position,
                    depth,
                });
            }
        }
    }
    contacts.sort_by(|one, other| other.depth.total_cmp(&one.depth));
    contacts
}

/// How far the deepest contact is, or `0.0` when the plant touches nothing.
pub fn deepest(contacts: &[Contact]) -> f32 {
    contacts.first().map(|contact| contact.depth).unwrap_or(0.0)
}
