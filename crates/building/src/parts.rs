//! The parts a building is furnished with: templates, and the places they stand.
//!
//! A grid says where a bay is, an opening says the wall is cut there, and a part says **what stands
//! in it** — a door leaf, a window frame, a stair block, a canopy. The three are different statements
//! and they compose: an opening is a hole, a part is a body, and a body in a hole is a door.
//!
//! # The two ideas, here
//!
//! **A template is a small skeleton, so everything that builds a building builds a part.** It holds
//! masses and roofs — the same two types the rest of the crate authors — in its own local frame, and
//! [`Template::skeleton`] hands them back as an ordinary [`BuildingSkeleton`] to expand. Nothing about
//! a part is a new kind of geometry: it is closed, welded and validated by the code that already does
//! that, and the two-level watertightness rule applies to it exactly as it does to a mass.
//!
//! **A placement names its bay by slot and pins the template's own origin in it.** That is D10's rule
//! again — a re-split carries the part, a slot that vanished is reported — and the pin is two
//! distances from the bay's own lower-left corner rather than a position in the world, so the same
//! placement lands correctly on a 7 m wall and on an 11 m one.
//!
//! # The frame a placement maps into
//!
//! A template is authored in its own frame and placed into a wall's:
//!
//! - the template's **`x`** runs **along** the edge, from the pin towards the far end of the bay;
//! - its **`y`** runs **up**, from the pin towards the wall's top;
//! - its **`z`** runs **across the face the bay names**, which is `x × y` — a right-handed frame, so an
//!   author who writes a part the way they write a building does not have to think about a reflection.
//!
//! A door leaf is therefore authored with its thickness along `z` and its width along `x`: it stands
//! in the hole, its face flush with the face the bay names when the pin says so.
//!
//! **The third bullet is the one that varies, and it varies because the face does.** A mass's bay is
//! measured on its outline and the outline is the wall's *outer* face, so a part's `z` runs from there
//! into the masonry. A courtyard's bay is measured on the void loop and that loop is the face which
//! looks *into* the courtyard, so the same part's `z` runs out of that face and into the courtyard
//! instead — flush with the same face, standing on the same wall, and turned to look at the person who
//! is standing in the courtyard. The frame is right-handed either way, which is the invariant worth
//! keeping: a frame that could be reproduced by a yaw about `up` is a frame a scene's instance
//! transform can carry, and a frame that cannot is one whose baked and instanced geometry would
//! disagree. See [`expand::part_frame`](crate::expand::part_frame) for the measurement that settled
//! the alternative, and [`PartFrame`] for what a caller may do with the answer.

use crate::grid::FaceRef;
use crate::mass::Mass;
use crate::math::Vec3;
use crate::mesh::BuildingMesh;
use crate::roof::Roof;
use crate::skeleton::BuildingSkeleton;
use crate::wall::WallId;

/// A named mount point on a template: where the part touches what it hangs from.
///
/// # Invariants
///
/// **A socket is a point, not a frame, and that is the whole of the simplification.** Every part in
/// this crate is authored in the placement's frame — `x` along the edge, `y` up, `z` into the wall — so
/// a joint needs no rotation to mate: the part keeps its authoring orientation and only its origin is
/// moved, so a socket point lands on an anchor point. A joint that could turn a part would need a
/// basis, a handedness and a sign convention, and each of those is a way for a canopy to hang upside
/// down; this slice does not have that problem because it cannot express the turn.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Socket {
    /// What the mount point is called: `"head"`, `"back"`, `"foot"`.
    pub name: String,
    /// Where it stands in the template's own frame, across and up.
    pub at: (f32, f32),
}

impl Socket {
    /// Builds a socket.
    pub fn new(name: impl Into<String>, at: (f32, f32)) -> Self {
        Self {
            name: name.into(),
            at,
        }
    }
}

/// A named place on the building a part can hang from.
///
/// # Invariants
///
/// It names a **feature of the document**, not a number: an opening's head or sill, a bay's own edges.
/// That is what makes a joint follow an edit — moving a door's head moves every part hung on it, which
/// a pin written in metres cannot do and cannot even notice. The anchor is resolved in the bay's own
/// frame (along the edge, up from its sill) by [`BuildingSkeleton::anchor`], so a joint is as
/// slot-carried as the bay itself.
///
/// [`BuildingSkeleton::anchor`]: crate::skeleton::BuildingSkeleton::anchor
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Anchor {
    /// The head of one of the wall's openings, at its own middle across.
    OpeningHead {
        /// The opening, by its index in the document's openings.
        opening: usize,
    },
    /// The sill of one of the wall's openings, at its own middle across.
    OpeningSill {
        /// The opening.
        opening: usize,
    },
    /// The middle of a bay's own sill.
    BaySill,
    /// The middle of a bay's own head.
    BayHead,
    /// The start of a bay along the edge, at the middle of its height.
    BayStart,
    /// The end of a bay along the edge, at the middle of its height.
    BayEnd,
}

/// How a placement is positioned in its bay.
///
/// # Invariants
///
/// Exactly one of the two, and a joint wins when both are given: a placement that names a socket is
/// **mounted**, and its pin is then only what a free placement would have used. This is the shape that
/// keeps [`Placement::reach`](Placement::reach) and the expansion reading one number — the pin the
/// joint resolves to — rather than each deciding for itself whether a part is mounted.
#[derive(Debug, Clone, PartialEq)]
pub struct Joint {
    /// The template's socket, by name.
    pub socket: String,
    /// The place on the building it hangs from.
    pub anchor: Anchor,
}

impl Joint {
    /// Builds a joint.
    pub fn new(socket: impl Into<String>, anchor: Anchor) -> Self {
        Self {
            socket: socket.into(),
            anchor,
        }
    }
}

/// Where a placement stands its template: the frame the template's own coordinates are read in.
///
/// # Invariants
///
/// **The same four directions the expansion uses, named once.** A scene that draws one template mesh with
/// *n* of these draws exactly the geometry `build` measured, because `build` is written on the same two
/// pieces — there is no second frame convention and no matrix: the crate's own vocabulary is an origin and
/// three unit directions, and [`apply`](Self::apply) is one dot product per direction.
///
/// The directions are `along` the wall's edge, `up`, and `inward` across the face the bay names, in
/// the order the template authored `x`, `y` and `z` — which is what
/// [`Template`]'s documentation means by "authored in the placement's frame".
///
/// **`inward` is always the walker's right, so `along × up` is `inward` on every face this crate
/// has**, a mass's outer wall and a courtyard's inner one alike. That is what makes the frame a
/// rotation rather than a reflection, and it is the property a caller can rely on: the map is
/// `origin + x · along + y · up + z · inward`, so the sign of `(along × up) · inward` is `+1` and a
/// yaw about `up` plus a translation is the whole transform. Measured on a part in a mass's bay and
/// on the same part in a courtyard's bay, both `+1.0` — see `examples/patio_probe.rs`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartFrame {
    /// Where the template's own origin lands, in world space.
    pub origin: Vec3,
    /// The world direction the template's `x` runs along.
    pub along: Vec3,
    /// The world direction its `y` runs along.
    pub up: Vec3,
    /// The world direction its `z` runs along, which is into the wall.
    pub inward: Vec3,
}

impl PartFrame {
    /// Builds a frame.
    pub fn new(origin: Vec3, along: Vec3, up: Vec3, inward: Vec3) -> Self {
        Self {
            origin,
            along,
            up,
            inward,
        }
    }

    /// Maps a point of the template's own frame into the world.
    ///
    /// `origin + x · along + y · up + z · inward`, which is the whole of the transform: no rotation, no
    /// scale, no matrix — the template is authored in the frame it is stood in.
    pub fn apply(&self, point: Vec3) -> Vec3 {
        let mut moved = self.origin;
        for (component, direction) in [(0, self.along), (1, self.up), (2, self.inward)] {
            for axis in 0..3 {
                moved[axis] += point[component] * direction[axis];
            }
        }
        moved
    }
}

/// Where a placement pins a template's origin, measured from its bay's own corner.
///
/// # Invariants
///
/// Both are distances **into** the bay and neither may be negative: a part whose pin is off its bay is
/// not a different part, it is a fault ([`Violation::PlacementOutsideBay`]), and the arithmetic is
/// arranged so that it cannot read as a smaller one. They are metres along the edge and metres up,
/// which is what a builder sets out with a tape rather than with a parameter.
///
/// [`Violation::PlacementOutsideBay`]: crate::validate::Violation::PlacementOutsideBay
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pin {
    /// From the bay's start along the edge.
    pub along: f32,
    /// Up from the bay's own sill.
    pub up: f32,
}

impl Pin {
    /// Builds a pin.
    pub fn new(along: f32, up: f32) -> Self {
        Self { along, up }
    }
}

/// A box in a template's own frame: what a template occupies, for the one question that needs it.
///
/// # Invariants
///
/// It is an **axis-aligned bound and not the shape**, computed from the template's own masses: their
/// outlines' plan extents, and their floors and tops. That is enough for the only thing this crate
/// asks of it — whether a placement fits in its bay — and it is deliberately not a claim about the
/// solid, which may be a ring, an L, or a frame with a hole in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The lowest and highest `x` the template's masses reach.
    pub x: (f32, f32),
    /// The lowest and highest `y`.
    pub y: (f32, f32),
    /// The lowest and highest `z`.
    pub z: (f32, f32),
}

/// A small authored skeleton in its own frame: a door, a frame, a stair, a canopy.
///
/// # Invariants
///
/// - The masses and roofs are **ordinary** ones, in local coordinates, with no attachment to anything
///   outside the template. A template that names a host it does not hold is the same fault as a mass
///   that does (and [`Building::validate`](crate::Building::validate) on [`Template::skeleton`]
///   says so), which is why a template is authored as a building rather than as a special case.
/// - Its name is its identity in a report and in a scene, and nothing keys off it: two templates may
///   share a name and are still two templates, so an author who renames one does not silently move
///   every placement of it. Placements name a template **by index** for that reason.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Template {
    /// What the template is called, for a report or a scene to show.
    pub name: String,
    /// Its masses, in its own frame.
    pub masses: Vec<Mass>,
    /// Its roofs, in its own frame.
    pub roofs: Vec<Roof>,
    /// Its mount points, which a placement can hang the part from.
    pub sockets: Vec<Socket>,
}

impl Template {
    /// Builds a template from its parts, with no socket to hang it from.
    pub fn new(name: impl Into<String>, masses: Vec<Mass>, roofs: Vec<Roof>) -> Self {
        Self {
            name: name.into(),
            masses,
            roofs,
            sockets: Vec::new(),
        }
    }

    /// Adds a socket, for a placement to name.
    pub fn with_socket(mut self, socket: Socket) -> Self {
        self.sockets.push(socket);
        self
    }

    /// Returns one socket by name.
    ///
    /// `O(sockets)`, and a template has a handful: a name is what an author writes in the document, so
    /// looking it up by name is the shape the document uses rather than an index nobody reads.
    pub fn socket(&self, name: &str) -> Option<&Socket> {
        self.sockets.iter().find(|socket| socket.name == name)
    }

    /// Expands the template **in its own frame**, once, as an ordinary mesh.
    ///
    /// # Invariants
    ///
    /// **It is a function of the template alone**, so one call answers for every placement of it: a scene
    /// draws this mesh with a [`PartFrame`] per placement and draws exactly what [`build`] measured,
    /// because `build` is written on the same two pieces. A template that cannot stand is a list of
    /// problems — the same refusal the expansion gives — rather than a mesh that is nearly right.
    ///
    /// [`build`]: crate::expand::build
    pub fn mesh(&self) -> Result<BuildingMesh, Vec<crate::expand::Problem>> {
        crate::expand::build(&self.skeleton()).map_err(|error| error.problems)
    }

    /// Returns the template as the skeleton it is, ready to expand.
    ///
    /// # Invariants
    ///
    /// **A part is built by the code that builds a building**, and this is the whole of that claim:
    /// the answer is an ordinary [`BuildingSkeleton`] whose masses are already in the template's own
    /// coordinates, so `Building::new(template.skeleton()).build()` and `validate()` apply unchanged.
    /// The caller places the result; the template does not know where it will stand.
    pub fn skeleton(&self) -> BuildingSkeleton {
        BuildingSkeleton::new(self.masses.clone(), self.roofs.clone())
    }

    /// Returns the box the template's own masses occupy, or `None` when it holds no mass.
    ///
    /// `O(nodes of its masses)`, once, and it deliberately ignores the roofs: a roof sits on the
    /// masses it was derived from, so the masses' box already contains what a bay has to hold.
    pub fn bounds(&self) -> Option<Bounds> {
        let mut bounds: Option<Bounds> = None;
        for mass in &self.masses {
            let mut reach = Bounds {
                x: (f32::INFINITY, f32::NEG_INFINITY),
                y: (mass.top, mass.top),
                z: (f32::INFINITY, f32::NEG_INFINITY),
            };
            for node in &mass.outline {
                reach.x.0 = reach.x.0.min(node.position[0]);
                reach.x.1 = reach.x.1.max(node.position[0]);
                reach.z.0 = reach.z.0.min(node.position[1]);
                reach.z.1 = reach.z.1.max(node.position[1]);
                reach.y.0 = reach.y.0.min(node.elevation - mass.slab);
            }
            bounds = Some(match bounds {
                None => reach,
                Some(held) => Bounds {
                    x: (held.x.0.min(reach.x.0), held.x.1.max(reach.x.1)),
                    y: (held.y.0.min(reach.y.0), held.y.1.max(reach.y.1)),
                    z: (held.z.0.min(reach.z.0), held.z.1.max(reach.z.1)),
                },
            });
        }
        bounds
    }
}

/// What a placement stands its template on.
///
/// # Invariants
///
/// **A target is a place the document already has** — a bay of a wall of a mass, or a wall run's own line —
/// so every part stands on something an author authored rather than at a coordinate. The two differ in the
/// one fact everything downstream reads: a bay is a rectangle with a width and a height, and a wall run is a
/// line with a length and a top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// A bay of a wall of a mass. The placement's frame is the wall's edge frame at the bay's own slot.
    Bay {
        /// Which wall of which mass.
        face: FaceRef,
        /// Which storey, counting from the face's base.
        row: usize,
        /// Which bay across, counting from the edge's start.
        column: usize,
    },
    /// A wall run's own line. `at.along` is the arc length from the run's start and `at.up` the height above
    /// the ground at `from`.
    ///
    /// **A part may reach past its wall**, and the crate says so rather than forbidding it: a gate leaf hangs
    /// from the post at the end of one run and closes the gap beyond it, and a sign hangs off a wall's middle
    /// and sticks out. What is checked is that the **pin** is on the run — a part hung where the wall is not
    /// is a part floating — and nothing about where the part's own geometry reaches, because that is what the
    /// template was authored to do.
    Wall {
        /// Which run.
        wall: WallId,
    },
}

/// One template standing in one bay or on one wall run.
///
/// # Invariants
///
/// It names its template **by index** into the document's list and what it stands on **by target**, and it
/// holds no position in the world: [`Pin`] is measured from the target's own corner, and the target is read
/// out of the document when the part is expanded or checked. So a grid that is re-split moves the parts that
/// hang in it, a slot that vanishes is reported rather than guessed at, and the document never holds two
/// statements of where the same part is.
///
/// **The pin is in the target's own frame**, so `at` means the same thing for both: the arc length across a
/// bay (from its start) and the height above its sill, or the arc length along a run (from its start) and the
/// height above the ground at `from`. `at` is ignored when `joint` is set, because a joint resolves the origin
/// from the document instead — see [`Joint`].
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Which template, by index in the document's list.
    pub template: usize,
    /// What it stands on.
    pub target: Target,
    /// Where the template's own origin is pinned, in the target's frame. Ignored when `joint` is set.
    pub at: Pin,
    /// What the part hangs from, when it is mounted rather than merely placed.
    pub joint: Option<Joint>,
}

impl Placement {
    /// Builds a free placement: a part pinned in a bay.
    pub fn new(template: usize, face: FaceRef, row: usize, column: usize, at: Pin) -> Self {
        Self {
            template,
            target: Target::Bay { face, row, column },
            at,
            joint: None,
        }
    }

    /// Builds a placement on a wall run: a part pinned along the wall's own line.
    ///
    /// # Invariants
    ///
    /// It is the shape a gate or a lamp needs — something hung on a wall rather than fitted into a bay — and
    /// `at.along` is measured **from the run's start**, so a part hung at the end of a wall is written as the
    /// wall's own length.
    pub fn on_wall(template: usize, wall: WallId, at: Pin) -> Self {
        Self {
            template,
            target: Target::Wall { wall },
            at,
            joint: None,
        }
    }

    /// Hangs the part from a socket and an anchor instead of a pin.
    pub fn with_joint(mut self, joint: Joint) -> Self {
        self.joint = Some(joint);
        self
    }

    /// Returns the box this placement occupies in its bay's own frame, or `None` when the template
    /// holds no mass.
    ///
    /// # Invariants
    ///
    /// The answer is in the bay's coordinates — `x` from the bay's start along the edge and `y` from
    /// its sill — so the containment check compares two intervals and needs no frame transform, and
    /// the same placement reads correctly on walls of different lengths. The bay is taken as an
    /// argument for the caller's convenience in naming it and is deliberately **not** subtracted from
    /// the pin, because a pin is measured from that very corner. `O(1)` after [`Template::bounds`].
    ///
    /// A template's own frame has its **origin at its floor**, which is what makes a pin mean what an
    /// author expects: authoring a part's outline at the elevation of its own slab puts its bounds at
    /// `0`, and `Pin::new(along, 0.0)` then stands its underside on the bay's own sill rather than a
    /// slab below or above it.
    pub fn reach(&self, template: &Template, at: Pin) -> Option<Bounds> {
        let bounds = template.bounds()?;
        Some(Bounds {
            x: (at.along + bounds.x.0, at.along + bounds.x.1),
            y: (at.up + bounds.y.0, at.up + bounds.y.1),
            z: bounds.z,
        })
    }
}

/// The templates a document authors, and the places they stand.
///
/// # Invariants
///
/// Templates are a **list indexed by placements** and placements are a **list in author order**, for
/// the reasons each type gives: a placement names its template by index, and the expansion names a
/// part's solid by the placement's index. Removing a template therefore shifts every later index, and
/// the verb that does it says so — the same asymmetry [`crate::ops::remove_opening`] documents.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parts {
    templates: Vec<Template>,
    placements: Vec<Placement>,
}

impl Parts {
    /// Builds an empty set of parts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a template and returns the index it was given.
    pub fn add_template(&mut self, template: Template) -> usize {
        self.templates.push(template);
        self.templates.len() - 1
    }

    /// Returns one template by index.
    pub fn template(&self, index: usize) -> Option<&Template> {
        self.templates.get(index)
    }

    /// Returns every template, in the order they were authored.
    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    /// Places a template in a bay and returns the placement's index.
    pub fn place(&mut self, placement: Placement) -> usize {
        self.placements.push(placement);
        self.placements.len() - 1
    }

    /// Removes one placement by index, returning it.
    pub fn remove(&mut self, index: usize) -> Option<Placement> {
        (index < self.placements.len()).then(|| self.placements.remove(index))
    }

    /// Returns one placement by index.
    pub fn placement(&self, index: usize) -> Option<&Placement> {
        self.placements.get(index)
    }

    /// Returns every placement, in the order they were authored.
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    /// Returns the index of every placement in one wall, in author order.
    ///
    /// # Invariants
    ///
    /// This is the lookup the expander walks, and it is `O(placements)` once per wall asked about
    /// rather than a scan per bay. The order is the author's, so two runs of an unchanged document
    /// build the same mesh.
    pub fn of_face(&self, face: FaceRef) -> Vec<usize> {
        self.placements
            .iter()
            .enumerate()
            .filter(|(_, placement)| {
                matches!(placement.target, Target::Bay { face: held, .. } if held == face)
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// Returns how many templates the document holds.
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Returns how many placements the document holds.
    pub fn len(&self) -> usize {
        self.placements.len()
    }

    /// Returns whether the document holds no placement.
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }
}
