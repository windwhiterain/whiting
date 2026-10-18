//! What a plant arrived at: the counts an expansion makes, and the crown measured against its envelope.
//!
//! # The one idea
//!
//! **A report is a measurement of the plant as it stands, not a promise about what it should have been.** It reads
//! the expansion the scene would draw and the crown the plant actually holds, band by band along the trunk, and
//! puts both beside the envelope's own radius in the same parameter space — which is the one comparison V4 and the
//! design's D13 ask for, and the one a number invented in a different space could not make.
//!
//! # Why the crown is measured on tips
//!
//! A crown's extent is what its branches *reach*, so the sample is every live node's tip: the end of the spine the
//! sweep draws, in the world. A leaf hangs inside that reach, and a trunk's own tip is the top of the crown — so
//! the measure is the whole plant's silhouette and not an average of its parts.

use shape::Section;
use shape::math::{length3, subtract};

use crate::envelope::Envelope;
use crate::expand::{Detail, expand_in};
use crate::handle::Handle;
use crate::leaf::LeafField;
use crate::plant::{Derived, Plant};

/// How many bands of the trunk's arc the crown is measured in.
pub const BANDS: usize = 8;

/// What the plant reaches in one band of the trunk, and what its envelope allows there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// The band's middle, as an arc-length fraction of the trunk.
    pub station: f32,
    /// How far the furthest tip of a node hanging in this band reaches from the trunk's axis.
    pub reached: f32,
    /// How far the envelope allows, or `0.0` when there is no envelope.
    pub allowed: f32,
}

/// What a plant arrived at.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The live nodes: what the plant draws, tombstones excluded.
    pub nodes: usize,
    /// The wood's solids, one per live node.
    pub wood_solids: usize,
    /// The triangles those solids are made of.
    pub triangles: usize,
    /// The leaf instances.
    pub leaves: usize,
    /// The groups those instances are drawn in.
    pub groups: usize,
    /// The crown, band by band, low to high.
    pub crown: Vec<Reach>,
}

impl Report {
    /// How much further than its envelope the plant reaches anywhere: zero or negative when it fits.
    pub fn worst_exceedance(&self) -> f32 {
        self.crown
            .iter()
            .map(|reach| reach.reached - reach.allowed)
            .fold(f32::MIN, f32::max)
    }
}

/// Measures a plant: its counts, and its crown against `envelope` when one is given.
///
/// **One derived table for the expansion and the crown walk**: the tips are measured on the same placed spines
/// the expansion swept, so they are derived once and the table — a local of this call, dropped when it
/// returns — cannot outlive the immutable borrow of the plant it was derived from.
pub fn report(
    plant: &Plant,
    detail: Detail,
    fields: &[LeafField],
    envelope: Option<&Envelope>,
) -> Report {
    let mut derived = Derived::default();
    let expansion = expand_in(plant, detail, fields, &mut derived);
    let triangles: usize = expansion
        .wood
        .iter()
        .map(|solid| solid.loft.triangles().len())
        .sum();
    let leaves: usize = expansion
        .leaves
        .iter()
        .map(|group| group.leaves.len())
        .sum();
    let mut reached = [0.0f32; BANDS];
    if let Ok(trunk) = plant.spine_in(&Handle::root(), &mut derived) {
        if trunk.length() > 0.0 {
            for (handle, _) in plant.live() {
                let Ok(spine) = plant.spine_in(&handle, &mut derived) else {
                    continue;
                };
                let tip = spine.at(1.0);
                let nearest = trunk.closest(tip);
                let band = ((nearest.t * BANDS as f32) as usize).min(BANDS - 1);
                let radial = length3(subtract(tip, nearest.position));
                reached[band] = reached[band].max(radial);
            }
        }
    }
    let crown = (0..BANDS)
        .map(|band| {
            let station = (band as f32 + 0.5) / BANDS as f32;
            Reach {
                station,
                reached: reached[band],
                // **The allowance is the envelope's own widest at that station**, because a crown's reach is measured
                // radially and a shape law can be wider one way than the other: the number compared has to be the
                // most the envelope allows anywhere at that station, or a wide direction would read as an excess.
                allowed: envelope.map_or(0.0, |held| {
                    held.radius.at(station) * held.shape.widest(station)
                }),
            }
        })
        .collect();
    Report {
        nodes: plant.live().count(),
        wood_solids: expansion.wood.len(),
        triangles,
        leaves,
        groups: expansion.leaves.len(),
        crown,
    }
}

/// A section's widest reach, for a caller that wants to read an envelope's own extent without a plant.
pub fn widest(section: &Section) -> f32 {
    section
        .ring(64)
        .iter()
        .map(|point| (point[0] * point[0] + point[1] * point[1]).sqrt())
        .fold(0.0f32, f32::max)
}
