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
//!
//! # What a trim derives once
//!
//! **The envelope is measured against the trunk, and the trunk does not move inside a pass.** So [`trim`]
//! reads it once per pass and asks [`Envelope::outside_of`] the thirty-three times per node its bisection
//! needs, and each node's placed spine is derived once rather than once per probe. **The two passes take two
//! tables, not one table across the edit**: the second pass measures the plant the first one trimmed, and a
//! table carried across `apply` would answer with the plant the trim started from — a wrong picture, with no
//! error to notice it by.

use shape::math::{Vec2, dot3, length3, subtract};
use shape::{Curve, Scalar, Section};

use crate::handle::Handle;
use crate::ops::{Change, Edit, Property, apply};
use crate::plant::{Derived, Plant};

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
/// - **The keys are polygons, and any polygon.** An outline may be concave — a crown between two walls is not
///   star-shaped about the trunk — so a key is stored as its ring of `RING_AROUND` points, and a reach is the **first**
///   place a ray from the origin crosses the loop and not a radius per direction. No analytic solution is wanted: an
///   ellipse is sampled like everything else, and the sample error is the price the general case charges.
/// - **Between two keys the shape is the blend of the two polygons, vertex by vertex**, which is what makes a concave
///   notch keep its shape as it deepens instead of turning into a radius. Blending the two reaches would be the cheaper
///   arithmetic and the wrong surface: for a concave loop a radiance-per-direction blend is not any polygon's reach.
/// - **An empty law is the circle**, so the coarse envelope is one number and no data.
/// - **Below the first key and above the last the nearest key holds**, because a family of outlines has to cover the
///   trunk: an envelope that answered nothing at its ends would trim a plant to nothing where nobody said so.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    keys: Vec<(f32, Ring)>,
}

/// How many points a key's polygon holds, and how many directions `widest` may consider.
///
/// # Invariants
///
/// **One count for every key**, because the blend is vertex by vertex: two keys sampled at different counts could not
/// be blended at all. 64 is the same ring a drawn section uses, so the polygon a reach is measured on is the polygon a
/// picture draws.
pub const RING_AROUND: usize = 128;

/// One key's outline as a closed polygon of [`RING_AROUND`] points, wound counter-clockwise.
#[derive(Debug, Clone, PartialEq)]
struct Ring {
    points: Vec<Vec2>,
}

impl Ring {
    /// The ring of a section, sampled by arc length so the points are evenly spaced along the loop.
    fn of(section: &Section) -> Ring {
        Ring {
            points: section.ring(RING_AROUND),
        }
    }

    /// The first place a ray from the origin crosses the loop, walked in order, in the section's own plane.
    ///
    /// # Invariants
    ///
    /// **The nearest crossing wins**, so a concave loop answers with its near edge and a ray that leaves and re-enters
    /// keeps the first exit: a loop that does not contain the origin answers `0.0`, which is not a size.
    fn reach(&self, direction: Vec2) -> f32 {
        let count = self.points.len();
        let mut nearest = f32::MAX;
        let mut start = self.points[count - 1];
        for index in 0..count {
            let end = self.points[index];
            nearest = nearest.min(hit(start, end, direction));
            start = end;
        }
        if nearest == f32::MAX { 0.0 } else { nearest }
    }

    /// The furthest point of the loop from the origin, which is the most the shape reaches in any direction.
    ///
    /// # Invariants
    ///
    /// **The farthest vertex and not a sampled maximum**: the first hit of a ray in any direction is a point of the
    /// loop, so the largest reach over all directions is attained at a vertex — an edge's interior is never further
    /// than its own ends — and the answer is exact for the polygon.
    fn widest(&self) -> f32 {
        self.points
            .iter()
            .map(|point| (point[0] * point[0] + point[1] * point[1]).sqrt())
            .fold(0.0f32, f32::max)
    }
}

/// Where a ray from the origin first crosses the edge `start → end`, as a multiple of `direction`, or `f32::MAX`.
///
/// # Invariants
///
/// A hit is **further than nothing and inside the edge**: `reach > 0` drops an edge behind the ray and `along` in
/// `[0, 1]` drops a crossing past an end. A ray parallel to the edge has no `base`, and an edge of no length has no
/// direction, so both answer `f32::MAX` rather than a division.
fn hit(start: Vec2, end: Vec2, direction: Vec2) -> f32 {
    let size = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
    if !size.is_finite() || size < 1.0e-9 {
        return f32::MAX;
    }
    // **The direction is normalised first, so the hit is a distance and not a fraction of the caller's vector** — the
    // same units [`Section::reach`] answers in, which is what lets an envelope compare a radius against a reach.
    let unit = [direction[0] / size, direction[1] / size];
    let edge = [end[0] - start[0], end[1] - start[1]];
    let base = unit[0] * edge[1] - unit[1] * edge[0];
    if base.abs() < 1.0e-12 {
        return f32::MAX;
    }
    let reach = (start[0] * edge[1] - start[1] * edge[0]) / base;
    let along = (start[0] * unit[1] - start[1] * unit[0]) / base;
    if reach > 0.0 && (0.0..=1.0).contains(&along) {
        reach
    } else {
        f32::MAX
    }
}

impl Outline {
    /// The circle: one shape at every station, which is the coarse envelope.
    pub const CIRCLE: Outline = Outline { keys: Vec::new() };

    /// A family of outlines keyed along the trunk, in the trunk's arc-length fraction.
    ///
    /// # Invariants
    ///
    /// The keys must **start at zero, end at one, and increase strictly**, which is the domain claim [`Scalar::new`]
    /// makes about its knots and for the same reason: a family that does not reach both ends of the trunk would have to
    /// be extended by a rule nobody wrote.
    pub fn of(keys: &[(f32, Section)]) -> Self {
        let mut sorted: Vec<(f32, Ring)> = keys
            .iter()
            .map(|(at, section)| (*at, Ring::of(section)))
            .collect();
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
            keys: vec![(0.0, Ring::of(&section)), (1.0, Ring::of(&section))],
        }
    }

    /// The two rings the station falls between, and how far the blend has gone.
    fn bracket(&self, station: f32) -> (usize, f32) {
        let station = station.clamp(0.0, 1.0);
        let index = self
            .keys
            .partition_point(|(at, _)| *at <= station)
            .min(self.keys.len() - 1);
        let (lower, upper) = match index {
            0 => (0.0, self.keys[0].0),
            _ => (self.keys[index - 1].0, self.keys[index].0),
        };
        let width = upper - lower;
        let blend = if width > 0.0 {
            (station - lower) / width
        } else {
            1.0
        };
        (index, blend)
    }

    /// How far the outline reaches in `direction` at `station`, as a fraction of the envelope's radius.
    ///
    /// # Invariants
    ///
    /// **The first hit of a ray on the blended loop**, so a concave outline is answered as the shape it is: one pass
    /// over the ring and no table. `Outline::CIRCLE` answers `1.0`, which is the coarse envelope's whole shape.
    pub fn reach(&self, station: f32, direction: Vec2) -> f32 {
        if self.keys.is_empty() {
            return 1.0;
        }
        let (index, blend) = self.bracket(station);
        let high = &self.keys[index].1;
        if blend >= 1.0 {
            return high.reach(direction);
        }
        let low = &self.keys[index - 1].1;
        if blend <= 0.0 {
            return low.reach(direction);
        }
        let count = low.points.len();
        let lerp = |one: Vec2, other: Vec2| -> Vec2 {
            [
                one[0] + (other[0] - one[0]) * blend,
                one[1] + (other[1] - one[1]) * blend,
            ]
        };
        let mut nearest = f32::MAX;
        let mut start = lerp(low.points[count - 1], high.points[count - 1]);
        for step in 0..count {
            let end = lerp(low.points[step], high.points[step]);
            nearest = nearest.min(hit(start, end, direction));
            start = end;
        }
        if nearest == f32::MAX { 0.0 } else { nearest }
    }

    /// The furthest the outline reaches at `station`, over every direction: how wide the envelope is there.
    ///
    /// # Invariants
    ///
    /// **The farthest vertex of the blended loop**, so the answer is exact for the polygon and costs one pass: the
    /// first hit of a ray in any direction is a point of the loop, and no point of an edge is further than its ends.
    pub fn widest(&self, station: f32) -> f32 {
        if self.keys.is_empty() {
            return 1.0;
        }
        let (index, blend) = self.bracket(station);
        let high = &self.keys[index].1;
        if blend >= 1.0 {
            return high.widest();
        }
        let low = &self.keys[index - 1].1;
        if blend <= 0.0 {
            return low.widest();
        }
        low.points
            .iter()
            .zip(high.points.iter())
            .map(|(one, other)| {
                let x = one[0] + (other[0] - one[0]) * blend;
                let y = one[1] + (other[1] - one[1]) * blend;
                (x * x + y * y).sqrt()
            })
            .fold(0.0f32, f32::max)
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
    ///
    /// **The trunk is read once per query here**, and [`Envelope::outside_of`] is the same measurement against
    /// a spine the caller already holds — which is what makes a trim's bisection cost a closest-point search
    /// rather than a curve rebuild per probe.
    pub fn outside(&self, plant: &Plant, point: [f32; 3]) -> f32 {
        let Ok(trunk) = plant.spine(&Handle::root()) else {
            return 0.0;
        };
        self.outside_of(&trunk, point)
    }

    /// How far a world point is outside, measured against a trunk spine already read.
    ///
    /// # Invariants
    ///
    /// **The same measurement [`Envelope::outside`] makes**, on a spine the caller has already derived: a
    /// trunk of no length answers `0.0`, exactly as a plant whose trunk cannot be resolved does. So a caller
    /// that holds one trunk for a whole pass gets the same numbers `outside` would have given it, and a
    /// thirty-two-step bisection does not rebuild the trunk thirty-two times.
    pub(crate) fn outside_of(&self, trunk: &Curve, point: [f32; 3]) -> f32 {
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
/// - **Two derivations, not one table across both passes.** The first pass reads the trunk once and derives
///   each node's placed spine once, holds them for the whole pass, and drops them before `apply` writes; the
///   second reads them again from the trimmed plant. A table carried across the edit would answer with the
///   plant the trim started from, and **a stale placed spine is a wrong picture with no error**.
pub fn trim(plant: &mut Plant, envelope: &Envelope) -> Trim {
    let mut edits: Vec<Edit> = Vec::new();
    let mut trimmed: Vec<(Handle, f32)> = Vec::new();
    let handles: Vec<Handle> = plant
        .live()
        .filter(|(handle, _)| !handle.is_root())
        .map(|(handle, _)| handle)
        .collect();
    // **The trunk is the envelope's own axis and the plant is not written until `apply` below**, so one
    // reading serves the whole pass — including the thirty-two bisection probes per node, which are the
    // reason this loop used to rebuild a spine per probe.
    let trunk = plant.spine(&Handle::root()).ok();
    {
        let outside = |point: [f32; 3]| {
            trunk
                .as_ref()
                .map_or(0.0, |trunk| envelope.outside_of(trunk, point))
        };
        let mut derived = Derived::default();
        for handle in handles {
            let Ok(spine) = plant.spine_in(&handle, &mut derived) else {
                continue;
            };
            let tip = spine.at(1.0);
            if outside(tip) <= 0.0 {
                continue;
            }
            let Ok(placement) = plant.placement_in(&handle, &mut derived) else {
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
                outside(probe) <= 0.0
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
    }
    apply(plant, &edits);
    // What the linear model could not know: read every candidate again, after the edits and with the fields on.
    let trunk = plant.spine(&Handle::root()).ok();
    let mut derived = Derived::default();
    let mut pressed: Vec<(Handle, f32)> = Vec::new();
    for handle in plant
        .live()
        .filter(|(handle, _)| !handle.is_root())
        .map(|(handle, _)| handle)
        .collect::<Vec<_>>()
    {
        let Ok(spine) = plant.spine_in(&handle, &mut derived) else {
            continue;
        };
        let excess = trunk
            .as_ref()
            .map_or(0.0, |trunk| envelope.outside_of(trunk, spine.at(1.0)));
        if excess > 1e-5 {
            pressed.push((handle, excess));
        }
    }
    Trim { trimmed, pressed }
}
