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
//! # And what stands under the floor
//!
//! A terrace is the right answer to a slope for masonry and the wrong one for a timber house on a
//! river bank: there the floor is levelled to the *uphill* grade and the downhill side stands on
//! posts, with air under it. [`posts`] adds those posts to an already seated document — it appends
//! solids and moves nothing — and the range of post lengths it produces **is** the site's relief
//! under the house, which is what makes a 吊脚楼 a building rather than a box on a plinth.
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
use crate::math::{Vec2, distance, lerp};
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
        for top in &mut skeleton.masses[index].crowns {
            *top += delta;
        }
        let host = MassId(index);
        // **A rhythm is authored against a face, and a face that moves takes its rhythm with it.** A grid's
        // `levels` are the face's own elevations, so a mass that the seat moves keeps its bays and its storeys
        // only if they move by the same `delta`. Without this, seating a house *after* it has been authored
        // leaves its grid spanning the elevation it had before the move, which `validate` names
        // `GridOutsideFace` — measured on the village's four houses, whose documents were cut at the origin and
        // then put on a slope.
        for face in skeleton.grids.faces() {
            if face.mass != host {
                continue;
            }
            if let Some(grid) = skeleton.grids.get(face).cloned() {
                let levels = grid.levels.iter().map(|level| level + delta).collect();
                skeleton.grids.replace(
                    face,
                    crate::grid::FaceGrid::new(levels, grid.divisions.clone()),
                );
            }
        }
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

/// What standing a building on posts did, one entry per mass that was given them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StiltReport {
    /// One entry per mass posts were placed under, in the document's own order.
    pub masses: Vec<Stilt>,
    /// How many posts were appended to the document's walls.
    pub posts: usize,
    /// How many post positions were dropped because the ground did not answer under them.
    pub refused: usize,
}

impl StiltReport {
    /// Returns how many masses were given posts.
    pub fn len(&self) -> usize {
        self.masses.len()
    }

    /// Returns whether no mass was given a post, which is what a flat site answers.
    pub fn is_empty(&self) -> bool {
        self.masses.is_empty()
    }

    /// Returns the deepest any one of these buildings hangs over its ground.
    ///
    /// It is the number this arc exists to make printable: the drop on the downhill side *is* the
    /// building's proportion, and a 吊脚楼 with a tenth of a metre of drop is a house with a plinth.
    pub fn deepest_drop(&self) -> f32 {
        self.masses.iter().map(Stilt::drop).fold(0.0_f32, f32::max)
    }
}

/// One mass's posts, and the relief of the ground they stand on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stilt {
    /// The mass the posts stand under.
    pub mass: MassId,
    /// The elevation of the building's own underside — the plate's bottom, one slab below the mass's
    /// lowest node. The posts reach [`posts`]'s `embed` above it, into the plate.
    pub underside: f32,
    /// The highest ground any of its posts stands on.
    pub highest: f32,
    /// The lowest ground any of its posts stands on.
    pub lowest: f32,
    /// How many posts it got.
    pub posts: usize,
}

impl Stilt {
    /// Returns how far the building hangs over the lowest ground it stands on.
    pub fn drop(&self) -> f32 {
        self.underside - self.lowest
    }

    /// Returns how much the ground rises across the house's own posts.
    pub fn relief(&self) -> f32 {
        self.highest - self.lowest
    }
}

/// Stands a seated building on posts, the way a house on a bank stands: the floor stays where
/// [`seat`] levelled it and the downhill side is carried by columns instead of by a plinth.
///
/// # Why this is a second call and not a mode of seating
///
/// Seating answers *where the floor is*, and its answer — the highest ground under the outline plus an
/// offset — is right for this house as well: a 吊脚楼's floor is the uphill grade. What a terrace and a
/// stilt house do **not** share is what stands under that floor, and that is what this function adds.
/// Keeping the two apart is what lets a caller seat a document and then decide, mass by mass, which
/// ones hang.
///
/// # Invariants
///
/// - **Nothing here moves.** This appends solids below an underside that seating already levelled; it
///   does not raise, lower or rotate a mass, a roof or a plate. A post that moved a floor would be a
///   second opinion about where the floor is.
/// - **A post's top is the building's underside and its bottom is the sampled ground.** Its length is
///   therefore a *query* and not a parameter, which is the whole of D4: the range of post lengths is the
///   site's relief under the house.
/// - **A post is placed where the ground is below the floor and nowhere else.** The uphill side of a
///   stilt house has no posts because its wall meets the hill; a post whose ground reaches the
///   underside would be a post buried in it. The comparison is against the **higher** of a post's two
///   foot samples, so a post is placed only if it clears the ground at both ends.
/// - **A ground that does not answer is refused, not guessed** — the rule `seat` follows for a mass
///   (`site.rs`'s own refusal) applied to a post: a non-finite sample skips that post and counts it.
/// - **Call it after seating.** A later [`seat`] moves these walls the way it moves a fence — each end
///   to the ground under it and the top by the change in its mean — which would leave a post standing on
///   the ground with its top no longer at the underside. Order is the caller's, and it is stated here
///   rather than enforced because the document has no way to mark a wall as a post.
/// - **A post is a [`Member`](crate::member::Member) now**, standing on one point of ground with its
///   head `embed` into the plate. It was a short `Wall` run — a section's length across the ground and
///   a level top — and that foot tilted with the hill it stood on, which is what a wall does and not
///   what a post on a stone does.
/// - **The post's top is `embed` above the underside**, so its top face stands *inside* the plate
///   rather than coplanar with the plate's own bottom. Two solids may touch, but two **coincident
///   faces** are two surfaces at one depth and a rasteriser draws them fighting: the stilt scene's first
///   render carried a comb of stripes along the plate's edge for exactly that reason, and a real post is
///   tenoned into the floor rather than butted against it.
///
/// `O(nodes + perimeter / spacing)` per mass.
pub fn posts(
    skeleton: &mut BuildingSkeleton,
    ground: impl Fn(Vec2) -> f32,
    spacing: f32,
    section: f32,
    embed: f32,
) -> StiltReport {
    let mut report = StiltReport::default();
    if !(spacing.is_finite() && section.is_finite() && embed.is_finite())
        || spacing <= 0.0
        || section <= 0.0
        || embed < 0.0
    {
        return report;
    }
    let half = section * 0.5;
    let mut appended: Vec<crate::member::Member> = Vec::new();
    for (index, mass) in skeleton.masses.iter().enumerate() {
        let underside = mass.lowest() - mass.slab;
        if !underside.is_finite() {
            continue;
        }
        let mut placed = 0usize;
        let mut highest = f32::NEG_INFINITY;
        let mut lowest = f32::INFINITY;
        for at in post_positions(&mass.outline, spacing) {
            // **A post is a member now, and a member's foot is one point.** The wall it used to be was
            // a run of its own section across the ground, so its foot tilted with the hill it stood on;
            // a real post stands on a stone, and the stone is level. The head is the same point it
            // always was, `embed` into the plate.
            let foot = ground(at);
            if !foot.is_finite() {
                report.refused += 1;
                continue;
            }
            if foot >= underside {
                continue;
            }
            let _ = half;
            highest = highest.max(foot);
            lowest = lowest.min(foot);
            appended.push(crate::member::Member::new(
                crate::member::Anchor::Free {
                    at: [at[0], foot, at[1]],
                },
                crate::member::Anchor::Free {
                    at: [at[0], underside + embed, at[1]],
                },
                // A square post's faces run with the walls it stands under, and a plan normal is the
                // one direction a member's `across` can be read without knowing the axis.
                [1.0, 0.0],
                section,
                section,
            ));
            placed += 1;
        }
        if placed > 0 {
            report.masses.push(Stilt {
                mass: MassId(index),
                underside,
                highest,
                lowest,
                posts: placed,
            });
        }
    }
    report.posts = appended.len();
    skeleton.members.extend(appended);
    report
}

/// Returns the plan points posts are placed at: every node of the closed outline, and the interior
/// stops of each edge at roughly `spacing`.
///
/// # Invariants
///
/// **Each edge contributes its own start and not its own end**, so the closed walk names every node
/// exactly once and no position twice: a corner post is one post. The count per edge is
/// `round(length / spacing)` and never zero, so a short edge still gets a node post and a long one
/// cannot be missed.
fn post_positions(outline: &[PlanNode], spacing: f32) -> Vec<Vec2> {
    let count = outline.len();
    let mut positions = Vec::new();
    if count < 2 {
        return positions;
    }
    for index in 0..count {
        let from = outline[index].position;
        let to = outline[(index + 1) % count].position;
        let length = distance(from, to);
        if length <= f32::EPSILON {
            continue;
        }
        let steps = (length / spacing).round().max(1.0) as usize;
        for step in 0..steps {
            positions.push(lerp(from, to, step as f32 / steps as f32));
        }
    }
    positions
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
        for node in &mass.outline {
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
            lowest: mass.lowest(),
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
        for axis in [0, 1] {
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
