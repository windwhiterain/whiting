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

/// Which of a mass's authored plan loops a statement is about.
///
/// # Invariants
///
/// **A mass holds two kinds of loop and every fault about either must say which.** The outline is
/// the wall's outer face and the wall is built inward from it; a void is a hole in the mass's plan
/// and a ring of wall is built *outward* from it. [`expand::offset_loop`](crate::expand) walks both
/// with one miter, and every way that walk can fail — a degenerate edge, a corner with no miter, a
/// join past its limit — is a statement about one of these two loops. Carrying the loop rather than
/// the mass alone is what keeps "offset edge 2 has no length left" from naming the courtyard's edge
/// as if it were the building's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AuthoredLoop {
    /// The mass's own outline, the loop its wall is built inward from.
    Outline {
        /// The mass that authored it.
        mass: MassId,
    },
    /// One of the mass's voids: the loop a courtyard's ring is built outward from.
    Void {
        /// The mass that authored it.
        mass: MassId,
        /// Which of that mass's voids, in the order it authored them.
        void: usize,
    },
}

impl AuthoredLoop {
    /// Returns the mass that holds this loop.
    pub fn mass(&self) -> MassId {
        match self {
            Self::Outline { mass } | Self::Void { mass, .. } => *mass,
        }
    }

    /// Returns the void's index, or `None` for a mass's outline.
    pub fn void(&self) -> Option<usize> {
        match self {
            Self::Outline { .. } => None,
            Self::Void { void, .. } => Some(*void),
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
///   frame `shape::math` defines (`x` east, `y` up, `z` **south**), `up × along` is the walker's
///   **left** — `gallery::built::edge_normal`'s formula for a wall's outer face — so a positively
///   wound loop is one whose solid lies on the walker's **right** and whose outer faces therefore
///   point away from it. (The prose in `gallery::built` that puts the enclosed ground on the
///   walker's *left* reads true only when `z` is read as north, which is not this frame's
///   convention; the formula there is unambiguous and is what is kept.) Walked the other way every
///   wall's outer face looks inward, and a building whose faces are all back-facing is not drawn at
///   all: a missing building rather than a visible fault, so it is
///   [`validate`](crate::validate)'s business and not the expander's — a loop whose signed plan area
///   comes out negative is reported by name.
/// - **The wall is a ring, closed at both ends**: outer wall, inner wall, the ring of wall top, and
///   the ring's base, which stands on the outline's own elevations — so a mass on sloping ground is
///   authored by giving its outline nodes their own elevations, and the plate is **not** part of it:
///   the plate is a [`Slab`](crate::slab::Slab) of its own, and the floor is its top.
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
    /// different ways. [`validate`](crate::validate) reports the ones that break that.
    ///
    /// **A void is drawn as a ring of wall of its own**, standing on the plate: the loop is the courtyard's inner
    /// face and the same loop offset outward by [`wall`](Self::wall) is the face the rooms see, closed at the top
    /// and at the base exactly as the mass's own ring is closed. The plate is not cut by it — the courtyard's floor
    /// **is** the plate's own top — so nothing has to be split and the two rings never share an edge. The loop may
    /// have reflex corners: what it may not have is a **bite narrower than `2 × wall`**, because the ring stands
    /// [`wall`](Self::wall) out of the loop and so that far into each side of a bite. Expansion refuses the rest
    /// through [`expand::offset_loop`](crate::expand::offset_loop)'s own faults, and
    /// [`validate`](crate::validate) reports it as
    /// [`DidNotExpand`](crate::validate::Violation::DidNotExpand).
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
    /// a diagram: `up × along` is the walker's **left** in the `(x, z)` plan of `shape::math`
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

    /// Returns the lowest elevation the outline stands at: **the mass's own ground line**.
    ///
    /// # What this is for
    ///
    /// An outline carries an elevation per node because a mass may stand on a slope, and the
    /// minimum over those nodes is the one number the crate's readings are keyed to. Three of them
    /// ask for it: [`crate::derive::starter`] seats the ground plate's top on it, so the plate is at
    /// the ground rather than above it; [`crate::derive::floor_slab`] refuses any floor **at or
    /// under** it, which is what keeps a floor out of the ground; and [`crate::site::report`] prints
    /// it as a building's own lowest ground, beside the ground the whole site was seated on.
    ///
    /// # Invariants
    ///
    /// **One spelling of the ground line, on the type that owns it.** Those three callers each wrote
    /// this fold out in full while the accessor stood unused beside them, which is the shape a
    /// leftover usually has — except here the accessor was the survivor and the three spellings were
    /// the duplication. A fourth place for the mass's ground to be read differently is a fourth
    /// place a plate could be seated off the wall it stands on.
    ///
    /// An outline with no nodes answers [`f32::INFINITY`], which is what the fold it replaced
    /// answered and what every guard above then refuses.
    ///
    /// `O(nodes)` in the outline, one walk of nodes the mass already holds.
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
