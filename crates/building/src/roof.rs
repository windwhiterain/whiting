//! S3: the roof skeleton — the planned structure above the wall top.
//!
//! A roof is an upright miniature massif: crest lines above (ridges, hips and valleys), an eave
//! loop below, and a correspondence between them that the author writes down. It is **not**
//! expanded by `mountain`: that crate's belts hang from valley lines only and its faces are wound
//! to look up, so a convex roof — every family this module offers — would come out empty and its
//! soffit could not exist at all. What is shared is the idea, not the walk. See
//! `.agents/proposals/agentic-building-generation.md` D2.
//!
//! # The one idea, here
//!
//! **A roof plane is a strip of rows, and a row is a crest point paired with an eave station.**
//! Two adjacent planes are watertight because they **share the row on their boundary** — the same
//! crest index and the same eave station — and not because two independent constructions happened
//! to meet. A triangle is a plane whose two rows name the same crest point; a quad is two rows
//! naming two; a fan is n rows. Nothing searches, and nothing compares coordinates.

use crate::mass::{MassId, PlanNode};
use crate::math::{Vec2, Vec3, cross, distance, lift};

/// Index of a roof in a [`BuildingSkeleton`](crate::BuildingSkeleton)'s list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoofId(pub usize);

/// A vertex of the crest network: where a ridge, hip or valley line turns or ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrestPoint {
    /// Plan position.
    pub position: Vec2,
    /// Elevation.
    pub elevation: f32,
}

impl CrestPoint {
    /// Builds a crest point.
    pub fn new(position: Vec2, elevation: f32) -> Self {
        Self {
            position,
            elevation,
        }
    }

    /// Returns the point in space.
    pub fn point(&self) -> Vec3 {
        lift(self.position, self.elevation)
    }
}

/// A station on the eave loop: an arc-length parameter of the **closed** loop, in `[0, 1)`.
///
/// # Invariants
///
/// A station is a parameter and not a copy of a point, which is the rule that keeps the eave
/// authored once: the loop is the only place an eave position lives, and every plane that sheds
/// onto that eave names it. Two planes naming one station therefore resolve to one sampled point
/// and share the vertex, with no coincidence test anywhere.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EaveStation {
    /// Arc-length parameter of the closed eave loop, in `[0, 1)`.
    pub t: f32,
}

impl EaveStation {
    /// Builds a station.
    pub fn new(t: f32) -> Self {
        Self { t }
    }
}

/// One row of a roof plane: a crest point, and the eave point it sheds to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoofRow {
    /// Index into [`Roof::crest`].
    pub crest: usize,
    /// Where this row meets the eave loop.
    pub eave: EaveStation,
}

impl RoofRow {
    /// Builds a row.
    pub fn new(crest: usize, eave: EaveStation) -> Self {
        Self { crest, eave }
    }
}

/// One planar face of a roof, as the rows it is spanned between.
///
/// # Invariants
///
/// The rows are in order along the plane's own run, from one of its edges to the other, and a
/// plane with fewer than two rows has no span and is reported rather than expanded. Consecutive
/// rows make one quad of the skin; the rows at the two ends are the plane's boundary with whatever
/// plane or fascia is next, and they are also the **shading group**: one plane is one planar
/// surface, so its facets are smooth across and hard at its edges.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RoofPlane {
    /// The rows, in order.
    pub rows: Vec<RoofRow>,
}

impl RoofPlane {
    /// Builds a plane from its rows.
    pub fn new(rows: Vec<RoofRow>) -> Self {
        Self { rows }
    }

    /// Returns the two rows a plane shares with its neighbours, if it has them.
    pub fn ends(&self) -> Option<(RoofRow, RoofRow)> {
        let first = *self.rows.first()?;
        let last = *self.rows.last()?;
        Some((first, last))
    }
}

/// A roof: the eave loop, the crest network, the planes between them, and the slab's thickness.
///
/// # Invariants
///
/// - **The eave is authored once.** There is no second copy of an eave edge, in a tracing line or
///   anywhere else: a plane names stations of this loop. Two copies of one edge can drift apart,
///   and the drift shows up as a skirt of faces nobody authored.
/// - **The roof is a solid slab of constant vertical thickness**: its skin, its soffit one
///   thickness below it, and a fascia band around the eave. Vertical thickness rather than normal
///   offset, so a nearly flat roof does not become a nearly degenerate solid.
/// - **A roof and its host mass are two solids**, and their contact is an **authored
///   intersection**: `host` names the mass the roof sits on, which is what authorizes the eave
///   elevations to sit below the host's wall top — the way `terraces` sinks a roof ten centimetres
///   into its wall. Two solids that overlap with no such naming are reported by
///   [`validate`](crate::validate).
/// - The eave loop is stored **open**, like a mass's outline: the edge from the last node back to
///   the first is implied.
#[derive(Debug, Clone, PartialEq)]
pub struct Roof {
    /// The eave loop, stored open.
    pub eaves: Vec<PlanNode>,
    /// The crest network's points.
    pub crest: Vec<CrestPoint>,
    /// The planes between them.
    pub planes: Vec<RoofPlane>,
    /// The slab's vertical thickness, measured down from the skin.
    pub thickness: f32,
    /// The mass this roof sits on, and the authorization for the two to touch.
    pub host: MassId,
}

impl Roof {
    /// Builds a roof over an eave loop, sitting on a mass.
    pub fn new(host: MassId, eaves: Vec<PlanNode>, thickness: f32) -> Self {
        Self {
            eaves,
            crest: Vec::new(),
            planes: Vec::new(),
            thickness,
            host,
        }
    }

    /// Sets the crest network.
    pub fn with_crest(mut self, crest: Vec<CrestPoint>) -> Self {
        self.crest = crest;
        self
    }

    /// Sets the planes.
    pub fn with_planes(mut self, planes: Vec<RoofPlane>) -> Self {
        self.planes = planes;
        self
    }

    /// Returns the total plan length of the closed eave loop.
    ///
    /// `O(n)` in the loop's nodes, and read once per expansion rather than once per station.
    pub fn eave_length(&self) -> f32 {
        self.eave_spans().1
    }

    /// Returns the cumulative arc length at each eave node, and the total.
    pub fn eave_spans(&self) -> (Vec<f32>, f32) {
        let mut spans = Vec::with_capacity(self.eaves.len() + 1);
        let mut total = 0.0;
        spans.push(0.0);
        for index in 0..self.eaves.len() {
            let next = (index + 1) % self.eaves.len();
            total += distance(self.eaves[index].position, self.eaves[next].position);
            spans.push(total);
        }
        (spans, total)
    }

    /// Returns the point a station names, or `None` when the loop has no length.
    ///
    /// # Invariants
    ///
    /// The parameter is **arc length and not index**, the way a station means everywhere in this
    /// workspace, so a row's station is a place on the loop rather than a node of it. The lookup is a
    /// binary search over the cumulative lengths; those lengths are rebuilt on every call, so a roof's
    /// rows cost `O(rows · nodes)` and not `O(rows · log nodes)`. That is the honest price of keeping
    /// the loop the only place an eave position lives — the alternative is a cached table on a
    /// document type, which is state an edit would have to invalidate — and it is affordable because
    /// a roof has tens of rows and an eave loop has four nodes.
    pub fn eave_point(&self, station: EaveStation) -> Option<Vec3> {
        let count = self.eaves.len();
        if count < 3 {
            return None;
        }
        let (spans, total) = self.eave_spans();
        if total <= f32::EPSILON {
            return None;
        }
        let t = station.t.rem_euclid(1.0);
        let target = t * total;
        let last = count - 1;
        let index = spans
            .partition_point(|length| *length <= target)
            .saturating_sub(1)
            .min(last);
        let (low, high) = (spans[index], spans[index + 1]);
        let width = high - low;
        let amount = if width > f32::EPSILON {
            (target - low) / width
        } else {
            0.0
        };
        let from = &self.eaves[index];
        let to = &self.eaves[(index + 1) % count];
        Some(lift(
            crate::math::lerp(from.position, to.position, amount),
            from.elevation + (to.elevation - from.elevation) * amount,
        ))
    }

    /// Returns a crest point in space, or `None` for an index off the list.
    pub fn crest_point(&self, index: usize) -> Option<Vec3> {
        self.crest.get(index).map(CrestPoint::point)
    }

    /// Returns the signed plan area the eave loop encloses.
    ///
    /// The same sign rule a mass's outline follows: positive is the winding whose outer faces
    /// (`up × along`) point away from the roof's interior — which, in the `x`-east `z`-south frame
    /// [`crate::math`] defines, means the solid is on the walker's right. See [`Mass::plan_area`].
    ///
    /// [`Mass::plan_area`]: crate::mass::Mass::plan_area
    pub fn plan_area(&self) -> f32 {
        let count = self.eaves.len();
        if count < 3 {
            return 0.0;
        }
        let mut total = 0.0;
        let mut previous = self.eaves[count - 1].position;
        for node in &self.eaves {
            total += cross(previous, node.position);
            previous = node.position;
        }
        total * 0.5
    }

    /// Returns how many rows the roof's planes hold, which is how many quads its skin can have.
    pub fn row_count(&self) -> usize {
        self.planes
            .iter()
            .map(|plane| plane.rows.len().saturating_sub(1))
            .sum()
    }
}
