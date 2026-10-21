//! The expanded mesh: closed planar-facet solids, every face naming the element that made it.
//!
//! # Invariants
//!
//! - **A face is wound so that its right-handed normal points out of the solid.** Unlike
//!   `mountain`'s mesh — a height field over the plan, whose faces all look up and which therefore
//!   negates the right-handed normal — a building has vertical walls, and "up" is not a direction
//!   a wall's outer face can be described by. The contract here is `cross(b - a, c - a)` pointing
//!   away from the solid's interior, and it is enforced globally by
//!   [`signed_volume_of`](BuildingMesh::signed_volume_of) being **positive**: an inside-out
//!   building is a named violation rather than a frame nobody can explain
//!   (`gallery::built` records the same failure as "a wall that is not drawn").
//! - **Points are welded by authored identity, never by coordinate.** Two faces built from one
//!   authored corner share one point index, which is what lets a boundary be counted at all: a
//!   mesh of loose facets has no edges to be manifold about.
//! - **A face's [`Element`] is also its shading group.** One authored surface — one wall face, one
//!   roof plane — is one planar surface, so its facets are smooth across and hard at its edges,
//!   and that is a structural fact here rather than a smoothing pass downstream.

use std::collections::HashMap;

use crate::mass::MassId;
use crate::math::{Vec3, cross3, dot3, subtract};
use crate::roof::RoofId;

/// Which surface of a mass a face belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MassSurface {
    /// The face seen from outside, one per outline edge **per band**.
    ///
    /// # Invariants
    ///
    /// **The band is part of the identity of the face and not a detail of it.** A mass with several sections has
    /// one strip of wall between each pair, and the strips are coplanar only while the sections' plans are equal;
    /// an element is a *planar* shading group, so two leaning strips in one element would be reported as a
    /// non-planar surface by the very check that exists to catch a real one. Naming the band keeps every strip its
    /// own group, and `band: 0` is the foot's.
    OuterWall {
        /// Index of the outline edge.
        edge: usize,
        /// Which pair of sections the strip is between: `0` is the foot's.
        band: usize,
    },
    /// The face seen from inside, one per outline edge per band.
    InnerWall {
        /// Index of the outline edge.
        edge: usize,
        /// Which pair of sections the strip is between: `0` is the foot's.
        band: usize,
    },
    /// The ring of wall top between the outer and inner faces, one per outline edge.
    ///
    /// # Invariants
    ///
    /// **A strip is one plane when the wall's top is level and two when it is crowned at one end.** The
    /// strip's four corners are the edge's two outer nodes and its two inner, mitered ones; at one
    /// elevation any three of them span a plane the fourth lies in, but a crowned strip lies in a plane
    /// only if the two ends' offset directions are parallel — and a corner's miter is never parallel to
    /// a straight node's perpendicular. So the strip **warps**, and since an element is a *planar*
    /// shading group — the rule the roof's profile was built on one arc earlier — the two triangles it
    /// has always been are given two elements instead of one. `half` is which: `0` is the first
    /// triangle and the whole strip when it is planar, `1` is the second triangle of a warped one.
    WallTop {
        /// Index of the outline edge.
        edge: usize,
        /// Which pair of sections the strip is between: `0` is the foot's.
        band: usize,
        /// Which triangle of the strip: `0`, or `1` for the second half of a warped strip.
        half: usize,
    },
    /// A surface of the wall's own thickness inside an opening: a jamb, a head or a sill.
    ///
    /// # Invariants
    ///
    /// One element per side per hole, because each side is its own plane and its own shading group —
    /// a reveal is masonry seen at ninety degrees to the wall it pierces, and smoothing a normal
    /// across that corner would round off the one edge that says how thick the wall is. `opening` is
    /// the hole's index in the document's own list ([`Openings`](crate::openings::Openings)), so a
    /// face traces back to the opening an author wrote and not merely to the wall it is in.
    ///
    /// **It carries the loop and not the edge alone, because a courtyard's wall is cut through the
    /// same way and is a solid of its own.** A reveal in a courtyard's wall belongs to that ring
    /// ([`SolidId::Void`]) and not to the mass that authored the void, so gathering the faces by
    /// element without it would put a ring's jambs in the mass's face set — two disjoint rings in one
    /// solid, whose Euler characteristic would then be the sum of two numbers that each mean nothing.
    Reveal {
        /// Which authored loop the hole is cut into: the mass's own outline, or one of its courtyards.
        loop_of: crate::mass::AuthoredLoop,
        /// Index of the loop edge the hole is cut into.
        edge: usize,
        /// Index of the opening in the document's list.
        opening: usize,
        /// Which side of the reveal.
        side: crate::openings::RevealSide,
    },
    /// The wall ring's **base**: the annulus between the outline and the inner loop, at the plate's top plane.
    ///
    /// # Invariants
    ///
    /// **It exists because a mass is becoming a volume, and a volume closes itself.** Once the plate is its own solid
    /// ([`Slab`](crate::mesh::Element::Slab)), the mass's faces are the wall ring and nothing else, and a ring is open
    /// at both ends: the top is [`WallTop`](Self::WallTop) and the base is this. It is called a base rather than a
    /// floor because the floor is the plate's own top face and the two are different elements of different solids —
    /// **and they are the same plane, not two planes apart**: the plate spans one thickness *below* the outline's own
    /// elevation up to it (`expand`'s ring walk builds the base at that elevation on both loops, for exactly this
    /// reason), so the ring stands on the plate rather than reaching into it. The ring's base is the seam between two
    /// solids and the plate's top is the floor a person walks on.
    WallBase,
    /// The face of a courtyard wall seen from **inside the courtyard**, one per void edge.
    ///
    /// # Invariants
    ///
    /// This is the **inner** loop of a courtyard's ring and it faces *into* the courtyard, so it is
    /// the same face as [`InnerWall`](Self::InnerWall) is for the mass's own ring — the wall's face
    /// turned towards the open space inside it. It belongs to a solid of its own
    /// ([`SolidId::Void`]), because a ring standing in the middle of a mass is a second ring and a
    /// single solid cannot be two of them.
    VoidWall {
        /// Which of the mass's voids, in the order it authored them.
        void: usize,
        /// Index of the void loop's edge.
        edge: usize,
    },
    /// The face of a courtyard wall seen from **the rooms**, one per void edge.
    ///
    /// This is the **outer** loop of the same ring, standing `wall` metres out from
    /// [`VoidWall`](Self::VoidWall), and it faces away from the courtyard into the rooms it
    /// encloses — the same face as [`OuterWall`](Self::OuterWall) is for the mass's own ring.
    VoidInner {
        /// Which of the mass's voids.
        void: usize,
        /// Index of the void loop's edge.
        edge: usize,
    },
    /// The strip of wall top between a courtyard's two loops, one per void edge.
    ///
    /// # Invariants
    ///
    /// **`half` is the same reading [`WallTop`](Self::WallTop) carries, and for the same reason**: a court
    /// under a crowned wall has a strip whose two ends stand at different elevations, so the strip warps
    /// and its two triangles are two planes — which means two elements, because an element is a *planar*
    /// shading group. `0` is the first triangle and the whole strip when it is planar, `1` is the second
    /// triangle of a warped one.
    VoidTop {
        /// Which of the mass's voids.
        void: usize,
        /// Index of the void loop's edge.
        edge: usize,
        /// Which triangle of the strip: `0`, or `1` for the second half of a warped strip.
        half: usize,
    },
    /// The annulus between a courtyard's two loops, at the void loop's own elevations.
    ///
    /// # Invariants
    ///
    /// **It is [`WallBase`](Self::WallBase)'s counterpart and not a face of the mass's own**, for the
    /// reason the mass's is its own: the plate is one solid under both the rooms and the courtyard,
    /// so the courtyard's wall stands *on* it and closes itself at its own base, and that base is a
    /// ring of its own annulus. Drawing it as a face of the mass's base would put the mass's solid
    /// and the courtyard's on one face set, which is two rings where the closure checks read one.
    VoidBase {
        /// Which of the mass's voids.
        void: usize,
    },
}

impl MassSurface {
    /// Returns the solid this surface is a face of.
    ///
    /// # Invariants
    ///
    /// **A courtyard's four surfaces and every reveal in its wall are one solid, and the mass's own
    /// are another**, which is the whole reason [`SolidId::Void`] exists: the faces are gathered by
    /// element, so an element that named the mass would gather the courtyard's ring into the mass's
    /// face set — two disjoint rings in one solid, whose Euler characteristic would then be the sum
    /// of two zeros and would read `0` for exactly the wrong reason. Every other surface is the
    /// mass's own, and a reveal reads the loop it was cut into rather than the mass it sits in, which
    /// is the same rule applied to the one surface that can be on either.
    pub fn solid(&self, mass: MassId) -> SolidId {
        match self {
            Self::VoidWall { void, .. }
            | Self::VoidInner { void, .. }
            | Self::VoidTop { void, .. }
            | Self::VoidBase { void } => SolidId::Void { mass, void: *void },
            Self::Reveal { loop_of, .. } => match loop_of {
                crate::mass::AuthoredLoop::Outline { mass } => SolidId::Mass(*mass),
                crate::mass::AuthoredLoop::Void { mass, void } => SolidId::Void {
                    mass: *mass,
                    void: *void,
                },
            },
            _ => SolidId::Mass(mass),
        }
    }
}

/// Which surface of a roof a face belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RoofSurface {
    /// The upper face of one strip of a plane, between two purlins.
    ///
    /// # Invariants
    ///
    /// **The strip is the shading group, not the plane.** A plane with a
    /// [`profile`](crate::roof::RoofPlane::profile) bends at every purlin, so its strips are not
    /// coplanar and smoothing across them would draw a curve where the building has a crease. `step`
    /// is the strip's index counting from the eave, so `step = 0` is the strip between the eave and
    /// the first purlin, and a plane with no profile has exactly one strip, `step = 0`.
    Skin {
        /// Index of the plane.
        plane: usize,
        /// Index of the strip along the slope, counting from the eave.
        step: usize,
    },
    /// The underside of one strip of a plane, one thickness below the skin.
    Soffit {
        /// Index of the plane.
        plane: usize,
        /// Index of the strip along the slope, counting from the eave.
        step: usize,
    },
    /// The band around the skin's boundary: one quad per boundary edge of the skin, extruded down
    /// by the roof's thickness.
    ///
    /// # Invariants
    ///
    /// The band is derived from the **skin's own boundary** rather than from the eave loop, which is
    /// what closes a gable's ends as well as its eaves with one rule and no case analysis. `edge` is
    /// therefore a position in that boundary's sorted edge list — **not** an index into
    /// [`Roof::eaves`](crate::roof::Roof), which it equals only for a roof whose whole boundary is
    /// eave.
    Fascia {
        /// Position in the skin's sorted boundary-edge list.
        edge: usize,
    },
}

/// The authored element a face came from: its provenance, and its shading group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Element {
    /// A face of a mass.
    Mass(MassId, MassSurface),
    /// A face of a roof.
    Roof(RoofId, RoofSurface),
    /// A face of a part: which placement stands it, and what it was inside its template.
    ///
    /// # Invariants
    ///
    /// The two halves answer the two questions a part's face can be asked. `placement` is the index of
    /// the placement in the document, which is also the index its [`SolidId::Part`] carries, so a face
    /// traces back to the authored placement rather than to the template it happens to be a copy of —
    /// the identity a scene needs when the same door leaf stands in four doorways. `surface` is the
    /// **template's own** element, unchanged, so a face still knows whether it is a wall, a slab, or a
    /// reveal, and a report can say "the second leaf's inner wall" without a second vocabulary.
    Part {
        /// The placement, by its index in the document's placements.
        placement: usize,
        /// What the face was inside the template.
        surface: PartSurface,
    },
    /// A face of a wall.
    Wall(crate::wall::WallId, crate::wall::WallSurface),
    /// A face of a slab.
    Slab(crate::slab::SlabId, crate::slab::SlabSurface),
    /// A face of a member: one straight piece of frame, which is the crate's sixth kind of solid.
    ///
    /// # Invariants
    ///
    /// **It is not a wall with a different elevation.** A wall is masonry: it stands on the ground its
    /// two ends carry and its top is level, so its geometry is the site's. A member is placed in the
    /// air between two [`Anchor`](crate::member::Anchor)s and its axis is any direction, so its two
    /// ends answer to what it joins rather than to the ground — and a post, a bracket and a purlin are
    /// one kind because that is the whole of what they are.
    Member(crate::member::MemberId, crate::member::MemberSurface),
    /// A face of one roll of a roof's covering.
    ///
    /// # Invariants
    ///
    /// **A roll is one solid from the eave to the crest**, and the bend at a purlin is inside it: the
    /// skin bends there and the bend is a face of the *skin*, while the roll is a body whose surface is
    /// the swept prism's boundary. `roll.roof` and `roll.index` are what name it.
    Roll(crate::covering::RollId, crate::covering::RollSurface),
}

/// What a part's face was inside the template it was copied from.
///
/// It is [`Element`] without the part, which is what keeps the recursion out of the type: a template
/// is a skeleton of masses and roofs, and a template holding a part is not a thing this crate builds,
/// so one level is the whole of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PartSurface {
    /// A face of one of the template's masses.
    Mass(MassId, MassSurface),
    /// A face of one of the template's roofs.
    Roof(RoofId, RoofSurface),
    /// A face of one of the template's walls.
    ///
    /// A `Template` holds masses and roofs and no walls, so nothing builds this today; the variant
    /// exists because the map from an element to a template's own element has to be total, and a
    /// `panic!` in that map would be a crash for a type the document can express.
    Wall(crate::wall::WallId, crate::wall::WallSurface),
    /// A face of one of the template's slabs.
    ///
    /// A `Template` holds masses and roofs and no slabs, so nothing builds this today; it exists for the
    /// same reason as the wall variant above — the map from an element to a template's own element has to
    /// be total, and a `panic!` there would be a crash for a type the document can express.
    Slab(crate::slab::SlabId, crate::slab::SlabSurface),
    /// A face of one of the template's members, which no template holds today for the reason the wall
    /// and slab variants give: the map has to be total.
    Member(crate::member::MemberId, crate::member::MemberSurface),
    /// A face of one of the template's covering rolls, for the same reason.
    Roll(crate::covering::RollId, crate::covering::RollSurface),
}

impl Element {
    /// Returns the solid this element belongs to.
    pub fn solid(&self) -> SolidId {
        match self {
            Self::Mass(id, surface) => surface.solid(*id),
            Self::Roof(id, _) => SolidId::Roof(*id),
            Self::Part { placement, .. } => SolidId::Part(*placement),
            Self::Wall(id, _) => SolidId::Wall(*id),
            Self::Slab(id, _) => SolidId::Slab(*id),
            Self::Member(id, _) => SolidId::Member(*id),
            Self::Roll(id, _) => SolidId::Roll(*id),
        }
    }
}

/// One closed solid the mesh is made of.
///
/// # Invariants
///
/// One variant per kind of body a document holds, and the index is that body's own index in the
/// document's list of its kind. A **part** is a solid of its own rather than a surface of its host's:
/// it is closed, it is oriented outward, and the two-level watertightness rule reads it exactly as it
/// reads a mass — which is what lets a door leaf stand in a hole without the wall and the leaf having
/// to agree about a single vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SolidId {
    /// A mass.
    Mass(MassId),
    /// **The wall ring one of a mass's voids is built as**: the courtyard itself, closed.
    ///
    /// # Invariants
    ///
    /// **It is a kind of its own because a solid is one connected closed surface, and a mass with a
    /// courtyard is two of them.** The alternative — folding the ring into [`Mass`](Self::Mass) —
    /// was measured to be green for the wrong reason: the mass's own ring reads Euler `0` and the
    /// courtyard's reads `0`, so one face set holding both sums to `0`, which is exactly what the
    /// check demands of a single ring. Every predicate still passes — each edge is used twice,
    /// there is no boundary, the signed volume is two positive volumes — so the closure checks
    /// would keep passing on a solid that is not a solid, and `SolidNotASphere`'s number would stop
    /// meaning what it says. With the ring its own solid the arithmetic is per-solid again: one
    /// ring is one handle, `2 − 2×1 = 0`, which is what the courtyard reports.
    ///
    /// `void` is the void's own index in [`Mass::voids`](crate::mass::Mass::voids), and `mass` is the
    /// mass that authored it, so a void is named the way its author named it — by two indices
    /// rather than by a new document-level list.
    Void {
        /// The mass whose void this ring stands in.
        mass: MassId,
        /// Which of that mass's voids, in the order it authored them.
        void: usize,
    },
    /// A roof.
    Roof(RoofId),
    /// A part, by its placement's index in the document's placements.
    Part(usize),
    /// A wall, by its index in the document's walls.
    Wall(crate::wall::WallId),
    /// A slab, by its index in the document's slabs.
    Slab(crate::slab::SlabId),
    /// A member, by its index in the document's members: one straight piece of frame, closed and
    /// oriented outward like every other solid.
    Member(crate::member::MemberId),
    /// One roll of a roof's covering, which is the crate's seventh kind of solid.
    ///
    /// It is a solid rather than a surface of the roof because **rolls do not touch each other** — they
    /// stand a period apart with a channel between them — and because a roll that crosses a purlin is
    /// still one body, its bend a section rather than a face.
    Roll(crate::covering::RollId),
}

/// One triangle, and the element that produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildingFace {
    /// The three point indices, wound so the right-handed normal points out.
    pub indices: [u32; 3],
    /// What the face came from.
    pub element: Element,
}

/// A building as geometry: welded points, triangles, and each triangle's provenance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildingMesh {
    /// The welded points.
    pub points: Vec<Vec3>,
    /// The triangles.
    pub faces: Vec<BuildingFace>,
}

impl BuildingMesh {
    /// Builds an empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a face's three corners.
    pub fn triangle(&self, face: &BuildingFace) -> [Vec3; 3] {
        face.indices.map(|index| self.points[index as usize])
    }

    /// Returns a face's normal, pointing out of the solid it belongs to.
    pub fn normal(&self, face: &BuildingFace) -> Vec3 {
        let [a, b, c] = self.triangle(face);
        crate::math::normalized3(cross3(subtract(b, a), subtract(c, a)))
    }

    /// Returns the box the mesh stands in, as (lowest, highest) corners.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for point in &self.points {
            for axis in 0..3 {
                low[axis] = low[axis].min(point[axis]);
                high[axis] = high[axis].max(point[axis]);
            }
        }
        (low, high)
    }

    /// Returns the box one solid stands in, as (lowest, highest) corners.
    ///
    /// # Invariants
    ///
    /// It reads the points the solid's **own faces** name, not the mesh's whole point list, which is
    /// the difference that matters as soon as a document holds more than one body: a part's box
    /// against its host's says where the part was placed, and the mesh's own box says nothing about
    /// either. `O(faces of that solid)`, the same pass the other `_of` readings make.
    pub fn bounds_of(&self, solid: SolidId) -> (Vec3, Vec3) {
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for face in self.faces_of(solid) {
            for corner in self.faces[face].indices {
                let point = self.points[corner as usize];
                for axis in 0..3 {
                    low[axis] = low[axis].min(point[axis]);
                    high[axis] = high[axis].max(point[axis]);
                }
            }
        }
        (low, high)
    }

    /// Returns the signed plan area of the whole mesh.
    ///
    /// # Invariants
    ///
    /// **On a closed mesh this is identically zero**, and that is a fact about closed surfaces rather
    /// than a defect: every face's plan projection is cancelled by another face's, so the sum of the
    /// directed plan areas vanishes — the discrete form of the divergence theorem's boundary term.
    /// It is therefore a check on a **subset** ([`plan_area_of`](BuildingMesh::plan_area_of): a roof's
    /// skin against its eave loop, the plan-facing faces of a mass) and never a statement about a
    /// whole solid. Measured 2026-10-16: every solid the census prints reports `0.0000`, and all that
    /// says is that the solid is closed.
    pub fn plan_area(&self) -> f32 {
        self.plan_area_of(&(0..self.faces.len()).collect::<Vec<_>>())
    }

    /// Returns the signed plan area of a set of faces.
    ///
    /// # Invariants
    ///
    /// The sign carries the winding: faces wound to look up contribute positively and faces wound
    /// down negatively. This is what makes it a check — over a subset the author meant to **tile** a
    /// region (a roof's skin over its eave loop), a total that does not match the authored area means
    /// a face is missing, doubled, or wound the other way. That comparison is the design's
    /// *coverage*, and it is [`validate`](crate::validate)'s business rather than this function's.
    pub fn plan_area_of(&self, faces: &[usize]) -> f32 {
        let mut total = 0.0;
        for index in faces {
            let [a, b, c] = self.triangle(&self.faces[*index]);
            total += crate::math::signed_area([a[0], a[2]], [b[0], b[2]], [c[0], c[2]]);
        }
        total
    }

    /// Returns the signed volume enclosed by a set of faces.
    ///
    /// # Invariants
    ///
    /// **Positive means the faces point out of the solid.** The divergence theorem over the closed
    /// surface, so a set of faces that is not closed gives a number that means nothing; the closure
    /// itself is [`boundary_loops_of`](BuildingMesh::boundary_loops_of)'s business and the two are
    /// read together. This is the check that turns an inside-out building into a report instead of
    /// a frame in which every wall is culled.
    pub fn signed_volume_of(&self, faces: &[usize]) -> f32 {
        let mut total = 0.0;
        for index in faces {
            let [a, b, c] = self.triangle(&self.faces[*index]);
            total += dot3(a, cross3(b, c));
        }
        total / 6.0
    }

    /// Returns the directed edges of a face set that have no opposite.
    ///
    /// An empty answer means the set is closed. The edges are directed the way their face winds,
    /// so a caller can walk a boundary rather than only count it.
    pub fn boundary_edges_of(&self, faces: &[usize]) -> Vec<(u32, u32)> {
        let mut used: HashMap<(u32, u32), usize> = HashMap::new();
        for index in faces {
            for edge in edges_of(&self.faces[*index].indices) {
                *used.entry(edge).or_insert(0) += 1;
            }
        }
        let mut boundary = Vec::new();
        for (edge, count) in &used {
            let opposite = (edge.1, edge.0);
            let matched = used.get(&opposite).copied().unwrap_or(0);
            let unmatched = count.saturating_sub(matched);
            for _ in 0..unmatched {
                boundary.push(*edge);
            }
        }
        boundary.sort_unstable();
        boundary
    }

    /// Returns how many closed loops a face set's boundary falls into; zero means it is closed.
    pub fn boundary_loops_of(&self, faces: &[usize]) -> usize {
        let boundary = self.boundary_edges_of(faces);
        if boundary.is_empty() {
            return 0;
        }
        let mut parent: HashMap<u32, u32> = HashMap::new();
        for (from, to) in &boundary {
            parent.entry(*from).or_insert(*from);
            parent.entry(*to).or_insert(*to);
        }
        fn find(parent: &mut HashMap<u32, u32>, index: u32) -> u32 {
            let mut root = index;
            while parent[&root] != root {
                root = parent[&root];
            }
            let mut walk = index;
            while parent[&walk] != root {
                let next = parent[&walk];
                parent.insert(walk, root);
                walk = next;
            }
            root
        }
        let pairs: Vec<(u32, u32)> = boundary.clone();
        for (from, to) in pairs {
            let (a, b) = (find(&mut parent, from), find(&mut parent, to));
            parent.insert(a, b);
        }
        let keys: Vec<u32> = parent.keys().copied().collect();
        let roots: std::collections::BTreeSet<u32> = keys
            .into_iter()
            .map(|index| find(&mut parent, index))
            .collect();
        roots.len()
    }

    /// Returns `V - E + F` over a set of faces.
    ///
    /// # Invariants
    ///
    /// Euler's characteristic is only meaningful on a closed surface that shares its vertices, so
    /// this reads the **welded** indices and not the corners a face happens to list. A sphere is
    /// `2`: every closed building solid is one, and a value that is not 2 means either a hole
    /// (`1` for a disc), a handle, or unwelded seams.
    pub fn euler_characteristic_of(&self, faces: &[usize]) -> i64 {
        let mut vertices: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        let mut edges: std::collections::BTreeSet<(u32, u32)> = std::collections::BTreeSet::new();
        for index in faces {
            let corners = self.faces[*index].indices;
            for corner in corners {
                vertices.insert(corner);
            }
            for (from, to) in edges_of(&corners) {
                edges.insert(if from < to { (from, to) } else { (to, from) });
            }
        }
        vertices.len() as i64 - edges.len() as i64 + faces.len() as i64
    }

    /// Returns whether every edge of a face set is used by exactly two of its faces.
    ///
    /// # Invariants
    ///
    /// This is **edge-manifoldness**, and it is all this function claims: an edge shared by three
    /// or more faces, or by one, fails. A vertex whose faces form more than one fan — two solids
    /// meeting at a point — passes, and is not what a building's checks rest on.
    pub fn is_manifold_of(&self, faces: &[usize]) -> bool {
        let mut used: HashMap<(u32, u32), usize> = HashMap::new();
        for index in faces {
            for (from, to) in edges_of(&self.faces[*index].indices) {
                let key = if from < to { (from, to) } else { (to, from) };
                *used.entry(key).or_insert(0) += 1;
            }
        }
        used.values().all(|count| *count == 2)
    }

    /// Returns the face indices of one solid.
    pub fn faces_of(&self, solid: SolidId) -> Vec<usize> {
        self.faces
            .iter()
            .enumerate()
            .filter_map(|(index, face)| (face.element.solid() == solid).then_some(index))
            .collect()
    }

    /// Returns a solid's signed volume; positive means it points outward.
    pub fn solid_signed_volume(&self, solid: SolidId) -> f32 {
        self.signed_volume_of(&self.faces_of(solid))
    }

    /// Returns a solid's signed plan area.
    pub fn solid_plan_area(&self, solid: SolidId) -> f32 {
        self.plan_area_of(&self.faces_of(solid))
    }

    /// Returns how many boundary loops a solid has; zero means it is closed.
    pub fn solid_boundary_loops(&self, solid: SolidId) -> usize {
        self.boundary_loops_of(&self.faces_of(solid))
    }

    /// Returns a solid's Euler characteristic; a closed solid is 2.
    pub fn solid_euler_characteristic(&self, solid: SolidId) -> i64 {
        self.euler_characteristic_of(&self.faces_of(solid))
    }

    /// Returns whether a solid is edge-manifold.
    pub fn solid_is_manifold(&self, solid: SolidId) -> bool {
        self.is_manifold_of(&self.faces_of(solid))
    }

    /// Returns per-corner normals, smooth within an authored surface and hard between them.
    pub fn corner_normals(&self) -> CornerNormals {
        CornerNormals::of(self)
    }
}

/// The three directed edges of a triangle, in its winding order.
fn edges_of(corners: &[u32; 3]) -> [(u32, u32); 3] {
    [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[0]),
    ]
}

/// Per-corner normals, grouped by the authored surface each corner belongs to.
///
/// # Invariants
///
/// The grouping key is the face's [`Element`], so two facets of one roof plane share their normals
/// at a shared vertex and a facet of the next plane does not: the crease falls where the author put
/// an edge, which is what makes a roof read as planes rather than as a faceted sheet. Accumulation
/// is area-weighted — an unweighted mean of a fan of slivers would be dominated by the smallest
/// triangle it happens to hold.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CornerNormals {
    per_face: Vec<[Vec3; 3]>,
}

impl CornerNormals {
    /// Builds the per-corner normals of a mesh.
    pub fn of(mesh: &BuildingMesh) -> Self {
        let mut accum: HashMap<(u32, Element), Vec3> = HashMap::new();
        for face in &mesh.faces {
            let [a, b, c] = mesh.triangle(face);
            let normal = cross3(subtract(b, a), subtract(c, a));
            for index in face.indices {
                let entry = accum.entry((index, face.element)).or_insert([0.0; 3]);
                entry[0] += normal[0];
                entry[1] += normal[1];
                entry[2] += normal[2];
            }
        }
        let per_face = mesh
            .faces
            .iter()
            .map(|face| {
                face.indices.map(|index| {
                    let sum = accum
                        .get(&(index, face.element))
                        .copied()
                        .unwrap_or([0.0, 1.0, 0.0]);
                    crate::math::normalized3(sum)
                })
            })
            .collect();
        Self { per_face }
    }

    /// Returns the normal at one corner of one face.
    pub fn at(&self, face: usize, corner: usize) -> Vec3 {
        self.per_face[face][corner.min(2)]
    }

    /// Returns how many faces the normals cover.
    pub fn len(&self) -> usize {
        self.per_face.len()
    }

    /// Returns whether there are no normals.
    pub fn is_empty(&self) -> bool {
        self.per_face.is_empty()
    }
}
