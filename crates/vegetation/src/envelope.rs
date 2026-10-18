//! The crown a plant may reach: V4's envelope, and the trimming law that keeps a plant inside it.
//!
//! # The one idea
//!
//! **An envelope is a radius law about the trunk, refined to a section when a circle is not enough.** A point's
//! station is where the trunk comes nearest to it, so the question *"is this outside the crown?"* costs one
//! closest-point query and one division — the coarse form is `distance ≤ radius(t)`, and the refined form is the
//! same sentence with the radius replaced by how far the section reaches in the point's own direction. Refining
//! is therefore a data change and not a new type, which is what V4 asked for.
//!
//! # The trim is an edit
//!
//! [`trim`] does not move a spine: it **writes length nudges and applies them through [`crate::ops`]**, the same
//! path the hand uses. So a trimmed plant is an edited plant — every length the trim took is a number at a handle,
//! the report says how much came off each one, and a caller can read, keep or undo the list. A tip that cannot be
//! saved (its node already stands outside the envelope) is **pressed**: reported, not deleted, because V4 says a
//! tip an author deliberately let past is a note rather than a violation.
//!
//! # What is measured, and what is promised
//!
//! The linear model — a length scale moves a node's tip along the segment from its origin — is exact *without*
//! fields and approximate with them, because a field bends a shorter spine differently. So the trim runs one pass
//! on the model and then **measures what is left**, which is why [`Trim::pressed`] exists rather than a claim.

use shape::math::{Vec2, dot3, length3, subtract};
use shape::{Scalar, Section};

use crate::handle::Handle;
use crate::ops::{Change, Edit, Property, apply};
use crate::plant::Plant;

/// The crown a plant may reach: a radius law about the trunk, and the shape law it is drawn with.
///
/// # The two laws, and why they are two
///
/// **The radius is a smooth scale and the shape is a family of outlines.** A crown's size varies continuously along
/// the trunk — a Hermite law, the same `Scalar` every other law in the crate uses — while its *shape* changes in steps
/// an author cares about: round at the base where the trunk is free, flattened at the top where a soffit is. Keeping
/// them apart means the coarse envelope is `radius` alone (the empty shape law is a circle) and refining it is filling
/// in a family, which is what the design asked for: *a radius curve first, refined to the surface of a curve in the
/// plane across the trunk at each parameter*.
#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    /// The radius at each arc-length fraction of the trunk, in the plant's units.
    pub radius: Scalar,
    /// The shape the radius is drawn with, at each fraction of the trunk.
    pub shape: Outline,
}

/// A law of outlines: the shape an envelope has at a station, and how it passes to the next one.
///
/// # Invariants
///
/// - **An empty law is the circle**, so the coarse envelope is one number and no data: `Envelope::radial` is the
///   coarse form, and a refined crown differs from it by the shape alone.
/// - **A section is star-shaped about its origin** (that is what [`Section::reach`] is: one radius per direction), so
///   the reach of the shape between two keys is the **linear blend of the two keys' reaches in the same direction** —
///   the same answer as blending the outlines themselves, at one query per key and no allocation.
/// - **Below the first key and above the last the nearest key holds**, because a family of outlines has to cover the
///   trunk: an envelope that answered nothing at its ends would trim a plant to nothing where nobody said so.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    keys: Vec<(f32, Section)>,
}

impl Outline {
    /// The circle: one shape at every station, which is the coarse envelope.
    pub const CIRCLE: Outline = Outline { keys: Vec::new() };

    /// A family of outlines keyed along the trunk, in the trunk's arc-length fraction.
    ///
    /// # Invariants
    ///
    /// The keys must **start at zero, end at one, and increase strictly**, which is the domain claim [`Scalar::new`]
    /// makes about its knots and for the same reason: a family that does not reach both ends of the trunk would have
    /// to be extended by a rule nobody wrote.
    pub fn of(keys: &[(f32, Section)]) -> Self {
        let mut sorted: Vec<(f32, Section)> = keys.to_vec();
        sorted.sort_by(|one, other| {
            one.0
                .partial_cmp(&other.0)
                .expect("a station is a finite fraction of a trunk")
        });
        assert!(
            !sorted.is_empty(),
            "an outline law with no key is the circle, which `Outline::CIRCLE` says once"
        );
        assert!(
            (sorted[0].0 - 0.0).abs() < 1e-6 && (sorted[sorted.len() - 1].0 - 1.0).abs() < 1e-6,
            "an outline law's first key is at zero and its last at one"
        );
        assert!(
            sorted.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "an outline law's stations increase strictly"
        );
        Outline { keys: sorted }
    }

    /// The same family with one section at every station, which is what a single refined section is.
    pub fn constant(section: Section) -> Self {
        Outline {
            keys: vec![(0.0, section.clone()), (1.0, section)],
        }
    }

    /// The furthest the outline reaches at `station`, over every direction: how wide the envelope is there.
    ///
    /// # Invariants
    ///
    /// **Sixty-four directions**, the same ring the report draws a section with, so the number a crown is reported
    /// against is the number a reader sees when that section is drawn: a shape whose widest direction falls between two
    /// samples is reported as the wider of them and never as the narrower.
    pub fn widest(&self, station: f32) -> f32 {
        let mut widest = 0.0f32;
        for step in 0..64 {
            let angle = step as f32 * (std::f32::consts::TAU / 64.0);
            let (sin, cos) = angle.sin_cos();
            widest = widest.max(self.reach(station, [cos, sin]));
        }
        widest
    }
    /// How far the outline reaches in `direction` at `station`, as a fraction of the envelope's radius.
    ///
    /// # Invariants
    ///
    /// **One query per bracketing key and one blend**, so a refined envelope costs a search over its own keys and not a
    /// pass over a table: the keys are few and `partition_point` finds them in `log n`.
    pub fn reach(&self, station: f32, direction: Vec2) -> f32 {
        if self.keys.is_empty() {
            return 1.0;
        }
        let station = station.clamp(0.0, 1.0);
        let after = self
            .keys
            .partition_point(|(at, _)| *at <= station)
            .min(self.keys.len() - 1);
        let (to, upper) = (&self.keys[after].1, self.keys[after].0);
        let (from, lower) = match after {
            0 => (to, upper),
            index => {
                let (at, section) = &self.keys[index - 1];
                (section, *at)
            }
        };
        let width = upper - lower;
        let blend = if width > 0.0 {
            (station - lower) / width
        } else {
            1.0
        };
        let reached = from.reach(direction);
        reached + (to.reach(direction) - reached) * blend
    }
}

/// How far a node was trimmed, and how far it could not be.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trim {
    /// One entry per node whose length was reduced, as the node and how much came off it.
    pub trimmed: Vec<(Handle, f32)>,
    /// One entry per node whose tip is still outside after the trim, as the node and how far.
    pub pressed: Vec<(Handle, f32)>,
}

impl Envelope {
    /// A circular envelope from `(t, radius)` knots along the trunk.
    pub fn radial(knots: &[(f32, f32)]) -> Self {
        Self {
            radius: Scalar::new(knots).expect("an envelope's radius is a law"),
            shape: Outline::CIRCLE,
        }
    }

    /// The same radius law drawn with a section other than a circle: a crown that is wider across than along.
    pub fn with_section(mut self, section: Section) -> Self {
        self.shape = Outline::constant(section);
        self
    }

    /// How far a world point is outside the envelope: zero inside, positive beyond.
    ///
    /// # Invariants
    ///
    /// - **The station is the trunk's nearest point to `point`**, so the query is one [`shape::Curve::closest`]
    ///   and the envelope needs no search of its own. A plant whose trunk has no length answers zero: a crown
    ///   with no axis is not a crown.
    /// - **The direction is read in the trunk's own frame there**, the same `(normal, binormal)` plane the sweep
    ///   draws a section in, so an envelope and the sections of the wood it encloses are measured in one space.
    /// - **The outside distance is the radial excess**: how much further the point is than the section reaches,
    ///   so a point inside is `0.0` and not a negative number a caller has to know about.
    pub fn outside(&self, plant: &Plant, point: [f32; 3]) -> f32 {
        let Ok(trunk) = plant.spine(&Handle::root()) else {
            return 0.0;
        };
        if trunk.length() <= 0.0 {
            return 0.0;
        }
        let nearest = trunk.closest(point);
        let frame = trunk.frame(nearest.t);
        let offset = subtract(point, nearest.position);
        let across = [dot3(offset, frame.normal), dot3(offset, frame.binormal)];
        let radial = length3([across[0], across[1], 0.0]);
        let allowed = self.radius.at(nearest.t) * self.shape.reach(nearest.t, across);
        (radial - allowed).max(0.0)
    }
}

/// The shortest a trim will leave a node, as a fraction of its own length.
///
/// # Invariants
///
/// **A trim cannot make a branch vanish.** V4's sentence is *"a branch's length is scaled so that its tip's
/// outside() is zero"*, and taken literally a tip that must come most of the way in scales the branch to nothing:
/// the first version took a bough from 1.054910 m to **0.000007 m**, which is a solid with no length and a node
/// nobody meant to keep. A tenth of the node is the floor, and a node that would have to go below it is
/// **pressed** — reported to its author, who decides whether it is pruned instead.
const MIN_SCALE: f32 = 0.1;

/// Pulls every branch whose tip is outside the envelope back until the tip is on it.
///
/// # Invariants
///
/// - **The trim is a list of length nudges**, applied through [`crate::ops::apply`], so a trimmed plant is an
///   edited plant and every number the trim chose is readable at its handle.
/// - **The trunk is not trimmed**: it is the envelope's own axis, so shortening it would move the crown rather
///   than fit it. Every other live node is a candidate, and a node whose tip is already inside is left alone.
/// - **A node whose origin is outside cannot be saved**: no length shortens it into the crown, so it appears in
///   [`Trim::pressed`] and its length is untouched.
/// - **The model is linear and the result is measured.** A length scale moves the tip along the segment from the
///   node's origin, which is exact until a field bends the shorter spine; the trim runs one pass and then reports
///   what is still outside rather than claiming the envelope is met.
pub fn trim(plant: &mut Plant, envelope: &Envelope) -> Trim {
    let mut edits: Vec<Edit> = Vec::new();
    let mut trimmed: Vec<(Handle, f32)> = Vec::new();
    let handles: Vec<Handle> = plant
        .live()
        .filter(|(handle, _)| !handle.is_root())
        .map(|(handle, _)| handle)
        .collect();
    for handle in handles {
        let Ok(spine) = plant.spine(&handle) else {
            continue;
        };
        let tip = spine.at(1.0);
        if envelope.outside(plant, tip) <= 0.0 {
            continue;
        }
        let Ok(placement) = plant.placement(&handle) else {
            continue;
        };
        let origin = placement.origin;
        let length = spine.length();
        if length <= 0.0 {
            continue;
        }
        let inside = |scale: f32| {
            let probe = [
                origin[0] + (tip[0] - origin[0]) * scale,
                origin[1] + (tip[1] - origin[1]) * scale,
                origin[2] + (tip[2] - origin[2]) * scale,
            ];
            envelope.outside(plant, probe) <= 0.0
        };
        if !inside(0.0) {
            // Even pulled back to its own origin the node is outside: the crown does not reach where it hangs.
            continue;
        }
        let mut low = 0.0f32;
        let mut high = 1.0f32;
        for _ in 0..32 {
            let middle = 0.5 * (low + high);
            if inside(middle) {
                low = middle;
            } else {
                high = middle;
            }
        }
        if low < MIN_SCALE {
            // The crown would take almost the whole branch: a note for its author, not a seven-micron limb.
            continue;
        }
        let removed = (1.0 - low) * length;
        if removed <= 1e-6 {
            continue;
        }
        trimmed.push((handle.clone(), removed));
        edits.push(Edit::new(
            handle,
            Change::Offset {
                property: Property::Length,
                by: -removed,
            },
        ));
    }
    apply(plant, &edits);
    // What the linear model could not know: read every candidate again, after the edits and with the fields on.
    let mut pressed: Vec<(Handle, f32)> = Vec::new();
    for handle in plant
        .live()
        .filter(|(handle, _)| !handle.is_root())
        .map(|(handle, _)| handle)
        .collect::<Vec<_>>()
    {
        let Ok(spine) = plant.spine(&handle) else {
            continue;
        };
        let excess = envelope.outside(plant, spine.at(1.0));
        if excess > 1e-5 {
            pressed.push((handle, excess));
        }
    }
    Trim { trimmed, pressed }
}
