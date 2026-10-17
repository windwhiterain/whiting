//! The site: ground under a building, and what it takes to sit there.
//!
//! # What this is for
//!
//! A building authored on a plane is a building that floats the moment it is drawn anywhere else. This
//! module writes the ground **into the document**: a mass's own elevations become the ground's, and from
//! then on the shell, the openings and the parts are the same code they were on a plane — because a
//! document that holds its ground needs no terrain to expand.
//!
//! # Why it takes a function
//!
//! P1's decision list says this crate takes **no `terrain` dependency**, and seating does not need one:
//! a height field is a function from plan to elevation. So the caller passes the function —
//! `|plan| field.sample(plan)` for a `terrain::HeightField` — and the crate stays independent of the
//! mountain pipeline. It also makes the slice measurable against an **analytic** ground, which is how
//! the probe's numbers are exact rather than plausible.
//!
//! # One level per mass, and it is the highest ground under it
//!
//! The shell's floor top is a polygon at its nodes' own elevations, so a quad with four different
//! elevations is a **ruled surface and not a plane** — and the crate's planarity contract would report
//! it, correctly. A building therefore sits on a **terrace**: every node of a mass is given one level,
//! the highest sampled ground under that outline plus the caller's offset. That is also what a builder
//! does with a slope — cut and fill to a level floor, and let the downhill side show its slab — and it
//! keeps every invariant the shell already has.
//!
//! # The sample set is stated, because an outline is not a point
//!
//! "The highest ground under the outline" has no exact answer from a sampling function, so the answer
//! is **the highest of the outline's nodes, its edges' midpoints, and its centroid** — `O(nodes)`, and
//! exact for a plane, which is what the probe checks. For a **concave** outline the centroid can lie
//! outside the polygon, and the error is in the safe direction: the level can only come out too high,
//! so the building sits above its ground rather than inside it. A proper interior sampling is a
//! triangulation question and is not this slice's.

use crate::mass::{MassId, PlanNode};
use crate::math::{Vec2, lerp};
use crate::skeleton::BuildingSkeleton;

/// What seating one mass did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seat {
    /// The mass that was seated.
    pub mass: MassId,
    /// The elevation every node of its outline was given.
    pub level: f32,
    /// The lowest ground the sample set found under it.
    pub lowest: f32,
    /// The highest, which is what the level is cut to.
    pub highest: f32,
    /// How far the whole mass — its outline, its top and its roofs — was moved.
    pub delta: f32,
}

impl Seat {
    /// Returns how far the ground falls away from the level on the downhill side.
    ///
    /// It is the number an author looks at: a metre of fall is a metre of foundation showing, and a
    /// tenth of a metre is a building sitting on its ground.
    pub fn fall(&self) -> f32 {
        self.level - self.lowest
    }
}

/// What seating a document did, one entry per mass, in the document's own order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SeatReport {
    /// One entry per mass, in the order the masses are held.
    pub masses: Vec<Seat>,
}

impl SeatReport {
    /// Returns how many masses were seated.
    pub fn len(&self) -> usize {
        self.masses.len()
    }

    /// Returns whether the document held no mass to seat.
    pub fn is_empty(&self) -> bool {
        self.masses.is_empty()
    }

    /// Returns the deepest fall any seated mass has to bridge, which is the site's own relief under
    /// its buildings.
    pub fn deepest_fall(&self) -> f32 {
        self.masses.iter().map(Seat::fall).fold(0.0_f32, f32::max)
    }

    /// Returns the highest level any mass was given.
    pub fn highest_level(&self) -> Option<f32> {
        self.masses.iter().map(|seat| seat.level).reduce(f32::max)
    }
}

/// Seats every mass of a skeleton on the ground `ground` gives, and reports what it did.
///
/// # Invariants
///
/// - **A mass moves rigidly.** The delta is applied to every node of its outline, to its `top`, and to
///   the eaves and crest of every roof that names it as host — so the walls keep their height and the
///   roof keeps its rise. Moving only the outline would shorten every wall by the terrain's height and
///   flatten every pitch, which is the one way a seat can be silently wrong.
/// - **One level per mass**, from the highest sampled ground under that mass's outline plus `offset`.
///   Two masses on one slope are therefore two terraces, not one, and the report says which.
/// - **A ground that does not answer under the whole outline is not seated at all.** Half a ground is
///   not a ground: a `NaN` or infinite sample means the terrain is unknown somewhere under the
///   building, and a level computed from the rest would be a level computed from the wrong set — the
///   guess this crate refuses everywhere else. The mass is left exactly as it was and is simply absent
///   from the report, so `report.len()` against `skeleton.masses.len()` says how many were refused.
/// - The seat is a **function of the ground and the plan**: two runs of the same pair produce the same
///   document, and the report is the only thing that says what changed.
pub fn seat(
    skeleton: &mut BuildingSkeleton,
    ground: impl Fn(Vec2) -> f32,
    offset: f32,
) -> SeatReport {
    let mut report = SeatReport::default();
    for index in 0..skeleton.masses.len() {
        let samples = samples_of(&skeleton.masses[index].outline);
        let mut lowest = f32::INFINITY;
        let mut highest = f32::NEG_INFINITY;
        let mut complete = true;
        for at in &samples {
            let height = ground(*at);
            if !height.is_finite() {
                complete = false;
                break;
            }
            lowest = lowest.min(height);
            highest = highest.max(height);
        }
        if !complete || !(lowest.is_finite() && highest.is_finite()) {
            continue;
        }
        let level = highest + offset;
        let reference = skeleton.masses[index]
            .outline
            .iter()
            .map(|node| node.elevation)
            .fold(f32::INFINITY, f32::min);
        let delta = level - reference;
        for node in &mut skeleton.masses[index].outline {
            node.elevation += delta;
        }
        skeleton.masses[index].top += delta;
        let host = MassId(index);
        for roof in skeleton.roofs.iter_mut() {
            if roof.host != host {
                continue;
            }
            for eave in &mut roof.eaves {
                eave.elevation += delta;
            }
            for crest in &mut roof.crest {
                crest.elevation += delta;
            }
        }
        report.masses.push(Seat {
            mass: host,
            level,
            lowest,
            highest,
            delta,
        });
    }
    report
}

/// Returns the plan points one outline is sampled at: its nodes, its edges' midpoints, its centroid.
///
/// # Invariants
///
/// The set is stated rather than discovered, because a sampling function cannot be inverted to say
/// where its maximum lies. It contains every extreme of a plane over a convex outline — a linear
/// function attains its maximum at a vertex — which is what makes the probe's comparison with the
/// analytic maximum an equality rather than an estimate.
fn samples_of(outline: &[PlanNode]) -> Vec<Vec2> {
    let count = outline.len();
    let mut samples = Vec::with_capacity(2 * count + 1);
    for node in outline {
        samples.push(node.position);
    }
    for index in 0..count {
        let from = outline[index].position;
        let to = outline[(index + 1) % count].position;
        samples.push(lerp(from, to, 0.5));
    }
    if count > 0 {
        let mut middle = [0.0_f32; 2];
        for node in outline {
            middle[0] += node.position[0];
            middle[1] += node.position[1];
        }
        samples.push([middle[0] / count as f32, middle[1] / count as f32]);
    }
    samples
}
