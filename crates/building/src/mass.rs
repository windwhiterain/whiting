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

/// One horizontal section of a mass: the wall's outer face at one elevation of the shell.
///
/// # Invariants
///
/// **Every node carries its own elevation, and that is the section's height at that node** — so a mass may
/// stand on a slope (its foot section's nodes differ) and its top may be a gable (its last section's nodes
/// differ) with the same arithmetic. The elevations of one section need not be equal; what the level's own
/// [`Level::lowest`] and [`Level::highest`] answer is their range.
///
/// **The outline is wound so its plan area is positive**, exactly as [`Mass::plan_area`] states for the whole
/// mass: it is the face a viewer sees and the wall is built inward from it, so the winding is what makes every
/// expanded face point outward.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    /// The outer face of the wall at this level, stored open (the last-to-first edge is implied).
    ///
    /// # Invariants
    ///
    /// **The plan winds positively** — counter-clockwise seen from above — which is what makes every expanded face point
    /// outward. The winding depends on the direction the outline is *walked*, so the same four corners listed for a wall
    /// running east and one running west wind opposite ways: round 72 measured [`Problem::OffsetInverted`] with areas
    /// `-15.81` and `-7.82` for exactly that pair. The answer is to list the run once and let one helper decide the order, not
    /// to remember which way each run goes.
    ///
    /// [`Problem::OffsetInverted`]: crate::expand::Problem::OffsetInverted
    pub outline: Vec<PlanNode>,
}

impl Level {
    /// Builds a level from an outline. The nodes carry their own elevations.
    pub fn new(outline: Vec<PlanNode>) -> Self {
        Self { outline }
    }

    /// Builds a level as the same loop every node of which stands at one elevation.
    pub fn flat(outline: &[PlanNode], elevation: f32) -> Self {
        Self {
            outline: outline
                .iter()
                .map(|node| PlanNode::new(node.position, elevation))
                .collect(),
        }
    }

    /// Returns the lowest elevation this section stands at.
    pub fn lowest(&self) -> f32 {
        self.outline
            .iter()
            .map(|node| node.elevation)
            .fold(f32::INFINITY, f32::min)
    }

    /// Returns the highest elevation this section stands at.
    pub fn highest(&self) -> f32 {
        self.outline
            .iter()
            .map(|node| node.elevation)
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Returns the signed plan area the section encloses.
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

    /// Returns this section's nodes moved by a plan offset, elevations untouched.
    pub fn translated(&self, offset: Vec2) -> Self {
        Self {
            outline: self
                .outline
                .iter()
                .map(|node| {
                    PlanNode::new(
                        [node.position[0] + offset[0], node.position[1] + offset[1]],
                        node.elevation,
                    )
                })
                .collect(),
        }
    }
}

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
/// How a mass is joined to the host it names.
///
/// # Invariants
///
/// **The relation is authored, and it is what makes two masses one building rather than a pile of boxes.**
/// A mass whose plan overlaps another's with no relation between them is reported by
/// [`validate`](crate::validate) and nothing repairs it. What a relation buys is not an exemption from that
/// reading but a *stronger* one: each kind names the geometry it claims, and `validate` checks the claim.
///
/// - a **wing** lands on the host's wall at an arc-length station, and the host's outline is split there
///   by [`ops::attach_mass`](crate::ops::attach_mass), because an outline turns only at a node and a wing
///   joining the middle of a wall has to land on a node that exists;
/// - a **storey** stands on the host's top, concentrically and inside its plan, and shares the host's plan
///   area by standing above it rather than beside it. That is the only form in which two plans may overlap
///   without either being a wing, and it is checked: the storey's own base must be the host's top and its
///   outline must be inside the host's, or `validate` names the mass.
///
/// A storey is what a drum tower's seven floors are, and a wing is what its gallery is; without the two
/// kinds one of them would have to be spelled as the other, which is a lie the validator would then have to
/// accept unread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Join {
    /// A wing on the host's wall: the station is an arc-length parameter of the host's closed outline, in
    /// `[0, 1)`, read the way a station is read everywhere else in this workspace.
    Wing {
        /// Where on the host's outline the wing lands.
        station: f32,
    },
    /// A storey standing on the host's top: same plan centre, base at the host's top, inside its plan.
    Storey,
    /// A mass standing **under** the host and carrying it: the host's foot is this mass's top, and this mass's plan
    /// is inside the host's.
    ///
    /// # Invariants
    ///
    /// **It is the mirror of [`Storey`](Self::Storey) and it is a relation rather than an overlap.** A bridge's
    /// masonry pier and the timber gallery over it share a plan for as long as the bridge stands; without a word for
    /// that, `validate`'s overlap question reports the pair as two masses in each other's way — which is what
    /// happened to this bridge's piers before the variant existed. The two claims are the ones a storey makes, read
    /// the other way round: the touching elevations are the carried mass's foot on the carrier's top, and the
    /// contained plan is the carrier's inside the carried.
    Under,
}

/// The host a mass is joined to, and how.
///
/// # Invariants
///
/// **One relation per mass**, and it is authored rather than derived: a mass that names a host is joined to
/// it, and the geometry the relation claims is checked by [`validate`](crate::validate) — a wing's station
/// must be on the host's outline, a storey's base must be the host's top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attachment {
    /// The mass this one lands on.
    pub host: MassId,
    /// How it is joined to that host.
    pub join: Join,
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
    /// The shell's horizontal sections, **bottom first, at least two**: the first is the mass's foot and the
    /// last is its top.
    ///
    /// # Invariants
    ///
    /// **A mass is a stack of sections and not a prism with a crown.** The wall between two consecutive sections
    /// is the band between their loops, so:
    ///
    /// - sections of equal plan give a vertical wall, which is every plain mass;
    /// - an upper section **inside** the one below gives a wall that leans in — 收分, the battered earth wall;
    /// - an upper section **outside** the one below gives a wall that leans out **and a soffit**, the horizontal
    ///   face under the projecting floor — 挑出, the cantilevered gallery.
    ///
    /// The top section's own nodes carry the wall's top elevations, so a gable that rises to its roof, a parapet
    /// and a stepped wall are all one section whose nodes differ — there is no separate crown list and no way for
    /// the two to drift.
    pub levels: Vec<Level>,
    /// Wall thickness, measured inward from the outline.
    ///
    /// # Invariants
    ///
    /// **It must be less than half the plan's narrowest extent**, because the inner loop *is* the outline offset inward by
    /// this amount and two such offsets meet in the middle: a mass 0.9 m deep with a 0.55 m wall folds its inner loop
    /// through itself, and the expansion answers [`Problem::OffsetEdgeFolded`] and then [`Problem::OffsetInverted`]. Round
    /// 70 lost a settlement to exactly that, and round 72 to a pier that was as thick as it was wide. A wall that *is* the
    /// mass's thickness has no inside at all, which is a projection and not a wall.
    ///
    /// [`Problem::OffsetEdgeFolded`]: crate::expand::Problem::OffsetEdgeFolded
    /// [`Problem::OffsetInverted`]: crate::expand::Problem::OffsetInverted
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
    /// through `crate::expand::offset_loop`'s own faults, and
    /// [`validate`](crate::validate) reports it as
    /// [`DidNotExpand`](crate::validate::Violation::DidNotExpand).
    pub voids: Vec<Vec<PlanNode>>,
}

impl Mass {
    /// Builds a mass with walls and a floor slab, standing on its outline and rising to `top`.
    ///
    /// **Two sections**: the foot, whose nodes carry the ground the mass stands on, and the top, every node of
    /// which is at `top`. A mass with more sections — a battered wall, a projecting floor — is authored by
    /// [inserting them](Mass::with_level) between the two.
    pub fn new(outline: Vec<PlanNode>, top: f32) -> Self {
        let top_level = Level::flat(&outline, top);
        Self {
            levels: vec![Level::new(outline), top_level],
            wall: 0.3,
            slab: 0.4,
            attachment: None,
            name: None,
            voids: Vec::new(),
        }
    }

    /// Builds a mass **from its sections**, bottom first, at least two.
    ///
    /// This is what a multi-section mass is authored with: a battered pier, a projecting floor's wall, an earth
    /// wall that leans in. [`Mass::new`] is the two-section case and is written in terms of this one's shape.
    pub fn of_sections(levels: Vec<Level>) -> Self {
        assert!(
            levels.len() >= 2,
            "a mass is at least a foot and a top; {} section(s) is not a mass",
            levels.len()
        );
        Self {
            levels,
            wall: 0.3,
            slab: 0.4,
            attachment: None,
            name: None,
            voids: Vec::new(),
        }
    }

    /// Sets the wall top over chosen nodes: one elevation per outline node.
    ///
    /// A crown shorter than the outline leaves the remaining nodes at the level top, which is what the accessor
    /// it replaces did; a crown longer than the outline is **cut** rather than indexed, because the top section's
    /// nodes are the only ones a wall top can belong to.
    pub fn with_crowns(mut self, crowns: Vec<f32>) -> Self {
        if let Some(top) = self.levels.last_mut() {
            for (node, crown) in top.outline.iter_mut().zip(crowns) {
                node.elevation = crown;
            }
        }
        self
    }

    /// Inserts a section **between the ones it belongs between**, by its own elevation.
    ///
    /// A section above every other becomes the new top and a section below every other becomes the new foot, so
    /// the stack cannot be authored out of order and the two ends stay the ends.
    pub fn with_level(mut self, level: Level) -> Self {
        let at = self
            .levels
            .iter()
            .position(|existing| level.lowest() < existing.lowest())
            .unwrap_or(self.levels.len());
        self.levels.insert(at, level);
        self
    }

    /// Returns the mass's foot: the section it stands on.
    pub fn foot(&self) -> &Level {
        &self.levels[0]
    }

    /// Returns the mass's top: the section its wall ends at.
    pub fn crown(&self) -> &Level {
        self.levels
            .last()
            .expect("a mass always has a foot and a top")
    }

    /// Returns the mass's outline: its foot section's loop, which is the loop in plan.
    pub fn outline(&self) -> &[PlanNode] {
        &self.foot().outline
    }

    /// Returns the mass's outline for writing.
    pub fn outline_mut(&mut self) -> &mut Vec<PlanNode> {
        &mut self.levels[0].outline
    }

    /// Returns the wall top over one node of the top section.
    pub fn top_at(&self, node: usize) -> f32 {
        self.crown()
            .outline
            .get(node)
            .map_or_else(|| self.highest_top(), |at| at.elevation)
    }

    /// Returns the highest wall top the mass has, which is the highest node of its top section.
    pub fn highest_top(&self) -> f32 {
        self.crown().highest()
    }

    /// Returns the lowest wall top the mass has, which is what a floor must stay under.
    pub fn lowest_top(&self) -> f32 {
        self.crown().lowest()
    }

    /// Returns whether the mass's top is level: every node of the top section at one elevation.
    ///
    /// A crowned edge is not a rectangle and therefore carries no grid, and this is the one reading of that.
    pub fn top_is_level(&self) -> bool {
        let top = self.crown();
        top.highest() - top.lowest() <= f32::EPSILON * 8.0
    }

    /// Returns the top section's lowest elevation, which is the level a wall top is written against.
    pub fn crown_top(&self) -> f32 {
        self.crown().lowest()
    }

    /// Returns one elevation per outline node for the top section: the crown the expander reads.
    pub fn crown_elevations(&self) -> Vec<f32> {
        self.crown()
            .outline
            .iter()
            .map(|node| node.elevation)
            .collect()
    }

    /// Returns the mass's top section for writing: the wall top's own elevations.
    pub fn crown_mut(&mut self) -> &mut Level {
        self.levels
            .last_mut()
            .expect("a mass always has a foot and a top")
    }

    /// Sets the wall top: **every node of the top section** at `top`, leaving the lower sections alone.
    ///
    /// That is what "set the top" means once a mass is a stack — a building whose wall rose to `top` still rises
    /// to it, and the sections under it are not the wall's top.
    pub fn set_top(&mut self, top: f32) {
        for node in &mut self.crown_mut().outline {
            node.elevation = top;
        }
    }

    /// Moves every section up or down by `delta`: the whole shell, foot included.
    ///
    /// This is what a seat does, and doing it per section rather than per field is why a stack cannot be left
    /// half-moved.
    pub fn shift(&mut self, delta: f32) {
        for level in &mut self.levels {
            for node in &mut level.outline {
                node.elevation += delta;
            }
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
        self.foot().plan_area()
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
        self.foot().lowest()
    }

    /// Returns the highest elevation the outline stands at.
    pub fn highest(&self) -> f32 {
        self.foot().highest()
    }

    /// Returns how many edges the closed outline has.
    pub fn edge_count(&self) -> usize {
        self.foot().outline.len()
    }
}
