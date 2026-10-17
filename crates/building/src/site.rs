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

use crate::grid::GRID_TOLERANCE;
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
    /// How many walls were given the ground under their own ends.
    pub walls: usize,
    /// How many slabs were moved with the building that hosts them.
    pub slabs: usize,
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
    // **Walls follow the ground instead of sitting on a terrace**, which is the difference between a
    // building and a fence: a building is cut and filled to a level floor, and a wall is built on the
    // ground it crosses, so each end takes the ground under *it*. Its top moves by the change in its own
    // mean ground, which keeps its height above the ground rather than above a plane.
    for index in 0..skeleton.walls.len() {
        let from_at = skeleton.walls[index].from.position;
        let to_at = skeleton.walls[index].to.position;
        let (from_ground, to_ground) = (ground(from_at), ground(to_at));
        if !(from_ground.is_finite() && to_ground.is_finite()) {
            continue;
        }
        let was = (skeleton.walls[index].from.elevation + skeleton.walls[index].to.elevation) * 0.5;
        let now = (from_ground + to_ground) * 0.5;
        skeleton.walls[index].from.elevation = from_ground;
        skeleton.walls[index].to.elevation = to_ground;
        skeleton.walls[index].top += now - was;
        report.walls += 1;
    }
    // **A hosted slab moves with its building, and an unhoused one does not.** A flat roof's elevations
    // are the wall top's, so a seat that moved the mass and not its plate would leave the roof behind at
    // the old level; a canopy standing on its own is a plate with no host and stays where it was authored.
    for slab in &mut skeleton.slabs {
        let Some(host) = slab.host else {
            continue;
        };
        let Some(seat) = report.masses.iter().find(|seat| seat.mass == host) else {
            continue;
        };
        slab.bottom += seat.delta;
        slab.top += seat.delta;
        report.slabs += 1;
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

/// What one building is, as a place.
#[derive(Debug, Clone, PartialEq)]
pub struct Building {
    /// The building, by its mass's index.
    pub mass: MassId,
    /// What its author called it, if anything.
    pub name: Option<String>,
    /// The plan area its outline encloses, as authored.
    pub footprint: f32,
    /// The elevation of its wall top.
    pub top: f32,
    /// The lowest ground under its own outline.
    pub lowest: f32,
    /// How many roofs sit on it.
    pub roofs: usize,
    /// How many openings its own walls carry.
    pub openings: usize,
    /// How many parts stand in its own bays.
    pub parts: usize,
}

/// Two buildings whose **plan bounding boxes** share area.
///
/// # Invariants
///
/// It is a **bound and not an intersection**, and it says so: two L-shaped buildings can have overlapping
/// boxes and no shared wall at all. It is a reading for an author to look at — wings overlap their hosts
/// by design, and two bodies that overlap may or may not be a mistake — which is why the report lists it
/// and nothing judges it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overlap {
    /// The earlier building, by its mass's index.
    pub first: MassId,
    /// The later.
    pub second: MassId,
    /// The area their bounding boxes share, in plan.
    pub area: f32,
}

/// What a site holds, as a place an agent can reason about.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SiteReport {
    /// Every building, in the document's own order.
    pub buildings: Vec<Building>,
    /// How many wall runs the site encloses itself with.
    pub walls: usize,
    /// How much wall that is, end to end.
    pub wall_length: f32,
    /// The ground the buildings stand on, as (lowest, highest) after seating, or `None` when the
    /// document holds no building.
    pub ground: Option<(f32, f32)>,
    /// Buildings whose bounding boxes share plan area.
    pub overlaps: Vec<Overlap>,
}

impl SiteReport {
    /// Returns one building by its mass.
    pub fn building(&self, mass: MassId) -> Option<&Building> {
        self.buildings.iter().find(|building| building.mass == mass)
    }

    /// Returns the total plan area the buildings stand on, which is a site's own footprint.
    pub fn footprint(&self) -> f32 {
        self.buildings
            .iter()
            .map(|building| building.footprint)
            .sum()
    }

    /// Returns how far the ground falls under the site's own buildings.
    pub fn relief(&self) -> f32 {
        self.ground
            .map(|(lowest, highest)| highest - lowest)
            .unwrap_or(0.0)
    }
}

/// Reports what a document holds as a **place**: its buildings by name, the ground they stand on, the
/// walls that enclose them, and which of them share plan area.
///
/// # Invariants
///
/// **It reads and never judges.** Every number here is a fact about the document that no single element
/// can state: a mass knows its own footprint, and only the site knows that two of them overlap by twelve
/// square metres or that its ground falls eight. That is the whole of "the site as the unit": a document
/// of several buildings is a site already, and what was missing was a way to look at it.
///
/// `O(masses · (nodes + walls))` for the summary and `O(masses²)` for the overlaps, once, on demand.
pub fn report(skeleton: &BuildingSkeleton) -> SiteReport {
    let mut buildings = Vec::with_capacity(skeleton.masses.len());
    let mut lowest = f32::INFINITY;
    let mut highest = f32::NEG_INFINITY;
    for (index, mass) in skeleton.masses.iter().enumerate() {
        let id = MassId(index);
        let mut own_lowest = f32::INFINITY;
        for node in &mass.outline {
            own_lowest = own_lowest.min(node.elevation);
            lowest = lowest.min(node.elevation);
            highest = highest.max(node.elevation);
        }
        let roofs = skeleton.roofs_of(id).len();
        let openings = skeleton
            .faces_of(id)
            .iter()
            .map(|face| skeleton.openings.of_face(*face).len())
            .sum();
        let parts = skeleton
            .faces_of(id)
            .iter()
            .map(|face| skeleton.parts.of_face(*face).len())
            .sum();
        buildings.push(Building {
            mass: id,
            name: mass.name.clone(),
            footprint: mass.plan_area().abs(),
            top: mass.top,
            lowest: own_lowest,
            roofs,
            openings,
            parts,
        });
    }
    let mut overlaps = Vec::new();
    for first in 0..skeleton.masses.len() {
        for second in (first + 1)..skeleton.masses.len() {
            let area = overlap_of(&skeleton.masses[first], &skeleton.masses[second]);
            if area > GRID_TOLERANCE {
                overlaps.push(Overlap {
                    first: MassId(first),
                    second: MassId(second),
                    area,
                });
            }
        }
    }
    SiteReport {
        buildings,
        walls: skeleton.walls.len(),
        wall_length: skeleton.walls.iter().map(|wall| wall.length()).sum(),
        ground: (lowest.is_finite() && highest.is_finite()).then_some((lowest, highest)),
        overlaps,
    }
}

/// A plan bounding box, which is what a site's overlaps are measured between.
struct Bounds2 {
    low: [f32; 2],
    high: [f32; 2],
}

impl Bounds2 {
    fn empty() -> Self {
        Self {
            low: [f32::INFINITY; 2],
            high: [f32::NEG_INFINITY; 2],
        }
    }

    fn grow(&mut self, at: Vec2) {
        for axis in 0..2 {
            self.low[axis] = self.low[axis].min(at[axis]);
            self.high[axis] = self.high[axis].max(at[axis]);
        }
    }

    fn of(mass: &crate::mass::Mass) -> Self {
        let mut bounds = Self::empty();
        for node in &mass.outline {
            bounds.grow(node.position);
        }
        bounds
    }
}

/// Returns the plan area two buildings' bounding boxes share.
fn overlap_of(first: &crate::mass::Mass, second: &crate::mass::Mass) -> f32 {
    let (a, b) = (Bounds2::of(first), Bounds2::of(second));
    let mut area = 1.0;
    for axis in 0..2 {
        let low = a.low[axis].max(b.low[axis]);
        let high = a.high[axis].min(b.high[axis]);
        area *= (high - low).max(0.0);
    }
    area
}
