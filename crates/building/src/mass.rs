//! S1: the mass skeleton — the planned volume of a building.
//!
//! A building is a tree or network of masses: a body, its wings, a tower, a porch. Each mass is a
//! closed prismatic shell that stands on the ground: an outline, a wall thickness, a floor slab,
//! and a wall top. Levels and bays are not here — they belong to the grid (S2) on the mass's
//! faces; a roof is not here either, it is S3 and it names the mass it sits on.

use crate::math::Vec2;

/// Index of a mass in a [`BuildingSkeleton`](crate::BuildingSkeleton)'s list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MassId(pub usize);

/// One node of a mass's outline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanNode {
    /// Plan position.
    pub position: Vec2,
    /// The elevation the wall stands on.
    pub elevation: f32,
}

impl PlanNode {
    /// Builds an outline node.
    pub fn new(position: Vec2, elevation: f32) -> Self {
        Self {
            position,
            elevation,
        }
    }
}

/// Where a mass lands on its host.
///
/// # Invariants
///
/// The attachment is **authored**, and that is the whole difference between a building and a pile
/// of overlapping boxes. A mass whose plan overlaps another's with no attachment between them is
/// reported by [`validate`](crate::validate) and nothing repairs it; a mass that names a host is a
/// wing, and the joint is a named place rather than a coincidence of coordinates.
///
/// `station` is an arc-length parameter over the host's **closed** outline, in `[0, 1)`, read the
/// way a station is read everywhere else in this workspace. The op that attaches a mass
/// ([`ops::attach_mass`](crate::ops::attach_mass)) splits the host's outline at that parameter
/// first, because an outline changes its direction only at a node: a wing joining the middle of a
/// wall has to land on a node that exists, or the two solids touch at a place one of them does not
/// have.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attachment {
    /// The mass this one lands on.
    pub host: MassId,
    /// Where on the host's outline, as an arc-length parameter of the closed loop.
    pub station: f32,
}

/// A mass: the closed shell of one volume of a building.
///
/// # Invariants
///
/// - **The outline is the outer face of the wall, and it is stored open** — the edge from the last
///   node back to the first is implied. The wall is built **inward** from it by `wall`, so the
///   outline is the face a viewer sees and the thickness never moves it.
/// - **The outline is wound so its `plan_area` is positive**, and the rule is written as the test
///   rather than as a memory of a diagram, because the memory is where this goes wrong: in the
///   frame `mountain::math` defines (`x` east, `y` up, `z` **south**), `up × along` is the walker's
///   **left** — `gallery::built::edge_normal`'s formula for a wall's outer face — so a positively
///   wound loop is one whose solid lies on the walker's **right** and whose outer faces therefore
///   point away from it. (The prose in `gallery::built` that puts the enclosed ground on the
///   walker's *left* reads true only when `z` is read as north, which is not this frame's
///   convention; the formula there is unambiguous and is what is kept.) Walked the other way every
///   wall's outer face looks inward, and a building whose faces are all back-facing is not drawn at
///   all: a missing building rather than a visible fault, so it is
///   [`validate`](crate::validate)'s business and not the expander's — a loop whose signed plan area
///   comes out negative is reported by name.
/// - **The solid is closed**: outer wall, inner wall, the ring of wall top, and the floor. Its
///   bottom is `slab` below the outline's own elevations, so a mass on sloping ground is authored
///   by giving its outline nodes their own elevations.
#[derive(Debug, Clone, PartialEq)]
pub struct Mass {
    /// The outer face of the wall, wound so [`Mass::plan_area`] is positive, stored open.
    pub outline: Vec<PlanNode>,
    /// The elevation of the wall top.
    pub top: f32,
    /// Wall thickness, measured inward from the outline.
    pub wall: f32,
    /// Floor slab thickness, measured down from the outline's elevations.
    pub slab: f32,
    /// Where this mass lands on another, if it is a wing rather than a body.
    pub attachment: Option<Attachment>,
    /// What the building is called, when its author named it.
    ///
    /// # Invariants
    ///
    /// **A name is for a reader and never for a lookup.** Nothing in the crate resolves a mass by name:
    /// two buildings may share one (`"shed"` beside `"shed"`), and an author who renames one must not
    /// silently repoint anything at it — the indices are the handles, and [`site::report`] is what
    /// prints the names beside them.
    ///
    /// [`site::report`]: crate::site::report
    pub name: Option<String>,
    /// Loops this mass has a hole into: courtyards, patios, any room whose plan is a ring.
    ///
    /// # Invariants
    ///
    /// **A void is authored, not inferred.** Each loop must stand strictly inside [`Mass::outline`] and the two must
    /// not cross: a hole is what the document says it is, never a self-crossing outline that happens to enclose
    /// different ways. [`validate`](crate::validate) reports the ones that break that, and expansion refuses a mass
    /// that has one until the void slice lands — a refusal rather than a solid that pretends the courtyard is not
    /// there.
    pub voids: Vec<Vec<PlanNode>>,
}

impl Mass {
    /// Builds a mass with walls and a floor slab, standing on its outline.
    pub fn new(outline: Vec<PlanNode>, top: f32) -> Self {
        Self {
            outline,
            top,
            wall: 0.3,
            slab: 0.4,
            attachment: None,
            name: None,
            voids: Vec::new(),
        }
    }

    /// Authors a courtyard: a loop inside the outline that the mass has a hole into.
    pub fn with_voids(mut self, voids: Vec<Vec<PlanNode>>) -> Self {
        self.voids = voids;
        self
    }

    /// Gives the building a name, for a report to print and an agent to speak of.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the wall thickness.
    pub fn with_wall(mut self, wall: f32) -> Self {
        self.wall = wall;
        self
    }

    /// Sets the floor slab thickness.
    pub fn with_slab(mut self, slab: f32) -> Self {
        self.slab = slab;
        self
    }

    /// Sets where this mass lands on its host.
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachment = Some(attachment);
        self
    }

    /// Returns the signed plan area the outline encloses.
    ///
    /// # Invariants
    ///
    /// The sign carries the winding, and the winding carries the orientation of every face this
    /// mass expands to. The rule, verified numerically against the frame rather than reasoned from
    /// a diagram: `up × along` is the walker's **left** in the `(x, z)` plan of `mountain::math`
    /// (`x` east, `z` south), so **positive means the solid is on the walker's right**, which is
    /// exactly the condition under which `up × along` — the formula `gallery::built` uses for a
    /// wall ring's outer face — points **away** from the solid. A loop that comes out negative is a
    /// wall whose every outer face looks inward, which is a building that is not drawn rather than a
    /// visible fault, so it is reported by [`validate`](crate::validate) and not repaired here.
    ///
    /// `O(n)` in the outline's nodes, which is what the area of a closed loop costs.
    pub fn plan_area(&self) -> f32 {
        let count = self.outline.len();
        if count < 3 {
            return 0.0;
        }
        let mut total = 0.0;
        let mut previous = self.outline[count - 1].position;
        for node in &self.outline {
            total += crate::math::cross(previous, node.position);
            previous = node.position;
        }
        total * 0.5
    }

    /// Returns the lowest elevation the outline stands at.
    pub fn lowest(&self) -> f32 {
        self.outline
            .iter()
            .map(|node| node.elevation)
            .fold(f32::INFINITY, f32::min)
    }

    /// Returns the highest elevation the outline stands at.
    pub fn highest(&self) -> f32 {
        self.outline
            .iter()
            .map(|node| node.elevation)
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Returns how many edges the closed outline has.
    pub fn edge_count(&self) -> usize {
        self.outline.len()
    }
}
